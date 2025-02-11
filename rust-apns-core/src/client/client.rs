//! The client module for sending requests and parsing responses

use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{self, Body, Client as HttpClient, StatusCode};
use hyper_alpn::AlpnConnector;
use serde::Serialize;
use std::fmt;
use std::io::Read;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    error::Error::{self, ResponseError},
    request::{payload::Payload, Request},
    response::response::Response,
    response::Result,
};

use super::{endpoint::Endpoint, signer::Signer};

/// Default user agent.
pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Handles requests to and responses from Apple Push Notification service.
/// Connects using a given connector. Handles the needed authentication and
/// maps responses.
///
/// The `send` method returns a future, which is successful when APNs receives
/// the notification and responds with a status OK. In any other case the future
/// fails. If APNs gives a reason for the failure, the returned `Err`
/// holds the response for handling.
#[derive(Debug, Clone)]
pub struct Client {
    endpoint: Endpoint,
    signer: Option<Signer>,
    http_client: HttpClient<AlpnConnector>,
}

impl Client {
    fn new(connector: AlpnConnector, signer: Option<Signer>, endpoint: Endpoint) -> Client {
        let mut builder = HttpClient::builder();
        builder.pool_idle_timeout(Some(Duration::from_secs(600)));
        builder.http2_only(true);

        Client {
            http_client: builder.build(connector),
            signer,
            endpoint,
        }
    }

    /// Create a connection to APNs using the provider client certificate which
    /// you obtain from your [Apple developer
    /// account](https://developer.apple.com/account/).
    ///
    /// Only works with the `openssl` feature.
    #[cfg(feature = "openssl")]
    pub fn certificate<R>(certificate: &mut R, password: &str, endpoint: Endpoint) -> Result<Client, Error>
    where
        R: Read,
    {
        let mut cert_der: Vec<u8> = Vec::new();
        certificate.read_to_end(&mut cert_der)?;

        let pkcs = openssl::pkcs12::Pkcs12::from_der(&cert_der)?.parse(password)?;
        let connector = AlpnConnector::with_client_cert(&pkcs.cert.to_pem()?, &pkcs.pkey.private_key_to_pem_pkcs8()?)?;

        Ok(Self::new(connector, None, endpoint))
    }

    /// Create a connection to APNs using system certificates, signing every
    /// request with a signature using a private key, key id and team id
    /// provisioned from your [Apple developer
    /// account](https://developer.apple.com/account/).
    pub fn token<S, T, R>(pkcs8_pem: R, key_id: S, team_id: T, endpoint: Endpoint) -> Result<Client, Error>
    where
        S: Into<String>,
        T: Into<String>,
        R: Read,
    {
        let connector = AlpnConnector::new();
        let signature_ttl = Duration::from_secs(60 * 55);
        let signer = Signer::new(pkcs8_pem, key_id, team_id, signature_ttl)?;

        Ok(Self::new(connector, Some(signer), endpoint))
    }

    /// Send a notification payload.
    ///
    /// See [ErrorReason](enum.ErrorReason.html) for possible errors.
    #[cfg_attr(feature = "tracing", ::tracing::instrument)]
    pub async fn send<T>(&self, req: Request<T>) -> Result<Response, Error>
    where
        T: Serialize,
    {
        let request = self.build_request(req).unwrap();
        let requesting = self.http_client.request(request);

        let response = requesting.await?;

        let apns_id = response
            .headers()
            .get("apns-id")
            .and_then(|s| s.to_str().ok())
            .map(String::from);

        match response.status() {
            StatusCode::OK => Ok(Response {
                apns_id,
                error: None,
                code: response.status().as_u16(),
            }),
            status => {
                let body = hyper::body::to_bytes(response).await?;

                Err(ResponseError(Response {
                    apns_id,
                    error: serde_json::from_slice(&body).ok(),
                    code: status.as_u16(),
                }))
            }
        }
    }

    fn build_request<T>(&self, req: Request<T>) -> Result<hyper::Request<Body>>
    where
        T: Serialize,
    {
        let path = self.endpoint.as_url().join(&req.device_token)?.to_string();
        let (payload_headers, payload): (_, Payload<T>) = req.try_into()?;

        let mut builder = hyper::Request::builder().uri(&path).method("POST");

        let headers = builder.headers_mut().unwrap();
        headers.extend(payload_headers);

        //let payload_size_limit = req.push_type.payload_size_limit();

        let body = serde_json::to_vec(&payload)?;
        let request_body = Body::from(body);

        Ok(builder.body(request_body).unwrap())

        // let body = serde_json::to_vec(&payload)?;
        // if body.len() > payload_size_limit {
        //     return Err(Error::PayloadTooLarge {
        //         size: body.len(),
        //         limit: payload_size_limit,
        //     });
        // }

        // let mut req = self.client.post(url).body(body);
        // for (name, value) in headers {
        //     if let Some(name) = name {
        //         req = req.header(name, value);
        //     }
        // }

        // Ok(Uuid::new_v4())

        //     #[cfg(feature = "jwt")]
        //     if let Some(token_factory) = &self.token_factory {
        //         let jwt = token_factory.get()?;
        //         req = req.bearer_auth(jwt);
        //     }

        //     let res = req.send().await?;

        //     if let Err(err) = res.error_for_status_ref() {
        //         if let Ok(reason) = res.json::<Reason>().await {
        //             Err(reason.into())
        //         } else {
        //             Err(err.into())
        //         }
        //     } else {
        //         let apns_id = res
        //             .headers()
        //             .get(&APNS_ID)
        //             .and_then(|v| v.to_str().ok())
        //             .and_then(|s| s.parse().ok())
        //             .unwrap_or_default();
        //         Ok(apns_id)
        //     }

    //     if let Some(ref signer) = self.signer {
    //         let auth = signer
    //             .with_signature(|signature| format!("Bearer {}", signature))
    //             .unwrap();

    //         builder = builder.header(AUTHORIZATION, auth.as_bytes());
    //     }
    }


}

#[cfg(test)]
mod tests {
    use crate::notification::{AlertNotificationBuilder, PushNotification};

    use super::*;
    use uuid::Uuid;
    // use crate::request::notification::AlertNotification;
    // use crate::request::notification::{CollapseId, NotificationOptions, Priority};
    // use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
    // use hyper::Method;
    // use hyper_alpn::AlpnConnector;

    const PRIVATE_KEY: &'static str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg8g/n6j9roKvnUkwu
lCEIvbDqlUhA5FOzcakkG90E8L+hRANCAATKS2ZExEybUvchRDuKBftotMwVEus3
jDwmlD1Gg0yJt1e38djFwsxsfr5q2hv0Rj9fTEqAPr8H7mGm0wKxZ7iQ
-----END PRIVATE KEY-----";

    #[test]
    fn test_production_request_uri() {
        let builder = PushNotification::Alert(AlertNotificationBuilder::default().build().unwrap());
        let payload = builder.build_request(None, None, String::from("a_test_id"), Uuid::new_v4()).unwrap();
        let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
        let request = client.build_request(payload).unwrap();
        let uri = format!("{}", request.uri());

        assert_eq!("https://api.push.apple.com/3/device/a_test_id", &uri);
    }

    //     #[test]
    //     fn test_sandbox_request_uri() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Sandbox);
    //         let request = client.build_request(payload);
    //         let uri = format!("{}", request.uri());

    //         assert_eq!("https://api.development.push.apple.com/3/device/a_test_id", &uri);
    //     }

    //     #[test]
    //     fn test_request_method() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);

    //         assert_eq!(&Method::POST, request.method());
    //     }

    //     #[test]
    //     fn test_request_content_type() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);

    //         assert_eq!("application/json", request.headers().get(CONTENT_TYPE).unwrap());
    //     }

    //     #[test]
    //     fn test_request_content_length() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload.clone());
    //         let payload_json = payload.to_json_string().unwrap();
    //         let content_length = request.headers().get(CONTENT_LENGTH).unwrap().to_str().unwrap();

    //         assert_eq!(&format!("{}", payload_json.len()), content_length);
    //     }

    //     #[test]
    //     fn test_request_authorization_with_no_signer() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);

    //         assert_eq!(None, request.headers().get(AUTHORIZATION));
    //     }

    //     #[test]
    //     fn test_request_authorization_with_a_signer() {
    //         let signer = Signer::new(
    //             PRIVATE_KEY.as_bytes(),
    //             "89AFRD1X22",
    //             "ASDFQWERTY",
    //             Duration::from_secs(100),
    //         )
    //         .unwrap();

    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), Some(signer), Endpoint::Production);
    //         let request = client.build_request(payload);

    //         assert_ne!(None, request.headers().get(AUTHORIZATION));
    //     }

    //     #[test]
    //     fn test_request_with_default_priority() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_priority = request.headers().get("apns-priority");

    //         assert_eq!(None, apns_priority);
    //     }

    //     #[test]
    //     fn test_request_with_normal_priority() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build(
    //             "a_test_id",
    //             NotificationOptions {
    //                 apns_priority: Some(Priority::Normal),
    //                 ..Default::default()
    //             },
    //         );

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_priority = request.headers().get("apns-priority").unwrap();

    //         assert_eq!("5", apns_priority);
    //     }

    //     #[test]
    //     fn test_request_with_high_priority() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build(
    //             "a_test_id",
    //             NotificationOptions {
    //                 apns_priority: Some(Priority::High),
    //                 ..Default::default()
    //             },
    //         );

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_priority = request.headers().get("apns-priority").unwrap();

    //         assert_eq!("10", apns_priority);
    //     }

    //     #[test]
    //     fn test_request_with_default_apns_id() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build("a_test_id", Default::default());

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_id = request.headers().get("apns-id");

    //         assert_eq!(None, apns_id);
    //     }

    //     #[test]
    //     fn test_request_with_an_apns_id() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build(
    //             "a_test_id",
    //             NotificationOptions {
    //                 apns_id: Some("a-test-apns-id"),
    //                 ..Default::default()
    //             },
    //         );

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_id = request.headers().get("apns-id").unwrap();

    //         assert_eq!("a-test-apns-id", apns_id);
    //     }

    //     #[test]
    //     fn test_request_with_default_apns_expiration() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build("a_test_id", Default::default());

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_expiration = request.headers().get("apns-expiration");

    //         assert_eq!(None, apns_expiration);
    //     }

    //     #[test]
    //     fn test_request_with_an_apns_expiration() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build(
    //             "a_test_id",
    //             NotificationOptions {
    //                 apns_expiration: Some(420),
    //                 ..Default::default()
    //             },
    //         );

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_expiration = request.headers().get("apns-expiration").unwrap();

    //         assert_eq!("420", apns_expiration);
    //     }

    //     #[test]
    //     fn test_request_with_default_apns_collapse_id() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build("a_test_id", Default::default());

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_collapse_id = request.headers().get("apns-collapse-id");

    //         assert_eq!(None, apns_collapse_id);
    //     }

    //     #[test]
    //     fn test_request_with_an_apns_collapse_id() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build(
    //             "a_test_id",
    //             NotificationOptions {
    //                 apns_collapse_id: Some(CollapseId::new("a_collapse_id").unwrap()),
    //                 ..Default::default()
    //             },
    //         );

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_collapse_id = request.headers().get("apns-collapse-id").unwrap();

    //         assert_eq!("a_collapse_id", apns_collapse_id);
    //     }

    //     #[test]
    //     fn test_request_with_default_apns_topic() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build("a_test_id", Default::default());

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_topic = request.headers().get("apns-topic");

    //         assert_eq!(None, apns_topic);
    //     }

    //     #[test]
    //     fn test_request_with_an_apns_topic() {
    //         let builder = DefaultNotificationBuilder::new();

    //         let payload = builder.build(
    //             "a_test_id",
    //             NotificationOptions {
    //                 apns_topic: Some("a_topic"),
    //                 ..Default::default()
    //             },
    //         );

    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload);
    //         let apns_topic = request.headers().get("apns-topic").unwrap();

    //         assert_eq!("a_topic", apns_topic);
    //     }

    //     #[tokio::test]
    //     async fn test_request_body() {
    //         let builder = DefaultNotificationBuilder::new();
    //         let payload = builder.build("a_test_id", Default::default());
    //         let client = Client::new(AlpnConnector::new(), None, Endpoint::Production);
    //         let request = client.build_request(payload.clone());

    //         let body = hyper::body::to_bytes(request).await.unwrap();
    //         let body_str = String::from_utf8(body.to_vec()).unwrap();

    //         assert_eq!(payload.to_json_string().unwrap(), body_str,);
    //     }
}

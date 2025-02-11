use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use tokio;
use uuid::Uuid;

use rust_apns_core::{
    client::{client::Client, Endpoint},
    notification::AlertNotificationBuilder,
    notification::PushNotification,
};

// An example client connectiong to APNs with a certificate and key
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();

    let mut certificate_file = String::new();
    let mut password = String::new();
    let mut device_token = String::new();
    let mut message = String::from("Ch-check it out!");
    let mut sandbox = false;
    let mut topic: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("APNs certificate-based push");
        ap.refer(&mut certificate_file)
            .add_option(&["-c", "--certificate"], Store, "Certificate PKCS12 file location");
        ap.refer(&mut password)
            .add_option(&["-p", "--password"], Store, "Certificate password");
        ap.refer(&mut device_token)
            .add_option(&["-d", "--device_token"], Store, "APNs device token");
        ap.refer(&mut message)
            .add_option(&["-m", "--message"], Store, "Notification message");
        ap.refer(&mut sandbox)
            .add_option(&["-s", "--sandbox"], StoreTrue, "Use the development APNs servers");
        ap.refer(&mut topic)
            .add_option(&["-o", "--topic"], StoreOption, "APNS topic");
        ap.parse_args_or_exit();
    }

    // Connecting to APNs using a client certificate
    let new_client = || -> Result<Client, Box<dyn std::error::Error + Sync + Send>> {
        #[cfg(feature = "openssl")]
        {
            // Which service to call, test or production?
            let endpoint = if sandbox {
                Endpoint::Development
            } else {
                Endpoint::Production
            };

            let mut certificate = std::fs::File::open(certificate_file)?;
            Ok(Client::certificate(&mut certificate, &password, endpoint)?)
        }
        #[cfg(all(not(feature = "openssl"), feature = "ring"))]
        {
            Err("ring does not support loading of certificates".into())
        }
    };
    let client = new_client()?;

    // Notification payload
    let alert = PushNotification::Alert(
        AlertNotificationBuilder::default()
            .body(message)
            .sound("default")
            .badge(1u32)
            .build()
            .unwrap(),
    );

    let payload = alert.build_request(topic, None, device_token, Uuid::new_v4()).unwrap();
    let response = client.send(payload).await?;

    println!("Sent: {:?}", response);

    Ok(())
}

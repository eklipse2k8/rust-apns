use derive_builder::Builder;
use serde_json::Value;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    request::{collapse::CollapseId, Request, Sound},
    Error,
};

/// Data-only notification.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataNotification(Value);

impl DataNotification {
    pub fn new(value: Value) -> Self {
        Self(value)
    }
}

/// Alert notification. (requires user's permission)
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[builder(setter(into, strip_option), default, build_fn(error = "Error"))]
pub struct AlertNotification {
    pub title: Option<String>,
    pub body: Option<String>,
    pub sound: Option<String>,
    pub badge: Option<u32>,
}

/// Push notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PushNotification {
    Data(DataNotification),
    Alert(AlertNotification),
}

impl PushNotification {
    pub fn build_request(
        self,
        topic: Option<String>,
        collapse_id: Option<CollapseId>,
        device_token: String,
        uid: Uuid,
    ) -> Result<Request<Value>, Error> {
        match self {
            PushNotification::Data(data) => Ok(Request::<Value> {
                device_token: device_token,
                push_type: crate::client::PushType::Background,
                id: Some(uid),
                expiration: Some(OffsetDateTime::now_utc() + Duration::days(1)),
                priority: crate::client::Priority::ConsiderPower,
                topic: topic,
                collapse_id: collapse_id.map(|c| c.value.to_string()),
                content_available: true,
                user_info: Some(data.0),
                ..Default::default()
            }),
            PushNotification::Alert(alert) => Ok(Request::<Value> {
                device_token: device_token,
                push_type: crate::client::PushType::Alert,
                id: Some(uid),
                expiration: Some(OffsetDateTime::now_utc() + Duration::days(1)),
                priority: crate::client::Priority::ConsiderPower,
                topic: topic,
                collapse_id: collapse_id.map(|c| c.value.to_string()),
                badge: alert.badge,
                sound: alert.sound.map(Sound::from),
                alert: Some(crate::request::Alert {
                    title: alert.title,
                    body: alert.body,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    }
}

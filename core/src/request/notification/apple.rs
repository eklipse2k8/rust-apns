use derive_builder::{Builder, UninitializedFieldError};
use serde_json::Value;
use thiserror::Error;

use crate::request::payload::Payload;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("missing required field: {0}")]
    BuilderMissingField(String),
}

impl From<UninitializedFieldError> for TypeError {
    fn from(err: UninitializedFieldError) -> Self {
        Self::BuilderMissingField(err.field_name().to_string())
    }
}

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
#[builder(setter(into, strip_option), default, build_fn(error = "TypeError"))]
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct APS {
    alert: Option<APSAlert<'a>>,
    badge: Option<u32>,
    sound: Option<String>,
    content_available: bool,
    mutable_content: bool,
}

impl<'a> Into<Payload<'a>> for PushNotification {
    fn into(self) -> Payload<'a> {
        match self {
            PushNotification::Data(silent) => Payload {
                topic: Some(APNS_TOPIC.to_string()),
                content_available: true,
                priority: apple_apns::Priority::ConsiderPower,
                user_info: Some(silent.0),
                ..Default::default()
            },
            PushNotification::Alert(alert) => Payload {
                topic: Some(APNS_TOPIC.to_string()),
                content_available: false,
                priority: apple_apns::Priority::ConsiderPower,
                alert: Some(apple_apns::Alert {
                    title: Some(alert.title),
                    body: Some(alert.body),
                    ..Default::default()
                }),
                sound: alert.sound.map(|s| apple_apns::Sound {
                    name: s,
                    ..Default::default()
                }),
                badge: alert.badge,
                user_info: None,
                ..Default::default()
            },
        }
    }
}

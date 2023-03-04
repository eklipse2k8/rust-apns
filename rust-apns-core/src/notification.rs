use derive_builder::Builder;
use serde_json::Value;

use crate::Error;

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

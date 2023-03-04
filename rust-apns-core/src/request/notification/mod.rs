///! The `aps` notification content builders
mod apple;
mod options;
mod web;

pub use self::apple::{AlertNotification, AlertNotificationBuilder, DataNotification, PushNotification};
pub use self::options::{CollapseId, NotificationOptions, Priority};
pub use self::web::{WebNotificationBuilder, WebPushAlert};

use crate::request::payload::Payload;

pub trait NotificationBuilder<'a> {
    /// Generates the request payload to be send with the `Client`.
    fn build(self, device_token: &'a str, options: NotificationOptions<'a>) -> Payload<'a>;
}

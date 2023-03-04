use std::fmt;

/// The importance how fast to bring the notification for the user..
#[derive(Debug, Clone)]
pub enum Priority {
    /// Send the push message immediately. Notifications with this priority must
    /// trigger an alert, sound, or badge on the target device. Cannot be used
    /// with the silent notification.
    High,

    /// Send the push message at a time that takes into account power
    /// considerations for the device. Notifications with this priority might be
    /// grouped and delivered in bursts. They are throttled, and in some cases
    /// are not delivered.
    Normal,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let priority = match self {
            Priority::High => "10",
            Priority::Normal => "5",
        };

        write!(f, "{}", priority)
    }
}

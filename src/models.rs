use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct ClientSubscription {
    pub(crate) endpoint: String,
    pub(crate) keys: SubscriptionKeys,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct SubscriptionKeys {
    pub(crate) p256dh: String,
    pub(crate) auth: String,
}

#[derive(Deserialize)]
pub(crate) struct NotificationRequest {
    #[serde(default = "default_notification_title")]
    pub(crate) title: String,
    #[serde(default, alias = "message")]
    pub(crate) body: String,
    #[serde(default)]
    pub(crate) subscribers: Vec<ClientSubscription>,
}

#[derive(Deserialize)]
pub(crate) struct UnsubscribeRequest {
    pub(crate) endpoint: String,
}

fn default_notification_title() -> String {
    "Rust 서버 알림".to_string()
}

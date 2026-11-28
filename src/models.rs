use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};

static NEXT_DEVICE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct ClientSubscription {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default = "default_device_name")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) registered_at: u64,
    pub(crate) endpoint: String,
    pub(crate) keys: SubscriptionKeys,
}

#[derive(Serialize)]
pub(crate) struct SubscriberSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) provider: String,
    pub(crate) registered_at: u64,
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

#[derive(Deserialize)]
pub(crate) struct RenameDeviceRequest {
    pub(crate) name: String,
}

fn default_notification_title() -> String {
    "Rust 서버 알림".to_string()
}

pub(crate) fn default_device_name() -> String {
    "새 기기".to_string()
}

pub(crate) fn current_time_millis() -> Result<u64, SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

pub(crate) fn new_device_id(registered_at: u64) -> String {
    let sequence = NEXT_DEVICE_ID.fetch_add(1, Ordering::Relaxed);
    format!("{registered_at:x}-{sequence:x}")
}

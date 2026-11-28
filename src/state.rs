use crate::models::{current_time_millis, default_device_name, new_device_id, ClientSubscription};
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use tokio::sync::Mutex;

pub(crate) struct AppState {
    pub(crate) subscriptions: Mutex<Vec<ClientSubscription>>,
    pub(crate) store_path: PathBuf,
}

pub(crate) async fn load_subscriptions(
    path: &Path,
) -> Result<Vec<ClientSubscription>, Box<dyn Error + Send + Sync>> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    match tokio::fs::read(path).await {
        Ok(contents) => {
            let mut subscriptions: Vec<ClientSubscription> = serde_json::from_slice(&contents)?;
            let migration_time = current_time_millis()?;
            if migrate_subscription_metadata(&mut subscriptions, migration_time) {
                save_subscriptions(path, &subscriptions).await?;
            }
            Ok(subscriptions)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            save_subscriptions(path, &[]).await?;
            Ok(Vec::new())
        }
        Err(error) => Err(Box::new(error)),
    }
}

pub(crate) async fn save_subscriptions(
    path: &Path,
    subscriptions: &[ClientSubscription],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let contents = serde_json::to_vec_pretty(subscriptions)?;
    tokio::fs::write(path, contents).await?;
    Ok(())
}

fn migrate_subscription_metadata(
    subscriptions: &mut [ClientSubscription],
    migration_time: u64,
) -> bool {
    let mut changed = false;

    for subscription in subscriptions {
        if subscription.id.is_empty() {
            subscription.id = new_device_id(migration_time);
            changed = true;
        }
        if subscription.name.trim().is_empty() {
            subscription.name = default_device_name();
            changed = true;
        }
        if subscription.registered_at == 0 {
            subscription.registered_at = migration_time;
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::migrate_subscription_metadata;
    use crate::models::ClientSubscription;

    #[test]
    fn migrates_existing_subscriptions_and_keeps_metadata() {
        let legacy =
            r#"[{"endpoint":"https://push.example/legacy","keys":{"p256dh":"key","auth":"auth"}}]"#;
        let mut subscriptions: Vec<ClientSubscription> =
            serde_json::from_str(legacy).expect("legacy subscription should deserialize");

        assert!(migrate_subscription_metadata(
            &mut subscriptions,
            1_800_000_000_000
        ));
        let subscription = &subscriptions[0];
        assert!(!subscription.id.is_empty());
        assert_eq!(subscription.name, "새 기기");
        assert_eq!(subscription.registered_at, 1_800_000_000_000);

        assert!(!migrate_subscription_metadata(
            &mut subscriptions,
            1_800_000_100_000
        ));
        assert_eq!(subscriptions[0].registered_at, 1_800_000_000_000);
    }
}

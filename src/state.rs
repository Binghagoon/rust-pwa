use crate::models::ClientSubscription;
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
        Ok(contents) => Ok(serde_json::from_slice(&contents)?),
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

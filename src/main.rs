mod models;
mod notifications;
mod server;
mod state;
mod subscriptions;

use state::{load_subscriptions, AppState};
use std::{error::Error, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    dotenvy::dotenv().ok();
    println!("[INFO] .env 파일을 성공적으로 로드했습니다.");

    let store_path = std::env::var_os("DEVICE_STORE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("db/devices.json"));
    let subscriptions = load_subscriptions(&store_path).await?;
    let state = Arc::new(AppState {
        subscriptions: Mutex::new(subscriptions),
        store_path,
    });
    let app = server::router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("--------------------------------------------------");
    println!("[🚀 SERVER] 푸시 서버가 실행되었습니다!");
    println!("[🔗 URL] 접속 주소: http://0.0.0.0:3000");
    println!("[🔔 TEST] 알림 전송 테스트: http://0.0.0.0:3000/api/send-notification");
    println!("--------------------------------------------------");

    axum::serve(listener, app).await?;
    Ok(())
}

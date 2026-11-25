use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer; // 라우터 요청 로깅 레이어
use web_push::*;

// 브라우저가 보내준 구독 정보 구조체
#[derive(Clone, Deserialize, Serialize)]
struct ClientSubscription {
    endpoint: String,
    keys: SubscriptionKeys,
}

#[derive(Clone, Deserialize, Serialize)]
struct SubscriptionKeys {
    p256dh: String,
    auth: String,
}

#[derive(Deserialize)]
struct NotificationRequest {
    subscribers: Vec<ClientSubscription>,
    message: String,
}

// 서버 상태 (메모리 캐시와 디스크 저장 경로)
struct AppState {
    subscriptions: Mutex<Vec<ClientSubscription>>,
    store_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 1. HTTP 요청 로깅(Tracing) 초기화
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // .env 파일 로드
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

    // 2. 라우터에 TraceLayer를 추가하여 모든 HTTP 요청/응답 자동 로깅
    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/api/subscribe", post(subscribe_handler))
        .route("/api/vapid-public-key", get(vapid_public_key_handler))
        .route("/api/send-notification", get(send_notification_handler))
        .route("/notifications", post(notifications_handler))
        .route("/subscribers", get(subscribers_handler))
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
                    tracing::info!(
                        method = %request.method(),
                        uri = %request.uri(),
                        "HTTP request received"
                    );
                })
                .on_response(|response: &axum::http::Response<_>, latency, _span: &tracing::Span| {
                    tracing::info!(
                        status = %response.status(),
                        latency = ?latency,
                        "HTTP response sent"
                    );
                }),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("--------------------------------------------------");
    println!("[🚀 SERVER] 푸시 서버가 실행되었습니다!");
    println!("[🔗 URL] 접속 주소: http://0.0.0.0:3000");
    println!("[🔔 TEST] 알림 전송 테스트: http://0.0.0.0:3000/api/send-notification");
    println!("--------------------------------------------------");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn load_subscriptions(
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

async fn save_subscriptions(
    path: &Path,
    subscriptions: &[ClientSubscription],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let contents = serde_json::to_vec_pretty(subscriptions)?;
    tokio::fs::write(path, contents).await?;
    Ok(())
}

async fn vapid_public_key_handler() -> String {
    std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default()
}

async fn subscribers_handler(State(state): State<Arc<AppState>>) -> Json<Vec<ClientSubscription>> {
    let subscribers = state.subscriptions.lock().await.clone();
    Json(subscribers)
}

// 1. 클라이언트 구독 정보 저장 엔드포인트
async fn subscribe_handler(
    State(state): State<Arc<AppState>>,
    Json(sub): Json<ClientSubscription>,
) -> Result<String, (StatusCode, String)> {
    println!("\n[📥 SUBSCRIBE] 새로운 푸시 구독 요청이 접수되었습니다.");
    println!("[📌 ENDPOINT] {}", sub.endpoint);

    let mut subscriptions = state.subscriptions.lock().await;
    let mut updated = subscriptions.clone();
    updated.retain(|existing| existing.endpoint != sub.endpoint);
    updated.push(sub);

    if let Err(error) = save_subscriptions(&state.store_path, &updated).await {
        tracing::error!(%error, "Failed to persist device subscriptions");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "기기 구독 정보를 저장하지 못했습니다.".to_string(),
        ));
    }

    *subscriptions = updated;
    println!("[✅ SUCCESS] 현재 총 구독자 수: {}명", subscriptions.len());
    Ok("Subscribed successfully".to_string())
}

// 2. 저장된 구독자들에게 푸시 알림을 발송하는 엔드포인트
async fn send_notification_handler(State(state): State<Arc<AppState>>) -> String {
    let subs = state.subscriptions.lock().await.clone();
    send_notifications(subs, "서버에서 보낸 실시간 푸시 알림입니다! 🔥").await
}

async fn notifications_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NotificationRequest>,
) -> String {
    println!("\n[📤 NOTIFY] POST /notifications 요청이 들어왔습니다.");

    let subscribers = if request.subscribers.is_empty() {
        state.subscriptions.lock().await.clone()
    } else {
        request.subscribers
    };

    send_notifications(subscribers, &request.message).await
}

async fn send_notifications(subs: Vec<ClientSubscription>, message: &str) -> String {
    println!("\n[📤 NOTIFY] 알림 전송을 시작합니다.");

    // 환경 변수에서 VAPID 키 로드
    let _public_key = match std::env::var("VAPID_PUBLIC_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("[❌ ERROR] VAPID_PUBLIC_KEY 환경 변수가 설정되지 않았습니다!");
            return "VAPID_PUBLIC_KEY missing".to_string();
        }
    };
    let private_key = match std::env::var("VAPID_PRIVATE_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("[❌ ERROR] VAPID_PRIVATE_KEY 환경 변수가 설정되지 않았습니다!");
            return "VAPID_PRIVATE_KEY missing".to_string();
        }
    };
    let subject = std::env::var("VAPID_SUBJECT").unwrap_or_else(|_| {
        println!("[⚠️ WARN] VAPID_SUBJECT가 없어 기본값(mailto:admin@example.com)을 사용합니다.");
        "mailto:admin@example.com".to_string()
    });

    if subs.is_empty() {
        println!("[ℹ️ INFO] 현재 등록된 구독자가 없습니다. 브라우저에서 먼저 '구독하기'를 눌러주세요.");
        return "No subscribers found".to_string();
    }

    println!("[📊 INFO] 총 {}명의 구독자에게 알림 발송을 시작합니다.", subs.len());

    // WebPushClient 생성
    let client = match IsahcWebPushClient::new() {
        Ok(c) => {
            println!("[✅ SUCCESS] WebPushClient 생성 완료");
            c
        }
        Err(e) => {
            eprintln!("[❌ ERROR] WebPushClient 생성 실패: {:?}", e);
            return "Failed to create WebPushClient".to_string();
        }
    };

    let mut failed = 0;

    // 등록된 모든 구독자에게 알림 발송
    for (index, sub) in subs.iter().enumerate() {
        println!("\n[🔄 PROCESS] [{}/{}] 구독자 처리 중...", index + 1, subs.len());

        let sub_info = SubscriptionInfo::new(
            &sub.endpoint,
            &sub.keys.p256dh,
            &sub.keys.auth,
        );

        let mut sig_builder = match VapidSignatureBuilder::from_base64(&private_key, &sub_info) {
            Ok(builder) => builder,
            Err(e) => {
                eprintln!("[❌ ERROR] VAPID 서명 빌더 생성 실패: {:?}", e);
                continue;
            }
        };
        
        let _ = sig_builder.add_claim("sub", subject.clone());
        let vapid_keys = match sig_builder.build() {
            Ok(keys) => keys,
            Err(e) => {
                eprintln!("[❌ ERROR] VAPID 키 빌드 실패: {:?}", e);
                continue;
            }
        };

        let mut builder = WebPushMessageBuilder::new(&sub_info);

        // 보낼 알림 내용 (JSON 페이로드)
        let payload = serde_json::json!({
            "title": "Rust 서버 알림",
            "body": message
        }).to_string();

        builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
        builder.set_vapid_signature(vapid_keys);

        let message = match builder.build() {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("[❌ ERROR] 메시지 빌드 실패: {:?}", e);
                continue;
            }
        };

        match client.send(message).await {
            Ok(_) => println!("[🎉 SUCCESS] 구독자[{}]에게 푸시 알림 전송 성공!", index + 1),
            Err(e) => {
                failed += 1;
                eprintln!("[❌ ERROR] 구독자[{}]에게 푸시 알림 전송 실패: {:?}", index + 1, e);
            }
        }
    }

    println!("[🏁 FINISH] 모든 알림 전송 시도가 완료되었습니다.\n");
    format!(
        "Notification trigger attempted: {} succeeded, {} failed",
        subs.len() - failed,
        failed
    )
}
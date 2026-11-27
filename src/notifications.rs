use crate::{models::NotificationRequest, state::AppState};
use axum::{extract::State, Json};
use std::sync::Arc;
use web_push::*;

pub(crate) async fn send_notification_handler(State(state): State<Arc<AppState>>) -> String {
    let subscribers = state.subscriptions.lock().await.clone();
    send_notifications(
        subscribers,
        "Rust 서버 알림",
        "서버에서 보낸 실시간 푸시 알림입니다! 🔥",
    )
    .await
}

pub(crate) async fn notifications_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NotificationRequest>,
) -> String {
    println!("\n[📤 NOTIFY] POST /notifications 요청이 들어왔습니다.");

    let subscribers = if request.subscribers.is_empty() {
        state.subscriptions.lock().await.clone()
    } else {
        request.subscribers
    };

    send_notifications(subscribers, &request.title, &request.body).await
}

async fn send_notifications(
    subscribers: Vec<crate::models::ClientSubscription>,
    title: &str,
    body: &str,
) -> String {
    println!("\n[📤 NOTIFY] 알림 전송을 시작합니다.");

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

    if subscribers.is_empty() {
        println!(
            "[ℹ️ INFO] 현재 등록된 구독자가 없습니다. 브라우저에서 먼저 '구독하기'를 눌러주세요."
        );
        return "No subscribers found".to_string();
    }

    println!(
        "[📊 INFO] 총 {}명의 구독자에게 알림 발송을 시작합니다.",
        subscribers.len()
    );

    let client = match IsahcWebPushClient::new() {
        Ok(client) => {
            println!("[✅ SUCCESS] WebPushClient 생성 완료");
            client
        }
        Err(error) => {
            eprintln!("[❌ ERROR] WebPushClient 생성 실패: {:?}", error);
            return "Failed to create WebPushClient".to_string();
        }
    };

    let mut failed = 0;

    for (index, subscription) in subscribers.iter().enumerate() {
        println!(
            "\n[🔄 PROCESS] [{}/{}] 구독자 처리 중...",
            index + 1,
            subscribers.len()
        );

        let subscription_info = SubscriptionInfo::new(
            &subscription.endpoint,
            &subscription.keys.p256dh,
            &subscription.keys.auth,
        );

        let mut signature_builder =
            match VapidSignatureBuilder::from_base64(&private_key, &subscription_info) {
                Ok(builder) => builder,
                Err(error) => {
                    eprintln!("[❌ ERROR] VAPID 서명 빌더 생성 실패: {:?}", error);
                    continue;
                }
            };

        let _ = signature_builder.add_claim("sub", subject.clone());
        let vapid_keys = match signature_builder.build() {
            Ok(keys) => keys,
            Err(error) => {
                eprintln!("[❌ ERROR] VAPID 키 빌드 실패: {:?}", error);
                continue;
            }
        };

        let mut builder = WebPushMessageBuilder::new(&subscription_info);
        let payload = serde_json::json!({
            "title": title,
            "body": body
        })
        .to_string();

        builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
        builder.set_vapid_signature(vapid_keys);

        let message = match builder.build() {
            Ok(message) => message,
            Err(error) => {
                eprintln!("[❌ ERROR] 메시지 빌드 실패: {:?}", error);
                continue;
            }
        };

        match client.send(message).await {
            Ok(_) => println!(
                "[🎉 SUCCESS] 구독자[{}]에게 푸시 알림 전송 성공!",
                index + 1
            ),
            Err(error) => {
                failed += 1;
                eprintln!(
                    "[❌ ERROR] 구독자[{}]에게 푸시 알림 전송 실패: {:?}",
                    index + 1,
                    error
                );
            }
        }
    }

    println!("[🏁 FINISH] 모든 알림 전송 시도가 완료되었습니다.\n");
    format!(
        "Notification trigger attempted: {} succeeded, {} failed",
        subscribers.len() - failed,
        failed
    )
}

use crate::{
    models::{ClientSubscription, UnsubscribeRequest},
    state::{save_subscriptions, AppState},
};
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

pub(crate) async fn vapid_public_key_handler() -> String {
    std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default()
}

pub(crate) async fn subscribers_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ClientSubscription>> {
    let subscribers = state.subscriptions.lock().await.clone();
    Json(subscribers)
}

pub(crate) async fn subscribe_handler(
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

pub(crate) async fn unsubscribe_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UnsubscribeRequest>,
) -> Result<String, (StatusCode, String)> {
    let mut subscriptions = state.subscriptions.lock().await;
    let mut updated = subscriptions.clone();
    updated.retain(|subscription| subscription.endpoint != request.endpoint);

    if let Err(error) = save_subscriptions(&state.store_path, &updated).await {
        tracing::error!(%error, "Failed to persist device unsubscription");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "기기 구독 해제 정보를 저장하지 못했습니다.".to_string(),
        ));
    }

    *subscriptions = updated;
    Ok("Unsubscribed successfully".to_string())
}

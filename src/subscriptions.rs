use crate::{
    models::{
        current_time_millis, default_device_name, new_device_id, ClientSubscription,
        RenameDeviceRequest, SubscriberSummary, UnsubscribeRequest,
    },
    state::{save_subscriptions, AppState},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

pub(crate) async fn vapid_public_key_handler() -> String {
    std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default()
}

pub(crate) async fn subscribers_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<SubscriberSummary>> {
    let subscribers = state.subscriptions.lock().await.clone();
    Json(
        subscribers
            .into_iter()
            .map(|subscription| SubscriberSummary {
                id: subscription.id,
                name: subscription.name,
                provider: subscription
                    .endpoint
                    .split('/')
                    .nth(2)
                    .unwrap_or("알 수 없는 서비스")
                    .to_string(),
                registered_at: subscription.registered_at,
            })
            .collect(),
    )
}

pub(crate) async fn subscribe_handler(
    State(state): State<Arc<AppState>>,
    Json(sub): Json<ClientSubscription>,
) -> Result<String, (StatusCode, String)> {
    println!("\n[📥 SUBSCRIBE] 새로운 푸시 구독 요청이 접수되었습니다.");
    println!("[📌 ENDPOINT] {}", sub.endpoint);

    let mut subscriptions = state.subscriptions.lock().await;
    let mut updated = subscriptions.clone();
    let mut sub = sub;
    if let Some(existing) = subscriptions
        .iter()
        .find(|existing| existing.endpoint == sub.endpoint)
    {
        sub.id = existing.id.clone();
        sub.name = existing.name.clone();
        sub.registered_at = existing.registered_at;
    } else {
        let registered_at = current_time_millis().map_err(|error| {
            tracing::error!(%error, "Failed to get device registration time");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "기기 등록 시간을 기록하지 못했습니다.".to_string(),
            )
        })?;
        sub.id = new_device_id(registered_at);
        sub.name = default_device_name();
        sub.registered_at = registered_at;
    }

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

pub(crate) async fn rename_subscriber_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<RenameDeviceRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err((
            StatusCode::BAD_REQUEST,
            "기기 별명은 1~40자여야 합니다.".to_string(),
        ));
    }

    let mut subscriptions = state.subscriptions.lock().await;
    let mut updated = subscriptions.clone();
    let Some(subscription) = updated
        .iter_mut()
        .find(|subscription| subscription.id == id)
    else {
        return Err((
            StatusCode::NOT_FOUND,
            "등록 기기를 찾을 수 없습니다.".to_string(),
        ));
    };
    subscription.name = name.to_string();

    if let Err(error) = save_subscriptions(&state.store_path, &updated).await {
        tracing::error!(%error, "Failed to persist device name");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "기기 별명을 저장하지 못했습니다.".to_string(),
        ));
    }

    *subscriptions = updated;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn delete_subscriber_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut subscriptions = state.subscriptions.lock().await;
    let mut updated = subscriptions.clone();
    let original_len = updated.len();
    updated.retain(|subscription| subscription.id != id);
    if updated.len() == original_len {
        return Err((
            StatusCode::NOT_FOUND,
            "등록 기기를 찾을 수 없습니다.".to_string(),
        ));
    }

    if let Err(error) = save_subscriptions(&state.store_path, &updated).await {
        tracing::error!(%error, "Failed to persist device deletion");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "기기 등록 정보를 삭제하지 못했습니다.".to_string(),
        ));
    }

    *subscriptions = updated;
    Ok(StatusCode::NO_CONTENT)
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

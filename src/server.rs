use crate::{
    notifications::{notifications_handler, send_notification_handler},
    state::AppState,
    subscriptions::{
        delete_subscriber_handler, rename_subscriber_handler, subscribe_handler,
        subscribers_handler, unsubscribe_handler, vapid_public_key_handler,
    },
};
use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub(crate) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/api/subscribe", post(subscribe_handler))
        .route("/api/unsubscribe", post(unsubscribe_handler))
        .route(
            "/api/subscribers/:id",
            patch(rename_subscriber_handler).delete(delete_subscriber_handler),
        )
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
                .on_response(
                    |response: &axum::http::Response<_>, latency, _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency = ?latency,
                            "HTTP response sent"
                        );
                    },
                ),
        )
        .with_state(state)
}

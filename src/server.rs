use crate::{
    notifications::{notifications_handler, send_notification_handler},
    state::{AdminCredentials, AppState},
    subscriptions::{
        delete_subscriber_handler, rename_subscriber_handler, subscribe_handler,
        subscribers_handler, unsubscribe_handler, vapid_public_key_handler,
    },
};
use axum::{
    body::Body,
    extract::State,
    http::{
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        HeaderValue, Request, StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_auth,
        ))
        .with_state(state)
}

async fn require_admin_auth(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !requires_admin_auth(request.uri().path()) {
        return next.run(request).await;
    }

    let Some(credentials) = state.admin_credentials.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Admin login is not configured.",
        )
            .into_response();
    };

    if has_valid_credentials(request.headers().get(AUTHORIZATION), credentials) {
        return next.run(request).await;
    }

    let mut response = (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    response.headers_mut().insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"Rust PWA Admin\", charset=\"UTF-8\""),
    );
    response
}

fn requires_admin_auth(path: &str) -> bool {
    matches!(
        path,
        "/admin.html" | "/api/send-notification" | "/notifications" | "/subscribers"
    ) || path.starts_with("/api/subscribers/")
}

fn has_valid_credentials(authorization: Option<&HeaderValue>, expected: &AdminCredentials) -> bool {
    let Some(authorization) = authorization.and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let Some((scheme, encoded)) = authorization.split_once(' ') else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("Basic") {
        return false;
    }

    let Ok(decoded) = STANDARD.decode(encoded.trim()) else {
        return false;
    };
    let Some(separator) = decoded.iter().position(|byte| *byte == b':') else {
        return false;
    };
    let Ok(username) = std::str::from_utf8(&decoded[..separator]) else {
        return false;
    };
    let Ok(password) = std::str::from_utf8(&decoded[separator + 1..]) else {
        return false;
    };

    username == expected.username.as_str() && password == expected.password.as_str()
}

#[cfg(test)]
mod tests {
    use super::{has_valid_credentials, requires_admin_auth};
    use crate::state::AdminCredentials;
    use axum::http::HeaderValue;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use std::str::FromStr;

    #[test]
    fn basic_auth_accepts_configured_credentials_and_rejects_invalid_values() {
        let expected = AdminCredentials {
            username: "admin".to_string(),
            password: "strong-password".to_string(),
        };
        let valid = HeaderValue::from_str(&format!(
            "Basic {}",
            STANDARD.encode("admin:strong-password")
        ))
        .expect("valid header");
        let invalid = HeaderValue::from_str(&format!(
            "Basic {}",
            STANDARD.encode("admin:wrong-password")
        ))
        .expect("valid header");

        assert!(has_valid_credentials(Some(&valid), &expected));
        assert!(!has_valid_credentials(Some(&invalid), &expected));
        assert!(!has_valid_credentials(None, &expected));
    }

    #[test]
    fn protects_admin_pages_and_actions_but_not_public_subscription_routes() {
        assert!(requires_admin_auth("/admin.html"));
        assert!(requires_admin_auth("/subscribers"));
        assert!(requires_admin_auth("/api/subscribers/device-id"));
        assert!(requires_admin_auth("/notifications"));
        assert!(!requires_admin_auth("/"));
        assert!(!requires_admin_auth("/api/subscribe"));
        assert!(!requires_admin_auth("/api/vapid-public-key"));
    }
}

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json, Response,
    },
    routing::{get, post},
    Router,
};
use base64::Engine;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::{
    assets::Assets,
    message::Message,
    relay::{RelayConfig, RelaySwitch},
    store::{MessageStore, StoreEvent},
};

pub fn router(store: Arc<MessageStore>, smtp_port: Option<u16>, relay: RelaySwitch) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/health", get(health))
        .route("/api/messages", get(list_messages).delete(clear_messages))
        .route(
            "/api/messages/{id}",
            get(get_message).delete(delete_message),
        )
        .route("/api/messages/{id}/raw", get(get_message_raw))
        .route("/api/messages/{id}/attachments/{idx}", get(get_attachment))
        .route("/api/messages/{id}/release", post(release_message))
        .route("/api/stream", get(stream))
        .route(
            "/api/relay",
            get(get_relay).put(put_relay).delete(delete_relay),
        )
        .fallback(serve_asset)
        .layer(CorsLayer::permissive())
        .with_state(UiState {
            store,
            smtp_port,
            relay,
        })
}

#[derive(Clone)]
struct UiState {
    store: Arc<MessageStore>,
    smtp_port: Option<u16>,
    relay: RelaySwitch,
}

impl axum::extract::FromRef<UiState> for Arc<MessageStore> {
    fn from_ref(s: &UiState) -> Self {
        s.store.clone()
    }
}

impl axum::extract::FromRef<UiState> for RelaySwitch {
    fn from_ref(s: &UiState) -> Self {
        s.relay.clone()
    }
}

async fn serve_index() -> Response {
    serve_asset_path("index.html").await
}

async fn serve_asset(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        return serve_asset_path("index.html").await;
    }
    serve_asset_path(path).await
}

async fn serve_asset_path(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or(HeaderValue::from_static("application/octet-stream")),
            );
            headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
            (StatusCode::OK, headers, content.data.into_owned()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn health(State(state): State<UiState>) -> Json<serde_json::Value> {
    let mut body = serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "count": state.store.len(),
    });
    if let Some(port) = state.smtp_port {
        body["smtp_port"] = serde_json::Value::from(port);
    }
    Json(body)
}

#[derive(Deserialize)]
struct ListParams {
    limit: Option<usize>,
}

async fn list_messages(
    State(store): State<Arc<MessageStore>>,
    Query(params): Query<ListParams>,
) -> Json<Vec<Message>> {
    let limit = params.limit.unwrap_or(100).min(10_000);
    Json(store.list(limit))
}

async fn get_message(
    State(store): State<Arc<MessageStore>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Message>, StatusCode> {
    store.get(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn get_message_raw(
    State(store): State<Arc<MessageStore>>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let m = store.get(id).ok_or(StatusCode::NOT_FOUND)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("message/rfc822"),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from(m.raw.len() as u64),
    );
    Ok((StatusCode::OK, headers, m.raw.to_vec()).into_response())
}

async fn get_attachment(
    State(store): State<Arc<MessageStore>>,
    Path((id, idx)): Path<(Uuid, usize)>,
) -> Result<Response, StatusCode> {
    let m = store.get(id).ok_or(StatusCode::NOT_FOUND)?;
    let att = m.attachments.get(idx).ok_or(StatusCode::NOT_FOUND)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&att.content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(att.size as u64));
    if let Some(name) = &att.filename {
        let header_value = format!("attachment; filename=\"{}\"", name.replace('"', ""));
        if let Ok(v) = HeaderValue::from_str(&header_value) {
            headers.insert(header::CONTENT_DISPOSITION, v);
        }
    }
    Ok((StatusCode::OK, headers, att.data.to_vec()).into_response())
}

async fn delete_message(
    State(store): State<Arc<MessageStore>>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    if store.delete(id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn clear_messages(State(store): State<Arc<MessageStore>>) -> StatusCode {
    store.clear();
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
struct ReleaseBody {
    smtp_url: String,
    insecure: Option<bool>,
}

async fn release_message(
    State(store): State<Arc<MessageStore>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReleaseBody>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let m = store.get(id).ok_or((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error":"not_found"})),
    ))?;
    let parsed = url::Url::parse(&body.smtp_url).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"invalid_url","reason": e.to_string()})),
        )
    })?;
    let cfg = RelayConfig::from_url(parsed, body.insecure.unwrap_or(false)).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"invalid_relay","reason": e.to_string()})),
        )
    })?;
    crate::relay::relay_message(&cfg, &m).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error":"relay_failed","reason": e.to_string()})),
        )
    })?;
    Ok(StatusCode::ACCEPTED)
}

async fn get_relay(State(switch): State<RelaySwitch>) -> Json<serde_json::Value> {
    let snap = switch.read().await.clone();
    Json(relay_to_json(&snap))
}

#[derive(Deserialize)]
struct PutRelayBody {
    url: String,
    insecure: Option<bool>,
}

async fn put_relay(
    State(switch): State<RelaySwitch>,
    Json(body): Json<PutRelayBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let parsed = url::Url::parse(&body.url).map_err(|e| {
        relay_error(
            StatusCode::BAD_REQUEST,
            "invalid_url",
            &format!("invalid URL '{}': {e}", body.url),
        )
    })?;
    let cfg = RelayConfig::from_url(parsed, body.insecure.unwrap_or(false))
        .map_err(|e| relay_error(StatusCode::BAD_REQUEST, "invalid_scheme", &e.to_string()))?;
    let mut guard = switch.write().await;
    *guard = Some(cfg.clone());
    drop(guard);
    Ok(Json(relay_to_json(&Some(cfg))))
}

async fn delete_relay(State(switch): State<RelaySwitch>) -> StatusCode {
    *switch.write().await = None;
    StatusCode::NO_CONTENT
}

fn relay_to_json(cfg: &Option<RelayConfig>) -> serde_json::Value {
    match cfg {
        Some(c) => serde_json::json!({
            "enabled": true,
            "url": redact(&c.url),
            "host": c.host,
            "port": c.port,
            "tls": c.use_tls,
            "insecure": c.insecure,
            "auth": c.auth.is_some(),
        }),
        None => serde_json::json!({
            "enabled": false,
            "url": null,
            "host": null,
            "port": null,
            "tls": false,
            "insecure": false,
            "auth": false,
        }),
    }
}

fn redact(u: &url::Url) -> String {
    if u.username().is_empty() {
        return u.to_string();
    }
    let mut redacted = u.clone();
    let _ = redacted.set_username("");
    let _ = redacted.set_password(None);
    let mut s = redacted.to_string();
    // Re-insert a placeholder so the user can see auth was set without
    // leaking the credentials back over the wire.
    if let Some(after_scheme) = s.find("://") {
        let prefix = &s[..after_scheme + 3];
        let rest = &s[after_scheme + 3..];
        s = format!("{prefix}***@{rest}");
    }
    s
}

fn relay_error(
    status: StatusCode,
    code: &str,
    message: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({"error": code, "reason": message})),
    )
}

async fn stream(
    State(store): State<Arc<MessageStore>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = store.subscribe();
    let initial = futures::stream::once(async {
        Ok::<_, Infallible>(
            Event::default()
                .event("hello")
                .data(format!(r#"{{"version":"{}"}}"#, env!("CARGO_PKG_VERSION"))),
        )
    });
    let updates = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(StoreEvent::Message(msg)) => Some(Ok(Event::default()
                .event("message")
                .data(serde_json::to_string(&*msg).unwrap_or_else(|_| "{}".into())))),
            Ok(StoreEvent::Cleared) => Some(Ok(Event::default().event("cleared").data("{}"))),
            Ok(StoreEvent::Deleted(id)) => Some(Ok(Event::default()
                .event("deleted")
                .data(format!(r#"{{"id":"{id}"}}"#)))),
            Err(_lagged) => Some(Ok(Event::default().event("resync").data("{}"))),
        }
    });
    Sse::new(initial.chain(updates)).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// Re-export for the integration test convenience.
#[allow(dead_code)]
pub fn b64_encode(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn serve_asset_falls_back_to_index_for_empty_path() {
        let resp = serve_asset(axum::http::Uri::from_static("/")).await;
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn serve_asset_404_for_unknown_path() {
        let resp = serve_asset(axum::http::Uri::from_static("/no-such-asset.txt")).await;
        assert_eq!(resp.status(), 404);
    }

    #[test]
    fn relay_to_json_renders_enabled_and_disabled() {
        let disabled = relay_to_json(&None);
        assert_eq!(disabled["enabled"], false);
        let cfg = RelayConfig::from_url(
            url::Url::parse("smtp://alice:s3cret@relay.example.com:2525").unwrap(),
            false,
        )
        .unwrap();
        let enabled = relay_to_json(&Some(cfg));
        assert_eq!(enabled["enabled"], true);
        assert_eq!(enabled["host"], "relay.example.com");
        assert_eq!(enabled["port"], 2525);
        assert_eq!(enabled["auth"], true);
        // No raw secret in the URL string.
        assert!(!enabled["url"].as_str().unwrap().contains("s3cret"));
    }

    #[test]
    fn redact_preserves_url_when_no_userinfo() {
        let u = url::Url::parse("smtp://relay.example.com:25").unwrap();
        let s = redact(&u);
        assert!(!s.contains('@'));
        assert!(s.contains("relay.example.com"));
    }

    #[test]
    fn redact_replaces_userinfo_with_placeholder() {
        let u = url::Url::parse("smtps://alice:hunter2@relay.example.com:465").unwrap();
        let s = redact(&u);
        assert!(s.contains("***@"));
        assert!(!s.contains("hunter2"));
    }

    #[test]
    fn relay_error_carries_status_and_payload() {
        let (status, body) = relay_error(StatusCode::BAD_REQUEST, "boom", "because");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.0["error"], "boom");
        assert_eq!(body.0["reason"], "because");
    }

    #[test]
    fn b64_encode_works() {
        assert_eq!(b64_encode(b"hi"), "aGk=");
    }
}

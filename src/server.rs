use axum::{
    extract::Json,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use zbus::Connection;
use crate::notifications::NotificationsProxy;

// TODO: Replace this with the SHA-256 hash of your secret token
const AUTH_HASH: &str = "CHANGEME";

#[derive(Deserialize)]
struct NotifyParams {
    #[serde(default)]
    app_name: String,
    #[serde(default)]
    replaces_id: u32,
    #[serde(default)]
    app_icon: String,
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    actions: Vec<String>,
    // Simplification: We only support string hints for now to keep JSON handling simple
    // and zvariant conversion straightforward.
    #[serde(default)]
    hints: HashMap<String, String>,
    #[serde(default = "default_expire_timeout")]
    expire_timeout: i32,
}

fn default_expire_timeout() -> i32 {
    -1
}

pub fn run() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let app = Router::new().route("/notify", post(notify_handler));
        let listener = tokio::net::TcpListener::bind("0.0.0.0:6767").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
}

async fn notify_handler(headers: HeaderMap, Json(payload): Json<NotifyParams>) -> impl IntoResponse {
    // 1. Authentication
    let auth_header = match headers.get("Authorization") {
        Some(h) => h.to_str().unwrap_or(""),
        None => return StatusCode::UNAUTHORIZED,
    };

    let mut hasher = Sha256::new();
    hasher.update(auth_header);
    let result = hasher.finalize();
    let hash_hex = hex::encode(result);

    if hash_hex != AUTH_HASH {
        return StatusCode::UNAUTHORIZED;
    }

    // 2. Connect to DBus
    let connection = match Connection::session().await {
        Ok(conn) => conn,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let proxy = match NotificationsProxy::new(&connection).await {
        Ok(p) => p,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    // 3. Convert hints
    // zbus expects HashMap<&str, &Value>. We need to construct Values that live long enough.
    // However, zvariant::Value is an enum.
    // The generated trait expects `HashMap<&str, &Value<'_>>`.
    // We can map our input hints.
    
    // We need to create a map where values are zvariant::Value.
    // Since we only support String hints in input, we convert them.
    let mut hints_map = HashMap::new();
    for (k, v) in &payload.hints {
        hints_map.insert(k.as_str(), zbus::zvariant::Value::Str(v.as_str().into()));
    }
    
    // Note: zbus proxy notify takes `HashMap<&str, &Value<'_>>`.
    // We have `HashMap<&str, Value>`. We need to take references to the values in our map.
    // Just creating the map of Values first.
    
    let actions_refs: Vec<&str> = payload.actions.iter().map(|s| s.as_str()).collect();
    let hints_refs: HashMap<&str, &zbus::zvariant::Value<'_>> = hints_map.iter().map(|(k, v)| (*k, v)).collect();

    match proxy.notify(
        &payload.app_name,
        payload.replaces_id,
        &payload.app_icon,
        &payload.summary,
        &payload.body,
        &actions_refs,
        hints_refs,
        payload.expire_timeout,
    ).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

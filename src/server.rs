use axum::{
    extract::Json,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::process::Command;

// The SHA-256 hash of your secret token
const AUTH_HASH: &str = "08f753dab1ae413defdde3afd3cc4a9f81f6b2a187b44b9ae09da62e5ae52f3f";

#[derive(Deserialize)]
struct NotifyParams {
    summary: String,
    #[serde(default)]
    body: String,
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

    // send notification
    let mut child = Command::new("zenity")
            .arg("--warning")
            .arg("--icon=printer-symbolic")
            .arg(format!("--text={}", payload.body))
            .arg(format!("--title={}", payload.summary))
            .spawn()
            .expect("failed to start zenity");

    StatusCode::OK
}

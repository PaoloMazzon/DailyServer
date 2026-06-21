use axum::{Json, Router, response::IntoResponse, routing::get};
use serde_json::json;

pub async fn api_endpoint() -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

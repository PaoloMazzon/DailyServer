use axum::{Json, response::IntoResponse};
use serde_json::json;
use spdlog::prelude::*;

pub async fn api_endpoint_get(payload: String) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

pub async fn api_endpoint_post(payload: String) -> impl IntoResponse {
    info!("payload: {}", payload);
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

#[cfg(test)]
mod tests {
    
}
use axum::{Json, response::IntoResponse};
use serde_json::json;

pub async fn api_endpoint_get(_payload: String) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

pub async fn api_endpoint_post(_payload: String) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

#[cfg(test)]
mod tests {
    
}
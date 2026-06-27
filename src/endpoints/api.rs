use axum::{Json, response::IntoResponse};
use axum::body::Body;
use axum::http::Request;
use serde_json::json;

pub async fn api_endpoint_post(_payload: Request<Body>) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

#[cfg(test)]
mod tests {
    
}
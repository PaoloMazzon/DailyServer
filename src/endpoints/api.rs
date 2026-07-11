use axum::{Json, response::IntoResponse};
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use serde_json::json;
use crate::state::rest_state::RestState;

pub async fn api_endpoint_post(_state: State<RestState>, _payload: Request<Body>) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

pub async fn api_endpoint_get(_state: State<RestState>, _payload: Request<Body>) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "message": "Hello, World!"
    });
    Json(json_response)
}

#[cfg(test)]
mod tests {
    
}
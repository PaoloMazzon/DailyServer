use axum::body::Body;
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use spdlog::prelude::*;

pub fn general_error_response(status: StatusCode, error: String) -> Response<Body> {
    (status, Json(json!({
        "error": error,
    }))).into_response()
}

pub fn not_found_response() -> Response<Body> {
    (StatusCode::NOT_FOUND, Json(json!({}))).into_response()
}

// Fallback if a builder fails
pub fn internal_error_response(error: String) -> Response<Body> {
    warn!("Creating fallback error response with error string {}", error);
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
        "error": error,
    }))).into_response()
}

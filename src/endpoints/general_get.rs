use axum::body::Body;
use axum::http::Request;
use axum::response::Html;
use axum::response::IntoResponse;

pub async fn endpoint_get(payload: Request<Body>) -> impl IntoResponse {
    Html(format!("{:?}", payload.uri()))
}
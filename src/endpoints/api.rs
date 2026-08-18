use axum::{Json, response::IntoResponse, http};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::response::Html;
use serde_json::json;
use crate::state::rest_state::RestState;
use crate::util::daily_seed::get_current_seed;

pub async fn api_endpoint_post(_state: State<RestState>, payload: Request<Body>) -> impl IntoResponse {
    let json_response = json!({
        "status": "ok",
        "uri": payload.uri().to_string()
    });
    todo!("Parse different URIs and post a highscore, returning the ID of that highscore");
    Json(json_response)
}

async fn seed_request() -> Result<Response<Body>, http::Error> {
    let response = match get_current_seed().await {
        Ok(seed) => {
            Response::builder()
                .status(StatusCode::OK)
                .body(format!("{{\"seed\": \"{}\"}}", seed).into())?
        },
        Err(e) => {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error: {}", e).into())?
        }
    };
    Ok(response)
}

pub async fn api_endpoint_get(_state: State<RestState>, payload: Request<Body>) -> impl IntoResponse {

    match payload.uri().path() {
        "/api/v1/daily-seed" => {
            seed_request().await.unwrap_or(Html("internal server error".to_string()).into_response())
        },
        "/api/v1/leaderboard" => {
            todo!("Parse the options in the uri and return a section of the leaderboard")
        }
        _ => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("".into())
                .unwrap_or(Html("internal server error".to_string()).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    
}
use std::time::Duration;
use axum::{Json, response::IntoResponse, http};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::response::Html;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::state::rest_state::RestState;
use crate::util::daily_seed::get_current_seed;
use spdlog::prelude::*;
use crate::state::database::DatabaseRow;

#[derive(Deserialize, Debug)]
struct HighscoreQuery {
    starting_index: i32,
    count: i32,
}

#[derive(Serialize, Debug)]
struct HighscoreResponse {
    actual_starting_index: i32,
    actual_count: i32,
    scores: Vec<DatabaseRow>,
}

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

async fn query_highscore(state: &mut RestState, query: HighscoreQuery) -> Result<Response<Body>, http::Error> {
    match state.accessor.read("".to_string(), Duration::from_millis(state.config.database_read_timeout_ms)).await {
        Ok(rows) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(serde_json::to_string(&HighscoreResponse {
                    actual_starting_index: 0, // TODO: This
                    actual_count: 0, // TODO: This
                    scores: rows
                }).unwrap_or("{}".to_string()).into())?)
        },
        Err(e) => {
            debug!("Highscore query \"{:?}\" failed, {}", query, e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error: {}", e).into())?)
        }
    }
}

pub async fn api_endpoint_get(mut state: State<RestState>, payload: Request<Body>) -> impl IntoResponse {
    debug!("URI: {:?}", payload.uri());

    match payload.uri().path() {
        "/api/v1/daily-seed" => {
            seed_request().await.unwrap_or(Html("internal server error".to_string()).into_response())
        },
        "/api/v1/leaderboard" => {
            match serde_urlencoded::from_str::<HighscoreQuery>(payload.uri().query().unwrap_or("")) {
                Ok(q) => query_highscore(&mut state.0, q).await
                    .unwrap_or(Html("internal server error".to_string()).into_response()),
                Err(e) => {
                    debug!("Bad get request, URI={:?}, e={}", payload.uri(), e);
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(format!("{}", e).into())
                        .unwrap_or(Html("internal server error".to_string()).into_response())
                }
            }
        },
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
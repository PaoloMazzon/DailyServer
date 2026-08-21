use std::time::Duration;
use axum::{response::IntoResponse, http, Json};
use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::state::rest_state::RestState;
use crate::util::daily_seed::get_current_seed;
use spdlog::prelude::*;
use crate::endpoints::util::{general_error_response, internal_error_response, not_found_response};
use crate::state::database::DatabaseRow;
use crate::util::date::{get_date, is_date_iso8601};

#[derive(Deserialize, Debug)]
struct HighscoreQuery {
    starting_index: i32,
    count: i32,
    date: String,
}

#[derive(Serialize, Debug)]
struct HighscoreResponse {
    scores: Vec<DatabaseRow>,
}

#[derive(Deserialize, Debug)]
struct SubmitRequest {
    name: String,
    extra_data: String,
    score: i64,
    daily_seed: i64, // to validate the seed a user is submitting for hasn't expired by submission time
}

#[derive(Serialize, Debug)]
struct SubmitResponse {
    submitted_id: i64,
}

async fn v1_submit_request(state: &mut RestState, paylod: Request<Body>) -> Result<Response<Body>, http::Error> {
    let uri = paylod.uri().clone();
    let body = paylod.into_body();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => {
            warn!("Requested submitted with body size above limit, {}", uri);
            return Ok(general_error_response(StatusCode::BAD_REQUEST, format!("Maximum request size is {} bytes", state.config.request_max_payload_size)))
        }
    };
    let submission: SubmitRequest = match serde_json::from_slice(&bytes) {
        Ok(s) => s,
        Err(e) => return Ok(general_error_response(StatusCode::BAD_REQUEST, format!("{}", e)))
    };

    if submission.daily_seed != get_current_seed().await.unwrap_or(0) {
        return Ok(general_error_response(StatusCode::CONFLICT, "Daily seed is out of date.".to_string()))
    }

    let row = DatabaseRow {
        id: 0,
        name: submission.name,
        extra_data: submission.extra_data,
        score: submission.score,
        date: get_date(),
    };

    match state.accessor.write_with_id(&state.config, row.clone()).await {
        Ok(id) => {
            let response = SubmitResponse { submitted_id: id };
            match serde_json::to_value(&response) {
                Ok(json) => {
                    info!("Submitted score {:?}, response {:?}.", row, json);
                    Ok((StatusCode::OK, Json(json)).into_response())
                },
                Err(e) => {
                    warn!("Failed to serialize json for struct {:?}, {}", response, e);
                    Ok(general_error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
                }
            }
        },
        Err(e) => {
            Ok(general_error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn api_endpoint_post(mut state: State<RestState>, payload: Request<Body>) -> impl IntoResponse {
    debug!("API POST URI: {:?}", payload.uri());

    match payload.uri().path() {
        "/api/v1/submit" => {
            v1_submit_request(&mut state, payload).await
                .unwrap_or_else(|e| internal_error_response(format!("{}", e)))
        },
        _ => {
            not_found_response()
        }
    }
}

async fn seed_request() -> Result<Response<Body>, http::Error> {
    let response = match get_current_seed().await {
        Ok(seed) => (StatusCode::OK, Json(json!({"seed": seed}))).into_response(),
        Err(e) => general_error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e))
    };
    Ok(response)
}

async fn query_highscore(state: &mut RestState, query: HighscoreQuery) -> Result<Response<Body>, http::Error> {
    if query.count > state.config.maximum_row_query {
        return Ok(general_error_response(StatusCode::BAD_REQUEST, format!("maximum amount of rows query-able at once is {}", state.config.maximum_row_query)))
    }

    if !is_date_iso8601(query.date.clone()) {
        return Ok(general_error_response(StatusCode::BAD_REQUEST, "Invalid date format, must be ISO-8601 formatted.".to_string()))
    }

    match state.accessor
        .read(format!("SELECT * FROM user WHERE date == \"{}\" ORDER BY score DESC LIMIT {} OFFSET {};", query.date, query.count, query.starting_index),
              Duration::from_millis(state.config.database_read_timeout_ms)).await {
        Ok(rows) => {
            let response = HighscoreResponse { scores: rows };
            match serde_json::to_value(&response) {
                Ok(json) => Ok((StatusCode::OK, Json(json)).into_response()),
                Err(e) => {
                    warn!("Failed to serialize struct {:?}, {}", response, e);
                    Ok(general_error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
                }
            }
        },
        Err(e) => {
            debug!("Highscore query \"{:?}\" failed, {}", query, e);
            Ok(general_error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)))
        }
    }
}

pub async fn api_endpoint_get(mut state: State<RestState>, payload: Request<Body>) -> impl IntoResponse {
    debug!("API GET URI: {:?}", payload.uri());

    match payload.uri().path() {
        "/api/v1/daily-seed" => {
            seed_request().await.unwrap_or_else(|e| internal_error_response(format!("{}", e)))
        },
        "/api/v1/leaderboard" => {
            match serde_urlencoded::from_str::<HighscoreQuery>(payload.uri().query().unwrap_or("")) {
                Ok(q) => query_highscore(&mut state.0, q).await
                    .unwrap_or_else(|e| internal_error_response(format!("{}", e))),
                Err(e) => {
                    debug!("Bad get request, URI={:?}, e={}", payload.uri(), e);
                    general_error_response(StatusCode::BAD_REQUEST, format!("{}", e))
                }
            }
        },
        _ => {
            not_found_response()
        }
    }
}

#[cfg(test)]
mod tests {
    
}
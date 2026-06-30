use std::path::Path;
use axum::body::{Body, HttpBody};
use axum::http;
use axum::http::{Request, Response, StatusCode};
use axum::response::{Html, IntoResponse};
use crate::util::config::file_in_ignore_list;
use spdlog::prelude::*;

/// Every URI for a GET request falls under one of these
#[derive(Debug)]
enum UriValidity {
    Valid(String),
    InIgnoreList,
    DoesNotExist,
    Malicious,
}

/// Returns true if a uri is not trying anything malicious like path traversal
fn uri_is_sanitized(uri: &String) -> bool {
    match uri.contains("..") {
        false => true,
        true => {
            warn!("Suspicious URI detected: {}", uri);
            false
        }
    }
}

/// Runs a URI against the ignore list, checks if its safe, checks if it exists, and parses index.html
fn classify_uri(uri: String) -> UriValidity {
    let mut path = match uri.as_str() {
        "/" | "" => "/index.html".to_string(),
        x => x.to_string()
    };
    path.insert_str(0, "site");

    if !uri_is_sanitized(&path) {
        UriValidity::Malicious
    } else if file_in_ignore_list(Path::new(path.as_str())) {
        UriValidity::InIgnoreList
    } else if !Path::new(path.as_str()).exists() {
        UriValidity::DoesNotExist
    } else {
        UriValidity::Valid(path)
    }
}

/// Asynchronously reads a file as utf8
async fn get_file_lossy(path: &str) -> String {
    String::from_utf8_lossy(tokio::fs::read(Path::new(path)).await.unwrap_or(Vec::new()).as_slice()).to_string()
}

/// Parses a get request and can fail, so the top level handles the error
async fn endpoint_get_safe(payload: Request<Body>) -> Result<Response<Body>, http::Error> {
    let body = match classify_uri(payload.uri().to_string()) {
        UriValidity::Valid(uri) => Response::builder()
            .status(StatusCode::OK)
            .body(get_file_lossy(uri.as_str()).await.into_response().into_body())?,
        UriValidity::InIgnoreList => Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(get_file_lossy("site/not_found.html").await.into_response().into_body())?,
        UriValidity::DoesNotExist => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(get_file_lossy("site/not_found.html").await.into_response().into_body())?,
        UriValidity::Malicious => Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(get_file_lossy("site/forbidden.html").await.into_response().into_body())?,
    };
    Ok(body)
}

/// Top-level get endpoint
#[axum::debug_handler]
pub async fn endpoint_get(payload: Request<Body>) -> impl IntoResponse {
    let payload_debug_string = format!("{:?}", payload);
    debug!("[GET] {} size {}", payload.uri(), payload.body().size_hint().exact().unwrap_or(0));
    match endpoint_get_safe(payload).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse GET request '{:?}', {}", payload_debug_string, e);
            Html("internal server error".to_string()).into_response()
        }
    }
}
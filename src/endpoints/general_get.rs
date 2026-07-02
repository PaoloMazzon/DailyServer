use std::path::Path;
use axum::body::{Body, HttpBody};
use axum::extract::State;
use axum::http;
use axum::http::{Request, Response, StatusCode};
use axum::response::{Html, IntoResponse};
use crate::util::config::file_in_ignore_list;
use spdlog::prelude::*;
use crate::state::rest_state::RestState;

/// Every URI for a GET request falls under one of these
#[derive(Debug, Eq, PartialEq)]
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

/// Returns a slice of everything after the first character
fn remove_first_char(s: &str) -> String {
    s.chars().next().map(|c| &s[c.len_utf8()..]).unwrap_or("").to_string()
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
    } else if file_in_ignore_list(Path::new(remove_first_char(uri.as_str()).as_str())) {
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
async fn endpoint_get_safe(state: RestState, payload: Request<Body>) -> Result<Response<Body>, http::Error> {
    let body = match classify_uri(payload.uri().to_string()) {
        UriValidity::Valid(uri) => Response::builder()
            .status(StatusCode::OK)
            .body(get_file_lossy(uri.as_str()).await.into_response().into_body())?,
        UriValidity::InIgnoreList => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(get_file_lossy(state.config.unauthorized_filename.as_str()).await.into_response().into_body())?,
        UriValidity::DoesNotExist => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(get_file_lossy(state.config.not_found_filename.as_str()).await.into_response().into_body())?,
        UriValidity::Malicious => Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(get_file_lossy(state.config.forbidden_filename.as_str()).await.into_response().into_body())?,
    };
    Ok(body)
}

/// Top-level get endpoint
#[axum::debug_handler]
pub async fn endpoint_get(state: State<RestState>,payload: Request<Body>) -> impl IntoResponse {
    let payload_debug_string = format!("{:?}", payload);
    debug!("[GET] {} size {}", payload.uri(), payload.body().size_hint().exact().unwrap_or(0));
    match endpoint_get_safe(state.0, payload).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse GET request '{:?}', {}", payload_debug_string, e);
            Html("internal server error".to_string()).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::util::config::init_ignore_list;
    use super::*;

    fn fake_ignore_list() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(".env\n".as_bytes()).unwrap();
        file
    }

    #[tokio::test]
    async fn test_get_file_lossy() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all("asd".as_bytes()).unwrap();
        assert_eq!(get_file_lossy(file.path().to_str().unwrap()).await.as_str(), "asd");
    }

    #[test]
    fn test_classify_uri() {
        init_ignore_list(Path::new(fake_ignore_list().path().to_str().unwrap())).unwrap();
        assert_eq!(classify_uri("/asd/../../asd".parse().unwrap()), UriValidity::Malicious);
        assert_eq!(classify_uri("/asdasda/dsasdad/asdasdad/asd".parse().unwrap()), UriValidity::DoesNotExist);
        assert_eq!(classify_uri("/.env".parse().unwrap()), UriValidity::InIgnoreList);
    }
}
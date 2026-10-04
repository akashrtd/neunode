use std::path::Path;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, Method};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::api::error::ApiError;

pub fn load_token(config_path: &Path) -> anyhow::Result<String> {
    if let Ok(token) = std::env::var("NEUNODE_API_KEY") {
        anyhow::ensure!(
            token.len() >= 32 && token.bytes().all(|byte| byte.is_ascii_graphic()),
            "NEUNODE_API_KEY must contain at least 32 printable characters"
        );
        return Ok(token);
    }
    let path = config_path.with_file_name("api-token");
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match crate::keystore::private_write(&path, crate::keystore::random_id().as_bytes()) {
            Ok(()) => (),
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::AlreadyExists) => {}
            Err(error) => return Err(error),
        }
    }
    crate::keystore::ensure_private(&path)?;
    let token = std::fs::read_to_string(path)?;
    anyhow::ensure!(
        token.len() >= 32 && token.bytes().all(|byte| byte.is_ascii_graphic()),
        "invalid API token file"
    );
    Ok(token)
}

fn matches_token(expected: &str, supplied: &str) -> bool {
    if supplied.len() != expected.len() {
        return false;
    }
    expected.bytes().zip(supplied.bytes()).fold(0u8, |difference, (a, b)| difference | (a ^ b)) == 0
}

pub async fn authorize(
    State((token, db)): State<(Arc<String>, Arc<neunode_storage::db::NeunodeDb>)>,
    request: Request,
    next: Next,
) -> Response {
    let mutation = !matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS)
        || request.uri().path() == "/ws/inference";
    if mutation {
        let supplied = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        let websocket_token = request
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|value| value.to_str().ok())
            .and_then(|protocols| {
                protocols
                    .split(',')
                    .map(str::trim)
                    .find_map(|protocol| protocol.strip_prefix("neunode-auth."))
            })
            .and_then(|hex| hex::decode(hex).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok());
        let supplied = supplied.or_else(|| {
            if request.uri().path() == "/ws/inference" {
                websocket_token.as_deref()
            } else {
                None
            }
        });
        if !supplied.is_some_and(|supplied| matches_token(&token, supplied)) {
            return ApiError::Unauthorized(
                "state-changing requests require the daemon bearer token".into(),
            )
            .into_response();
        }
    }
    if mutation {
        let path = request.uri().path();
        let breaker = if path.starts_with("/api/v1/tokens/")
            || path == "/api/v1/inference/request"
            || path == "/ws/inference"
        {
            Some("token_volume")
        } else if path.starts_with("/api/v1/reputation/") {
            Some("reputation")
        } else if path.starts_with("/api/v1/bounties") || path == "/api/bounties/create" {
            Some("bounty_drain")
        } else {
            None
        };
        if let Some(breaker) = breaker {
            if crate::cmd_security::is_breaker_tripped(&db, breaker) {
                return ApiError::Unavailable(format!("circuit breaker {breaker} is open"))
                    .into_response();
            }
        }
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_entire_token() {
        assert!(matches_token("a secret", "a secret"));
        assert!(!matches_token("a secret", "a secreu"));
        assert!(!matches_token("a secret", "a secretx"));
    }
}

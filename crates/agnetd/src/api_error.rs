use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    Forbidden(String),
    Unauthorized(String),
    Unavailable(String),
    NotFound(String),
    BadRequest(String),
    NoIdentity,
    MeshNotRunning,
    Internal(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message())
    }
}
impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
    success: bool,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NoIdentity => StatusCode::UNAUTHORIZED,
            ApiError::MeshNotRunning => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Unavailable(_) => "UNAVAILABLE",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::NoIdentity => "NO_IDENTITY",
            ApiError::MeshNotRunning => "MESH_NOT_RUNNING",
            ApiError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn message(&self) -> String {
        match self {
            ApiError::Forbidden(msg) | ApiError::Unauthorized(msg) | ApiError::Unavailable(msg) => {
                msg.clone()
            }
            ApiError::NotFound(msg) => msg.clone(),
            ApiError::BadRequest(msg) => msg.clone(),
            ApiError::NoIdentity => "no active identity — run `agnetd identity create`".to_string(),
            ApiError::MeshNotRunning => {
                "mesh not running — run `agnetd mesh start` first".to_string()
            }
            ApiError::Internal(msg) => msg.clone(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorBody {
            error: ErrorDetail { code: self.error_code().to_string(), message: self.message() },
            success: false,
        };
        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<neunode_knowledge::KnowledgeError> for ApiError {
    fn from(err: neunode_knowledge::KnowledgeError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<neunode_storage::error::StorageError> for ApiError {
    fn from(err: neunode_storage::error::StorageError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<neunode_reputation::error::ReputationError> for ApiError {
    fn from(err: neunode_reputation::error::ReputationError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

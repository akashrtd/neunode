use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Standard success envelope matching the SDK format.
#[derive(Serialize, utoipa::ToSchema)]
pub struct SuccessEnvelope<T: Serialize> {
    pub data: T,
    pub success: bool,
}

pub fn ok<T: Serialize>(data: T) -> Response {
    (StatusCode::OK, Json(SuccessEnvelope { data, success: true })).into_response()
}

pub fn created<T: Serialize>(data: T) -> Response {
    (StatusCode::CREATED, Json(SuccessEnvelope { data, success: true })).into_response()
}

/// Generic empty success response.
#[derive(Serialize, utoipa::ToSchema)]
pub struct Ack {
    pub message: String,
}

pub fn ack(msg: &str) -> Response {
    ok(Ack { message: msg.to_string() })
}

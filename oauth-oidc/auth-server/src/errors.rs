use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    RedisError(redis::RedisError),
    JsonError(serde_json::Error),
    InValidParameter,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RedisError(e) => write!(f, "Redis error: {}", e),
            Self::JsonError(e) => write!(f, "JSON serialization error: {}", e),
            Self::InValidParameter => write!(f, "Invalid parameter"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        Self::RedisError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::RedisError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            Self::JsonError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error"),
            Self::InValidParameter => (StatusCode::BAD_REQUEST, "Invalid parameter"),
        };

        tracing::error!("Application error: {:?}", self);
        (status, error_message).into_response()
    }
}

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

// 今回は適当にエラーを返しているが、実際に返すべきエラーは RFC に定義されているので、それに従うべき
// https://openid.net/specs/openid-connect-core-1_0.html#AuthError
#[derive(Debug)]
pub enum AppError {
    RedisError(redis::RedisError),
    JsonError(serde_json::Error),
    JwtEncodeError(String),
    JwkCreateError(String),
    Unauthorized(String),
    InValidParameter,
    InValidHeader,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RedisError(e) => write!(f, "Redis error: {}", e),
            Self::JsonError(e) => write!(f, "JSON serialization error: {}", e),
            Self::JwtEncodeError(e) => write!(f, "JWT encoding error: {}", e),
            Self::JwkCreateError(e) => write!(f, "JWK creation error: {}", e),
            Self::Unauthorized(e) => write!(f, "Unauthorized error: {}", e),
            Self::InValidParameter => write!(f, "Invalid parameter"),
            Self::InValidHeader => write!(f, "Invalid header"),
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
            Self::JwtEncodeError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JWT encoding error"),
            Self::JwkCreateError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JWK creation error"),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized access"),
            Self::InValidParameter => (StatusCode::BAD_REQUEST, "Invalid parameter"),
            Self::InValidHeader => (StatusCode::BAD_REQUEST, "Invalid header"),
        };

        tracing::error!("Application error: {:?}", self);
        (status, error_message).into_response()
    }
}

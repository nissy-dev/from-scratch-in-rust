use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    JwtDecodeError(String),
    JwkFetchError(String),
    InValidHeader,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JwtDecodeError(e) => write!(f, "JWT decode error: {}", e),
            Self::JwkFetchError(e) => write!(f, "JWK fetching error: {}", e),
            Self::InValidHeader => write!(f, "Invalid header provided"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::JwkFetchError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JWK fetching error"),
            Self::JwtDecodeError(_) => (StatusCode::UNAUTHORIZED, "JWT decode error"),
            Self::InValidHeader => (StatusCode::BAD_REQUEST, "Invalid header provided"),
        };

        tracing::error!("Application error: {:?}", self);
        (status, error_message).into_response()
    }
}

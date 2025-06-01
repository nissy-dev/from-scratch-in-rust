use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    JwtDecodeError(String),
    JwkToRsaPublicKeyPemError(String),
    FetchError(String),
    TokenInactive,
    InvalidAudience,
    InValidHeader,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JwtDecodeError(e) => write!(f, "JWT decode error: {}", e),
            Self::JwkToRsaPublicKeyPemError(e) => {
                write!(f, "JWK to RSA public key PEM error: {}", e)
            }
            Self::FetchError(e) => write!(f, "Fetching error: {}", e),
            Self::TokenInactive => write!(f, "Token is not active"),
            Self::InvalidAudience => write!(f, "Invalid audience in JWT"),
            Self::InValidHeader => write!(f, "Invalid header provided"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::JwtDecodeError(_) => (StatusCode::UNAUTHORIZED, "JWT decode error"),
            Self::JwkToRsaPublicKeyPemError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "JWK to RSA public key PEM error",
            ),
            Self::FetchError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "fetching error"),
            Self::TokenInactive => (StatusCode::UNAUTHORIZED, "Token is not active"),
            Self::InvalidAudience => (StatusCode::FORBIDDEN, "Invalid audience in JWT"),
            Self::InValidHeader => (StatusCode::BAD_REQUEST, "Invalid header provided"),
        };

        tracing::error!("Application error: {:?}", self);
        (status, error_message).into_response()
    }
}

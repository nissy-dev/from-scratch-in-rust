use std::collections::HashMap;

use axum::{extract::State, http::HeaderMap};
use reqwest;
use serde::Deserialize;
use shared::jwt::{decode_jwt_rs256, jwk_to_rsa_public_key_pem, Jwk};

use crate::{errors::AppError, AppState};

pub async fn resource(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<String, AppError> {
    // Authorization ヘッダから Bearer トークンを取得
    let token = parse_auth_header(&headers)?;
    // jwk に fetch して public_pem を取得する処理を取得する
    let jwks = fetch_jwks(&state.auth_server_url)
        .await
        .map_err(|e| AppError::JwkFetchError(e.to_string()))?;

    decode_jwt_rs256(&token, &jwks).map_err(|e| AppError::JwtDecodeError(e.to_string()))?;

    Ok("Resource accessed!".into())
}

fn parse_auth_header(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(AppError::InValidHeader)?;
    let auth_header = auth_header.to_str().map_err(|_| AppError::InValidHeader)?;
    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::InValidHeader);
    }
    let token = &auth_header[7..];
    Ok(token.to_string())
}

#[derive(Deserialize)]
pub struct JwksResponse {
    keys: Vec<Jwk>,
}

// 毎回アクセスするのはコストがかかるので、本当はキャッシュする
pub async fn fetch_jwks(
    auth_server_url: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let resp = reqwest::get(format!("{}/.well-known/jwks.json", auth_server_url))
        .await?
        .json::<JwksResponse>()
        .await?;

    let mut map = HashMap::new();
    for jwk in resp.keys {
        // JWK から公開鍵を生成
        let public_key_pem = jwk_to_rsa_public_key_pem(&jwk)?;
        map.insert(jwk.kid, public_key_pem);
    }

    Ok(map)
}

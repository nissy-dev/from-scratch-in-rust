use std::collections::HashMap;

use axum::{extract::State, http::HeaderMap};
use shared::jwt::{decode_jwt_rs256, jwk_to_rsa_public_key_pem};

use crate::{errors::AppError, AppState};

pub async fn resource(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<String, AppError> {
    // Authorization ヘッダから Bearer トークンを取得
    let token = parse_auth_header(&headers)?;

    // jwk に fetch して public_pem を取得する
    let resp = state
        .auth_server_client
        .jwks()
        .await
        .map_err(|e| AppError::FetchError(format!("Failed to fetch JWKS: {}", e)))?;
    let mut jwks = HashMap::new();
    for jwk in resp.keys {
        // JWK から公開鍵を生成
        let public_key_pem = jwk_to_rsa_public_key_pem(&jwk)
            .map_err(|e| AppError::JwkToRsaPublicKeyPemError(e.to_string()))?;
        jwks.insert(jwk.kid, public_key_pem);
    }
    let claims =
        decode_jwt_rs256(&token, &jwks).map_err(|e| AppError::JwtDecodeError(e.to_string()))?;
    if claims.aud != state.expected_audience {
        return Err(AppError::InvalidAudience);
    }

    // introspect のエンドポイントで token が active かどうかを確認する
    // token の発行と token の利用のタイミングが違うので、発行から利用までの間に token が失効したり、
    // scope などが変化してないかを auth server に問い合わせる
    let resp = state
        .auth_server_client
        .introspect_token(&token)
        .await
        .map_err(|e| AppError::FetchError(format!("Failed to introspect token: {}", e)))?;
    if !resp.active {
        return Err(AppError::TokenInactive);
    }

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

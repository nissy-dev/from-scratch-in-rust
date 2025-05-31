use axum::{
    extract::{Json, Query, State},
    http::HeaderMap,
    response::Redirect,
    Form,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, prelude::BASE64_STANDARD, Engine};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::jwt::{encode_jwt_rs256, rsa_public_key_to_jwk, Claims, Jwk};
use uuid::Uuid;

use crate::{errors::AppError, AppState};

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthorizeStoredData {
    code: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthorizeRequest {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: Option<String>,
}

pub async fn authorize(
    Query(params): Query<AuthorizeRequest>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let mut conn = state.redis.get_multiplexed_async_connection().await?;

    if params.response_type != "code" {
        return Err(AppError::InValidParameter);
    }

    // code_challenge_method が S256 であることを確認する
    // 本当は plain を指定することも可能だが、今回は S256 のみをサポートする
    // 安全性を考慮して、 S256 を使うことが推奨されている
    if params.code_challenge_method != "S256" {
        return Err(AppError::InValidParameter);
    }

    // 登録されている Client か確認する
    let key = format!("oauth2:client:{}", params.client_id);
    let str_data = conn.get::<_, String>(key).await?;
    let auth_client: OAuthClient = serde_json::from_str(&str_data)?;
    if auth_client.redirect_uri != params.redirect_uri {
        return Err(AppError::InValidParameter);
    }

    let auth_code = Uuid::new_v4().to_string();
    // redis にデータを保存する (5分有効にする)
    let auth_data = AuthorizeStoredData {
        code: auth_code.clone(),
        client_id: params.client_id.clone(),
        redirect_uri: params.redirect_uri.clone(),
        state: params.state.clone(),
        code_challenge: params.code_challenge.clone(),
        code_challenge_method: params.code_challenge_method.clone(),
        scope: params.scope.clone(),
    };
    let serialized = serde_json::to_string(&auth_data)?;
    let key = format!("oauth2:code:{}", auth_code);
    conn.set_ex::<_, _, String>(key, serialized, 300).await?;

    let redirect_uri = format!(
        "{}?code={}&state={}",
        params.redirect_uri, &auth_code, params.state
    );
    Ok(Redirect::to(&redirect_uri))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TokenRequest {
    grant_type: String,
    // Public client の場合は、 code_verifier, code, redirect_uri を使う。
    code_verifier: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    // confidential client の場合は、 scope を使う。
    // これは authorize リクエストの scope と同じものを指定する。
    scope: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    scope: Option<String>,
}

pub async fn token(
    headers: HeaderMap,
    State(state): State<AppState>,
    Form(params): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    match params.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_flow(&state, &params).await,
        "client_credentials" => handle_client_credentials_flow(&state, &params, &headers).await,
        _ => Err(AppError::InValidParameter),
    }
}

async fn handle_authorization_code_flow(
    state: &AppState,
    params: &TokenRequest,
) -> Result<Json<TokenResponse>, AppError> {
    let code = params.code.as_ref().ok_or(AppError::InValidParameter)?;
    let code_verifier = params
        .code_verifier
        .as_ref()
        .ok_or(AppError::InValidParameter)?;
    let redirect_uri = params
        .redirect_uri
        .as_ref()
        .ok_or(AppError::InValidParameter)?;

    let mut conn = state.redis.get_multiplexed_async_connection().await?;

    let key = format!("oauth2:code:{}", code);
    let str_data = conn.get::<_, String>(key).await?;
    let auth_data: AuthorizeStoredData = serde_json::from_str(&str_data)?;
    let scope = auth_data.scope.as_ref();

    // redis から取得した redirect_uri と リクエストから来る redirect_uri が一致するか確認する
    if &auth_data.redirect_uri != redirect_uri {
        return Err(AppError::InValidParameter);
    }

    // redis から取得した code_challenge と リクエストから来る code_verifier が一致するか検証する
    // 今回は S256 のみをサポートするので、 code_verifier を SHA256 でハッシュ化して、 base64url エンコードする
    let hash = Sha256::digest(code_verifier.as_bytes());
    let generated_code_challenge = URL_SAFE_NO_PAD.encode(hash);
    if generated_code_challenge != auth_data.code_challenge {
        return Err(AppError::InValidParameter);
    }

    let claims = Claims::new(
        "http://localhost:3123".to_string(),
        // resource server の URL
        "http://localhost:6244".to_string(),
        // authorization_code_flow の場合はユーザ認証して、ユーザー ID を使う場合が多い
        uuid::Uuid::new_v4().to_string(),
        scope.cloned(),
    );
    let jwt = encode_jwt_rs256(&claims, &state.key_id, &state.private_key)
        .map_err(|e| AppError::JwtEncodeError(e.to_string()))?;

    Ok(Json(TokenResponse {
        access_token: jwt,
        token_type: "Bearer".into(),
        expires_in: claims.exp - claims.iat,
        scope: scope.cloned(),
    }))
}

async fn handle_client_credentials_flow(
    state: &AppState,
    params: &TokenRequest,
    headers: &HeaderMap,
) -> Result<Json<TokenResponse>, AppError> {
    let scope = params.scope.as_ref();
    let mut conn = state.redis.get_multiplexed_async_connection().await?;

    // Basic ヘッダーから client_id と client_secret を取得する
    let (client_id, client_secret) = parse_basic_auth(headers)?;

    // redis から取得した client_secret と リクエストから来る client_secret が一致するか確認する
    let key = format!("oauth2:client:{}", client_id);
    let str_data = conn.get::<_, String>(key).await?;
    let auth_client: OAuthClient = serde_json::from_str(&str_data)?;
    if auth_client.client_secret != client_secret {
        return Err(AppError::InValidParameter);
    }

    let claims = Claims::new(
        "http://localhost:3123".to_string(),
        // resource server の URL
        "http://localhost:6244".to_string(),
        // client_credentials_flow の場合は client_id を使う
        client_id,
        scope.cloned(),
    );
    let jwt = encode_jwt_rs256(&claims, &state.key_id, &state.private_key)
        .map_err(|e| AppError::JwtEncodeError(e.to_string()))?;

    Ok(Json(TokenResponse {
        access_token: jwt,
        token_type: "Bearer".into(),
        expires_in: claims.exp - claims.iat,
        scope: scope.cloned(),
    }))
}

fn parse_basic_auth(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(AppError::InValidHeader)?;
    let auth_header = auth_header.to_str().map_err(|_| AppError::InValidHeader)?;
    if !auth_header.starts_with("Basic ") {
        return Err(AppError::InValidHeader);
    }
    let base64_str = &auth_header[6..];
    let decoded = BASE64_STANDARD
        .decode(base64_str)
        .map_err(|_| AppError::InValidHeader)?;
    let decoded_str = String::from_utf8(decoded).map_err(|_| AppError::InValidHeader)?;
    let mut parts = decoded_str.split(':');
    let client_id = parts.next().ok_or(AppError::InValidHeader)?.to_string();
    let client_secret = parts.next().ok_or(AppError::InValidHeader)?.to_string();
    Ok((client_id, client_secret))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OAuthClient {
    name: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[derive(Deserialize)]
pub struct CreateClientRequest {
    name: String,
    redirect_uri: String,
}

#[derive(Serialize)]
pub struct CreateClientResponse {
    client_id: String,
    redirect_uri: String,
    // 本来は client_secret を返すべきではないが、デモ用に返す
    client_secret: String,
}

pub async fn create_client(
    State(state): State<AppState>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<Json<CreateClientResponse>, AppError> {
    let client_id = Uuid::new_v4().to_string();
    let client_secret = Uuid::new_v4().to_string();

    // redis にデータを保存しているが、実際には DB に保存して永続化する
    let client = OAuthClient {
        name: payload.name,
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_uri: payload.redirect_uri.clone(),
    };
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    let serialized = serde_json::to_string(&client)?;
    let key = format!("oauth2:client:{}", client_id);
    conn.set_nx::<_, _, String>(key, serialized).await?;

    Ok(Json(CreateClientResponse {
        client_id,
        redirect_uri: payload.redirect_uri,
        client_secret,
    }))
}

#[derive(Serialize)]
pub struct JwksResponse {
    keys: Vec<Jwk>,
}

pub async fn jwks_handler(State(state): State<AppState>) -> Result<Json<JwksResponse>, AppError> {
    let jwk = rsa_public_key_to_jwk(&state.public_key, &state.key_id)
        .map_err(|e| AppError::JwkCreateError(e.to_string()))?;
    Ok(Json(JwksResponse { keys: vec![jwk] }))
}

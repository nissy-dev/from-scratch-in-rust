use axum::{
    extract::{Json, Query, State},
    http::HeaderMap,
    response::Redirect,
    Form,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, prelude::BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::jwt::{encode_jwt_rs256, rsa_public_key_to_jwk, Claims, Jwk};
use uuid::Uuid;

use crate::{
    errors::AppError,
    store::{AuthorizeCodeData, ClientData, TokenData},
    AppState,
};

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
    State(mut state): State<AppState>,
) -> Result<Redirect, AppError> {
    // response_type が code であることを確認する
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
    let auth_client = state.store.read_client_data(&params.client_id).await?;
    if auth_client.redirect_uri != params.redirect_uri {
        return Err(AppError::InValidParameter);
    }

    // 5分間有効な認可コードを生成する
    let auth_code = Uuid::new_v4().to_string();
    let auth_data = AuthorizeCodeData {
        code: auth_code.clone(),
        client_id: params.client_id.clone(),
        redirect_uri: params.redirect_uri.clone(),
        state: params.state.clone(),
        code_challenge: params.code_challenge.clone(),
        code_challenge_method: params.code_challenge_method.clone(),
        scope: params.scope.clone(),
    };
    state
        .store
        .write_auth_code_data(&auth_code, &auth_data, 300)
        .await?;

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
    State(mut state): State<AppState>,
    Form(params): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    match params.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_flow(&mut state, &params).await,
        "client_credentials" => handle_client_credentials_flow(&mut state, &params, &headers).await,
        _ => Err(AppError::InValidParameter),
    }
}

async fn handle_authorization_code_flow(
    state: &mut AppState,
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

    // 認可コードから redis に保存していたデータを取得
    let auth_code_data = state.store.read_auth_code_data(code).await?;

    // 取得した redirect_uri と リクエストから来る redirect_uri が一致するか確認する
    if &auth_code_data.redirect_uri != redirect_uri {
        return Err(AppError::InValidParameter);
    }

    // 取得した code_challenge と リクエストから来る code_verifier が一致するか検証する
    // 今回は S256 のみをサポートするので、 code_verifier を SHA256 でハッシュ化して、 base64url エンコードする
    let hash = Sha256::digest(code_verifier.as_bytes());
    let generated_code_challenge = URL_SAFE_NO_PAD.encode(hash);
    if generated_code_challenge != auth_code_data.code_challenge {
        return Err(AppError::InValidParameter);
    }

    let scope = auth_code_data.scope.as_ref();
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

    // トークン情報を redis に保存する
    let token_data = TokenData { active: true };
    let expires_in = claims.exp - claims.iat;
    state
        .store
        .write_token_data(&jwt, &token_data, expires_in as u64)
        .await?;

    Ok(Json(TokenResponse {
        access_token: jwt,
        token_type: "Bearer".into(),
        expires_in,
        scope: scope.cloned(),
    }))
}

async fn handle_client_credentials_flow(
    state: &mut AppState,
    params: &TokenRequest,
    headers: &HeaderMap,
) -> Result<Json<TokenResponse>, AppError> {
    let scope = params.scope.as_ref();

    // Basic ヘッダーから client_id と client_secret を取得する
    let (client_id, client_secret) = parse_basic_auth(headers)?;

    let client_data = state.store.read_client_data(&client_id).await?;
    if client_data.client_secret != client_secret {
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

// 今回は適当だが、Dynamic Client Registration の仕様に従うことが望ましい
pub async fn create_client(
    State(mut state): State<AppState>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<Json<CreateClientResponse>, AppError> {
    let client_id = Uuid::new_v4().to_string();
    let client_secret = Uuid::new_v4().to_string();

    // redis にデータを保存しているが、実際には DB に保存して永続化する
    let client = ClientData {
        name: payload.name,
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_uri: payload.redirect_uri.clone(),
    };
    state.store.write_client_data(&client_id, &client).await?;

    Ok(Json(CreateClientResponse {
        client_id,
        redirect_uri: payload.redirect_uri,
        client_secret,
    }))
}

#[derive(Deserialize)]
pub struct IntrospectRequest {
    token: String,
}

#[derive(Serialize)]
pub struct IntrospectResponse {
    active: bool,
}

pub async fn introspect(
    State(mut state): State<AppState>,
    Form(payload): Form<IntrospectRequest>,
) -> Result<Json<IntrospectResponse>, AppError> {
    let token_data = state
        .store
        .read_token_data(&payload.token)
        .await
        .map_or_else(|_| TokenData { active: false }, |data| data);

    Ok(Json(IntrospectResponse {
        active: token_data.active,
    }))
}

#[derive(Serialize)]
pub struct JwksResponse {
    keys: Vec<Jwk>,
}

pub async fn jwks(State(state): State<AppState>) -> Result<Json<JwksResponse>, AppError> {
    let jwk = rsa_public_key_to_jwk(&state.public_key, &state.key_id)
        .map_err(|e| AppError::JwkCreateError(e.to_string()))?;
    Ok(Json(JwksResponse { keys: vec![jwk] }))
}

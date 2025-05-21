use axum::{
    extract::{Json, Query, State},
    response::Redirect,
    Form,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{errors::AppError, AppState};

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

    // 登録されている Client か確認する
    let key = format!("oauth2:client:{}", params.client_id);
    let str_data = conn.get::<_, String>(key).await?;
    let auth_client: OAuthClient = serde_json::from_str(&str_data)?;
    if auth_client.redirect_uri != params.redirect_uri {
        return Err(AppError::InValidParameter);
    }

    let auth_code = Uuid::new_v4().to_string();
    // redis にデータを保存する (5分有効にする)
    let serialized = serde_json::to_string(&params)?;
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
    code: String,
    redirect_uri: String,
    // Public client の場合は、 code_verifier を使う。
    // Credential client の場合は、あらかじめ登録しておいた client_id と client_secret を使う。
    code_verifier: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    scope: Option<String>,
}

#[axum::debug_handler]
pub async fn token(
    State(state): State<AppState>,
    Form(params): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // 今回は OAuth の認可コードフローを想定しているので、 grant_type は authorization_code であることを確認する
    if params.grant_type != "authorization_code" {
        return Err(AppError::InValidParameter);
    }

    let mut conn = state.redis.get_multiplexed_async_connection().await?;

    let key = format!("oauth2:code:{}", params.code);
    let str_data = conn.get::<_, String>(key).await?;
    let auth_data: AuthorizeRequest = serde_json::from_str(&str_data)?;

    // redis から取得した redirect_uri と リクエストから来る redirect_uri が一致するか確認する
    if auth_data.redirect_uri != params.redirect_uri {
        return Err(AppError::InValidParameter);
    }

    // redis から取得した code_challenge と リクエストから来る code_verifier が一致するか検証する
    // 本当は code_challenge_method で指定された方法で検証する必要があるが、
    // 今回は S256 だと仮定して検証している。(基本的には S256 が使われることが多い)
    let hash = Sha256::digest(&params.code_verifier.as_bytes());
    let generated_code_challenge = URL_SAFE_NO_PAD.encode(hash);
    if generated_code_challenge != auth_data.code_challenge {
        return Err(AppError::InValidParameter);
    }

    Ok(Json(TokenResponse {
        access_token: Uuid::new_v4().to_string(),
        token_type: "Bearer".into(),
        expires_in: 3600,
        scope: auth_data.scope,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OAuthClient {
    name: String,
    client_id: String,
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
}

pub async fn create_client(
    State(state): State<AppState>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<Json<CreateClientResponse>, AppError> {
    let client_id = Uuid::new_v4().to_string();

    // redis にデータを保存しているが、実際には DB に保存して永続化する
    let client = OAuthClient {
        name: payload.name,
        client_id: client_id.clone(),
        redirect_uri: payload.redirect_uri.clone(),
    };
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    let serialized = serde_json::to_string(&client)?;
    let key = format!("oauth2:client:{}", client_id);
    conn.set_nx::<_, _, String>(key, serialized).await?;

    Ok(Json(CreateClientResponse {
        client_id,
        redirect_uri: payload.redirect_uri,
    }))
}

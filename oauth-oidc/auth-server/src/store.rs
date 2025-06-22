use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};

use crate::errors::AppError;

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientData {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthorizeCodeData {
    pub code: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: Option<String>,
    pub nonce: Option<String>,
}

// introspect の処理で利用する
#[derive(Deserialize, Serialize, Debug)]
pub struct TokenData {
    pub active: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SessionData {
    pub user_name: String,
    pub user_id: String,
}

// 本当は user_id を設定したり、Password のハッシュ化とかも必要だけど、
// 今回は簡略化のために省略する
#[derive(Deserialize, Serialize, Debug)]
pub struct UserData {
    pub id: String,
    pub name: String,
    pub password: String,
}

pub struct Store {
    client: Arc<redis::Client>,
}

impl Store {
    pub const CLIENT: &'static str = "oauth2:client";
    pub const AUTH_CODE: &'static str = "oauth2:code";
    pub const TOKEN: &'static str = "oauth2:token";
    pub const SESSION: &'static str = "oauth2:session";
    pub const USER: &'static str = "oauth2:user";

    pub fn new(client: redis::Client) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    pub async fn write_client_data(
        &mut self,
        client_id: &str,
        data: &ClientData,
    ) -> Result<(), AppError> {
        self.write(Self::CLIENT, client_id, data, None).await
    }

    pub async fn read_client_data(&mut self, client_id: &str) -> Result<ClientData, AppError> {
        self.read(Self::CLIENT, client_id).await
    }

    pub async fn write_auth_code_data(
        &mut self,
        code: &str,
        data: &AuthorizeCodeData,
        expiry_in: u64,
    ) -> Result<(), AppError> {
        self.write(Self::AUTH_CODE, code, data, Some(expiry_in))
            .await
    }

    pub async fn read_auth_code_data(&mut self, code: &str) -> Result<AuthorizeCodeData, AppError> {
        self.read(Self::AUTH_CODE, code).await
    }

    pub async fn write_token_data(
        &mut self,
        token: &str,
        data: &TokenData,
        expiry_in: u64,
    ) -> Result<(), AppError> {
        self.write(Self::TOKEN, token, data, Some(expiry_in)).await
    }

    pub async fn read_token_data(&mut self, token: &str) -> Result<TokenData, AppError> {
        self.read(Self::TOKEN, token).await
    }

    pub async fn write_session_data(
        &mut self,
        session_id: &str,
        data: &SessionData,
    ) -> Result<(), AppError> {
        self.write(Self::SESSION, session_id, data, None).await
    }

    pub async fn read_session_data(&mut self, session_id: &str) -> Result<SessionData, AppError> {
        self.read(Self::SESSION, session_id).await
    }

    pub async fn write_user_data(&mut self, data: &UserData) -> Result<(), AppError> {
        self.write(Self::USER, &data.name, data, None).await
    }

    pub async fn read_user_data(&mut self, name: &str) -> Result<UserData, AppError> {
        self.read(Self::USER, name).await
    }

    async fn write<T: Serialize>(
        &mut self,
        key_prefix: &str,
        key_id: &str,
        data: &T,
        expiry_seconds: Option<u64>,
    ) -> Result<(), AppError> {
        let key = self.make_key(key_prefix, key_id);
        let serialized = serde_json::to_string(data)?;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        match expiry_seconds {
            Some(seconds) => conn.set_ex::<_, _, String>(key, serialized, seconds).await,
            None => conn.set_nx::<_, _, String>(key, serialized).await,
        }?;
        Ok(())
    }

    pub async fn read<T: DeserializeOwned + Debug>(
        &mut self,
        key_prefix: &str,
        key_id: &str,
    ) -> Result<T, AppError> {
        let key = self.make_key(key_prefix, key_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let data: String = conn.get(key).await?;
        Ok(serde_json::from_str(&data)?)
    }

    fn make_key(&self, prefix: &str, id: &str) -> String {
        format!("{}:{}", prefix, id)
    }
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
        }
    }
}

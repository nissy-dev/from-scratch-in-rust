use serde::Deserialize;
use shared::jwt::Jwk;

#[derive(Debug, Clone)]
pub struct AuthServerClient {
    base_url: String,
}

#[derive(Deserialize)]
pub struct IntrospectResponse {
    pub active: bool,
}

#[derive(Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<Jwk>,
}

impl AuthServerClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn introspect_token(
        &self,
        token: &str,
    ) -> Result<IntrospectResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/introspect", self.base_url))
            .form(&[("token", token)])
            .send()
            .await?
            .json::<IntrospectResponse>()
            .await?;
        Ok(resp)
    }

    pub async fn jwks(&self) -> Result<JwksResponse, Box<dyn std::error::Error>> {
        let resp = reqwest::get(format!("{}/.well-known/jwks.json", self.base_url))
            .await?
            .json::<JwksResponse>()
            .await?;
        Ok(resp)
    }
}

// JWT の署名と検証を行うモジュール
// 今回は推奨されている非対称鍵認証方式のうち RS256 アルゴリズムを使用するが、
// production では ECDSA などのより安全なアルゴリズムを使用する方が良い。

use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rsa::{
    pkcs1v15,
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey, LineEnding},
    signature::{RandomizedSigner, SignatureEncoding, Verifier},
    traits::PublicKeyParts,
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    // JWT の発行者
    pub iss: String,
    // JWT の受信者
    pub aud: String,
    // 有効期限、UNIX タイムスタンプ
    pub exp: i64,
    // JWT の用途の識別子 (ユーザーIDなど)
    pub sub: String,
    // JWT の発行日時、UNIX タイムスタンプ
    pub iat: i64,
    // JWT の識別子
    pub jti: String,
    // JWT のスコープ
    pub scope: Option<String>,
    // 以下は id token のための拡張
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Claims {
    pub fn new(iss: String, aud: String, sub: String, scope: Option<String>) -> Self {
        let now = Utc::now().timestamp();
        let exp = now + 3600; // 1時間後に有効期限を設定

        Claims {
            iss,
            aud,
            exp,
            sub,
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
            scope,
            nonce: None,
            name: None,
        }
    }

    pub fn new_id_token(
        iss: String,
        aud: String,
        sub: String,
        scope: Option<String>,
        nonce: Option<String>,
        name: Option<String>,
    ) -> Self {
        let iat = chrono::Utc::now().timestamp();
        let exp = iat + 3600; // 1時間後に有効期限を設定
        Claims {
            iss,
            sub,
            aud,
            exp,
            iat,
            jti: uuid::Uuid::new_v4().to_string(),
            scope,
            nonce,
            name,
        }
    }
}

pub fn encode_jwt_rs256(
    claims: &Claims,
    kid: &str,
    private_pem: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let header = JwtHeader {
        alg: "RS256".to_string(),
        typ: "JWT".to_string(),
        kid: kid.to_string(),
    };
    let header_json = serde_json::to_string(&header)?;
    let payload_json = serde_json::to_string(&claims)?;

    let header_enc = URL_SAFE_NO_PAD.encode(header_json);
    let payload_enc = URL_SAFE_NO_PAD.encode(payload_json);
    let signing_input = format!("{}.{}", header_enc, payload_enc);

    // 署名対象のデータをハッシュ化
    let hashed = Sha256::digest(signing_input.as_bytes());

    // 秘密鍵で署名
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_pem)?;
    let signing_key = pkcs1v15::SigningKey::<sha2::Sha256>::new(private_key);
    let mut rng = rand::thread_rng(); // rand@v8 しか使えない
    let signature = signing_key.sign_with_rng(&mut rng, &hashed);

    // 署名をエンコード
    let signature_enc = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{}.{}.{}", header_enc, payload_enc, signature_enc))
}

pub fn decode_jwt_rs256(
    token: &str,
    jwks: &HashMap<String, String>,
) -> Result<Claims, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid token format".into());
    }

    let (header_enc, payload_enc, signature_enc) = (parts[0], parts[1], parts[2]);
    let signing_input = format!("{}.{}", header_enc, payload_enc);

    let header: JwtHeader = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header_enc)?)?;
    let public_key_pem = jwks
        .get(&header.kid)
        .ok_or("public key not found for kid")?;

    // 公開鍵で署名を検証
    let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)?;
    let verifying_key = pkcs1v15::VerifyingKey::<sha2::Sha256>::new(public_key);

    let hashed = Sha256::digest(signing_input.as_bytes());
    let signature = URL_SAFE_NO_PAD.decode(signature_enc)?;
    let signature = pkcs1v15::Signature::try_from(signature.as_slice())?;
    verifying_key.verify(&hashed, &signature)?;

    let payload_json = String::from_utf8(URL_SAFE_NO_PAD.decode(payload_enc)?)?;
    let claims: Claims = serde_json::from_str(&payload_json)?;

    let now = Utc::now().timestamp();
    if claims.exp < now {
        return Err("token expired".into());
    }

    Ok(claims)
}

// JWK は 暗号鍵を表現するための JSON 形式のデータ構造
#[derive(Serialize, Deserialize)]
pub struct Jwk {
    // JWT の署名アルゴリズムのファミリータイプ
    kty: String,
    // JWT の署名アルゴリズム
    alg: String,
    // JWT の用途 (例: "sig" は署名用)
    #[serde(rename = "use")]
    use_: String,
    // 鍵の識別子
    pub kid: String,
    // RSA 鍵のパラメータ、モジュラス
    n: String,
    // RSA 鍵のパラメータ、指数
    e: String,
}

pub fn rsa_public_key_to_jwk(pem: &str, kid: &str) -> Result<Jwk, Box<dyn std::error::Error>> {
    let public_key = RsaPublicKey::from_public_key_pem(pem)?;
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

    Ok(Jwk {
        kty: "RSA".to_string(),
        alg: "RS256".to_string(),
        use_: "sig".to_string(),
        kid: kid.to_string(),
        n,
        e,
    })
}

pub fn jwk_to_rsa_public_key_pem(jwk: &Jwk) -> Result<String, Box<dyn std::error::Error>> {
    let n_bytes = URL_SAFE_NO_PAD.decode(&jwk.n)?;
    let e_bytes = URL_SAFE_NO_PAD.decode(&jwk.e)?;

    let n = rsa::BigUint::from_bytes_be(&n_bytes);
    let e = rsa::BigUint::from_bytes_be(&e_bytes);

    let key = RsaPublicKey::new(n, e)?;
    key.to_public_key_pem(LineEnding::LF).map_err(|e| e.into())
}

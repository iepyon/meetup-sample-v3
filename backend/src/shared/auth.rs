use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use super::error::{ApiError, ApiResult};

/// トークン有効期限（24 時間）
const TOKEN_EXPIRY_SECONDS: u64 = 86_400;

/// HS256 の署名鍵として受け付ける最小バイト数（総当たりで鍵を推測されないよう 256 bit 以上）
pub const JWT_SECRET_MIN_BYTES: usize = 32;

/// 起動時に `JWT_SECRET` を検証する。未設定・空白のみ・短すぎる値は起動を拒否する
/// （既知の既定値や推測しやすい値で動かさない）。
pub fn validate_jwt_secret(value: Option<&str>) -> Result<String, String> {
    let Some(secret) = value.filter(|v| !v.trim().is_empty()) else {
        return Err("JWT_SECRET 環境変数が未設定です。32 バイト以上のランダム値を設定してください（例: openssl rand -hex 32）。".into());
    };
    if secret.len() < JWT_SECRET_MIN_BYTES {
        return Err(format!(
            "JWT_SECRET が {JWT_SECRET_MIN_BYTES} バイト未満です。32 バイト以上のランダム値を設定してください（例: openssl rand -hex 32）。"
        ));
    }
    Ok(secret.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    #[serde(rename = "accountId")]
    account_id: String,
    iat: u64,
    exp: u64,
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn generate_token(
    account_id: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let iat = now_seconds();
    let claims = Claims {
        account_id: account_id.to_string(),
        iat,
        exp: iat + TOKEN_EXPIRY_SECONDS,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

fn verify_token(token: &str, secret: &str) -> Option<String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 0;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .map(|data| data.claims.account_id)
}

/// `Authorization: Bearer <token>` から accountId を取り出す。無効なら None。
pub fn optional_account(headers: &HeaderMap, secret: &str) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    verify_token(token, secret)
}

/// 認証必須。無効なら 401。
pub fn require_account(headers: &HeaderMap, secret: &str) -> ApiResult<String> {
    optional_account(headers, secret).ok_or_else(ApiError::unauthorized)
}

#[cfg(test)]
mod tests;

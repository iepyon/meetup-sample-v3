use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use super::error::{ApiError, ApiResult};

/// トークン有効期限（24 時間）
const TOKEN_EXPIRY_SECONDS: u64 = 86_400;

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
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const SECRET: &str = "unit-secret";

    fn headers(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        h
    }

    fn encode_claims(claims: &serde_json::Value, alg: Algorithm, secret: &str) -> String {
        encode(
            &Header::new(alg),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn 有効なトークンからaccount_idを取り出せる() {
        let token = generate_token("acc-1", SECRET).unwrap();
        assert_eq!(
            optional_account(&headers(&format!("Bearer {token}")), SECRET).as_deref(),
            Some("acc-1")
        );
    }

    #[test]
    fn 期限切れのトークンは無効() {
        let past = now_seconds() - 10;
        let token = encode_claims(
            &serde_json::json!({ "accountId": "a", "iat": past - 100, "exp": past }),
            Algorithm::HS256,
            SECRET,
        );
        assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
    }

    #[test]
    fn 別のsecretで署名されたトークンは無効() {
        let token = generate_token("a", "other-secret").unwrap();
        assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
    }

    #[test]
    fn alg違いのトークンは無効() {
        let exp = now_seconds() + 1000;
        let token = encode_claims(
            &serde_json::json!({ "accountId": "a", "iat": 0, "exp": exp }),
            Algorithm::HS384,
            SECRET,
        );
        assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
    }

    #[test]
    fn account_idが無い_または数値のトークンは無効() {
        let exp = now_seconds() + 1000;
        for claims in [
            serde_json::json!({ "iat": 0, "exp": exp }),
            serde_json::json!({ "accountId": 1, "iat": 0, "exp": exp }),
        ] {
            let token = encode_claims(&claims, Algorithm::HS256, SECRET);
            assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
        }
    }

    #[test]
    fn bearer形式でないヘッダは無効() {
        let token = generate_token("a", SECRET).unwrap();
        assert!(optional_account(&headers(&token), SECRET).is_none());
        assert!(optional_account(&headers(&format!("Basic {token}")), SECRET).is_none());
        assert!(optional_account(&HeaderMap::new(), SECRET).is_none());
        assert!(require_account(&HeaderMap::new(), SECRET).is_err());
    }
}

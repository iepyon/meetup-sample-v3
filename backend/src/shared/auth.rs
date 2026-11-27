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
mod tests {

    use super::*;
    use axum::http::HeaderValue;
    use meetup_test_macros::describe;

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

    describe!("JWT 認証", {
        test!(
            "JWT_SECRET は未設定と 32 バイト未満を拒否する",
            {
                assert!(super::validate_jwt_secret(None).is_err());
                assert!(super::validate_jwt_secret(Some("   ")).is_err());
                assert!(super::validate_jwt_secret(Some(&"a".repeat(31))).is_err());
                assert_eq!(
                    super::validate_jwt_secret(Some(&"a".repeat(32))).unwrap(),
                    "a".repeat(32)
                );
            }
        );

        test!(
            "有効なトークンから account_id を取り出せる",
            {
                let token = generate_token("acc-1", SECRET).unwrap();
                assert_eq!(
                    optional_account(&headers(&format!("Bearer {token}")), SECRET).as_deref(),
                    Some("acc-1")
                );
            }
        );

        test!("期限切れのトークンは無効", {
            let past = now_seconds() - 10;
            let token = encode_claims(
                &serde_json::json!({ "accountId": "a", "iat": past - 100, "exp": past }),
                Algorithm::HS256,
                SECRET,
            );
            assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
        });

        test!("別の secret で署名されたトークンは無効", {
            let token = generate_token("a", "other-secret").unwrap();
            assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
        });

        test!("alg 違いのトークンは無効", {
            let exp = now_seconds() + 1000;
            let token = encode_claims(
                &serde_json::json!({ "accountId": "a", "iat": 0, "exp": exp }),
                Algorithm::HS384,
                SECRET,
            );
            assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
        });

        test!(
            "account_id が無い、または数値のトークンは無効",
            {
                let exp = now_seconds() + 1000;
                for claims in [
                    serde_json::json!({ "iat": 0, "exp": exp }),
                    serde_json::json!({ "accountId": 1, "iat": 0, "exp": exp }),
                ] {
                    let token = encode_claims(&claims, Algorithm::HS256, SECRET);
                    assert!(
                        optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none()
                    );
                }
            }
        );

        test!("Bearer 形式でないヘッダは無効", {
            let token = generate_token("a", SECRET).unwrap();
            assert!(optional_account(&headers(&token), SECRET).is_none());
            assert!(optional_account(&headers(&format!("Basic {token}")), SECRET).is_none());
            assert!(optional_account(&HeaderMap::new(), SECRET).is_none());
            assert!(require_account(&HeaderMap::new(), SECRET).is_err());
        });
    });
}

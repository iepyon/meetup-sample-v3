//! `shared/auth.rs` の単体テスト

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
                assert!(optional_account(&headers(&format!("Bearer {token}")), SECRET).is_none());
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

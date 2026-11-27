use std::collections::HashMap;

use axum::body::Bytes;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use super::error::{ApiError, ApiResult};

// ============================================================
// リクエスト検証（旧 express-openapi-validator 相当）
// 違反は 400 `{ code: 'VALIDATION_ERROR', message }` に統一する。
// ============================================================

pub type JsonObject = Map<String, Value>;

/// JSON オブジェクトのボディをパースする。空・非 JSON は 400。
pub fn json_object(body: &Bytes) -> ApiResult<JsonObject> {
    let value: Value = serde_json::from_slice(body).map_err(|_| ApiError::invalid_json())?;
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(ApiError::validation("request/body must be object")),
    }
}

fn missing(key: &str) -> ApiError {
    ApiError::validation(format!("request/body must have required property '{key}'"))
}

fn type_error(key: &str, expected: &str) -> ApiError {
    ApiError::validation(format!("request/body/{key} must be {expected}"))
}

pub fn required_string(obj: &JsonObject, key: &str, min: usize, max: usize) -> ApiResult<String> {
    match obj.get(key) {
        None => Err(missing(key)),
        Some(Value::String(s)) => check_len(key, s, min, max).map(|()| s.clone()),
        Some(_) => Err(type_error(key, "string")),
    }
}

/// `string | null` かつ省略可能なフィールド。
pub fn optional_nullable_string(
    obj: &JsonObject,
    key: &str,
    max: usize,
) -> ApiResult<Option<String>> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => check_len(key, s, 0, max).map(|()| Some(s.clone())),
        Some(_) => Err(type_error(key, "string,null")),
    }
}

fn check_len(key: &str, s: &str, min: usize, max: usize) -> ApiResult<()> {
    let len = s.chars().count();
    if len < min {
        return Err(ApiError::validation(format!(
            "request/body/{key} must NOT have fewer than {min} characters"
        )));
    }
    if len > max {
        return Err(ApiError::validation(format!(
            "request/body/{key} must NOT have more than {max} characters"
        )));
    }
    Ok(())
}

/// bcrypt が扱える最大バイト数（これを超える入力は黙って切り捨てられるため拒否する）
pub const BCRYPT_MAX_BYTES: usize = 72;

/// 新規登録時のパスワードの最小文字数（NIST SP 800-63B の最低値）
pub const PASSWORD_MIN_CHARS: usize = 8;

/// ログイン用。既存アカウントを締め出さないよう最小長は課さない。
pub fn required_password(obj: &JsonObject, key: &str) -> ApiResult<String> {
    password_with_min(obj, key, 1)
}

/// 新規登録用。推測されやすい短いパスワードを拒否する。
pub fn required_new_password(obj: &JsonObject, key: &str) -> ApiResult<String> {
    password_with_min(obj, key, PASSWORD_MIN_CHARS)
}

fn password_with_min(obj: &JsonObject, key: &str, min: usize) -> ApiResult<String> {
    let value = required_string(obj, key, min, usize::MAX)?;
    if value.len() > BCRYPT_MAX_BYTES {
        return Err(ApiError::validation(format!(
            "request/body/{key} must NOT be longer than {BCRYPT_MAX_BYTES} bytes"
        )));
    }
    Ok(value)
}

/// メールアドレスの最大長（RFC 5321 の実質上限）
pub const EMAIL_MAX_CHARS: usize = 254;

/// メールアドレス。ASCII の大文字を小文字にそろえて返す（大文字小文字違いの重複登録を防ぎ、ログインでも同じ値で照合する）。
/// DB 側の照合に使う SQLite の `lower()` も ASCII のみを変換するため、範囲をそろえて ASCII に限る。
pub fn required_email(obj: &JsonObject, key: &str) -> ApiResult<String> {
    let value = required_string(obj, key, 1, EMAIL_MAX_CHARS)?;
    if is_valid_email(&value) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err(ApiError::validation(format!(
            "request/body/{key} must match format \"email\""
        )))
    }
}

pub fn is_valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && !value.chars().any(char::is_whitespace)
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

pub fn required_enum<'a>(
    obj: &JsonObject,
    key: &str,
    allowed: &'a [&'a str],
) -> ApiResult<&'a str> {
    match obj.get(key) {
        None => Err(missing(key)),
        Some(Value::String(s)) => allowed.iter().copied().find(|a| a == s).ok_or_else(|| {
            ApiError::validation(format!(
                "request/body/{key} must be equal to one of the allowed values: {}",
                allowed.join(", ")
            ))
        }),
        Some(_) => Err(type_error(key, "string")),
    }
}

pub fn required_int(obj: &JsonObject, key: &str, min: i64, max: i64) -> ApiResult<i64> {
    let value = match obj.get(key) {
        None => return Err(missing(key)),
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
            .ok_or_else(|| type_error(key, "integer"))?,
        Some(_) => return Err(type_error(key, "integer")),
    };
    if value < min {
        return Err(ApiError::validation(format!(
            "request/body/{key} must be >= {min}"
        )));
    }
    if value > max {
        return Err(ApiError::validation(format!(
            "request/body/{key} must be <= {max}"
        )));
    }
    Ok(value)
}

pub fn required_datetime(obj: &JsonObject, key: &str) -> ApiResult<DateTime<Utc>> {
    let value = required_string(obj, key, 0, usize::MAX)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| {
            ApiError::validation(format!(
                "request/body/{key} must match format \"date-time\""
            ))
        })
}

// ------------------------------------------------------------
// パス・クエリ
// ------------------------------------------------------------

/// UUID 形式（8-4-4-4-12 の 16 進）を検証する。
pub fn uuid_param(name: &str, value: &str) -> ApiResult<String> {
    let parts: Vec<&str> = value.split('-').collect();
    let lens = [8usize, 4, 4, 4, 12];
    let ok = parts.len() == 5
        && parts
            .iter()
            .zip(lens)
            .all(|(p, l)| p.len() == l && p.chars().all(|c| c.is_ascii_hexdigit()));
    if ok {
        Ok(value.to_string())
    } else {
        Err(ApiError::validation(format!(
            "request/params/{name} must match format \"uuid\""
        )))
    }
}

/// `limit`（0..=100, 既定 20）と `offset`（0.., 既定 0）。
pub fn page_params(query: &HashMap<String, String>) -> ApiResult<(i64, i64)> {
    let limit = int_query(query, "limit", 0, Some(100))?.unwrap_or(20);
    let offset = int_query(query, "offset", 0, None)?.unwrap_or(0);
    Ok((limit, offset))
}

fn int_query(
    query: &HashMap<String, String>,
    key: &str,
    min: i64,
    max: Option<i64>,
) -> ApiResult<Option<i64>> {
    let Some(raw) = query.get(key) else {
        return Ok(None);
    };
    let value: i64 = raw
        .parse()
        .map_err(|_| ApiError::validation(format!("request/query/{key} must be integer")))?;
    if value < min {
        return Err(ApiError::validation(format!(
            "request/query/{key} must be >= {min}"
        )));
    }
    if let Some(max) = max
        && value > max
    {
        return Err(ApiError::validation(format!(
            "request/query/{key} must be <= {max}"
        )));
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn メールアドレスの形式を検証できる() {
        assert!(is_valid_email("a@example.com"));
        assert!(!is_valid_email("a@example"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("a b@example.com"));
        assert!(!is_valid_email("plain"));
    }

    #[test]
    fn uuid形式を検証できる() {
        assert!(uuid_param("id", "550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(uuid_param("id", "not-a-uuid").is_err());
        assert!(uuid_param("id", "me").is_err());
    }

    #[test]
    fn ページ指定の範囲外は400になる() {
        let mut q = HashMap::new();
        q.insert("limit".to_string(), "101".to_string());
        assert!(page_params(&q).is_err());
        q.insert("limit".to_string(), "10".to_string());
        assert_eq!(page_params(&q).unwrap(), (10, 0));
    }

    fn obj(v: serde_json::Value) -> JsonObject {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn 文字列の長さは文字数で境界を検証する() {
        let ok = obj(serde_json::json!({ "n": "あ".repeat(100) }));
        assert!(required_string(&ok, "n", 1, 100).is_ok());
        let long = obj(serde_json::json!({ "n": "あ".repeat(101) }));
        assert!(required_string(&long, "n", 1, 100).is_err());
        let empty = obj(serde_json::json!({ "n": "" }));
        assert!(required_string(&empty, "n", 1, 100).is_err());
        assert!(required_string(&obj(serde_json::json!({})), "n", 1, 100).is_err());
        assert!(required_string(&obj(serde_json::json!({ "n": 1 })), "n", 1, 100).is_err());
    }

    #[test]
    fn nullable文字列は省略とnullを許し上限を検証する() {
        assert_eq!(
            optional_nullable_string(&obj(serde_json::json!({})), "d", 5).unwrap(),
            None
        );
        assert_eq!(
            optional_nullable_string(&obj(serde_json::json!({ "d": null })), "d", 5).unwrap(),
            None
        );
        assert!(
            optional_nullable_string(&obj(serde_json::json!({ "d": "123456" })), "d", 5).is_err()
        );
        assert!(optional_nullable_string(&obj(serde_json::json!({ "d": 1 })), "d", 5).is_err());
    }

    #[test]
    fn 列挙値は許可された文字列だけを受け付ける() {
        let allowed = ["A", "B"];
        assert_eq!(
            required_enum(&obj(serde_json::json!({ "e": "A" })), "e", &allowed).unwrap(),
            "A"
        );
        assert!(required_enum(&obj(serde_json::json!({ "e": "C" })), "e", &allowed).is_err());
        assert!(required_enum(&obj(serde_json::json!({})), "e", &allowed).is_err());
    }

    #[test]
    fn 整数は範囲と型を検証する() {
        assert_eq!(
            required_int(&obj(serde_json::json!({ "c": 1 })), "c", 1, 1000).unwrap(),
            1
        );
        assert_eq!(
            required_int(&obj(serde_json::json!({ "c": 1.0 })), "c", 1, 1000).unwrap(),
            1
        );
        for bad in [
            serde_json::json!({ "c": 0 }),
            serde_json::json!({ "c": 1001 }),
            serde_json::json!({ "c": 1.5 }),
            serde_json::json!({ "c": "1" }),
            serde_json::json!({}),
        ] {
            assert!(
                required_int(&obj(bad.clone()), "c", 1, 1000).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn 日時はrfc3339のみ受け付ける() {
        assert!(
            required_datetime(
                &obj(serde_json::json!({ "t": "2030-01-01T10:00:00.000Z" })),
                "t"
            )
            .is_ok()
        );
        assert!(
            required_datetime(
                &obj(serde_json::json!({ "t": "2030-01-01T10:00:00+09:00" })),
                "t"
            )
            .is_ok()
        );
        assert!(required_datetime(&obj(serde_json::json!({ "t": "2030-01-01" })), "t").is_err());
        assert!(required_datetime(&obj(serde_json::json!({ "t": "nope" })), "t").is_err());
    }

    #[test]
    fn jsonボディはオブジェクトのみ受け付ける() {
        assert!(json_object(&Bytes::from_static(b"{}")).is_ok());
        for bad in [&b"[]"[..], b"null", b"", b"{oops"] {
            assert!(json_object(&Bytes::copy_from_slice(bad)).is_err());
        }
    }

    #[test]
    fn パスワードはbcryptの上限バイト数まで() {
        assert!(required_password(&obj(serde_json::json!({ "p": "a".repeat(72) })), "p").is_ok());
        assert!(required_password(&obj(serde_json::json!({ "p": "a".repeat(73) })), "p").is_err());
        // 日本語は 1 文字 3 バイトなので 25 文字で 75 バイト
        assert!(required_password(&obj(serde_json::json!({ "p": "あ".repeat(25) })), "p").is_err());
    }

    #[test]
    fn 新規登録のパスワードは8文字以上() {
        let p = |v: &str| obj(serde_json::json!({ "p": v }));
        assert!(required_new_password(&p("1234567"), "p").is_err());
        assert!(required_new_password(&p("12345678"), "p").is_ok());
        // ログインでは最小長を課さない（既存アカウントを締め出さない）
        assert!(required_password(&p("1"), "p").is_ok());
    }

    #[test]
    fn メールアドレスは小文字にそろえ254文字までに制限する() {
        let e = |v: &str| obj(serde_json::json!({ "e": v }));
        assert_eq!(
            required_email(&e("User@Example.COM"), "e").unwrap(),
            "user@example.com"
        );
        let long = format!("{}@example.com", "a".repeat(EMAIL_MAX_CHARS));
        assert!(required_email(&e(&long), "e").is_err());
    }

    #[test]
    fn ページ指定の不正値と境界() {
        let q = |k: &str, v: &str| HashMap::from([(k.to_string(), v.to_string())]);
        assert!(page_params(&q("limit", "-1")).is_err());
        assert!(page_params(&q("limit", "abc")).is_err());
        assert!(page_params(&q("offset", "-1")).is_err());
        assert_eq!(page_params(&q("limit", "100")).unwrap(), (100, 0));
        assert_eq!(page_params(&q("limit", "0")).unwrap(), (0, 0));
    }
}

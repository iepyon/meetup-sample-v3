use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{ALLOW, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::error::ApiError;

// ============================================================
// レート制限（IP ベース・固定ウィンドウ）
// ============================================================

const WINDOW: Duration = Duration::from_secs(60);
const GLOBAL_LIMIT: u32 = 300;
const PUBLIC_READ_LIMIT: u32 = 60;
/// バケット数がこの値を超えたら、期限切れのウィンドウを掃除する（メモリの無制限な増加を防ぐ）
const EVICT_THRESHOLD: usize = 10_000;
/// 掃除後もこの数を超える場合（偽装 IP の大量送信など）は、新規キーを受け付けず拒否する
const MAX_BUCKETS: usize = 50_000;
/// ログイン・登録のブルートフォース / bcrypt による CPU 消費対策（IP あたり/分）
const AUTH_LIMIT: u32 = 20;

/// (種別, IP) → (ウィンドウ開始時刻, カウント)
type Buckets = HashMap<(&'static str, String), (Instant, u32)>;

#[derive(Default)]
pub struct RateLimiter {
    enabled: bool,
    /// true のとき X-Forwarded-For の末尾（信頼する 1 段のプロキシが付与）を使う。
    /// 既定は false（直接公開時にヘッダ偽装で制限を回避されないよう、接続元 IP を使う）
    trust_proxy: bool,
    buckets: Mutex<Buckets>,
}

impl RateLimiter {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            trust_proxy: std::env::var("TRUST_PROXY").as_deref() == Ok("true"),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// 上限内なら true。
    fn check(&self, kind: &'static str, ip: &str, limit: u32) -> bool {
        if !self.enabled {
            return true;
        }
        // poison されても（fail-open にせず）内部状態を引き継いで継続する
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        if buckets.len() >= EVICT_THRESHOLD {
            buckets.retain(|_, (started, _)| now.duration_since(*started) < WINDOW);
        }
        // 掃除後も多すぎる場合（偽装 IP の大量送信など）は、新規キーを受け付けず拒否する
        if buckets.len() >= MAX_BUCKETS && !buckets.contains_key(&(kind, ip.to_string())) {
            return false;
        }
        let entry = buckets.entry((kind, ip.to_string())).or_insert((now, 0));
        if now.duration_since(entry.0) >= WINDOW {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= limit
    }
}

/// リバースプロキシ 1 段を信頼し、X-Forwarded-For の末尾を採用する（`trust proxy: 1` 相当）。
fn client_ip(req: &Request, trust_proxy: bool) -> String {
    if trust_proxy
        && let Some(xff) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        && let Some(last) = xff.split(',').next_back()
    {
        let ip = last.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 未ログイン到達可能な公開 GET パスか。
fn is_public_read_path(path: &str) -> bool {
    let segments: Vec<&str> = path.trim_end_matches('/').split('/').skip(1).collect();
    matches!(
        segments.as_slice(),
        ["communities"] | ["communities", _] | ["communities", _, "members" | "events"]
    )
}

fn too_many_requests() -> Response {
    ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "TOO_MANY_REQUESTS",
        "リクエストが多すぎます。しばらく待ってください",
    )
    .into_response()
}

/// レート制限 + Cache-Control 付与。
/// 既定は `private, no-store`。未ログインの公開 GET だけ短期 public キャッシュにする。
pub async fn limits_and_cache(
    State(limiter): State<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    let ip = client_ip(&req, limiter.trust_proxy);
    if !limiter.check("global", &ip, GLOBAL_LIMIT) {
        return too_many_requests();
    }

    let is_public_read = req.method() == Method::GET && is_public_read_path(req.uri().path());
    let has_auth = req.headers().contains_key(AUTHORIZATION);
    if is_public_read && !limiter.check("public", &ip, PUBLIC_READ_LIMIT) {
        return too_many_requests();
    }

    if req.method() == Method::POST
        && matches!(req.uri().path(), "/auth/login" | "/auth/register")
        && !limiter.check("auth", &ip, AUTH_LIMIT)
    {
        return too_many_requests();
    }

    let mut res = next.run(req).await;
    let value = if is_public_read && !has_auth {
        "public, max-age=30, stale-while-revalidate=60"
    } else {
        "private, no-store"
    };
    res.headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(value));
    res
}

/// axum 標準の本文なしエラー（405 / 413 / 415 など）を `{ code, message }` に揃える。
pub async fn normalize_errors(req: Request, next: Next) -> Response {
    let res = next.run(req).await;
    let is_json = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if is_json {
        return res;
    }
    let (code, message) = match res.status() {
        StatusCode::METHOD_NOT_ALLOWED => ("METHOD_NOT_ALLOWED", "Method not allowed"),
        StatusCode::PAYLOAD_TOO_LARGE => ("PAYLOAD_TOO_LARGE", "Request body too large"),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => ("UNSUPPORTED_MEDIA_TYPE", "Unsupported media type"),
        StatusCode::BAD_REQUEST => ("BAD_REQUEST", "Bad request"),
        _ => return res,
    };
    let allow = res.headers().get(ALLOW).cloned();
    let mut replaced = ApiError::new(res.status(), code, message).into_response();
    if let Some(allow) = allow {
        replaced.headers_mut().insert(ALLOW, allow);
    }
    replaced
}

pub async fn fallback() -> Response {
    ApiError::not_found().into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 公開getパスを判定できる() {
        assert!(is_public_read_path("/communities"));
        assert!(is_public_read_path("/communities/abc"));
        assert!(is_public_read_path("/communities/abc/members"));
        assert!(is_public_read_path("/communities/abc/events"));
        assert!(!is_public_read_path("/auth/me"));
        assert!(!is_public_read_path("/communities/abc/members/me"));
    }

    #[test]
    fn 上限を超えると拒否する() {
        let limiter = RateLimiter::new(true);
        assert!((0..3).all(|_| limiter.check("t", "1.1.1.1", 3)));
        assert!(!limiter.check("t", "1.1.1.1", 3));
        assert!(limiter.check("t", "2.2.2.2", 3));
    }

    #[test]
    fn 期限切れのバケットは掃除されメモリが増え続けない() {
        let limiter = RateLimiter::new(true);
        {
            // 期限切れのバケットを閾値ぶん仕込む
            let mut buckets = limiter.buckets.lock().unwrap();
            let expired = Instant::now() - WINDOW - Duration::from_secs(1);
            for i in 0..EVICT_THRESHOLD {
                buckets.insert(("t", format!("10.0.{}.{}", i / 256, i % 256)), (expired, 1));
            }
        }
        assert!(limiter.check("t", "9.9.9.9", 3));
        let len = limiter.buckets.lock().unwrap().len();
        assert_eq!(len, 1, "期限切れが掃除され、新しい 1 件だけが残る");
    }

    #[test]
    fn 無効時は常に許可する() {
        let limiter = RateLimiter::new(false);
        assert!((0..1000).all(|_| limiter.check("t", "1.1.1.1", 1)));
    }
}

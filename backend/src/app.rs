use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::header::{ACCEPT, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use axum::middleware::{Next, from_fn, from_fn_with_state};
use axum::response::{Html, Json, Response};
use axum::routing::{delete, get, patch, post};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::controller as auth_controller;
use crate::community::controller as community_controller;
use crate::community::errors::CommunityDomainEvent;
use crate::event::controller as event_controller;
use crate::event::errors::EventCreatedEvent;
use crate::shared::event_bus::EventBus;
use crate::shared::middleware::{
    RateLimiter, fallback, limits_and_cache, normalize_errors, require_json_body, security_headers,
};
use crate::shared::openapi;

/// リクエストボディの上限（旧 express.json() の既定 100kb と同じ）
const BODY_LIMIT_BYTES: usize = 100 * 1024;

/// Composition Root: ハンドラが共有する依存性。
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub jwt_secret: Arc<String>,
    pub community_events: Arc<EventBus<CommunityDomainEvent>>,
    pub event_events: Arc<EventBus<EventCreatedEvent>>,
}

impl AppState {
    pub fn new(pool: SqlitePool, jwt_secret: impl Into<String>) -> Self {
        Self {
            pool,
            jwt_secret: Arc::new(jwt_secret.into()),
            community_events: Arc::new(EventBus::default()),
            event_events: Arc::new(EventBus::default()),
        }
    }
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn openapi_json() -> Json<Value> {
    Json(openapi::document())
}

async fn docs() -> Html<&'static str> {
    Html(openapi::SWAGGER_UI_HTML)
}

/// trunk が出力するハッシュ付きアセット（例: `meetup-frontend-<16 桁 hex>_bg.wasm`）か。
/// 内容が変わると名前も変わるため、長期キャッシュしてよい。
fn is_hashed_asset(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or_default();
    let Some((stem, ext)) = file.rsplit_once('.') else {
        return false;
    };
    let stem = stem.strip_suffix("_bg").unwrap_or(stem);
    matches!(ext, "js" | "wasm" | "css")
        && stem.rsplit_once('-').is_some_and(|(_, hash)| {
            hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit())
        })
}

fn set_cache(res: &mut Response, value: &'static str) {
    res.headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(value));
}

/// 本番配信時、ブラウザのページ遷移（`Accept: text/html`）は API と同じパス
/// （例: `/communities/<id>`）でも SPA の index.html を返す。API 呼び出し（JSON）はそのまま通す。
/// SPA の応答には配信用の Cache-Control を付ける（index.html は毎回再検証、ハッシュ付きアセットは長期キャッシュ）。
async fn spa_for_html_navigation(dist: Arc<String>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let is_docs = matches!(path, "/docs" | "/openapi.json" | "/health");
    let hashed = is_hashed_asset(path);
    let wants_html = req
        .headers()
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"));
    if req.method() == Method::GET && wants_html && !is_docs {
        let index = ServeFile::new(format!("{dist}/index.html"));
        // ServeFile のエラー型は Infallible
        let Ok(res) = index.oneshot(req).await;
        let mut res = res.map(axum::body::Body::new);
        set_cache(&mut res, "no-cache");
        return res;
    }
    let mut res = next.run(req).await;
    let is_json = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if !is_json && !is_docs && res.status().is_success() {
        set_cache(
            &mut res,
            if hashed {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            },
        );
    }
    res
}

/// アプリケーションを組み立てる。`rate_limit_enabled=false` でレート制限を無効化する（テスト・E2E 用）。
pub fn create_app(state: AppState, rate_limit_enabled: bool) -> Router {
    create_app_with_frontend(state, rate_limit_enabled, None)
}

/// `frontend_dist` を渡すと、ビルド済み SPA を同一オリジンで配信する（本番用）。
/// SPA 配信もレート制限・Cache-Control・セキュリティヘッダの内側に置く
/// （layer の後で fallback を差し替えると、新しい fallback には layer がかからないため）。
pub fn create_app_with_frontend(
    state: AppState,
    rate_limit_enabled: bool,
    frontend_dist: Option<String>,
) -> Router {
    let limiter = Arc::new(RateLimiter::new(rate_limit_enabled));
    let router = api_routes();
    let router = match frontend_dist {
        Some(dist) => {
            let index = ServeFile::new(format!("{dist}/index.html"));
            let dist = Arc::new(dist);
            router
                .fallback_service(ServeDir::new(dist.as_str()).fallback(index))
                .layer(from_fn(move |req, next| {
                    spa_for_html_navigation(Arc::clone(&dist), req, next)
                }))
        }
        None => router.fallback(fallback),
    };
    router
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(from_fn(require_json_body))
        .layer(from_fn_with_state(limiter, limits_and_cache))
        .layer(from_fn(normalize_errors))
        .layer(from_fn(security_headers))
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(docs))
        // Auth
        .route("/auth/register", post(auth_controller::register))
        .route("/auth/login", post(auth_controller::login))
        .route("/auth/me", get(auth_controller::me))
        // Community
        .route(
            "/communities",
            post(community_controller::create).get(community_controller::list),
        )
        .route("/communities/{id}", get(community_controller::get_one))
        // Member
        .route(
            "/communities/{id}/members",
            post(community_controller::join).get(community_controller::list_members),
        )
        .route(
            "/communities/{id}/members/me",
            delete(community_controller::leave_me),
        )
        .route(
            "/communities/{id}/members/{member_id}",
            delete(community_controller::leave_by_member_id),
        )
        .route(
            "/communities/{id}/members/{member_id}/approve",
            patch(community_controller::approve),
        )
        .route(
            "/communities/{id}/members/{member_id}/reject",
            patch(community_controller::reject),
        )
        // Event
        .route(
            "/communities/{id}/events",
            post(event_controller::create).get(event_controller::list),
        )
}

#[cfg(test)]
mod tests {
    use super::is_hashed_asset;

    #[test]
    fn trunkのハッシュ付きアセットだけを長期キャッシュの対象にする() {
        assert!(is_hashed_asset("/meetup-frontend-5d3e04fd1c160851_bg.wasm"));
        assert!(is_hashed_asset("/meetup-frontend-2bd9def59dec682a.js"));
        assert!(is_hashed_asset("/input-90c05ea414850b4a.css"));
        assert!(!is_hashed_asset("/index.html"));
        assert!(!is_hashed_asset("/communities"));
        assert!(!is_hashed_asset("/app.js"));
    }
}

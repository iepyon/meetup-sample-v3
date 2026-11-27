use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::{Html, Json};
use axum::routing::{delete, get, patch, post};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::auth::controller as auth_controller;
use crate::community::controller as community_controller;
use crate::community::errors::CommunityDomainEvent;
use crate::event::controller as event_controller;
use crate::event::errors::EventCreatedEvent;
use crate::shared::event_bus::EventBus;
use crate::shared::middleware::{RateLimiter, fallback, limits_and_cache, normalize_errors};
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

/// アプリケーションを組み立てる。`rate_limit_enabled=false` でレート制限を無効化する（テスト・E2E 用）。
pub fn create_app(state: AppState, rate_limit_enabled: bool) -> Router {
    let limiter = Arc::new(RateLimiter::new(rate_limit_enabled));

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
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(from_fn_with_state(limiter, limits_and_cache))
        .layer(from_fn(normalize_errors))
        .with_state(state)
}

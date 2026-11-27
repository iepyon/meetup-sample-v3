//! 統合テストの共通ヘルパー（テストごとにインメモリ SQLite のアプリを作り、oneshot で呼ぶ）。
//! 各モジュールは `use crate::common::*;` でヘルパーとよく使う型をまとめて読み込む
#![allow(dead_code, unused_imports)]

pub use axum::Router;
pub use axum::body::Body;
pub use axum::http::{Request, StatusCode};
pub use http_body_util::BodyExt;
pub use meetup_backend::infrastructure::connect;
pub use meetup_backend::{AppState, create_app};
pub use serde_json::{Value, json};
pub use tower::ServiceExt;

pub const SECRET: &str = "test-secret";

pub async fn app() -> Router {
    let pool = connect("sqlite::memory:").await.expect("connect");
    create_app(AppState::new(pool, SECRET), false)
}

pub async fn app_with_pool() -> (Router, sqlx::SqlitePool) {
    let pool = connect("sqlite::memory:").await.expect("connect");
    (create_app(AppState::new(pool.clone(), SECRET), false), pool)
}

pub async fn call(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let req = if let Some(b) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
    } else {
        builder.body(Body::empty())
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

pub struct User {
    pub id: String,
    pub token: String,
}

pub async fn register(app: &Router, name: &str) -> User {
    let (status, body) = call(
        app,
        "POST",
        "/auth/register",
        None,
        Some(
            json!({ "name": name, "email": format!("{name}@example.com"), "password": "pass1234" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    User {
        id: body["account"]["id"].as_str().unwrap().to_string(),
        token: body["token"].as_str().unwrap().to_string(),
    }
}

pub async fn create_community(app: &Router, user: &User, name: &str, visibility: &str) -> String {
    let (status, body) = call(
        app,
        "POST",
        "/communities",
        Some(&user.token),
        Some(json!({ "name": name, "description": "説明", "category": "TECH", "visibility": visibility })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["community"]["id"].as_str().unwrap().to_string()
}

/// 現在時刻から開始・終了を時間単位でずらしたイベント作成ボディ
pub fn event_body(starts_in_hours: i64, ends_in_hours: i64) -> Value {
    let now = chrono::Utc::now();
    json!({
        "title": "もくもく会",
        "description": "説明",
        "startsAt": (now + chrono::Duration::hours(starts_in_hours)).to_rfc3339(),
        "endsAt": (now + chrono::Duration::hours(ends_in_hours)).to_rfc3339(),
        "format": "ONLINE",
        "capacity": 50,
    })
}

/// レスポンスをそのまま返す（ヘッダや本文の形式を検証するテスト用）
pub async fn raw(app: &Router, req: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(req).await.unwrap()
}

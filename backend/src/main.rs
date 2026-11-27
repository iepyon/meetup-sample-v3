use std::net::SocketAddr;

use axum::extract::Request;
use axum::http::header::ACCEPT;
use axum::middleware::{Next, from_fn};
use axum::response::Response;
use meetup_backend::infrastructure::connect;
use meetup_backend::{AppState, create_app};
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

/// 本番配信時、ブラウザのページ遷移（`Accept: text/html`）は API と同じパス
/// （例: `/communities/<id>`）でも SPA の index.html を返す。API 呼び出し（JSON）はそのまま通す。
async fn spa_for_html_navigation(dist: String, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let is_docs = matches!(path, "/docs" | "/openapi.json" | "/health");
    let wants_html = req
        .headers()
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"));
    if req.method() == axum::http::Method::GET && wants_html && !is_docs {
        let index = ServeFile::new(format!("{dist}/index.html"));
        // ServeFile のエラー型は Infallible
        let Ok(res) = index.oneshot(req).await;
        return res.map(axum::body::Body::new);
    }
    next.run(req).await
}

#[tokio::main]
async fn main() {
    // 必須環境変数は起動時に検証する（fail-fast）。既知の既定値へのフォールバックは認証バイパスになるため置かない。
    let jwt_secret = match std::env::var("JWT_SECRET") {
        Ok(secret) if !secret.trim().is_empty() => secret,
        _ => {
            eprintln!(
                "JWT_SECRET 環境変数が未設定です。安全なランダム値（推奨: 32 バイト以上）を設定してください。"
            );
            std::process::exit(1);
        }
    };

    if jwt_secret.len() < 32 {
        eprintln!(
            "警告: JWT_SECRET が 32 バイト未満です。本番では 32 バイト以上のランダム値を設定してください。"
        );
    }

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "file:./dev.db".to_string());
    let pool = connect(&database_url).await.unwrap_or_else(|_| {
        eprintln!("データベースに接続できませんでした");
        std::process::exit(1);
    });

    let rate_limit_enabled = std::env::var("RATE_LIMIT_DISABLED").as_deref() != Ok("true");
    let mut app = create_app(AppState::new(pool, jwt_secret), rate_limit_enabled);

    // FRONTEND_DIST が指定されていれば、ビルド済み SPA を同一オリジンで配信する（本番用）
    if let Ok(dist) = std::env::var("FRONTEND_DIST") {
        let index = ServeFile::new(format!("{dist}/index.html"));
        app = app.fallback_service(ServeDir::new(&dist).fallback(index));
        let dist_for_spa = dist.clone();
        app = app.layer(from_fn(move |req, next| {
            spa_for_html_navigation(dist_for_spa.clone(), req, next)
        }));
    }

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|_| {
            eprintln!("ポート {port} をバインドできませんでした");
            std::process::exit(1);
        });
    println!("Server running on http://localhost:{port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|_| std::process::exit(1));
}

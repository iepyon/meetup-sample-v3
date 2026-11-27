use std::net::SocketAddr;

use meetup_backend::infrastructure::connect;
use meetup_backend::shared::auth::validate_jwt_secret;
use meetup_backend::{AppState, create_app_with_frontend};

#[tokio::main]
async fn main() {
    // 必須環境変数は起動時に検証する（fail-fast）。既知の既定値へのフォールバックは認証バイパスになるため置かない。
    let jwt_secret = validate_jwt_secret(std::env::var("JWT_SECRET").ok().as_deref())
        .unwrap_or_else(|message| {
            eprintln!("{message}");
            std::process::exit(1);
        });

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "file:./dev.db".to_string());
    let pool = connect(&database_url).await.unwrap_or_else(|_| {
        eprintln!("データベースに接続できませんでした");
        std::process::exit(1);
    });

    let rate_limit_enabled = std::env::var("RATE_LIMIT_DISABLED").as_deref() != Ok("true");
    // FRONTEND_DIST が指定されていれば、ビルド済み SPA を同一オリジンで配信する（本番用）
    let app = create_app_with_frontend(
        AppState::new(pool, jwt_secret),
        rate_limit_enabled,
        std::env::var("FRONTEND_DIST").ok(),
    );

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

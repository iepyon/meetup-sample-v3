//! テスト対象のサーバ。backend を同一プロセスで起動し、ビルド済み SPA を同一オリジンで配信する。
//!
//! テストごとにインメモリ SQLite と空きポート（127.0.0.1:0）を使うため、
//! `d dev` の 3000 / 5173 番や他のテストとは衝突しない。

use std::net::SocketAddr;
use std::path::PathBuf;

use meetup_backend::infrastructure::connect;
use meetup_backend::{AppState, create_app_with_frontend};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// E2E 専用の固定 JWT シークレット（32 バイト以上。本番では使わない）
const JWT_SECRET: &str = "e2e-only-fixed-value-not-for-production-0123";

/// `trunk build --dist` の出力先。`E2E_FRONTEND_DIST` で上書きできる
pub fn frontend_dist() -> PathBuf {
    std::env::var_os("E2E_FRONTEND_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/e2e-dist"))
}

pub struct TestServer {
    pub base_url: String,
    task: JoinHandle<()>,
}

impl TestServer {
    pub async fn spawn() -> Self {
        let dist = frontend_dist();
        assert!(
            dist.join("index.html").is_file(),
            "{} に SPA がありません。先に frontend で `trunk build --dist ../target/e2e-dist` を実行してください（`d e2e` は自動で行う）",
            dist.display()
        );
        let pool = connect("sqlite::memory:")
            .await
            .expect("インメモリ SQLite に接続できること");
        let app = create_app_with_frontend(
            AppState::new(pool, JWT_SECRET),
            false,
            Some(dist.to_string_lossy().into_owned()),
        );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("空きポートをバインドできること");
        let addr = listener
            .local_addr()
            .expect("バインド先アドレスを取得できること");
        // main.rs と同じく ConnectInfo 付きで配信する（レート制限ミドルウェアが参照する）
        let task = tokio::spawn(async move {
            let service = app.into_make_service_with_connect_info::<SocketAddr>();
            if axum::serve(listener, service).await.is_err() {
                eprintln!("E2E サーバが異常終了しました");
            }
        });
        Self {
            base_url: format!("http://{addr}"),
            task,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

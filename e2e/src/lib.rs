//! Meetup アプリの E2E テスト支援（Rust / Chrome DevTools Protocol）。
//!
//! [`E2e::start`] がテストごとに次を用意する:
//! - backend を同一プロセスで起動（インメモリ SQLite・空きポート・ビルド済み SPA を同一オリジンで配信）
//! - 専用プロファイルのヘッドレス Chromium（localStorage などはテスト間で共有しない）
//!
//! テスト支援専用のクレートなので、失敗は panic で表す（本番コードの unwrap 禁止規約の対象外）。

pub mod flows;
pub mod page;
pub mod server;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};

use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use tokio::task::JoinHandle;

pub use page::{Locator, Page};
pub use server::TestServer;

static PROFILE_SEQ: AtomicU64 = AtomicU64::new(0);

/// 1 テスト分のサーバ + ブラウザ + ページ。`Deref` で [`Page`] の操作をそのまま呼べる
pub struct E2e {
    page: Page,
    /// `close()` で取り出して正常終了させる。残っていれば Drop で kill する
    browser: Option<Browser>,
    handler: JoinHandle<()>,
    profile_dir: PathBuf,
    _server: TestServer,
}

impl E2e {
    pub async fn start() -> Self {
        let server = TestServer::spawn().await;
        // 既定のプロファイルは全プロセス共通のため、並列実行で衝突しないよう個別に作る
        sweep_stale_profiles();
        let profile_dir = profile_root()
            .join(std::process::id().to_string())
            .join(PROFILE_SEQ.fetch_add(1, Ordering::Relaxed).to_string());
        let config = BrowserConfig::builder()
            .user_data_dir(&profile_dir)
            // コンテナ内の非 root ユーザーでは Chromium のサンドボックスが使えない
            .no_sandbox()
            .arg("--disable-dev-shm-usage")
            .window_size(1280, 900)
            .build()
            .unwrap_or_else(|e| panic!("ブラウザ設定を作れませんでした: {e}"));
        let (browser, mut events) = Browser::launch(config).await.unwrap_or_else(|e| {
            panic!("Chromium を起動できませんでした（CHROME 環境変数で実行ファイルを指定できます）: {e}")
        });
        let handler = tokio::spawn(async move { while events.next().await.is_some() {} });
        let page = browser
            .new_page("about:blank")
            .await
            .unwrap_or_else(|e| panic!("ページを開けませんでした: {e}"));
        Self {
            page: Page::new(page, server.base_url.clone()),
            browser: Some(browser),
            handler,
            profile_dir,
            _server: server,
        }
    }

    /// ブラウザを正常に閉じる（後始末の残りは Drop が行う）
    pub async fn close(mut self) {
        if let Some(mut browser) = self.browser.take() {
            let _ = browser.close().await;
            let _ = browser.wait().await;
        }
    }
}

/// テストが panic して `close()` に届かなかった場合も、Chromium を kill してプロファイルを消す
impl Drop for E2e {
    fn drop(&mut self) {
        // Browser の drop で子プロセスが kill される（kill_on_drop）。その後でプロファイルを消す
        drop(self.browser.take());
        self.handler.abort();
        // kill 直後は子プロセス（ネットワークサービス等）が書き込みを続けて消し切れないことがある。
        // その残りは次回起動時の sweep_stale_profiles が回収する
        let _ = std::fs::remove_dir_all(&self.profile_dir);
    }
}

/// プロファイルは `<temp>/meetup-e2e/<テストプロセスの pid>/<連番>` に作る
fn profile_root() -> PathBuf {
    std::env::temp_dir().join("meetup-e2e")
}

/// 終了済みのテストプロセスが残したプロファイルを消す（プロセスごとに 1 回）。
/// 生存確認に /proc を使うため Linux（Docker / CI）でのみ動き、それ以外では何もしない
fn sweep_stale_profiles() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let proc_dir = Path::new("/proc");
        let Ok(entries) = std::fs::read_dir(profile_root()) else {
            return;
        };
        if !proc_dir.is_dir() {
            return;
        }
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(pid) = name.to_str().and_then(|n| n.parse::<u32>().ok()) else {
                continue;
            };
            if pid != std::process::id() && !proc_dir.join(pid.to_string()).exists() {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    });
}

impl Deref for E2e {
    type Target = Page;

    fn deref(&self) -> &Page {
        &self.page
    }
}

/// テストデータの一意化用サフィックス（ミリ秒時刻 + 連番）
pub fn unique_suffix() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}{}",
        chrono::Utc::now().timestamp_millis(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

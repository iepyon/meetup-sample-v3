//! 複数のテストで使うユーザー操作の手順（旧 Playwright 版の各 spec のヘルパー関数に対応）

use chrono::{Duration, Local};
use serde_json::json;

use crate::Page;

pub const PASSWORD: &str = "password123";

/// 新規登録してホームへ遷移するまで待つ（登録するとログイン状態になる）
pub async fn register(page: &Page, name: &str, email: &str) {
    page.goto("/register").await;
    page.get_by_label("名前").fill(name).await;
    page.get_by_label("メールアドレス").fill(email).await;
    page.get_by_label("パスワード").fill(PASSWORD).await;
    page.get_by_role("button").name("登録").click().await;
    page.expect_path("/").await;
}

pub async fn login(page: &Page, email: &str) {
    page.goto("/login").await;
    page.get_by_label("メールアドレス").fill(email).await;
    page.get_by_label("パスワード").fill(PASSWORD).await;
    page.get_by_role("button").name("ログイン").click().await;
    page.expect_path("/").await;
}

pub async fn logout(page: &Page) {
    page.get_by_role("button").name("ログアウト").click().await;
    page.get_by_role("link")
        .name("ログイン")
        .expect_visible()
        .await;
}

/// ヘッダーの「コミュニティ作成」からフォームを開く
pub async fn open_community_form(page: &Page) {
    page.get_by_role("link")
        .name("コミュニティ作成")
        .click()
        .await;
    page.get_by_role("heading")
        .name("コミュニティ作成")
        .expect_visible()
        .await;
}

/// コミュニティ作成フォームの入力値。`None` の select は既定値のまま
pub struct CommunityForm<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub category: Option<&'a str>,
    pub visibility: Option<&'a str>,
}

/// コミュニティを作成し、詳細ページの見出しが出るまで待つ
pub async fn create_community(page: &Page, form: CommunityForm<'_>) {
    open_community_form(page).await;
    page.get_by_label("コミュニティ名").fill(form.name).await;
    page.locator("textarea#description")
        .fill(form.description)
        .await;
    if let Some(category) = form.category {
        page.locator("select#category")
            .select_option(category)
            .await;
    }
    if let Some(visibility) = form.visibility {
        page.locator("select#visibility")
            .select_option(visibility)
            .await;
    }
    page.get_by_role("button").name("作成").click().await;
    page.get_by_role("heading")
        .name(form.name)
        .exact()
        .expect_visible()
        .await;
}

/// 一覧（`via_link` のリンク先）からコミュニティカードの見出しをクリックして詳細を開く
pub async fn open_community_from(page: &Page, via_link: &str, name: &str) {
    page.get_by_role("link").name(via_link).click().await;
    page.get_by_role("heading")
        .name(name)
        .level(2)
        .exact()
        .click()
        .await;
    page.get_by_role("heading")
        .name(name)
        .level(1)
        .exact()
        .expect_visible()
        .await;
}

/// API で登録して参加申請する（非公開コミュニティは非メンバーに 404 で UI から辿れないため）
pub async fn apply_via_api(page: &Page, community_id: &str, name: &str, email: &str) {
    let body = json!({ "name": name, "email": email, "password": PASSWORD });
    let (status, registered) = page
        .fetch_json("POST", "/auth/register", Some(&body), None)
        .await;
    assert_eq!(status, 201, "登録に失敗しました: {registered}");
    let token = registered["token"].as_str().unwrap_or_default();
    let path = format!("/communities/{community_id}/members");
    let (status, applied) = page.fetch_json("POST", &path, None, Some(token)).await;
    assert_eq!(status, 201, "参加申請に失敗しました: {applied}");
    assert_eq!(applied["status"], "PENDING");
}

/// `datetime-local` 用のローカル時刻（YYYY-MM-DDTHH:mm）。今日から `days` 日後（負なら過去）の `hour` 時
pub fn local_datetime_from_today(days: i64, hour: u32) -> String {
    let date = (Local::now() + Duration::days(days)).date_naive();
    format!("{}T{hour:02}:00", date.format("%Y-%m-%d"))
}

//! Playwright 風の自動待機付きページ操作。
//!
//! すべての操作・アサーションは `locator.js` をページ内で評価し、条件を満たすまで
//! [`TIMEOUT`] の間ポーリングする。タイムアウトしたら状態とページ本文を添えて panic する
//! （テスト支援コードのため panic で失敗を表す）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(100);
const LOCATOR_JS: &str = include_str!("locator.js");

static TOKEN: AtomicU64 = AtomicU64::new(0);

/// ロケーターの指定。`locator.js` の `resolve` にそのまま渡す
#[derive(Serialize, Clone, Debug, Default)]
struct Spec {
    kind: &'static str,
    role: Option<String>,
    name: Option<String>,
    text: Option<String>,
    selector: Option<String>,
    exact: bool,
    level: Option<u8>,
    parent: Option<Box<Spec>>,
    has: Option<Box<Spec>>,
}

impl Spec {
    fn describe(&self) -> String {
        let target = match self.kind {
            "role" => format!(
                "role={} name={:?} level={:?}",
                self.role.as_deref().unwrap_or_default(),
                self.name,
                self.level
            ),
            "css" => format!("css={}", self.selector.as_deref().unwrap_or_default()),
            kind => format!("{kind}={:?}", self.text.as_deref().unwrap_or_default()),
        };
        let exact = if self.exact { " exact" } else { "" };
        let has = self.has.as_ref().map(|h| format!(" has({})", h.describe()));
        let parent = self
            .parent
            .as_ref()
            .map(|p| format!("{} >> ", p.describe()));
        format!(
            "{}{target}{exact}{}",
            parent.unwrap_or_default(),
            has.unwrap_or_default()
        )
    }
}

#[derive(Deserialize, Debug)]
struct State {
    count: usize,
    visible: usize,
    ready: bool,
    text: Option<String>,
}

pub struct Page {
    inner: chromiumoxide::Page,
    base_url: String,
}

impl Page {
    pub(crate) fn new(inner: chromiumoxide::Page, base_url: String) -> Self {
        Self { inner, base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// `base_url` からの相対パスへ遷移する
    pub async fn goto(&self, path: &str) {
        let url = format!("{}{path}", self.base_url);
        if let Err(e) = self.inner.goto(url.as_str()).await {
            panic!("{url} へ遷移できませんでした: {e}");
        }
    }

    pub fn get_by_role(&self, role: &str) -> Locator<'_> {
        self.root(Spec {
            kind: "role",
            role: Some(role.to_string()),
            ..Spec::default()
        })
    }

    pub fn get_by_label(&self, text: &str) -> Locator<'_> {
        self.root(Spec {
            kind: "label",
            text: Some(text.to_string()),
            ..Spec::default()
        })
    }

    pub fn get_by_text(&self, text: &str) -> Locator<'_> {
        self.root(Spec {
            kind: "text",
            text: Some(text.to_string()),
            ..Spec::default()
        })
    }

    pub fn locator(&self, selector: &str) -> Locator<'_> {
        self.root(Spec {
            kind: "css",
            selector: Some(selector.to_string()),
            ..Spec::default()
        })
    }

    fn root(&self, spec: Spec) -> Locator<'_> {
        Locator { page: self, spec }
    }

    /// 現在の `pathname + search`
    pub async fn path(&self) -> String {
        self.eval("location.pathname + location.search")
            .await
            .unwrap_or_else(|e| panic!("URL を取得できませんでした: {e}"))
    }

    /// URL のパス（+クエリ）が `expected` になるまで待つ（Playwright の `toHaveURL` 相当）
    pub async fn expect_path(&self, expected: &str) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let actual: Result<String, String> =
                self.eval("location.pathname + location.search").await;
            if actual.as_deref() == Ok(expected) {
                return;
            }
            if Instant::now() >= deadline {
                self.fail(format!(
                    "URL が {expected} になりません（実際: {actual:?}）"
                ))
                .await;
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// ページのオリジンで fetch する（Playwright の `page.request` 相当）。`(ステータス, JSON 本文)` を返す
    pub async fn fetch_json(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        token: Option<&str>,
    ) -> (u16, Value) {
        let args =
            serde_json::json!({ "method": method, "path": path, "body": body, "token": token });
        let script = format!(
            "(async (a) => {{
                const headers = {{ 'Content-Type': 'application/json' }};
                if (a.token) headers['Authorization'] = 'Bearer ' + a.token;
                const res = await fetch(a.path, {{
                    method: a.method, headers,
                    body: a.body === null ? undefined : JSON.stringify(a.body),
                }});
                const text = await res.text();
                return [res.status, text ? JSON.parse(text) : null];
            }})({args})"
        );
        self.eval(&script)
            .await
            .unwrap_or_else(|e| panic!("{method} {path} に失敗しました: {e}"))
    }

    async fn eval<T: DeserializeOwned>(&self, expression: &str) -> Result<T, String> {
        let params = EvaluateParams::builder()
            .expression(expression)
            .await_promise(true)
            .return_by_value(true)
            .build()?;
        let result = self
            .inner
            .evaluate(params)
            .await
            .map_err(|e| e.to_string())?;
        result.into_value().map_err(|e| e.to_string())
    }

    async fn state(&self, spec: &Spec, op: &str, arg: &str, token: &str) -> Result<State, String> {
        let call = serde_json::to_string(&(spec, op, arg, token)).map_err(|e| e.to_string())?;
        // [spec, op, arg, token] を展開して locator.js の関数に渡す（末尾の `;` は式にならないので外す）
        let function = LOCATOR_JS.trim_end().trim_end_matches(';');
        self.eval(&format!("(\n{function}\n)(...{call})")).await
    }

    /// `done` を満たす状態になるまでポーリングする
    async fn wait_state(
        &self,
        spec: &Spec,
        what: &str,
        op: &str,
        arg: &str,
        done: impl Fn(&State) -> bool,
    ) -> String {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let token = format!("e2e-{}", TOKEN.fetch_add(1, Ordering::Relaxed));
            let state = self.state(spec, op, arg, &token).await;
            if state.as_ref().is_ok_and(&done) {
                return token;
            }
            if Instant::now() >= deadline {
                self.fail(format!(
                    "{what}: {}（最後の状態: {state:?}）",
                    spec.describe()
                ))
                .await;
            }
            tokio::time::sleep(POLL).await;
        }
    }

    async fn click(&self, spec: &Spec) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let token = self
                .wait_state(spec, "クリックできません", "mark", "", |s| s.ready)
                .await;
            let selector = format!("[data-e2e-target=\"{token}\"]");
            // 目印を付けてから再描画で要素が差し替わった場合は、もう一度探し直す
            let clicked = match self.inner.find_element(selector).await {
                Ok(el) => el.click().await.map(|_| ()).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            };
            match clicked {
                Ok(()) => return,
                Err(e) if Instant::now() >= deadline => {
                    self.fail(format!(
                        "クリックに失敗しました: {}（{e}）",
                        spec.describe()
                    ))
                    .await
                }
                Err(_) => tokio::time::sleep(POLL).await,
            }
        }
    }

    /// 失敗理由にページ本文の先頭を添えて panic する
    async fn fail(&self, message: String) -> ! {
        let body: String = self
            .eval("document.body ? document.body.innerText.slice(0, 1000) : ''")
            .await
            .unwrap_or_default();
        panic!("{message}\n--- ページ本文（先頭）---\n{body}");
    }
}

/// 要素の指定。操作・アサーションは自動で待機する
#[derive(Clone)]
pub struct Locator<'p> {
    page: &'p Page,
    spec: Spec,
}

impl<'p> Locator<'p> {
    /// アクセシブルネーム（role）で絞り込む。既定は大文字小文字無視の部分一致
    pub fn name(mut self, name: &str) -> Self {
        self.spec.name = Some(name.to_string());
        self
    }

    /// 名前・テキストを完全一致にする
    pub fn exact(mut self) -> Self {
        self.spec.exact = true;
        self
    }

    /// 見出しレベル（role = heading）
    pub fn level(mut self, level: u8) -> Self {
        self.spec.level = Some(level);
        self
    }

    /// `other` に一致する子孫を持つ要素だけに絞り込む（Playwright の `filter({ has })`）
    pub fn filter_has(mut self, other: Locator<'_>) -> Self {
        self.spec.has = Some(Box::new(other.spec));
        self
    }

    pub fn get_by_role(&self, role: &str) -> Self {
        self.child(self.page.get_by_role(role))
    }

    pub fn get_by_text(&self, text: &str) -> Self {
        self.child(self.page.get_by_text(text))
    }

    fn child(&self, mut inner: Locator<'p>) -> Self {
        inner.spec.parent = Some(Box::new(self.spec.clone()));
        inner
    }

    pub async fn click(&self) {
        self.page.click(&self.spec).await;
    }

    /// 入力欄・select の値を差し替える（select は option の value を渡す）
    pub async fn fill(&self, value: &str) {
        self.page
            .wait_state(&self.spec, "入力できません", "fill", value, |s| {
                s.ready
            })
            .await;
    }

    pub async fn select_option(&self, value: &str) {
        self.fill(value).await;
    }

    /// 一致する要素がちょうど 1 つで、表示されている
    pub async fn expect_visible(&self) {
        self.page
            .wait_state(&self.spec, "表示されません", "state", "", |s| {
                s.count == 1 && s.visible == 1
            })
            .await;
    }

    /// 一致する要素が無いか、1 つだけで非表示である（複数一致は Playwright の strict mode と同じく失敗）
    pub async fn expect_hidden(&self) {
        self.page
            .wait_state(
                &self.spec,
                "非表示になりません",
                "state",
                "",
                |s| s.count <= 1 && s.visible == 0,
            )
            .await;
    }

    pub async fn expect_count(&self, expected: usize) {
        self.page
            .wait_state(
                &self.spec,
                &format!("{expected} 件になりません"),
                "state",
                "",
                |s| s.count == expected,
            )
            .await;
    }

    /// テキスト（空白正規化後）が完全一致する（Playwright の `toHaveText`）
    pub async fn expect_text(&self, expected: &str) {
        self.page
            .wait_state(
                &self.spec,
                &format!("テキストが {expected:?} になりません"),
                "state",
                "",
                |s| s.count == 1 && s.text.as_deref() == Some(expected),
            )
            .await;
    }
}

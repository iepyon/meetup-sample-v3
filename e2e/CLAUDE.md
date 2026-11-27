# E2E テスト (Rust / chromiumoxide)

Meetup アプリケーション（Rust: axum + Leptos）のエンドツーエンドテスト。backend を同一プロセスで起動し、ヘッドレス Chromium を Chrome DevTools Protocol（[chromiumoxide](https://crates.io/crates/chromiumoxide)）で操作してユーザーフロー全体を検証する。

## Commands

```bash
d e2e                      # trunk build（target/e2e-dist へ）→ cargo test -p meetup-e2e -- --ignored
d e2e --test auth          # 追加引数は cargo test の `--` より前に渡る（テストファイルの指定など）
./scripts/docker-worktree.sh <name> e2e   # worktree でも同時に実行できる（ポートを共有しない）

# コンテナ内で直接
(cd frontend && trunk build --dist ../target/e2e-dist)
cargo test -p meetup-e2e -- --ignored
```

- テストは全件 `#[ignore]` 付き。`cargo test --workspace`（`d test` / pre-commit / CI quality）では実行されず、ブラウザ無しで通る。clippy / fmt の対象には含まれる
- ブラウザは自動検出（`chromium` / `google-chrome` など）。`CHROME=<実行ファイル>` で差し替えられる。Docker イメージには Debian の `chromium` を導入済み、CI はランナーの Google Chrome を使う
- SPA の場所は既定で `target/e2e-dist`。`E2E_FRONTEND_DIST` で上書きできる。見つからなければ panic で案内する

## 構成

| ファイル | 役割 |
|---------|------|
| `src/lib.rs` | `E2e::start()` — テストごとにサーバ + 専用プロファイルの Chromium + ページを用意。`Deref` で `Page` の操作を呼べる |
| `src/server.rs` | `create_app_with_frontend` をインメモリ SQLite・`127.0.0.1:0`・レート制限無効で起動し、SPA を同一オリジンで配信 |
| `src/page.rs` | Playwright 風の `Page` / `Locator`。操作とアサーションは自動待機（既定 10 秒ポーリング） |
| `src/locator.js` | ページ内で評価するロケーター本体（role / label / text / css と `has` 絞り込み） |
| `src/flows.rs` | 共通の操作手順（登録・ログイン・ログアウト・コミュニティ作成・API での参加申請など） |
| `tests/auth.rs` | 新規登録、ログイン、ログイン失敗、ログアウト |
| `tests/community.rs` | コミュニティ作成、詳細表示、カテゴリフィルター |
| `tests/member.rs` | 参加申請の承認・拒否、コミュニティ参加・退会 |
| `tests/event.rs` | イベント作成、入力エラー、権限チェック |

`d dev`（3000 / 5173）とは独立している。テストごとにサーバ・DB・ブラウザが別なので、並列実行してもデータが混ざらない。

## Locator API（Playwright 対応表）

| Playwright | Rust |
|-----------|------|
| `page.getByRole("button", { name, exact: true })` | `page.get_by_role("button").name(name).exact()` |
| `getByRole("heading", { level: 2 })` | `.get_by_role("heading").level(2)` |
| `page.getByLabel(t)` / `getByText(t)` / `locator(css)` | `get_by_label(t)` / `get_by_text(t)` / `locator(css)` |
| `locator.filter({ has })` | `.filter_has(other)` |
| `click()` / `fill(v)` / `selectOption(v)` | `.click()` / `.fill(v)` / `.select_option(v)` |
| `toBeVisible()` / `not.toBeVisible()` | `.expect_visible()` / `.expect_hidden()` |
| `toHaveCount(n)` / `toHaveText(s)` | `.expect_count(n)` / `.expect_text(s)` |
| `expect(page).toHaveURL(p)` | `page.expect_path(p)` |
| `page.request.post(...)` | `page.fetch_json("POST", path, body, token)` |

- 名前・テキストは既定で大文字小文字無視の部分一致、`.exact()` で完全一致
- 操作と `expect_visible` / `expect_text` は一致がちょうど 1 件であることを要求する（Playwright の strict mode 相当）
- `fill` は値を差し替えて `input` / `change` を発火する（`datetime-local` や `select` も同じ）。クリックは CDP のマウスイベント
- タイムアウト時は最後の状態とページ本文の先頭を添えて panic する

## テスト作成時の注意

- テストは `describe!` / `test!`（`meetup-test-macros`）で書き、名前は日本語の文字列にする（ファイル = 機能、入れ子の `describe!` = 観点。旧 Playwright の describe 構造と同じ）。各テストは `E2e::start()` から始め、最後に `page.close().await` を呼ぶ
- `test!` の直前に `#[tokio::test(flavor = "multi_thread")]` と `#[ignore = "E2E: ..."]` をそろえて書く
- 否定のアサーション（`expect_hidden` / `expect_count(0)`）は即座に通り得る。先に肯定の状態（エラー表示・ボタンの切り替わりなど）を待ってから書く
- メールアドレスは英数字の接頭辞 + `unique_suffix()` で作る（日本語を入れると入力検証で弾かれる）
- 複数ユーザーが関わるテストは、登録 → 操作 → ログアウト → 別ユーザー登録 → 操作の流れ。UI から辿れない前提（非公開コミュニティへの申請など）は `fetch_json` で API を直接呼ぶ

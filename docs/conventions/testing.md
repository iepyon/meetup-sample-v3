# Testing Conventions

## Backend
- 単体テスト: 対象モジュールの `#[cfg(test)] mod tests`（`models.rs` の遷移関数、`shared/validation.rs` など純粋ロジック）。テストが 60 行を超えたら `<モジュール>/tests.rs` に出して `#[cfg(test)] mod tests;` で読む（例: `shared/validation/tests.rs`）。短いものはモジュール内に書いてよい
- HTTP 統合テスト: `backend/tests/api/`。テストのバイナリを 1 つにまとめ（リンクを 1 回で済ませる）、観点ごとにモジュールを分ける: `main.rs`（mod 宣言）/ `auth.rs`（認証）/ `community.rs`（コミュニティ）/ `member.rs`（参加・脱退・ロール）/ `member_review.rs`（参加申請の審査）/ `member_list.rs`（メンバー一覧）/ `event.rs`（イベント）/ `http.rs`（404・405・413・415・セキュリティヘッダ・OpenAPI）/ `rate_limit.rs`（レート制限・Cache-Control）/ `spa.rs`（本番構成の SPA 配信）。共通ヘルパーは `common.rs`。各モジュールは `use crate::common::*;` でヘルパーとよく使う型をまとめて読む。新しい観点は `main.rs` に `mod` を足す。ファイルが 400 行を超えそうならさらに分ける。絞り込みは `cargo test -p meetup-backend --test api member::`。`connect("sqlite::memory:")` でテストごとに分離した SQLite を作り、`create_app(AppState::new(pool, secret), false)` を `tower::ServiceExt::oneshot` で直接呼ぶ
- エンドポイントごとに正常系・異常系（400/401/403/404/409/422）と可視性（PRIVATE の 404、DRAFT は作成者のみ等）を検証する
- カバレッジ目標: 80% 以上（`cargo llvm-cov` を導入している場合）

## Frontend
- 純粋関数（`frontend/src/utils.rs`）と DTO（`types.rs`）は単体テスト（`utils/tests.rs` / `types/tests.rs`。ネイティブで `cargo test -p meetup-frontend`）
- ページ・コンポーネントの単体テストは書かない — 純粋関数テスト + E2E でカバーする

## E2E
- Rust（chromiumoxide）: `e2e/tests/*.rs`。全テストの `test!` に `#[tokio::test(flavor = "multi_thread")]` と `#[ignore = "E2E: ..."]` を付け、`d e2e` で実行する
- `E2e::start()` がテストごとに backend を同一プロセスで起動し（インメモリ SQLite・空きポート・ビルド済み SPA）、専用プロファイルの Chromium を開く。テスト間でデータもブラウザ状態も共有しない
- ロケーターは `get_by_role` / `get_by_label` / `get_by_text` を優先する。否定のアサーション（`expect_hidden` など）の前に、肯定の状態を待つ

## Test naming
- テストは `meetup-test-macros` の `describe!` / `test!` で書き、テスト名を**日本語の文字列**で付ける（Jest / Playwright の describe / test 相当。単体・統合・E2E のすべてで使う）

  ```rust
  use meetup_test_macros::describe;

  describe!("認証", {
      describe!("新規登録", {
          test!("同じメールアドレスの登録は 409 になる", async {
              // ...
          });
      });
  });
  ```

- `describe!` は `mod` に、`test!` はテスト関数に展開される。名前の文字列は識別子に変換される（空白・記号の並びは `_` 1 つにし、先頭・末尾のものは削る。数字始まりは `_` を前置）。`cargo test` の表示は `認証::新規登録::同じメールアドレスの登録は_409_になる` で、絞り込みもこの形で書く（例: `cargo test 公開_GET_パス`）
- 同じ `describe!` の中で変換後の名前が重なるとコンパイルエラーになる。また生成された `mod` / `fn` は兄弟の名前を覆い隠すので、ヘルパーやクレートと同じ名前（`app` など）は付けない
- 本体が `async { … }` なら `#[tokio::test]`、`{ … }` なら `#[test]` が付く。`#[tokio::test(flavor = "multi_thread")]` などのテスト属性や `#[ignore = "…"]` は `test!` の直前に書く（テスト属性を書けば既定の属性は付かない）
- マクロ自身のテストも `test-macros/tests/naming.rs` で `describe!` / `test!` を使う（変換結果は生成された `mod` を名前で呼べるかで確かめる）
- ヘルパー関数は `describe!` の外（または中）に普通に書ける。`describe!` は `use super::*;` を持つ
- 英字の略語・HTTP ステータス・識別子は原文どおりに書く（`409`、`X-Forwarded-For`、`PENDING` など）。名前が長いと rustfmt が `test!(` を縦に展開するが、それが rustfmt の正しい整形
- **Acceptance criteria mapping**: Each issue acceptance criterion maps 1:1 to a test name

# Testing Conventions

## Backend
- 単体テスト: 対象モジュール内の `#[cfg(test)] mod tests`（`models.rs` の遷移関数、`shared/validation.rs` など純粋ロジック）
- HTTP 統合テスト: `backend/tests/`（`api.rs` = 業務 API / `system.rs` = 横断的な挙動 / `security.rs` = セキュリティ。共通ヘルパーは `common/mod.rs`）。ファイル上限（800 行）を超えそうなら観点ごとにファイルを分ける。`connect("sqlite::memory:")` でテストごとに分離した SQLite を作り、`create_app(AppState::new(pool, secret), false)` を `tower::ServiceExt::oneshot` で直接呼ぶ
- エンドポイントごとに正常系・異常系（400/401/403/404/409/422）と可視性（PRIVATE の 404、DRAFT は作成者のみ等）を検証する
- カバレッジ目標: 80% 以上（`cargo llvm-cov` を導入している場合）

## Frontend
- 純粋関数（`frontend/src/utils.rs`）は `#[cfg(test)]` の単体テスト（ネイティブで `cargo test -p meetup-frontend`）
- ページ・コンポーネントの単体テストは書かない — 純粋関数テスト + E2E でカバーする

## E2E
- Rust（chromiumoxide）: `e2e/tests/*.rs`。全テストに `#[tokio::test(flavor = "multi_thread")]` と `#[ignore = "E2E: ..."]` を付け、`d e2e` で実行する
- `E2e::start()` がテストごとに backend を同一プロセスで起動し（インメモリ SQLite・空きポート・ビルド済み SPA）、専用プロファイルの Chromium を開く。テスト間でデータもブラウザ状態も共有しない
- ロケーターは `get_by_role` / `get_by_label` / `get_by_text` を優先する。否定のアサーション（`expect_hidden` など）の前に、肯定の状態を待つ

## Test naming
- テスト名（Rust の関数名。E2E も含む）は日本語で記述する
- **Acceptance criteria mapping**: Each issue acceptance criterion maps 1:1 to a test name

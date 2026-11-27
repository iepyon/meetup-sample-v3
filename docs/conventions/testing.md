# Testing Conventions

## Backend
- 単体テスト: 対象モジュール内の `#[cfg(test)] mod tests`（`models.rs` の遷移関数、`shared/validation.rs` など純粋ロジック）
- HTTP 統合テスト: `backend/tests/api.rs`。`connect("sqlite::memory:")` でテストごとに分離した SQLite を作り、`create_app(AppState::new(pool, secret), false)` を `tower::ServiceExt::oneshot` で直接呼ぶ
- エンドポイントごとに正常系・異常系（400/401/403/404/409/422）と可視性（PRIVATE の 404、DRAFT は作成者のみ等）を検証する
- カバレッジ目標: 80% 以上（`cargo llvm-cov` を導入している場合）

## Frontend
- 純粋関数（`frontend/src/utils.rs`）は `#[cfg(test)]` の単体テスト（ネイティブで `cargo test -p meetup-frontend`）
- ページ・コンポーネントの単体テストは書かない — 純粋関数テスト + E2E でカバーする

## E2E
- Playwright（TypeScript）: `e2e/tests/*.spec.ts`
- `webServer` が backend（`cargo run -p meetup-backend`, port 3000）と frontend（`trunk serve`, port 5173）を自動起動する

## Test naming
- テスト名（Rust の関数名・Playwright の describe/test）は日本語で記述する
- **Acceptance criteria mapping**: Each issue acceptance criterion maps 1:1 to a test name

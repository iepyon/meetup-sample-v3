# Testing Conventions

## Backend
- 単体テスト: 対象モジュール内の `#[cfg(test)] mod tests`（`models.rs` の遷移関数、`shared/validation.rs` など純粋ロジック）
- HTTP 統合テスト: `backend/tests/`（`api.rs` / `api_members.rs` = 業務 API / `system.rs` = 横断的な挙動 / `security.rs` = セキュリティ。共通ヘルパーは `common/mod.rs`）。ファイル上限（800 行）を超えそうなら観点ごとにファイルを分ける。`connect("sqlite::memory:")` でテストごとに分離した SQLite を作り、`create_app(AppState::new(pool, secret), false)` を `tower::ServiceExt::oneshot` で直接呼ぶ
- エンドポイントごとに正常系・異常系（400/401/403/404/409/422）と可視性（PRIVATE の 404、DRAFT は作成者のみ等）を検証する
- カバレッジ目標: 80% 以上（`cargo llvm-cov` を導入している場合）

## Frontend
- 純粋関数（`frontend/src/utils.rs`）は `#[cfg(test)]` の単体テスト（ネイティブで `cargo test -p meetup-frontend`）
- ページ・コンポーネントの単体テストは書かない — 純粋関数テスト + E2E でカバーする

## E2E
- Rust（chromiumoxide）: `e2e/tests/*.rs`。`meetup-test-runner` で書き、`main` で最上位の `describe` に `.multi_thread().ignore(REQUIRES_BROWSER)` を付ける。`d e2e`（`--ignored`）で実行する
- `E2e::start()` がテストごとに backend を同一プロセスで起動し（インメモリ SQLite・空きポート・ビルド済み SPA）、専用プロファイルの Chromium を開く。テスト間でデータもブラウザ状態も共有しない
- ロケーターは `get_by_role` / `get_by_label` / `get_by_text` を優先する。否定のアサーション（`expect_hidden` など）の前に、肯定の状態を待つ

## Test naming
テスト名は**日本語の文字列**で付ける（Jest / Playwright の describe / test 相当）。置き場所で書き方が 2 通りある。

| 置き場所 | 書き方 | `cargo test` の表示 |
|---|---|---|
| `tests/` 配下（backend の HTTP 統合テスト・E2E） | `meetup-test-runner` の `describe` / `test`（関数。libtest-mimic で標準のハーネスを置き換える） | 文字列のまま: `認証 > 新規登録 > 同じメールアドレスの登録は 409 になる` |
| `src` 内の単体テスト（`#[cfg(test)] mod tests`） | `meetup-test-macros` の `describe!` / `test!`（マクロ） | 識別子に変換: `認証::新規登録::同じメールアドレスの登録は_409_になる` |

### `tests/` 配下: `meetup-test-runner`

```rust
// Cargo.toml: [[test]] name = "api" / harness = false
use meetup_test_runner::{Node, describe, run, test};

fn main() {
    run(vec![認証()]);
}

fn 認証() -> Node {
    describe("認証", vec![
        describe("新規登録", vec![
            test("同じメールアドレスの登録は 409 になる", || async {
                // ...
            }),
        ]),
    ])
}
```

- テストファイルを足したら `Cargo.toml` に `[[test]] name = "…" / harness = false` を追加する（無いと標準のハーネスで `main` を探して失敗する）
- 本体は `|| async { … }`。テストごとに tokio のランタイムを作る（既定は current_thread）。`.multi_thread()` / `.ignore(理由)` は `test` にも `describe` にも付けられ、`describe` なら配下の全テストに効く（E2E は `run(vec![認証().multi_thread().ignore(REQUIRES_BROWSER)])`）
- 絞り込みはフルネームの部分一致（`cargo test --test api 409`）。`--ignored` / `--list` も標準と同じ。フルネームが重なると実行前に panic する
- トップレベルの `describe` ごとに `fn 名前() -> Node` を 1 つ作り、`main` から呼ぶ。この登録関数は 1 グループのテストを並べるだけなので、関数 50 行の上限（`coding.md`）の対象外とする
- ヘルパー関数は登録関数の外に普通に書く

### `src` 内の単体テスト: `meetup-test-macros`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use meetup_test_macros::describe;

    describe!("入力検証", {
        test!("UUID 形式を検証できる", {
            // ...
        });
    });
}
```

- `src` 内のテストは標準のハーネスでしか動かないため、マクロで名前を識別子に変換する。`describe!` は `mod` に、`test!` はテスト関数に展開される（空白・記号の並びは `_` 1 つにし、先頭・末尾のものは削る。数字始まりは `_` を前置）。絞り込みはこの形で書く（例: `cargo test 公開_GET_パス`）
- 同じ `describe!` の中で変換後の名前が重なるとコンパイルエラーになる。また生成された `mod` / `fn` は兄弟の名前を覆い隠すので、ヘルパーやクレートと同じ名前（`app` など）は付けない
- 本体が `async { … }` なら `#[tokio::test]`、`{ … }` なら `#[test]` が付く。テスト属性や `#[ignore = "…"]` は `test!` の直前に書く（テスト属性を書けば既定の属性は付かない）
- ヘルパー関数は `describe!` の外（または中）に普通に書ける。`describe!` は `use super::*;` を持つ
- マクロ自身のテストは `test-macros/tests/naming.rs`、ランナー自身のテストは `test-runner/src/lib.rs` にある（どちらもマクロで書く）

### 共通
- 英字の略語・HTTP ステータス・識別子は原文どおりに書く（`409`、`X-Forwarded-For`、`PENDING` など）。名前が長いと rustfmt が `test(` / `test!(` を縦に展開するが、それが rustfmt の正しい整形
- **Acceptance criteria mapping**: Each issue acceptance criterion maps 1:1 to a test name

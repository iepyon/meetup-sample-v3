# Coding Conventions

## ファイル・命名

- ファイル名は `snake_case.rs`。コンテキストごとに役割別モジュールに分ける: `controller.rs` / `usecases.rs` / `models.rs` / `repository.rs` / `errors.rs`
- 関数は最大 50 行、ファイルは最大 400 行（ハード上限 800 行）
- `unwrap()` / `expect()` / `panic!` は本番コードで使わない（テストと起動時の fail-fast を除く）。`unsafe` は原則禁止
- `cargo fmt --all` と `cargo clippy --workspace --all-targets -- -D warnings` を常に通す（frontend は `--target wasm32-unknown-unknown` でも）

## ドメインロジック

- 生成関数（`create_*()`）と遷移関数は `Result<T, E>` を返す。エラーはコンテキストごとの `errors.rs` のドメインエラー enum
- **UseCase は orchestration only** — ドメインロジックは `models.rs` に置く
- UseCase 内で禁止: `Utc::now()`、ID の直接生成（controller から受け取る）、集約の部分更新
- 列挙値は `models.rs` の列挙型（`CommunityVisibility` など）を使い、文字列リテラルを直書きしない
- 入力制約は `shared/validation.rs` と models に集約し、`shared/openapi.rs` と食い違わせない

## HTTP / エラー

- controller は 検証（400）→ 認証（401）→ 認可（403）→ UseCase の順で処理する
- ドメインエラーは `impl From<XxxError> for ApiError` で HTTP にマップする。`Internal` は詳細を返さず 500（sqlx のエラーメッセージはクエリ引数を含み得るため、ログには種別のみ出す）
- エラー本文は `{ "code": "...", "message": "..." }` 形式

## SQL

- sqlx は **バインドパラメータのみ**使う（`.bind()` / `QueryBuilder::push_bind`）。値を SQL 文字列へ連結しない
- 日時は `shared/time.rs` の ISO 8601 文字列（ミリ秒・Z）で保存・出力する
- スキーマは `backend/src/infrastructure/schema.sql`（起動時に冪等適用）

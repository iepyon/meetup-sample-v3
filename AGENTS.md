# AGENTS.md

Meetup community management full-stack application (Rust: axum + sqlx + SQLite backend, Leptos (CSR / WebAssembly) frontend)

## Commands

**コンテナランタイム**: ローカル開発は **Docker / Docker Compose**（`./scripts/docker-dev.sh`）の単一経路（Intel / Apple Silicon 両対応）。**コマンドの詳細・レシピ・worktree 運用は [`docs/docker-usage.md`](docs/docker-usage.md) を参照する**（Docker 一本化の経緯は `docs/decision-logs/arch-20260703-revert-to-docker-as-primary.md` および `docs/decision-logs/arch-20260703-remove-apple-container-fallback.md`、Rust 全面移行の経緯は `docs/decision-logs/arch-20261127-rewrite-in-rust.md`）。

**実行経路**: ホストからは必ず `./scripts/docker-dev.sh` 経由（下記 alias）。コンテナ内（`/.dockerenv` が存在する）ではラッパー不要で `cargo test --workspace` のように直接実行する。

```bash
alias d="./scripts/docker-dev.sh"

d up                       # コンテナ起動（初回は d build → d up → d install）
d install                  # cargo fetch + lefthook install（git pre-commit の有効化）
d dev                      # backend(3000) + frontend(5173, trunk serve) の dev サーバ起動
d test                     # cargo test --workspace（backend 単体・HTTP 統合 + frontend 純粋関数）
d e2e                      # E2E テスト（trunk build → cargo test -p meetup-e2e -- --ignored。ヘッドレス Chromium）
d lint                     # cargo clippy -D warnings（d lint backend / frontend で対象を絞れる）
d typecheck                # cargo check（frontend は wasm32 ターゲット）
d review                   # fmt --check + clippy + test の総合確認
d shell                    # コンテナ内シェル
d down                     # コンテナ停止・削除
d bash -c "<任意コマンド>"   # catch-all（レシピ集は docs/docker-usage.md §4.5）
```

worktree で main と同時に起動する場合は `./scripts/docker-worktree.sh <name> <command>`（`docs/docker-usage.md` §6）。

## General Rules

- 削除・リファクタリングを依頼された場合、指定されたファイル/ディレクトリのみを対象とする。関連ファイルや参照の削除は明示的に依頼されない限り行わない
- **ホスト実行時**: コンテナ CLI を直接使わず、必ず `./scripts/docker-dev.sh` ラッパー経由でコマンドを実行する
- **コンテナ内実行時**: ラッパーは不要。`cargo test --workspace` のように直接実行する
- シェルスクリプトは macOS/zsh 互換にする。GNU 固有フラグ (`grep -P`)、bash 固有機能 (連想配列) は使用禁止。POSIX 互換または zsh 互換の代替を使う

## Git Conventions

- コミットメッセージは日本語で記述し、Conventional Commits prefix を使用する

## Architecture

コードベース・ドメイン知識の詳細地図は `docs/wiki/index.md`（LLM 保守の Wiki。運用ルールは `docs/wiki/SCHEMA.md`、操作は `/wiki` スキル）。

Cargo workspace: `backend/`（axum API）, `frontend/`（Leptos SPA）, `e2e/`（ブラウザ E2E）。npm は使わない（Node は Docker イメージ内の Claude Code CLI 用のみ）。

### Backend

DDD bounded contexts。各コンテキストは `controller.rs` -> `usecases.rs` -> `models.rs`, `repository.rs`（+ `errors.rs`）の層構造。

- `backend/src/auth/` — Authentication context (registration, login, JWT)
- `backend/src/community/` — Community context (community CRUD, membership management)
- `backend/src/event/` — Event context (event creation and lifecycle within a community)
- `backend/src/shared/` — 共通基盤（エラー型 `ApiError`、検証 `validation.rs`、JWT `auth.rs`、レート制限/キャッシュ `middleware.rs`、Event Bus、OpenAPI `openapi.rs`）
- `backend/src/infrastructure/` — SQLite 接続と `schema.sql`（起動時に冪等適用）
- `backend/src/app.rs` — Composition Root（`AppState` とルーティング）
- `backend/tests/` — HTTP 統合テスト（インメモリ SQLite）。`api.rs`（認証・コミュニティ・メンバー・イベント）/ `system.rs`（横断的な挙動: 404・405・レート制限・キャッシュ等）/ `security.rs`（セキュリティヘッダ・415・SPA 配信等）、共通ヘルパーは `common/mod.rs`

### Frontend

Leptos 0.8 (CSR) + Trunk + Tailwind CSS — WebAssembly SPA

- `frontend/src/pages/` — ページ（一覧・詳細・作成・イベント作成・ログイン/登録）
- `frontend/src/components/` — 共有 UI (Button, Input, Card, ErrorAlert, Layout, バッジ)
- `frontend/src/auth.rs` — 認証コンテキスト（`provide_auth`）
- `frontend/src/api.rs` / `token.rs` / `types.rs` — API クライアント・トークン保存・DTO
- `frontend/src/utils.rs` — 純粋関数（ラベル変換・絞り込み）。ネイティブでテストできる

### E2E

Rust 製のブラウザ E2E（`meetup-e2e`。chromiumoxide で Chrome DevTools Protocol を直接操作）。auth / community / member / event のユーザーフローを検証する。詳細は `e2e/CLAUDE.md`。

- テストごとに backend を同一プロセスで起動する（インメモリ SQLite・空きポート・`target/e2e-dist` のビルド済み SPA を同一オリジンで配信）。`d dev` や他の worktree とポートを共有しない
- `e2e/src/page.rs` が Playwright 風の自動待機付き Locator（`get_by_role` / `get_by_label` / `get_by_text`）を提供する
- テストは `#[ignore]` 付きで、`cargo test --workspace` では実行されない（`d e2e` で実行）

## Core Patterns

- **Result<T, E>** — ドメイン関数と UseCase の戻り値。`panic` / `unwrap` で失敗を表現しない
- **Discriminated Union Errors** — コンテキストごとに `errors.rs` のドメインエラー enum（`CommunityNotFound` など）。`impl From<XxxError> for ApiError` で HTTP にマップする
- **列挙型ドメイン値** — `CommunityVisibility` / `MemberRole` などは `models.rs` の enum（`as_str()` / `parse()`）。文字列リテラルを直書きしない
- **検証は境界で** — `shared/validation.rs` が旧 OpenAPI バリデータ相当（400 `VALIDATION_ERROR`）。controller は 検証 → 認証 → 認可 → UseCase の順
- **Event Bus** — In-memory `EventBus<E>` for cross-context side effects
- **UseCase は orchestration only** — `Utc::now()` と ID 生成は controller から受け取る

## Coding Conventions

@docs/conventions/coding.md

## Testing

@docs/conventions/testing.md

## Quality Gates

**AI が実装完了前に必ず実行すること: `d review`** — `cargo fmt --check` / `clippy -D warnings`（native + wasm32）/ `cargo test --workspace`。

- **git pre-commit（lefthook）**: コミット時に cargo fmt / clippy / test が自動実行される（`lefthook.yml`）。`d install` が `lefthook install` で有効化する（worktree ごとに必要）
- **GitHub Actions CI**: push(main) / PR で `quality` ジョブ、PR のときのみ `e2e` ジョブ。`--no-verify` でローカルゲートを迂回しても CI が最後の砦になる → [CI・品質ゲート](docs/wiki/infra/ci-quality-gates.md)
- **AI ハーネス hook（1 本 / `.claude/hooks/` / Claude Code 専用）**: secret-leak-detector（Bash 出力監視）。lint / 型チェック / テストは lefthook と CI が担い、hook は**そこで代替できない AI 固有のリスクだけ**を見る。npm 撤去に伴い dep-install-guard（`npm install` ガード）と prettier-format（e2e の TS 整形）は役目を終えて削除した（→ [decision-log](docs/decision-logs/arch-20261127-e2e-in-rust.md)）。発火タイミングの一覧は [AI hooks](docs/wiki/infra/ai-hooks.md)

品質ゲート本体は `lefthook.yml` の git pre-commit と CI で AI 非依存に管理する（AI hook が無効でもゲートは効く）。かつて Copilot CLI との併用対応（`.github/hooks/hooks.json`）と、lefthook / CI と重複する hook 2 本を持っていたが、Claude Code 専用に一本化して撤去した（→ [decision-log](docs/decision-logs/arch-20260915-claude-code-only-harness.md)）。

## Security

- 認証: `shared/auth.rs` の `require_account`（認証必須・401）と `optional_account`（認証任意）を適切に使い分ける。JWT は HS256・24 時間有効。`JWT_SECRET` が未設定または 32 バイト未満なら起動しない（既定値へのフォールバック禁止。compose も既定値を持たず `.env` 必須）
- 入力検証: `shared/validation.rs` で全 API エンドポイントを検証する
- SQL は sqlx の **バインドパラメータのみ**（文字列連結禁止。SQL インジェクション防止）
- エラーレスポンスにスタックトレースや DB エラー詳細を含めない（`Internal` は 500 + 固定文言。ログには種別のみ）
- 機密ファイル（.env, credentials, *.pem, *.key）を読み書きしない
  - 防御は 2 層: ①**実シークレットを作業ツリーに置かない**（`.env` は `.gitignore` 済み、共有するのは `.env.example` のみ）②`secret-leak-detector.sh` による Bash 出力の出口検査
  - `settings.json` の deny リスト（読み取りコマンドの列挙）は `node -e` 等で容易に回避でき、上記 2 層に対する実効的な上積みが無い一方で保守コストが高かったため撤去した（→ [decision-log](docs/decision-logs/arch-20260915-claude-code-only-harness.md)）

## Gotchas

- フロントエンドのビルドには `trunk` と `wasm32-unknown-unknown` ターゲットが必要（Docker イメージには導入済み）
- `trunk serve` は `/auth` `/communities` `/health` を backend (3000) へプロキシする（`frontend/Trunk.toml`）。本番は `FRONTEND_DIST=frontend/dist` を指定した backend が API と SPA を同一ポートで配信する（`Accept: text/html` のページ遷移は API と同じパスでも index.html を返す）
- DB スキーマは `backend/src/infrastructure/schema.sql`。変更時は `CREATE TABLE IF NOT EXISTS` のため既存 DB には自動反映されない（開発 DB は削除して再作成する）
- コミュニティ一覧は未ログイン時 PUBLIC のみ返す（旧 TS 実装は repository が visibility 条件を無視しており非公開も返していた。Rust 版で意図どおりに修正）
- インメモリ SQLite は接続ごとに別 DB になるため、`:memory:` では接続プールを 1 本に固定している
- `trunk` / `cargo` の初回ビルドは数分かかる
- E2E のブラウザは自動検出（Docker は Debian の `chromium`、CI はランナーの Google Chrome）。`CHROME=<実行ファイル>` で差し替えられる。SPA は `d e2e` が毎回 `target/e2e-dist` にビルドし直す（`d dev` の `frontend/dist` とは別）

## Context Management

- タスク間で `/clear` を実行し、コンテキストをリセットする
- 調査はサブエージェントに委譲し、メインコンテキストを保護する。調査は `docs/wiki/index.md` を起点にし、発見が再利用価値を持つ場合は `/wiki ingest` で Wiki に還元する
- コンパクション時は変更ファイル一覧とテストコマンドを保持する

# バックエンド・フロントエンドを TypeScript から Rust に全面移行する

日付: 2026-11-27
種類: arch
ステータス: 承認済み

## コンテキスト

アプリケーション本体は Express.js + Prisma + SQLite（backend）と React 19 + Vite（frontend）の TypeScript 実装だった（約 6,600 行）。`/goal` で「フロントエンドもバックエンドもすべて Rust に書き換える」が指示された。

## 決定

アプリケーション本体を **Rust に全面移行**し、TS の `backend/` `frontend/` を撤去した。

| 領域 | 旧 | 新 |
| --- | --- | --- |
| Web フレームワーク | Express 5 | axum 0.8 + tokio |
| DB アクセス | Prisma 7（better-sqlite3） | sqlx 0.8（SQLite、バインドパラメータのみ） |
| スキーマ | `backend/prisma/schema/*.prisma`（`db push`） | `backend/src/infrastructure/schema.sql`（起動時に冪等適用） |
| 認証 | jsonwebtoken / bcrypt | jsonwebtoken 9（HS256, 24h）/ bcrypt 0.17（cost 10）。互換トークン形式 |
| 入力検証・OpenAPI | zod + express-openapi-validator | `shared/validation.rs` の手書き検証 + `shared/openapi.rs` の手書き OpenAPI JSON |
| フロントエンド | React 19 + Vite + React Router | Leptos 0.8（CSR / WebAssembly）+ Trunk + Tailwind CSS 4 |
| テスト | Vitest + Supertest + RTL | `cargo test`（単体 + HTTP 統合）。UI は Playwright E2E でカバー |
| 品質ゲート | prettier / eslint / tsc / vitest | cargo fmt / clippy `-D warnings`（native + wasm32）/ cargo test |

- DDD の境界づけられたコンテキスト（auth / community / event / shared / infrastructure）と層構造（controller → usecases → models / repository）は維持した
- `Result<T, E>` は std の `Result` + コンテキストごとのドメインエラー enum（`Internal` バリアントは 500）に置き換えた
- E2E（Playwright, TypeScript）は**そのまま維持**し、Rust 版を起動して検証する。npm workspace は `e2e` のみになった
- 本番相当は `FRONTEND_DIST` を指定した backend 単一バイナリが API と SPA を同一ポートで配信する

## 検証

- 移行前に、既存 Playwright E2E 11 件（auth / community / member / event）を Rust 版（axum + Leptos）に対して実行し、**全件通過**してから TS を撤去した
- backend: 単体 15 件 + HTTP 統合 26 件（認証・可視性・承認フロー・権限・日時検証・レート制限・キャッシュヘッダ等）

## 理由と代替案

- 指示（全面 Rust 化）に従う。フロントエンドは React に近いコンポーネント/シグナルモデルを持ち、既存 UI を移植しやすい Leptos を選んだ（Yew / Dioxus も候補）
- ORM は SeaORM も候補だったが、既存の「リポジトリ + UseCase」構造にそのまま載り、SQL を明示できる sqlx を選んだ

**意図した挙動の差異（バグ修正）:**

- コミュニティ一覧（未ログイン / `member=me` なし）は PUBLIC のみ返す。旧実装は UseCase が `visibility: 'PUBLIC'` を渡していたが、`PrismaCommunityRepository.findAll` が受け取って無視しており、**非公開コミュニティも一覧に出ていた**
- `PATCH .../reject`（204）は空ボディを正しく扱う。旧フロントは空ボディを `response.json()` していた
- PRIVATE コミュニティの閲覧（詳細・イベント一覧）は ACTIVE メンバーのみ。旧実装は PENDING（承認待ち）の申請者にも見えていた（メンバー一覧は ACTIVE のみで不整合だった）
- イベント一覧は、存在しない / 閲覧不可のコミュニティで 404（旧実装は PRIVATE でも誰にでも返し、存在しなければ空配列だった）
- メンバー ID 指定の脱退は、そのメンバーが指定コミュニティのものであることも照合する
- 権限なしの 403 の `code` は `FORBIDDEN`（旧 middleware 単体は `FORBIDDEN`、メッセージ同一）
- ログイン: 未登録メールでもダミーハッシュで bcrypt 検証を 1 回走らせ、応答時間でメールの登録有無を判別できないようにした
- レート制限: 既定は接続元 IP を使用（`TRUST_PROXY=true` のときだけ X-Forwarded-For 末尾を信用）。`/auth/login` `/auth/register` は 20 回/分/IP。バケット数に上限と期限切れ掃除あり
- ログイン/登録に失敗したとき、旧フロントは失敗を握りつぶして `/` へ遷移していた。Rust 版はエラー表示してその画面に留まる

## 破壊的変更・移行上の注意

- **旧 Prisma が作った SQLite DB はそのままでは読めない**（テーブル/カラム名が変わった: 例 `Account.passwordHash` → `accounts.password_hash`）。データ移行は用意していない。開発 DB は作り直す
- **`backend/prisma/seed.ts` は削除した**（seed データの代替は未用意）
- `tutorial/1day-tutorial.md` は TS 前提のまま（教材のため未更新）
- docker-compose のポートは `127.0.0.1` のみに公開するよう変更した

## レビュー指摘への対応

セキュリティ・コード・テストの 3 レビュー（code-reviewer / security-reviewer / test-analyzer）の指摘のうち、次を反映した。

- `/docs` の Swagger UI に SRI（sha384）と `crossorigin` を付与（固定バージョン 5.17.14 の実ファイルからハッシュを算出）
- `reject` はオーナーを拒否（削除）できない（422 `CANNOT_REJECT_OWNER`。コミュニティの無主化を防ぐ）
- パスワードは最大 72 バイト（bcrypt の切り捨て上限）。超過は 400
- `JWT_SECRET` が 32 バイト未満なら起動時に警告（拒否はしない。E2E / 開発用の短い値を許すため）
- `event` が `community` の controller に依存していた点を解消（`community/access.rs` へ移動）。イベント一覧の可視性判定は `list_events_query` に移した
- `errors.rs` から HTTP マッピングを `error_mappings.rs` に分離（ドメインが axum に依存しない）
- 列挙マクロ `string_enum!` を `shared/macros.rs` に一本化
- フロント: 承認/拒否ボタンが `/auth/me` 復元の遅れで出ない不具合（`is_owner` の非リアクティブ評価）を修正。`:id` 切替時の古いレスポンス上書きを世代番号で防止。取得完了前に「参加する」が一瞬出る問題と、イベント取得エラーの握りつぶしを修正
- テスト追加: JWT 異常系（期限切れ・別 secret・alg 違い・欠落）、認証なし 401、404、非 UUID、ページネーション、ADMIN 権限、PUBLISHED の可視性、OpenAPI/`/docs`、405 の `Allow`、413、EventBus、validation 網羅、フロントの serde 互換

## 既知の未対応（見送ったもの）

- Dockerfile の `rustup` は `curl | sh`（チェックサム未検証）、GitHub Actions のサードパーティ Action はタグ指定（SHA 未固定）
- セキュリティヘッダ（CSP 等）なし。トークンは localStorage 保存のため `inner_html` を使わない運用が前提
- `JWT_SECRET` の最小長は警告のみ（docker-compose の既定値 `docker-dev-secret` は開発用）
- `reject` は ACTIVE な一般メンバーも削除できる（キックとして許容。OWNER のみ保護）
- カバレッジ 80% ゲート・層依存/循環依存/複雑度の自動検査は未移植
- Leptos のページ/コンポーネント単体テストなし（UI は E2E 依存）。E2E に PRIVATE 承認フロー等は未追加
- 1 テストに複数の検証が入っているものがある（受け入れ基準 1:1 に未分割）
- `event::models::CreateEventInput` の `now` / `created_at` / `updated_at` の重複

## 影響

**ポジティブ:**

- 型安全性とメモリ安全性の向上。バックエンド 1 バイナリ + 静的ファイルで配信できる
- Prisma / zod / Express 等の npm 依存（脆弱性対応の頻度が高かった）が本体から消えた

**ネガティブ・リスク:**

- ビルドが重い（初回の cargo / trunk ビルドに数分。Docker イメージに Rust ツールチェーンと trunk を含むためイメージが大きい）
- 旧 `d review` の層依存・循環依存・複雑度の自動検査（dependency-cruiser / madge）は、Rust 版には**未移植**（clippy と規約レビューで代替）。必要なら `cargo-modules` 等を追加する
- カバレッジ 80% ゲートも未移植（`cargo llvm-cov` の導入が必要）
- Docker イメージと GitHub Actions ワークフローは、ローカルでは実行検証していない（初回の `d build` / CI 実行で要確認）
- Leptos のコンポーネント単体テストは書いていない（UI は E2E 依存）
- `docs/decision-logs/` 以前のログや `docs/archive/` は TS 時代の記述のまま（時点の記録として改変しない）

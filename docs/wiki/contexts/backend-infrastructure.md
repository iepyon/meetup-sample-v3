---
type: context
title: infrastructure (backend)
description: sqlx の SQLite 接続とスキーマ（schema.sql）の冪等適用、テストでのインメモリ DB
tags: [backend, sqlx, sqlite, testing]
sources:
  - backend/src/infrastructure/mod.rs
  - backend/src/infrastructure/schema.sql
  - backend/tests/common/mod.rs
  - backend/tests/api.rs
  - backend/tests/api_members.rs
  - backend/tests/system.rs
  - backend/tests/security.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# infrastructure (backend)

SQLite への接続（`SqlitePool`）とスキーマ適用を担う。旧 Prisma スキーマ・`db push`・マイグレーションは廃止された（[rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)）。

## 接続（infrastructure/mod.rs）

- `connect(database_url)` — `file:./dev.db` / `sqlite:...` / 素のパス / `:memory:` を受け付け、`create_if_missing(true)`・`foreign_keys(true)` で接続する。最大接続数は 5、**インメモリ DB は接続ごとに別 DB になるため 1 接続に固定**
- 接続後に `apply_schema` を必ず実行する
- 接続先は環境変数 `DATABASE_URL`（既定 `file:./dev.db`、`main.rs` が読む）

## スキーマ（infrastructure/schema.sql）

- `accounts` / `communities` / `community_members` / `events` の 4 テーブルとインデックス。`CREATE TABLE IF NOT EXISTS` を `;` で分割して順に実行するため**冪等**（起動のたびに実行される）
- 列挙値は `CHECK` 制約（category / visibility / role / status / format など）。ID は UUID の TEXT、日時は ISO 8601 の TEXT
- **既存テーブルの列変更・削除は行わない**（マイグレーション機構なし）。スキーマ変更を反映するには開発 DB ファイル（`dev.db`）を削除して作り直す
- `sqlx` の `migrate` feature は Cargo.toml で有効だが、`sqlx::migrate!` は使っていない

## テスト（backend/tests/）

- HTTP 統合テスト 65 件（2026-11-27 時点）を 4 ファイルに分割: `api.rs`（認証・コミュニティ・イベント）/ `api_members.rs`（メンバー）/ `system.rs`（404・405・413・レート制限・キャッシュ・OpenAPI 等の横断的な挙動）/ `security.rs`（セキュリティヘッダ・415・SPA 配信・権限の境界）。共通ヘルパー（`app` / `app_with_pool` / `call` / `register` / `create_community` / `event_body`）は `common/mod.rs`。テストは `meetup-test-runner`（libtest-mimic、`harness = false`）の `describe` / `test` で書き、名前は `認証 > …` と文字列のまま表示される
- 各テストが `connect("sqlite::memory:")` で独立したインメモリ DB を作り、`create_app(AppState::new(pool, SECRET), false)` を `tower::ServiceExt::oneshot` で直接呼ぶ（ポートは開かない）
- 単体テストは各モジュール内の `#[cfg(test)] mod tests`（`community/models.rs` / `event/models.rs` / `shared/validation.rs` / `shared/middleware.rs` / `shared/auth.rs` / `app.rs`）

## ルール

- SQL は `sqlx::query` / `QueryBuilder` で書き、ユーザー入力は**必ず `bind` / `push_bind`**。文字列連結で値を埋め込まない
- sqlx のエラーメッセージにはクエリ引数が含まれ得るため、`log_internal` は種別だけを出力し、レスポンスには `INTERNAL_ERROR` のみ返す

## 関連

- [Docker 開発環境](../infra/docker-dev.md) — 実行環境側の構成

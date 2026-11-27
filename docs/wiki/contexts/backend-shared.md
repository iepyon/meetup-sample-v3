---
type: context
title: shared（共有カーネル）
description: エラー型・JWT・Event Bus・手書き検証・middleware・OpenAPI など横断基盤
tags: [backend, shared-kernel]
sources:
  - backend/src/shared
  - backend/src/app.rs
  - backend/src/main.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# shared（共有カーネル）

全コンテキスト横断の基盤を提供する共有カーネル。個々のパターンの詳細は [patterns/](../patterns/) の各ページ参照。

## 提供物マップ（backend/src/shared/）

| ファイル | 提供物 | 詳細ページ |
|---|---|---|
| `error.rs` | `ApiError { status, code, message }`、`ApiResult<T>`、`log_internal`（sqlx エラーの種別のみログ）、`is_unique_violation` | [Discriminated Union Errors](../patterns/discriminated-union-errors.md) |
| `auth.rs` | `generate_token`（HS256・24h）、`optional_account` / `require_account` | [auth (backend)](backend-auth.md) |
| `validation.rs` | 手書きのリクエスト検証（`json_object` / `required_string` / `required_email` / `required_enum` / `required_int` / `required_datetime` / `uuid_param` / `page_params`）。違反は 400 `VALIDATION_ERROR` | [OpenAPI-driven Validation](../patterns/openapi-driven-validation.md) |
| `openapi.rs` | 手書きの OpenAPI JSON（`document()`）と Swagger UI の HTML。`/openapi.json` / `/docs` で公開 | 同上 |
| `event_bus.rs` | `EventBus<E>`（`subscribe` / `publish`） | [Event Bus](../patterns/event-bus.md) |
| `middleware.rs` | レート制限・Cache-Control・エラー正規化・fallback（下記） | — |
| `time.rs` | `to_iso` / `from_iso`（ミリ秒精度・Z 終端の ISO 8601。DB にも同形式で保存するため文字列比較が時刻順と一致） | — |

`Result` 型は共有カーネルに置かず std の `Result` を使う（[Result 型](../patterns/result-type.md)）。ID も `String`（UUID）で、Branded Types は無い（[Branded Types](../patterns/branded-types.md)）。

## middleware（shared/middleware.rs。`app.rs` の `create_app` で配線）

- `limits_and_cache`（`from_fn_with_state`）— IP ベース・固定ウィンドウ（60 秒）のレート制限と `Cache-Control` 付与。全体 300 回/分、**未ログイン到達可能な公開 GET**（`/communities`・`/communities/{id}`・`…/members`・`…/events`）は別枠で 60 回/分。超過は 429 `TOO_MANY_REQUESTS`。`X-Forwarded-For` は末尾（リバースプロキシ 1 段信頼）を採用。`Cache-Control` は既定 `private, no-store`、認証ヘッダなしの公開 GET のみ `public, max-age=30, stale-while-revalidate=60`
- `normalize_errors` — axum 標準の本文なしエラー（400 / 405 / 413 / 415）を `{ code, message }` に揃える
- `fallback` — 未定義パスを 404 `NOT_FOUND` にする
- リクエストボディ上限は `DefaultBodyLimit`（100 KiB）。`create_app(state, rate_limit_enabled)` に `false` を渡すとレート制限が無効になる（統合テスト・E2E 用。E2E は環境変数 `RATE_LIMIT_DISABLED=true`）
- 認可（OWNER / ADMIN）は middleware ではなく community の controller 関数 `require_community_role`（[community (backend)](backend-community.md)）

## 起動処理（backend/src/main.rs）

`JWT_SECRET` 必須（未設定・空・32 バイト未満は終了）、`DATABASE_URL`（既定 `file:./dev.db`）、`PORT`（既定 3000）、`RATE_LIMIT_DISABLED`、`FRONTEND_DIST`（指定時はビルド済み SPA を同一オリジンで配信し、`Accept: text/html` の GET は `/docs` `/openapi.json` `/health` 以外 index.html を返す）。

## 関連

- [infrastructure](backend-infrastructure.md) — DB 接続とスキーマ適用

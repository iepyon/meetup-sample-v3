---
type: context
title: auth (backend)
description: アカウント登録・ログイン・JWT 発行と現在アカウント取得
tags: [backend, auth, jwt]
sources:
  - backend/src/auth
  - backend/src/shared/auth.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# auth (backend)

アカウント登録・ログイン・JWT 発行・現在アカウント取得を担う bounded context。分割の経緯は [bounded-context-split](../../decision-logs/arch-20260406-bounded-context-split.md)、Rust 移行の経緯は [rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)。

## エンドポイント（auth/controller.rs、ルート登録は backend/src/app.rs の `create_app`）

| メソッド | パス | 認証 | 内容 |
|---|---|---|---|
| POST | /auth/register | 不要 | 登録。201 で `{token, account}` |
| POST | /auth/login | 不要 | ログイン。200 で `{token, account}` |
| GET | /auth/me | `require_account` | 現在アカウント取得（`{account}`）。アカウントが無ければ 404 `ACCOUNT_NOT_FOUND` |

## 構成（backend/src/auth/）

層は `controller.rs` → `usecases.rs` → `models.rs` / `repository.rs`、エラーは `errors.rs`（いずれも 1 ファイル 1 層）。

- **controller**: 入力検証（`shared/validation.rs` の `required_string` / `required_email`）→ `RegisterCommand` / `LoginCommand` を組み立て → usecase 呼び出し → `generate_token`。ID（`uuid::Uuid::new_v4()`）と時刻（`chrono::Utc::now()`）は controller で生成して command に渡す
- **usecases**: `register`（重複メール確認 → bcrypt ハッシュ化（`spawn_blocking`, cost 10）→ `create_account()` → 永続化）、`login`（メール検索 → bcrypt 検証 → JWT 発行、`LoginResult { token, account }`）
- **models**: `Account`、`create_account()`（失敗しない factory）、`Account::to_response()`（`passwordHash` を含めない）
- **repository**: `AccountRepository`（sqlx。`find_by_id` / `find_by_email` / `save`）
- **errors**: `RegisterError`（`DuplicateEmail { email }` / `Internal`）、`LoginError`（`InvalidCredentials` / `Internal`）。`From<…> for ApiError` で HTTP に変換（409 `DUPLICATE_EMAIL` / 401 `INVALID_CREDENTIALS` / 500）

## JWT（backend/src/shared/auth.rs）

`generate_token(account_id, secret)` — HS256、有効期限 24 時間、claims は `accountId` / `iat` / `exp`（旧実装と互換）。署名鍵は `main.rs` が起動時に環境変数 `JWT_SECRET` から読み、未設定・空なら終了する（既定値へのフォールバックなし）。`AppState.jwt_secret` 経由で controller に渡る。

## 連携・特記

- Event Bus の発行・購読ともに**なし**（[Event Bus](../patterns/event-bus.md) は community / event のみ発行）
- `require_account`（必須。無効なら 401）/ `optional_account`（任意。無効なら `None`）は [shared](backend-shared.md) の `shared/auth.rs` が提供し、`Authorization: Bearer` から accountId を取り出す

## 関連

- frontend 側: [auth (frontend)](frontend-auth.md)
- [Account 集約](../domain/auth/account.md)

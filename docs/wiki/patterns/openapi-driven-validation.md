---
type: pattern
title: 手書き検証と手書き OpenAPI
description: shared/validation.rs の手書き検証（400 VALIDATION_ERROR）と shared/openapi.rs の手書き OpenAPI JSON
tags: [validation, api, core-pattern]
sources:
  - backend/src/shared/validation.rs
  - backend/src/shared/openapi.rs
  - backend/src/app.rs
  - docs/decision-logs/arch-20260419-validation-in-controllers.md
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# 手書き検証と手書き OpenAPI

TypeScript 実装は Zod スキーマから OpenAPI を生成し `express-openapi-validator` で全リクエストを検証していた。Rust 版はどちらも無く、**検証は `shared/validation.rs`、OpenAPI は `shared/openapi.rs` の手書き JSON**（[rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)）。検証を controller 層に置く方針は [validation-in-controllers](../../decision-logs/arch-20260419-validation-in-controllers.md) のとおり。

## 検証（backend/src/shared/validation.rs）

- ボディ: `json_object(&Bytes)`（空・非 JSON は 400 `INVALID_JSON`、オブジェクト以外は 400 `VALIDATION_ERROR`）→ `required_string` / `optional_nullable_string` / `required_email` / `required_enum` / `required_int` / `required_datetime`
- path・query: `uuid_param(name, value)`、`page_params(query)`（`limit` 0〜100・既定 20、`offset` 0 以上・既定 0）
- 違反はすべて 400 `VALIDATION_ERROR`。message は旧 validator に近い形式（例: `request/body/name must NOT have more than 100 characters`）
- 各 controller ハンドラの先頭で明示的に呼ぶ（ミドルウェアによる自動検証ではない）。**呼び忘れた入力は検証されない**
- 検証順序は「検証（400）→ 認証（401）→ 認可（403）→ UseCase」。作成系は旧実装と同じくボディ検証が認証より先

## OpenAPI（backend/src/shared/openapi.rs）

- `document()` が `serde_json::json!` で OpenAPI 3.0.3 の JSON を組み立て、`GET /openapi.json` が返す。`GET /docs` は Swagger UI の HTML（`SWAGGER_UI_HTML`）
- ルート定義（`app.rs` の `create_app`）・検証・OpenAPI はそれぞれ**独立**。**エンドポイントや制約を変えるときは 3 箇所を手で同期する**必要があり、自動では検出されない
- `shared/openapi.rs` が検証に使われることはない（ドキュメント専用）

## 注意（旧 Gotcha の置き換え）

旧実装の「`app.ts` に side-effect import を足さないと OpenAPI に登録されない」は存在しない。代わりに「`openapi.rs` の更新漏れ」がドキュメントと実装のずれの原因になる。

## 関連

- [Schema-derived types](schema-derived-types.md) — 列挙値の許容リスト `ALL` を検証に使う

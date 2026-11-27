---
type: pattern
title: Discriminated Union Errors
description: エラーをユースケースごとの enum で表し、errors.rs の From<…> for ApiError で HTTP に変換する
tags: [error-handling, core-pattern]
sources:
  - backend/src/shared/error.rs
  - backend/src/community/errors.rs
  - backend/src/auth/errors.rs
  - backend/src/event/errors.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Discriminated Union Errors

エラーを判別可能な `enum`（Rust の代数的データ型）で表現し、HTTP レスポンスへの変換を各コンテキストの `errors.rs` に集約する。[Result 型](result-type.md) の `E` に入る型。

## 定義場所

- 各コンテキストの `errors.rs` — 例: `backend/src/community/errors.rs` の `CreateCommunityError { DuplicateCommunityName { name }, TooManyCommunities, Internal }`。ユースケースごとに enum を分ける（`JoinCommunityError` / `LeaveCommunityError` / `ApproveMemberError` …）
- 共通の HTTP エラーは `backend/src/shared/error.rs` の `ApiError { status, code, message }`。コンストラクタ `validation()` / `invalid_json()` / `unauthorized()` / `forbidden()` / `not_found()` / `internal()`

## HTTP へのマッピング

各 `errors.rs` に `impl From<XxxError> for ApiError` を置く。controller は UseCase の `Result` に対して `?` を使うだけで `ApiError` に変換され、`IntoResponse` が `{ "code": "...", "message": "..." }` の JSON にする。

- `match` に **`_` 節を書かない** — 網羅性をコンパイラが保証し、バリアント追加漏れをコンパイルエラーにする
- 業務エラーの例: 409 `DUPLICATE_COMMUNITY_NAME` / `ALREADY_MEMBER` / `DUPLICATE_EMAIL`、422 `TOO_MANY_COMMUNITIES` / `OWNER_CANNOT_LEAVE` / `MEMBER_ALREADY_ACTIVE` / `EVENT_DATE_IN_PAST` / `EVENT_END_BEFORE_START`、404 `COMMUNITY_NOT_FOUND` / `MEMBER_NOT_FOUND`
- 共通コード: 400 `VALIDATION_ERROR` / `INVALID_JSON`、401 `UNAUTHORIZED`、403 `FORBIDDEN`、404 `NOT_FOUND`、429 `TOO_MANY_REQUESTS`、500 `INTERNAL_ERROR`

## ルール

- `Internal` バリアントは 500 にマップし、詳細は返さない。sqlx のエラーは `log_internal`（種別のみ）でログに残す
- message にスタックトレースやクエリ引数を含めない（セキュリティ要件）
- 文字列や panic でエラーを表現しない

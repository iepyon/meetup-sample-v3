---
type: pattern
title: Result<T, E>
description: 例外を投げず成功/失敗を型で表現する。std の Result + コンテキストごとのドメインエラー enum
tags: [error-handling, core-pattern]
sources:
  - backend/src/community/models.rs
  - backend/src/community/errors.rs
  - backend/src/auth/usecases.rs
  - docs/decision-logs/arch-20260419-result-type-no-throw.md
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Result<T, E>

例外（panic）で業務エラーを表さず、成功/失敗を型で表現する。なぜ throw 禁止かは [decision-log](../../decision-logs/arch-20260419-result-type-no-throw.md) 参照。Rust 移行後は**独自の Result 型を持たず、標準の `Result<T, E>` を使う**（[rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)）。`ok()` / `err()` / `map` などのヘルパーは不要になった。

## 使い方

- 失敗しうる factory・遷移関数と UseCase は `Result<T, ドメインエラー>` を返す。分岐は `match` / `?` / `ok_or(...)` で行う
- エラー型 `E` はコンテキストごとの enum（[Discriminated Union Errors](discriminated-union-errors.md)）。各 enum は **`Internal` バリアント**（sqlx 等のインフラ失敗。HTTP 500 にマップされる）を持つ
- sqlx のエラーは `From<sqlx::Error> for <XxxError>` で `Internal` に変換し、`?` で伝播させる（community は `internal_from_sqlx!` マクロ、event は個別 impl、auth は `map_err`）
- 失敗しない関数は `Result` を返さない。例: `create_account`（`Account`）、`create_community`（`(Community, CommunityMember)`）、`join_community`（`CommunityMember`）。失敗しうる `create_event` / `approve_member` / `leave_community` は `Result` を返す

## お手本

- `backend/src/community/models.rs` — `approve_member`（`Result<CommunityMember, ApproveMemberError>`）、`leave_community`
- `backend/src/community/usecases.rs` — `?` で repository / domain のエラーを `Internal` や業務エラーへ流す

## 注意

HTTP 層の `ApiResult<T> = Result<T, ApiError>`（`backend/src/shared/error.rs`）は controller のハンドラ戻り値で、各ドメインエラーは `From<…> for ApiError` で変換される。frontend の `api` 関数が返す `Result<T, ApiFailure>`（`frontend/src/api.rs`）は別の型で、backend とは共有されていない。

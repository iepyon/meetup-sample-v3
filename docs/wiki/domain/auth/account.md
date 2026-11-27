---
type: domain
title: Account 集約
description: 認証の主体となるアカウント。登録時にメール重複を禁止
tags: [aggregate, auth]
sources:
  - backend/src/auth/models.rs
  - backend/src/auth/usecases.rs
  - backend/src/auth/controller.rs
  - backend/src/infrastructure/schema.sql
verified: 5891e84
---

# Account 集約

認証コンテキストの集約。ログイン・JWT の主体。

## フィールド（auth/models.rs の `Account`）

| フィールド | 型 | 制約 |
|---|---|---|
| id | `String` | UUID（controller が発行） |
| name | `String` | 1〜100 文字（controller の `required_string`） |
| email | `String` | email 形式（`required_email` → `is_valid_email`） |
| password_hash | `String` | bcrypt ハッシュ済み（登録入力の生パスワードは 1 文字以上。`required_string(.., 1, usize::MAX)`） |
| created_at | `DateTime<Utc>` | — |

API レスポンスは `Account::to_response()`（`id` / `name` / `email` / `createdAt`。`passwordHash` は含めない）。

## factory・遷移

- `create_account(input) -> Account`（auth/models.rs）— 失敗しない factory（`Result` を返さない）。パスワードのハッシュ化は usecase（`register`）が bcrypt（cost 10）で行い、ハッシュ済みの値を渡す
- 状態 enum・遷移関数はなし

## 不変条件・業務ルール

- **メールの一意性**: 重複メールでの登録は `RegisterError::DuplicateEmail` — 強制ポイント: usecases.rs `register` の `find_by_email` 事前チェック + **DB unique 制約**（schema.sql の `email TEXT NOT NULL UNIQUE`。保存時の unique 違反も `DuplicateEmail` にマップ）の二重
- **ログイン失敗の情報秘匿**: `LoginError::InvalidCredentials` でメール不存在とパスワード不一致を区別しない — 強制ポイント: usecases.rs `login`
- **入力形式（name 長・email 形式）**: 強制ポイント: controller（`shared/validation.rs`）のみ。DB 制約なし

## 関連

- [auth (backend)](../../contexts/backend-auth.md) / [用語集](../glossary.md)

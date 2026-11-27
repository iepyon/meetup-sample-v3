---
type: pattern
title: Branded Types（Rust 版では未採用）
description: 旧 TypeScript 実装の型安全な ID。Rust 版は ID を String（UUID）で扱い、境界で uuid_param 検証する
tags: [type-safety, core-pattern, not-adopted]
sources:
  - backend/src/shared/validation.rs
  - backend/src/community/models.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Branded Types（Rust 版では未採用）

TypeScript 実装では `AccountId` / `CommunityId` などの brand 付き型で ID の取り違えをコンパイル時に防いでいた。**Rust 版にはこのパターンは存在しない**。`Community.id` / `CommunityMember.account_id` などの ID はすべて `String`（UUID 文字列）で、newtype も導入していない（[rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)）。

## 現状の ID の扱い

- **発行**: controller が `uuid::Uuid::new_v4().to_string()` で生成して command に渡す（UseCase 内では生成しない。[UseCase orchestration only](../../decision-logs/arch-20260419-usecase-orchestration-only.md)）
- **外部入力の検証**: path の ID は `shared/validation.rs` の `uuid_param(name, value)` で 8-4-4-4-12 の 16 進を検証し、違反は 400 `VALIDATION_ERROR`
- **DB**: `schema.sql` の ID 列は TEXT、外部キーは `foreign_keys(true)` で有効
- 引数の取り違え（例: `community_id` と `account_id`）は型では防げない。シグネチャのパラメータ名と、`find_by_ids(community_id, account_id)` のような呼び出し規約に依存する

## 関連

- [Schema-derived types](schema-derived-types.md) — 列挙値は `string_enum!` マクロで型安全にしている
- 型で ID を区別したい場合は newtype（`struct CommunityId(String)`）の導入が選択肢だが、現状は未実施

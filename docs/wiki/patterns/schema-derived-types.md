---
type: pattern
title: Schema-derived Types（列挙型の単一定義）
description: 列挙値は string_enum! マクロの 1 箇所の定義から型・文字列定数・parse を導出する
tags: [type-safety, validation, core-pattern]
sources:
  - backend/src/community/models.rs
  - backend/src/event/models.rs
  - backend/src/infrastructure/schema.sql
  - frontend/src/types.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Schema-derived Types（列挙型の単一定義）

TypeScript 実装では Zod スキーマを単一の真実として型・定数を導出していた。Rust 版に Zod は無く、**列挙値を `string_enum!` マクロ（backend）/ `api_enum!` マクロ（frontend）の 1 箇所で定義**し、そこから enum 型・文字列表現・パースを導出する。

## お手本（backend/src/community/models.rs）

```
string_enum!(CommunityCategory { Tech => "TECH", Business => "BUSINESS", Hobby => "HOBBY" });
```

マクロが生成するもの: `enum CommunityCategory`、`ALL: &[&str]`（検証用の許容値一覧）、`as_str()`、`parse(&str) -> Option<Self>`。同様に `CommunityVisibility` / `MemberRole` / `MemberStatus`（community）、`EventFormat` / `EventStatus`（`event/models.rs`）。

## 検証との接続

- controller の入力検証は `required_enum(&obj, "category", CommunityCategory::ALL)`（`shared/validation.rs`）で許容値を `ALL` から取る。違反は 400 `VALIDATION_ERROR`
- DB の `schema.sql` にも同じ値の `CHECK` 制約があるが、**二重定義**（マクロと SQL は手で同期する）。値を追加するときは両方と frontend の `api_enum!`（`frontend/src/types.rs`）を更新する
- 文字列リテラル（`"PUBLIC"` など）をロジックに直書きしない。`CommunityVisibility::Public` のようなバリアントを使う（SQL 文中のリテラルは例外）
- ドメイン制約（文字数・範囲）は Zod ではなく controller の `required_string(.., min, max)` / `required_int(.., min, max)` に書く。OpenAPI（`shared/openapi.rs`）の `minLength` / `maximum` は**手で複製**しているため、変更時は両方を更新する（[OpenAPI-driven Validation](openapi-driven-validation.md)）

## 関連

- [Readonly Interface](readonly-interface.md) — エンティティ型と遷移関数
- ドメイン制約の実際の値は [domain/](../domain/) の各集約ページ参照

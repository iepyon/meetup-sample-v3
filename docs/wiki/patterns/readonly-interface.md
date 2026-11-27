---
type: pattern
title: Readonly Interface
description: エンティティを振る舞いを持たない struct + 新しい値を返す遷移関数で不変的に扱う
tags: [immutability, core-pattern]
sources:
  - backend/src/community/models.rs
  - backend/src/event/models.rs
  - frontend/src/types.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Readonly Interface

エンティティを「振る舞いを持たないデータ struct」として定義し、状態変更は**新しい値を返す遷移関数**で表現する。TypeScript の `readonly` interface に相当する意図を、Rust では所有権と `&T` 渡しで実現している。

## お手本

- `backend/src/community/models.rs` — `Community` / `CommunityMember` は `pub` フィールドのみの struct（`#[derive(Debug, Clone)]`）。メソッドはレスポンス変換の `to_response()` だけで、状態変更メソッドは持たない
- 遷移関数は `&CommunityMember` を受け取り新しい値を返す: `approve_member(member) -> Result<CommunityMember, ApproveMemberError>`（`CommunityMember { status: Active, ..member.clone() }`）
- factory も free function: `create_community(input) -> (Community, CommunityMember)` / `create_event(input) -> Result<Event, CreateEventError>`
- frontend も同様: `frontend/src/types.rs` の `Community` / `CommunityMember` は DTO の struct

## ルール

- 集約に `&mut self` で状態を変えるメソッドを生やさない。遷移は models の関数が新しい値を返す形にする
- 集約の部分更新に struct update 構文（`..member.clone()`）を UseCase 内で使うのは禁止 — 遷移ロジックは models 側に置く（[UseCase orchestration only](../../decision-logs/arch-20260419-usecase-orchestration-only.md)）
- 注意: フィールドは `pub` で、TS の `readonly` のような型レベルの不変保証はコンパイラが強制しない（`let mut` を使わない規約で守る）

## 関連

- 各集約の実際の遷移関数一覧は [domain/](../domain/) の集約ページ参照
- [Result 型](result-type.md) / [Schema-derived types](schema-derived-types.md)

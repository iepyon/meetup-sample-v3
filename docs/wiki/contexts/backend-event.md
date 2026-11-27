---
type: context
title: event (backend)
description: コミュニティ配下のイベント作成（DRAFT）とイベント一覧
tags: [backend, event]
sources:
  - backend/src/event
  - backend/src/app.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# event (backend)

コミュニティ配下のイベント作成とライフサイクルを担う bounded context。層は `controller.rs` → `usecases.rs` → `models.rs` / `repository.rs`、エラーとドメインイベントは `errors.rs`。

## エンドポイント（event/controller.rs、ルートは `app.rs` の `/communities/{id}/events`）

| メソッド | パス | 認証 | 内容 |
|---|---|---|---|
| GET | /communities/{id}/events | `optional_account` | 一覧（`limit` / `offset`）。DRAFT は作成者本人のみ可視。開始日時の昇順 |
| POST | /communities/{id}/events | `require_account` + `require_community_role` | 作成。DRAFT で 201 `{event}` |

作成時の検証は `shared/validation.rs`（title 1〜100 / description 最大 1000・nullable / `startsAt`・`endsAt` は RFC 3339 / format enum / capacity 1〜1000）。検証 400 → 認証 401 → ロール 403 → usecase（422 / 404）の順。

## 構成（backend/src/event/）

- **usecases**: `create_event_command`（コミュニティ存在確認 → `create_event()`（開始/終了日時の検証）→ 永続化 → `EventCreated` 発行）
- **models**: `Event`、`EventFormat` / `EventStatus`（`string_enum!` マクロ由来）、`create_event()`（`Result<Event, CreateEventError>`、必ず DRAFT）
- **repository**: `EventRepository`（`save` / `list_by_community`）
- **errors**: `CreateEventError`（`CommunityNotFound` / `EventDateInPast` / `EventEndBeforeStart` / `Internal`）、`EventCreatedEvent`（struct）

## 特記

- **一覧は usecase を経由しない**: `GET /communities/{id}/events` は controller が `EventRepository::list_by_community` を直接呼ぶ（他コンテキストの query パターンと非対称。Read モデルの意図的な例外）。可視性条件は SQL の `(status != 'DRAFT' OR created_by = ?)`
- 一覧ハンドラはコミュニティの存在・公開設定・メンバーシップを確認しない（確認するのは DRAFT の可視性のみ）
- **community への import 依存**: `controller.rs` が community の `Repos` / `require_community_role` / `MANAGER_ROLES`、`usecases.rs` が `CommunityRepository` を直接 import。依存方向は event → community の一方向
- Event Bus は `AppState.event_events`（`EventBus<EventCreatedEvent>`）で community の bus とは**別インスタンス**。`EventCreated` を発行するが購読者なし（[Event Bus](../patterns/event-bus.md)）

## 関連

- [Event 集約](../domain/event/event.md) / [イベントのライフサイクル](../domain/event/event-lifecycle.md)
- [community (backend)](backend-community.md)

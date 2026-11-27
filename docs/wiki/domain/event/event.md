---
type: domain
title: Event 集約
description: コミュニティ配下のイベント。生成時は必ず DRAFT、日時バリデーション付き
tags: [aggregate, event]
sources:
  - backend/src/event/models.rs
  - backend/src/event/usecases.rs
  - backend/src/event/controller.rs
  - backend/src/event/repository.rs
  - backend/src/infrastructure/schema.sql
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 5891e84
---

# Event 集約

コミュニティ配下で開催されるイベント。

## フィールド（event/models.rs の `Event`）

| フィールド | 型 | 制約 |
|---|---|---|
| id | `String` | UUID |
| community_id | `String` | `communities.id` への外部キー |
| created_by | `String` | 作成者のアカウント ID（`accounts.id` への外部キー。API レスポンスの `event` には含めない） |
| title | `String` | 1〜100 文字（controller の `required_string`） |
| description | `Option<String>` | 最大 1000 文字・nullable |
| starts_at / ends_at | `DateTime<Utc>` | RFC 3339 の date-time（`required_datetime`） |
| format | `EventFormat` | `ONLINE` \| `OFFLINE` \| `HYBRID` |
| capacity | `i64` | 整数 1〜1000（`required_int`） |
| status | `EventStatus` | `DRAFT` \| `PUBLISHED` \| `CLOSED` \| `CANCELLED` |
| created_at / updated_at | `DateTime<Utc>` | — |

## factory・遷移

- `create_event(input) -> Result<Event, CreateEventError>`（event/models.rs）— 生成時は必ず `status=DRAFT`。`input.now`（controller が渡す現在時刻）と比較する

## 不変条件・業務ルール

- **日時ルール**: `starts_at <= now` → `CreateEventError::EventDateInPast`、`ends_at <= starts_at` → `EventEndBeforeStart`（いずれも 422）— 強制ポイント: models の `create_event` factory（DB 制約なし）
- **入力の値域**（title・description の長さ、capacity 1〜1000、format）— 強制ポイント: controller の `shared/validation.rs`。format / status は DB CHECK 制約もあるが、長さ・capacity は DB 制約なし
- **作成できるのは OWNER / ADMIN のみ** — 強制ポイント: event/controller.rs の `create` が呼ぶ `require_community_role`（community の controller 関数。非 OWNER/ADMIN は 403。[メンバーシップのルール](../community/membership-rules.md)）
- **DRAFT は作成者本人のみ閲覧可** — 強制ポイント: `EventRepository::list_by_community` の SQL 条件 `(status != 'DRAFT' OR created_by = ?)`（[イベントのライフサイクル](event-lifecycle.md)）
- 作成成功で `EventCreated` イベント発行（event/usecases.rs の `create_event_command`。[ドメインイベント](../domain-events.md)）
- **【設計のみ・D3】定員は確定者数（APPROVED）を下回る値に減らせない** — 編集（`UpdateEvent`）時に `newCapacity < approvedCount` なら `CapacityBelowApprovedCountError`。超過状態は満席判定（`GetRemainingCapacity`）を狂わせ、キャンセル時の自動繰り上げが超過を膨らませるため。強制ポイント: 未実装（設計のみ）。participation BC の確定者数取得に依存する。決定は EventStorming セッション `meetup-20260712` の D3

## 現状の実装

- **実装済み**: DRAFT でのイベント作成・一覧・可視性ルール
- **先行定義**: `EventStatus` の `PUBLISHED` / `CLOSED` / `CANCELLED` は enum 値のみで、**遷移させる関数は未実装**（[イベントのライフサイクル](event-lifecycle.md)）
- **未実装（設計のみ）**: 上記 D3 の定員減員ガード（`CapacityBelowApprovedCountError`）。DML / issue にのみ存在する新ルール

## 関連

- [event (backend)](../../contexts/backend-event.md) / [用語集](../glossary.md) / [イベントのライフサイクル](event-lifecycle.md)

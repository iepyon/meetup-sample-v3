---
type: domain
title: イベントのライフサイクル
description: EventStatus（DRAFT/PUBLISHED/CLOSED/CANCELLED）の現状 — 遷移関数は未実装、DRAFT のみ生成される
tags: [business-rule, event, lifecycle]
sources:
  - backend/src/event/models.rs
  - backend/src/event/controller.rs
  - backend/src/event/repository.rs
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 5891e84
---

# イベントのライフサイクル

## 状態の定義（event/models.rs の `EventStatus`）

`DRAFT`（下書き・作成直後）→ `PUBLISHED`（公開）→ `CLOSED`（締切）、および `CANCELLED`（中止）。

## 遷移の設計（未実装・設計のみ）

EventStorming セッション `meetup-20260712` で確定した遷移条件（DML / [issues-20260712](../../../issues-20260712.md) の event context）。コードは未実装で、着手時の仕様として保持する。

| 遷移 | CMD | 条件 | 違反時のエラー |
|---|---|---|---|
| DRAFT → PUBLISHED | `PublishEvent` | `status=DRAFT` のみ | `EventAlreadyPublishedError` |
| PUBLISHED → CLOSED | `CloseEvent` | `status=PUBLISHED` のみ | `EventNotYetHeldError` |
| PUBLISHED → CANCELLED | `CancelEvent` | `status=PUBLISHED` **かつ開催日前**のみ | 開催済み: `EventAlreadyOccurredError` / 非公開等: `EventNotCancellableError` |
| （状態不変）編集 | `UpdateEvent` | `status` が DRAFT / PUBLISHED のみ | `EventNotEditableError` |

- **中止（CANCELLED）** は開催日前限定。当日会場に人が来る事故を防ぐため、開催済みは「終了（CLOSED）」で閉じる。`EventCancelled` は中止理由 `reason` を持ち、全員向け中止通知の本文素材になる
- **終了（CloseEvent）／中止（CancelEvent）はそれぞれ通知ポリシーの起点**（アンケート依頼 / 中止のお知らせ）。[ドメインイベント一覧](../domain-events.md)参照

## 現状の実装

- **実装済み**: `create_event` は必ず `status=DRAFT` で生成する。可視性ルール・日時ルールも実装済み
- **先行定義**: `PUBLISHED` / `CLOSED` / `CANCELLED` は enum 値のみ存在し、**遷移させる関数は未実装**。将来のライフサイクル実装の下地であり、「publish/close/cancel が実装済み」と読むのは誤り
- **未実装（設計のみ）**: 上表の遷移条件・エラー・`EventCancelled.reason`。DML / issue にのみ存在する
- 新しい遷移を実装するときは [CommunityMember](../community/community-member.md) の `approve_member` と同じく、models 側に `Result` を返す遷移関数（`approve_member` のように新しい値を返す関数）を置く（[Readonly Interface](../../patterns/readonly-interface.md)）

## 可視性ルール

- **DRAFT は作成者本人のみ閲覧可**。一覧（`GET /communities/{id}/events`）は「DRAFT でない OR 自分が created_by」で絞る（`EventRepository::list_by_community` の SQL。開始日時の昇順）。未ログインは DRAFT を一切見られない
- この一覧は usecase を経由せず controller が repository を直接呼ぶ Read モデルの例外（[event (backend)](../../contexts/backend-event.md) 参照）

## 日時ルール

- 開始日時が過去 → `EventDateInPast`、終了が開始以前 → `EventEndBeforeStart` — 強制ポイント: models の `create_event` factory（DB 制約なし）

## 関連

- [Event 集約](event.md) / [メンバーシップのルール](../community/membership-rules.md)（作成権限）
- 下流の設計: [Participation 集約](../participation/participation.md) / [Notification 集約](../notification/notification.md)

---
type: domain
title: Notification 集約（設計のみ）
description: 「どの出来事で・誰に・何を送ったか」の通知レコード。6 種の宛先ルールを持つ。コード未実装
tags: [aggregate, notification, design-only]
sources:
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 1929475
---

# Notification 集約（設計のみ）

「どの出来事をきっかけに、誰に、何を送ったか」を一手に引き受け、通知の全量をこの文脈の一覧で見えるようにする。宛先決定は各 POLICY が担い、本集約は送付事実の監査可能な記録に徹する。

> **実装状況: 未実装（設計のみ）**。backend に notification BC は存在しない。EventStorming セッション `meetup-20260712`（DML / [issues-20260712](../../../issues-20260712.md) の notification context）に基づく着手前の設計。配送手段（メール/プッシュ）は既製サービスへ委譲し、ここは記録と種別だけを持つ。

## フィールド（DML aggregates.Notification）

| フィールド | 型 | 制約 |
|---|---|---|
| notificationId | `NotificationId`（UUID の String 想定） | UUID |
| type | `NotificationType` | 6 種（下表） |
| recipientId | `AccountId` | 宛先。主催者通知は主催者の accountId |
| eventId | `EventId` | 対象イベント |
| createdAt | Date | — |

## 6 種の通知と宛先ルール

各行は「トリガーとなる上流イベント → 購読 POLICY → 宛先」。宛先ルールがこの集約の要。

| type | トリガー / POLICY | 宛先 |
|---|---|---|
| `APPROVAL` | `ParticipationApproved` / `SendApprovalOnApproved` | 承認された本人 |
| `WAITLIST_PROMOTED` | `WaitlistPromoted` / `NotifyWaitlistPromotion` | 繰り上がった**本人のみ**（主催者へは送らない・**D2**） |
| `PARTICIPANT_CANCELLED` | `ParticipationCancelled` / `NotifyOrganizerOnCancel` | イベント主催者 |
| `REMINDER` | `EventDateApproached` / `SendReminder` | **承認済み参加者のみ**（キャンセル待ちには送らない） |
| `SURVEY` | `EventClosed` / `SendSurveyOnClose` | 承認済み参加者 |
| `EVENT_CANCELLED` | `EventCancelled` / `NotifyEventCancelled` | 申込済み＋確定＋キャンセル待ちの**全員**・文面共通（**D4**） |

- BULK 通知（リマインダー・アンケート・中止）は宛先 0 件なら no-op
- 中止通知の宛先クエリ `AllActiveParticipations` は `status IN (APPLIED, APPROVED, WAITLISTED)`。承認前 APPLIED も含めるのが D4 の決定（承認まで数日空く運用で申込者に中止が届かない穴を塞ぐ）

## 意図的に「無い」通知（D5）

**イベント編集（`EventUpdated`）の変更通知は送らない**。会場変更などは主催者が手動周知するのが現状の運用で、条件付き自動通知の設計は宿題 A1。「抜けている」のではなく「無いことが決まっている」。

## 現状の実装

- **未実装（設計のみ）**: 集約・6 通知の UseCase・POLICY いずれもコードに存在しない。[Event Bus](../../patterns/event-bus.md) は publish のみで購読者ゼロのため、ここに至る連鎖も未配線

## 関連

- [ドメインイベント一覧](../domain-events.md) / [Participation 集約](../participation/participation.md) / [Event 集約](../event/event.md) / [用語集](../glossary.md)

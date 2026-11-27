---
type: domain
title: ドメインイベント一覧
description: 7 つのドメインイベントの発火ポイント。現状 publish のみで購読者はゼロ
tags: [domain-events, business-rule]
sources:
  - backend/src/community/errors.rs
  - backend/src/community/usecases.rs
  - backend/src/event/errors.rs
  - backend/src/event/usecases.rs
  - backend/src/app.rs
  - docs/decision-logs/arch-20260419-in-memory-event-bus.md
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 5891e84
---

# ドメインイベント一覧

[Event Bus](../patterns/event-bus.md) に発行されるドメインイベントの一覧と発火条件。

## 一覧

| イベント | 発行元 usecase（関数） | 発火条件 |
|---|---|---|
| `CommunityCreated` | community/usecases.rs `create_community_command` | コミュニティ作成成功 |
| `MemberJoined` | `join_community_command` | **PUBLIC** への参加（ACTIVE） |
| `MemberApplicationSubmitted` | `join_community_command` | **PRIVATE** への参加申請（PENDING） |
| `MemberApproved` | `approve_member_command` | 承認成功 |
| `MemberRejected` | `reject_member_command` | 拒否成功 |
| `MemberLeft` | `leave_community_command` | 脱退成功 |
| `EventCreated` | event/usecases.rs `create_event_command` | イベント作成成功 |

## 定義場所

- community 側: `community/errors.rs` の enum `CommunityDomainEvent`（6 バリアント。エラー型と同居している点に注意）
- event 側: `event/errors.rs` の struct `EventCreatedEvent`
- bus は `app.rs` の `AppState`（`community_events` / `event_events`）が保持

## 現状の実装

- **実装済み**: 上記 7 イベントの publish（各 usecase から発行）
- **先行定義**: イベント型は定義済みだが、**購読者（subscriber）はコードベースに一つも存在しない**。クロスコンテキストの副作用は未配線で、community と event の bus は別インスタンス（`AppState` の別フィールド）で共有もされていない
- ここに書かれたイベントは「発火ポイントと将来の拡張点」であり、「実装済みの副作用」ではない。設計背景は [in-memory-event-bus](../../decision-logs/arch-20260419-in-memory-event-bus.md)

## 設計のみ（未実装）のイベントとポリシー連鎖

EventStorming セッション `meetup-20260712`（DML / [issues-20260712](../../issues-20260712.md)）で定義された、参加ライフサイクルと通知のイベント。**コードには一切存在しない**（対応する BC も未実装）。ポリシー（POLICY）は上流イベントを購読して下流コマンドを起こす因果連鎖。

| 発行元 BC | イベント | 購読ポリシー → 起こす結果 |
|---|---|---|
| event | `EventPublished` | （申込受付の開始条件） |
| event | `EventUpdated` | なし（変更通知は意図的に送らない・D5） |
| event | `EventClosed` | `SendSurveyOnClose` → `SurveySent`（承認済みへアンケート） |
| event | `EventCancelled` | `NotifyEventCancelled` → `CancellationNotificationSent`（申込済み＋確定＋待ち全員・D4） |
| event | `EventDateApproached` | `SendReminder` → `ReminderSent`（承認済みへリマインダー） |
| participation | `ParticipationApplied` / `ParticipationWaitlisted` | （満席判定による WHEN 分岐） |
| participation | `ParticipationApproved` | `SendApprovalOnApproved` → `ApprovalNotificationSent`（本人へ確定通知） |
| participation | `ParticipationCancelled`（param `previousStatus`） | `PromoteFromWaitlist` → `WaitlistPromoted`（`previousStatus=APPROVED` 時のみ）／ `NotifyOrganizerOnCancel` → `OrganizerNotifiedOnCancel`（主催者へ） |
| participation | `WaitlistPromoted` | `NotifyWaitlistPromotion` → `WaitlistPromotionNotified`（**本人のみ**・D2） |
| checkin | `CheckedIn` | なし（終端イベント） |

- 通知系イベント（`*Sent` / `*Notified`）は notification BC が発行する終端イベント。宛先ルールは [Notification 集約](notification/notification.md) 参照
- 繰り上げ連鎖の詳細（発火条件・暴発ガード）は [キャンセル待ちと自動繰り上げ](participation/waitlist-promotion.md) 参照

## 関連

- [Community 集約](community/community.md) / [CommunityMember 集約](community/community-member.md) / [Event 集約](event/event.md)
- 設計のみ: [Participation 集約](participation/participation.md) / [CheckIn 集約](checkin/checkin.md) / [Notification 集約](notification/notification.md)

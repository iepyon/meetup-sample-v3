---
type: domain
title: Participation 集約（設計のみ）
description: 参加申込 1 件の状態（申込済/待ち/確定/キャンセル）。満席時のキャンセル待ちと自動繰り上げを担う CORE。コード未実装
tags: [aggregate, participation, design-only]
sources:
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 1929475
---

# Participation 集約（設計のみ）

参加申込 1 件の状態の単一の真実源。満席でも断らず申込順で待たせ、キャンセルの瞬間に自動で繰り上げる、このドメインの差別化の心臓部（CORE_SUBDOMAIN）。

> **実装状況: 未実装（設計のみ）**。backend に participation BC は存在しない。本ページは EventStorming セッション `meetup-20260712`（DML / [issues-20260712](../../../issues-20260712.md) の participation context）に基づく着手前の設計。

## フィールド（DML aggregates.Participation）

| フィールド | 型 | 制約 |
|---|---|---|
| participationId | `ParticipationId`（UUID の String 想定） | UUID |
| eventId | `EventId` | 対象イベント |
| accountId | `AccountId` | 申込者 |
| status | `ParticipationStatus` | `APPLIED` \| `WAITLISTED` \| `APPROVED` \| `CANCELLED` |
| appliedAt | Date | 申込日時。キャンセル待ちの順序（申込順・追い越し禁止）の根拠 |

## 遷移関数（設計）

すべて `Result<Participation, E>` を返す想定（[Result 型](../../patterns/result-type.md)）。

- `applyForEvent` — 生成。残席（`GetRemainingCapacity`）があれば `APPLIED`、満席なら `WAITLISTED` で直接生成（SAME-TX の WHEN 分岐）
- `approveParticipations` — `APPLIED → APPROVED`（主催者の一括承認）
- `promoteWaitlistEntry` — `WAITLISTED → APPROVED`（キャンセル時の自動繰り上げ・再承認不要）
- `cancelParticipation` — `APPLIED | APPROVED → CANCELLED`

## 状態遷移

```
（残席あり申込）─► APPLIED ─approveParticipations─► APPROVED
（満席申込）───► WAITLISTED ─promoteWaitlistEntry─► APPROVED
APPLIED ─cancelParticipation─► CANCELLED
APPROVED ─cancelParticipation─► CANCELLED（繰り上げ・主催者通知の起点）
```

- WAITLISTED は入口状態の一つ（満席時に直接生成）。APPLIED からの遷移ではない
- WAITLISTED / CANCELLED からのキャンセルは不可（待ち離脱の扱いは Q4/Q5 の検討事項）

## 不変条件・業務ルール

- **同一人物は同一イベントに 1 申込のみ** — 違反は `AlreadyAppliedError`。強制ポイント: 未実装（設計では models + DB 複合 unique の二重を想定）
- **申込できるのは PUBLISHED イベントのみ** — 違反は `EventNotAvailableError`。強制ポイント: 未実装
- **承認対象は APPLIED のみ**（キャンセル待ち・確定済みは対象外）— 違反は `InvalidStatusTransitionError`。強制ポイント: 未実装
- **`ParticipationCancelled` は `previousStatus` を持つ** — 確定（APPROVED）のキャンセルか申込済み（APPLIED）のキャンセルかで繰り上げ要否が変わる判断材料（[キャンセル待ちと自動繰り上げ](waitlist-promotion.md)）
- 残席の算出: `GetRemainingCapacity` = `Event.capacity − count(status IN (APPLIED, APPROVED))`。APPLIED も席を消費する

## 現状の実装

- **未実装（設計のみ）**: 集約・状態・遷移関数・エラーいずれもコードに存在しない。DML / issue にのみ存在する

## 関連

- [キャンセル待ちと自動繰り上げ](waitlist-promotion.md) / [Event 集約](../event/event.md) / [Notification 集約](../notification/notification.md)
- [ドメインイベント一覧](../domain-events.md) / [用語集](../glossary.md)

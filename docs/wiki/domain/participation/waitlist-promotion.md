---
type: domain
title: キャンセル待ちと自動繰り上げ（設計のみ）
description: 満席時のキャンセル待ちと、キャンセル発生時の先頭自動繰り上げの業務ルール。発火条件と暴発ガードを含む。コード未実装
tags: [business-rule, participation, waitlist, design-only]
sources:
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 1929475
---

# キャンセル待ちと自動繰り上げ（設計のみ）

満席でも断らず待たせ、キャンセルの瞬間に席を無駄にせず繰り上げる CORE の業務ルール。

> **実装状況: 未実装（設計のみ）**。EventStorming セッション `meetup-20260712`（DML / [issues-20260712](../../../issues-20260712.md)）に基づく着手前の設計。

## キャンセル待ちの登録

- 申込時に満席（`GetRemainingCapacity = 0`）なら、断らず `WAITLISTED` で登録し、**申込順（`appliedAt` 昇順）**で列に並ぶ
- 順序は追い越し禁止。繰り上げは必ず列の先頭 1 件から

## 自動繰り上げ（POLICY PromoteFromWaitlist）

キャンセル（`ParticipationCancelled`）を購読し、キャンセル待ち先頭（`NextWaitlistedParticipation`）を `APPROVED` に繰り上げる。

- **再承認なしの自動確定**（[D1](#意思決定)）。一度申し込んだ人へ空いた瞬間に席を渡す
- **1 キャンセル = 1 繰り上げ**。超過を作らない
- 繰り上がった本人にのみ確定通知（`WaitlistPromotionNotified`）。**主催者へは送らない**（[D2](#意思決定)）

## 発火条件と暴発ガード

意味チェック（saga）で洗い出した、繰り上げを誤爆させないための条件:

- **`previousStatus = APPROVED` のキャンセル時のみ発火する**。APPLIED（承認前・枠未消費）のキャンセルで繰り上げると定員超過を作る。判断材料は `ParticipationCancelled.previousStatus`
- **対象イベントが CANCELLED / CLOSED なら繰り上げない**。中止済み・終了済みイベントへ繰り上げてしまう暴発を防ぐ
- キャンセル待ちが 0 件なら no-op

## D3 との関係

定員を確定者数未満に減らせないルール（[D3](#意思決定)・[Event 集約](../event/event.md)）は、この繰り上げの前提を守るためのもの。超過状態を許すと満席判定が狂い、キャンセル発生時の自動繰り上げが超過をさらに膨らませる。

## 意思決定

DML `meetup-20260712` の decisions より。

- **D1**: 繰り上げは主催者の再承認を挟まず自動確定
- **D2**: 繰り上げ通知は本人のみ（主催者へは送らない）
- **D3**: 定員は確定者数を下回る値に減らせない

## 未決事項（Q4 / Q5）

- **Q4**: イベント中止後の参加レコードの後始末（一括取消するか・待ち列をクリアするか）。一括取消は `ParticipationCancelled` 連発 → 繰り上げ暴発の連鎖リスクがあり、ガード設計とセットで要検討
- **Q5**: 一括承認で承認されなかった申込（否認・保留）の扱い。否認イベント・通知が未定義

## 現状の実装

- **未実装（設計のみ）**: ポリシー・繰り上げロジックいずれもコードに存在しない

## 関連

- [Participation 集約](participation.md) / [Notification 集約](../notification/notification.md) / [ドメインイベント一覧](../domain-events.md)

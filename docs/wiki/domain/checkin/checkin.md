---
type: domain
title: CheckIn 集約（設計のみ）
description: 当日来場の事実記録。参加確定 1 件に対し最大 1 回のチェックインを保証する。コード未実装
tags: [aggregate, checkin, design-only]
sources:
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
  - docs/issues-20260712.md
verified: 1929475
---

# CheckIn 集約（設計のみ）

当日の来場受付。確定済みの参加に対して 1 回だけチェックインを記録する。出席実績をアンケート・リマインダーとは別軸で残し、出席率把握に使う。

> **実装状況: 未実装（設計のみ）**。backend に checkin BC は存在しない。EventStorming セッション `meetup-20260712`（DML / [issues-20260712](../../../issues-20260712.md) の checkin context）に基づく着手前の設計。

## フィールド（DML aggregates.CheckIn）

| フィールド | 型 | 制約 |
|---|---|---|
| checkInId | `CheckInId`（UUID の String 想定） | UUID |
| participationId | `ParticipationId` | 参加確定 1 件につき最大 1 レコード |
| checkedInAt | Date | チェックイン時刻 |

## 遷移関数（設計）

- `checkIn` — `CheckIn` を生成し `CheckedIn`（終端イベント）を発火する。状態機械は持たない（生成のみ）

## 不変条件・業務ルール

- **参加確定（APPROVED）の参加のみチェックイン可** — 対応する [Participation](../participation/participation.md) が APPROVED でなければ `ParticipationNotApprovedError`。強制ポイント: 未実装
- **1 参加につきチェックインは 1 回のみ** — 重複は `CheckInAlreadyExistsError`。強制ポイント: 未実装（設計では participationId の DB unique を想定）
- 現場の主要関心事は「未確定者の受付通過」と「二重チェックイン」の拒否（T6・T8）

## 現状の実装

- **未実装（設計のみ）**: 集約・コマンド・エラーいずれもコードに存在しない

## 関連

- [Participation 集約](../participation/participation.md) / [ドメインイベント一覧](../domain-events.md)

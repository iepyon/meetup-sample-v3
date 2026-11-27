---
type: domain
title: ユビキタス言語（用語集）
description: ドメイン用語の日本語⇔コード上の英語名の対応表
tags: [glossary, ubiquitous-language]
sources:
  - backend/src/community/models.rs
  - backend/src/event/models.rs
  - backend/src/infrastructure/schema.sql
  - docs/eventstorming/eventstorming-20260712-1141.dml.yaml
verified: 5891e84
---

# ユビキタス言語（用語集）

コードの列挙型はすべて `string_enum!` マクロ（backend。frontend は `api_enum!`）の 1 箇所の定義から導出する（[Schema-derived types](../patterns/schema-derived-types.md)。ハードコード文字列禁止）。コード上の型名は backend の `community/models.rs` / `event/models.rs` のもの。

## 集約・エンティティ

| 日本語 | コード | 定義 |
|---|---|---|
| アカウント | `Account` | [Account 集約](auth/account.md) |
| コミュニティ | `Community` | [Community 集約](community/community.md) |
| メンバー（所属） | `CommunityMember` | [CommunityMember 集約](community/community-member.md) |
| イベント | `Event` | [Event 集約](event/event.md) |
| 参加申込 | `Participation` | [Participation 集約](participation/participation.md)（**設計のみ・未実装**） |
| チェックイン記録 | `CheckIn` | [CheckIn 集約](checkin/checkin.md)（**設計のみ・未実装**） |
| 通知レコード | `Notification` | [Notification 集約](notification/notification.md)（**設計のみ・未実装**） |

## 紛らわしい用語の弁別

| 用語 | 意味 |
|---|---|
| `Event` 集約 | 勉強会イベント（コミュニティ配下で開催されるもの）。[Event 集約](event/event.md) |
| ドメインイベント | Event Bus に publish される「起きた事実」の記録。[ドメインイベント一覧](domain-events.md) |
| `EventCreated` | 「Event 集約が作成された」ことを表すドメインイベント（両者が交差する例） |
| キャンセル（参加） vs 中止（イベント） | `Participation` の「キャンセル」は参加の取りやめ（`CancelParticipation`）。`Event` の「中止」はイベント自体の取り止め（`CancelEvent`）。英語も別（cancel-participation / cancel-event）で、CS の表記ゆれの温床 |
| 承認 vs 確定 | 同一の瞬間を指す二面語。主催者視点の操作が「承認」（`ApproveParticipations`）、参加者視点の結果が「確定」（`ParticipationApproved` = 参加が確定した）。ラベル差は意図的 |

## CommunityCategory（コミュニティのカテゴリ）

| 値 | 意味 |
|---|---|
| `TECH` | 技術 |
| `BUSINESS` | ビジネス |
| `HOBBY` | 趣味 |

## CommunityVisibility（公開設定）

| 値 | 意味 |
|---|---|
| `PUBLIC` | 公開。誰でも閲覧でき、参加は即 ACTIVE |
| `PRIVATE` | 非公開。メンバーのみ閲覧、参加は承認制（PENDING から開始）。非メンバーには存在秘匿 |

## MemberRole（ロール）

| 値 | 意味 |
|---|---|
| `OWNER` | 作成者。コミュニティ作成時に自動生成、脱退不可 |
| `ADMIN` | 承認・拒否・イベント作成が可能 |
| `MEMBER` | 一般メンバー（参加時の既定） |

## MemberStatus（所属状態）

| 値 | 意味 |
|---|---|
| `PENDING` | 承認待ち（PRIVATE への参加申請直後） |
| `ACTIVE` | アクティブな所属 |

## EventFormat（開催形式）

| 値 | 意味 |
|---|---|
| `ONLINE` / `OFFLINE` / `HYBRID` | オンライン / オフライン / ハイブリッド開催 |

## EventStatus（イベント状態）

| 値 | 意味 |
|---|---|
| `DRAFT` | 下書き（作成直後。作成者のみ閲覧可） |
| `PUBLISHED` / `CLOSED` / `CANCELLED` | 公開 / 締切 / 中止（**遷移は未実装**。[イベントのライフサイクル](event/event-lifecycle.md)） |

## ParticipationStatus（参加状態・設計のみ）

DML `meetup-20260712` 由来。コード未実装。[Participation 集約](participation/participation.md)。

| 値 | 意味 |
|---|---|
| `APPLIED` | 申込済（承認待ち。残席がある状態で受付） |
| `WAITLISTED` | キャンセル待ち（満席時に申込順で登録） |
| `APPROVED` | 参加確定（承認 or 繰り上げで到達） |
| `CANCELLED` | 参加キャンセル済 |

## NotificationType（通知種別・設計のみ）

DML `meetup-20260712` 由来。コード未実装。[Notification 集約](notification/notification.md)。

| 値 | 意味 |
|---|---|
| `APPROVAL` | 参加確定通知 |
| `WAITLIST_PROMOTED` | 繰り上げ通知 |
| `PARTICIPANT_CANCELLED` | 主催者向けキャンセル通知 |
| `REMINDER` | 開催日接近リマインダー |
| `SURVEY` | アンケート依頼 |
| `EVENT_CANCELLED` | 中止のお知らせ |

## 権限まわり

権限の全体像は[メンバーシップのロールと権限](community/membership-rules.md)、イベント一覧は[ドメインイベント一覧](domain-events.md)参照。

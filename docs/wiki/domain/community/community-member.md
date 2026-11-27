---
type: domain
title: CommunityMember 集約
description: コミュニティへの所属。role（OWNER/ADMIN/MEMBER）と status（PENDING/ACTIVE）を持つ
tags: [aggregate, community, membership]
sources:
  - backend/src/community/models.rs
  - backend/src/community/usecases.rs
  - backend/src/community/repository.rs
  - backend/src/infrastructure/schema.sql
verified: 5891e84
---

# CommunityMember 集約

アカウントとコミュニティの所属関係。

## フィールド（community/models.rs の `CommunityMember`）

| フィールド | 型 | 制約 |
|---|---|---|
| id | `String` | UUID |
| community_id | `String` | `communities.id` への外部キー |
| account_id | `String` | `accounts.id` への外部キー |
| role | `MemberRole` | `OWNER` \| `ADMIN` \| `MEMBER` |
| status | `MemberStatus` | `PENDING` \| `ACTIVE` |
| created_at | `DateTime<Utc>` | — |

## factory・遷移関数（community/models.rs）

- `join_community(community, account_id, member_id, now) -> CommunityMember` — `role=MEMBER` で生成。コミュニティが PUBLIC なら `status=ACTIVE`（即参加）、PRIVATE なら `status=PENDING`（承認待ち）。`Result` を返さない
- `approve_member(member) -> Result<CommunityMember, ApproveMemberError>` — `PENDING → ACTIVE`。すでに ACTIVE なら `MemberAlreadyActive`
- `leave_community(member) -> Result<(), LeaveCommunityError>` — OWNER は `OwnerCannotLeave`

## 状態遷移

```
（PUBLIC 参加）────────────► ACTIVE
（PRIVATE 参加）─► PENDING ─approve_member─► ACTIVE
```

**注意（非対称）**: 承認（approve）は status 更新だが、**拒否（reject）と脱退（leave）は状態遷移ではなくレコードの物理削除**（usecases.rs の `reject_member_command` / `leave_community_command` が `members.delete()` を呼ぶ）。

## 不変条件・業務ルール

- **重複参加禁止**: 既存 membership があれば `JoinCommunityError::AlreadyMember` — 強制ポイント: usecases.rs `join_community_command` の `find_by_ids` + **DB 複合 unique 制約**（schema.sql の `UNIQUE (community_id, account_id)`。保存時の unique 違反も同エラーにマップ）の二重
- **OWNER は脱退不可** — 強制ポイント: models の `leave_community` 遷移関数
- **越権防止**: approve/reject は対象メンバーが URL の communityId に属するか照合（不一致は `MemberNotFound`）— 強制ポイント: usecases.rs `approve_member_command` / `reject_member_command`
- **メンバー ID 指定の脱退は本人のメンバー ID のみ**（他人の ID は `MemberNotFound`）— 強制ポイント: usecases.rs `leave_community_command`
- 誰が approve/reject できるか（認可）は controller の `require_community_role` で強制 — [メンバーシップのルール](membership-rules.md) 参照
- role / status の値域: **DB CHECK 制約**（schema.sql）。status の既定は `PENDING`

## 現状の実装

- **実装済み**: 参加・承認・拒否・脱退・一覧
- 注意: `reject_member_command` は対象の `status` / `role` を検査せず削除する（PENDING 以外・OWNER も対象にできる実装）。強制ポイントは controller のロール確認のみ

## 関連

- [Community 集約](community.md) / [ドメインイベント](../domain-events.md) / [用語集](../glossary.md)

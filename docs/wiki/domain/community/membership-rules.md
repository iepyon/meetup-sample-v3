---
type: domain
title: メンバーシップのロールと権限
description: 誰が何をできるか — OWNER/ADMIN/MEMBER の権限と PUBLIC/PRIVATE の違い
tags: [business-rule, authorization, membership]
sources:
  - backend/src/community/controller.rs
  - backend/src/community/usecases.rs
  - backend/src/event/controller.rs
  - backend/src/shared/auth.rs
  - docs/decision-logs/arch-20260419-validation-in-controllers.md
verified: 5891e84
---

# メンバーシップのロールと権限

「誰が何をできるか」の一覧。**認可は usecase ではなく controller 層で行う**（[validation-in-controllers](../../../decision-logs/arch-20260419-validation-in-controllers.md)）。認証は `shared/auth.rs` の `require_account`（必須・401）/ `optional_account`（任意）、ロール確認は `community/controller.rs` の `require_community_role(repos, community_id, account_id, MANAGER_ROLES)`（`MANAGER_ROLES` = OWNER / ADMIN。event の controller も同関数を使う）。ロール不足・非メンバー・コミュニティ不在はいずれも 403 `FORBIDDEN`。

## 権限マトリクス

| 操作 | 未認証 | 認証済み非メンバー | MEMBER | ADMIN | OWNER |
|---|---|---|---|---|---|
| コミュニティ作成 | ✗ | ✓（上限 10） | ✓ | ✓ | ✓ |
| コミュニティ一覧 | ✓（PUBLIC のみ） | ✓（PUBLIC のみ。`member=me` で所属分） | ✓ | ✓ | ✓ |
| PUBLIC コミュニティ閲覧 | ✓ | ✓ | ✓ | ✓ | ✓ |
| PRIVATE コミュニティ閲覧 | ✗（NotFound） | ✗（NotFound） | ✓ | ✓ | ✓ |
| 参加申請 | ✗ | ✓（重複禁止） | —（既メンバー） | — | — |
| メンバー承認・拒否 | ✗ | ✗ | ✗ | ✓ | ✓ |
| イベント作成 | ✗ | ✗ | ✗ | ✓ | ✓ |
| 脱退 | ✗ | — | ✓ | ✓ | **✗（OwnerCannotLeave）** |

## PUBLIC / PRIVATE の違い

- **参加時の初期 status**: PUBLIC = `ACTIVE`（即参加）、PRIVATE = `PENDING`（承認待ち）。[CommunityMember](community-member.md) の `join_community` が分岐
- **閲覧**: PRIVATE は非メンバーに対し存在自体を秘匿し `CommunityNotFound` を返す（403 ではない。usecases.rs `get_community_query`）。ACTIVE でない PENDING メンバーが「メンバー一覧」を見ようとしても 404
- **一覧**: `GET /communities` は PUBLIC のみ。PRIVATE が出るのは `?member=me` の本人の所属分だけ
- **発行イベント**: PUBLIC 参加 = `MemberJoined`、PRIVATE 参加 = `MemberApplicationSubmitted`（[ドメインイベント](../domain-events.md)）

## ロールの意味

- `OWNER` — 作成者。コミュニティ作成時に自動生成され、脱退不可
- `ADMIN` — 承認・拒否・イベント作成が可能
- `MEMBER` — 一般メンバー（参加時の既定ロール）

## メンバー一覧の情報露出

`list_members_read_query`（Read モデル）は `accountId` を **ACTIVE メンバーにのみ露出**し、アカウント列挙攻撃を防ぐ（レスポンスに `accountName` は常に含まれる）。PRIVATE のメンバー一覧は ACTIVE メンバー以外には 404。

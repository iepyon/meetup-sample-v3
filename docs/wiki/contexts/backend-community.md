---
type: context
title: community (backend)
description: コミュニティ CRUD とメンバーシップ管理（参加・脱退・承認・拒否）。UseCase は command / query 関数、メンバー一覧は Read モデル
tags: [backend, community, membership]
sources:
  - backend/src/community
  - backend/src/app.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# community (backend)

コミュニティの CRUD とメンバーシップ管理を担う中核 bounded context。層は `controller.rs` → `usecases.rs` → `models.rs` / `repository.rs`、エラーとドメインイベントは `errors.rs`。

## エンドポイント

すべて `community/controller.rs` のハンドラ。ルートは `backend/src/app.rs` の `create_app` で登録する。

| メソッド | パス | 認証 | 内容 |
|---|---|---|---|
| POST | /communities | `require_account` | 作成（検証 400 → 認証 401 の順） |
| GET | /communities | `optional_account` | 一覧（`?member=me` / `?category` / `limit` / `offset`）。`member=me` は認証必須（401）。**それ以外は PUBLIC のみ** |
| GET | /communities/{id} | `optional_account` | 取得（PRIVATE はメンバーのみ。それ以外は 404） |
| POST | /communities/{id}/members | `require_account` | 参加（201） |
| DELETE | /communities/{id}/members/me | `require_account` | 自身の脱退 |
| DELETE | /communities/{id}/members/{member_id} | `require_account` | メンバー ID 指定の脱退（本人のメンバー ID のみ有効） |
| GET | /communities/{id}/members | `optional_account` | メンバー一覧（Read モデル） |
| PATCH | /communities/{id}/members/{member_id}/approve | `require_account` + `require_community_role` | 承認 |
| PATCH | /communities/{id}/members/{member_id}/reject | `require_account` + `require_community_role` | 拒否（204） |

path の `{id}` / `{member_id}` は `uuid_param` で UUID 形式を検証し、違反は 400 `VALIDATION_ERROR`。

## usecases（community/usecases.rs）

UseCase は `Ctx { communities, members, bus }`（リポジトリと Event Bus への参照）を受け取る関数。ドメインロジックは `models.rs` に委譲する。

- **command**: `create_community_command`（名前重複・上限 `MAX_COMMUNITIES_PER_USER = 10` を確認 → `create_community()` → `save_with_owner`（コミュニティとオーナーを 1 トランザクションで保存））、`join_community_command`（PUBLIC は即 ACTIVE、PRIVATE は PENDING）、`leave_community_command`（OWNER は脱退不可）、`approve_member_command`、`reject_member_command`（レコード削除。対象が当該コミュニティに属するか照合）
- **query**: `get_community_query`（可視性判定つき）、`list_communities_query`（`member_account_id` 指定なら所属コミュニティ、未指定なら `visibility = PUBLIC` を repository に渡す）、`list_members_read_query`（`accountId` は ACTIVE メンバーのみ露出しアカウント列挙を防止）

## ドメインイベント（発行のみ、購読者ゼロ）

`AppState.community_events`（`EventBus<CommunityDomainEvent>`）を `Ctx.bus` に渡す。発行: `CommunityCreated` / `MemberJoined` / `MemberApplicationSubmitted` / `MemberLeft` / `MemberApproved` / `MemberRejected`（`CommunityDomainEvent` は `errors.rs` に定義）。詳細は [Event Bus](../patterns/event-bus.md)。

## 特記

- **Composition Root は `app.rs` の `AppState`**（`pool` / `jwt_secret` / 2 つの Event Bus）。controller の `Repos::new(&state)` がハンドラごとにリポジトリを組み立て、`Repos::ctx` で `Ctx` にする
- **認可は controller 層**: `require_community_role(repos, community_id, account_id, MANAGER_ROLES)`（`MANAGER_ROLES` = OWNER / ADMIN）。メンバーでない・ロール不足・コミュニティ不在はいずれも 403 `FORBIDDEN` で、存在を漏らさない
- **メンバー一覧は SQL で名前を JOIN した Read モデル**（`CommunityMemberRepository::list_active_with_names` → `MemberReadRow`）。作成日時の昇順で ACTIVE のみ返す。集約を経由しない点が「UseCase は集約を操作する」という通常形からの意図的な逸脱
- 一覧の `find_all` は `QueryBuilder` で条件を組み立て、値はすべて `push_bind` する
- [event コンテキスト](backend-event.md)がこのコンテキストの `CommunityRepository`・`Repos`・`require_community_role`・`MANAGER_ROLES` を直接 import して再利用している（依存方向: event → community）

## 関連

- [Community 集約](../domain/community/community.md) / [CommunityMember 集約](../domain/community/community-member.md) / [ロールと権限](../domain/community/membership-rules.md)
- frontend 側: [community (frontend)](frontend-community.md)

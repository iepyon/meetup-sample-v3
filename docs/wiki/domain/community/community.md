---
type: domain
title: Community 集約
description: コミュニティ本体。作成時にオーナーメンバーを同時生成、カテゴリと公開設定を持つ
tags: [aggregate, community]
sources:
  - backend/src/community/models.rs
  - backend/src/community/usecases.rs
  - backend/src/community/controller.rs
  - backend/src/community/repository.rs
  - backend/src/infrastructure/schema.sql
verified: 5891e84
---

# Community 集約

Meetup コミュニティ本体。

## フィールド（community/models.rs の `Community`）

| フィールド | 型 | 制約 |
|---|---|---|
| id | `String` | UUID |
| name | `String` | 1〜100 文字（controller の `required_string`） |
| description | `Option<String>` | 最大 1000 文字・nullable（`optional_nullable_string`） |
| category | `CommunityCategory` | `TECH` \| `BUSINESS` \| `HOBBY` |
| visibility | `CommunityVisibility` | `PUBLIC` \| `PRIVATE`（作成後は不変の分類） |
| created_at / updated_at | `DateTime<Utc>` | — |

## factory・遷移

- `create_community(input) -> (Community, CommunityMember)`（community/models.rs）— **Community とオーナー [CommunityMember](community-member.md) を同時生成**して返す（`Result` を返さない）。オーナーは `role=OWNER`, `status=ACTIVE`
- 状態遷移関数はなし（visibility は生成時に決まる分類）

## 不変条件・業務ルール

- **名前の一意性**: 同名コミュニティは `CreateCommunityError::DuplicateCommunityName` — 強制ポイント: usecases.rs `create_community_command` の `find_by_name` + **DB unique 制約**（schema.sql の `name TEXT NOT NULL UNIQUE`。保存時の unique 違反も同エラーにマップ）の二重
- **作成上限**: 1 ユーザー `MAX_COMMUNITIES_PER_USER = 10` まで（自分が OWNER のコミュニティ数）。超過は `TooManyCommunities`（422）— 強制ポイント: usecases.rs のカウントチェックのみ（**DB 制約なし**。同時リクエストでは破れうる）
- **オーナーの同時生成**: Community 作成は必ずオーナーメンバーを伴う — 強制ポイント: models の `create_community` factory（usecase 側では組み立てない）。保存は `CommunityRepository::save_with_owner` が 1 トランザクションで行う
- **PRIVATE の存在秘匿**: 非メンバー・未認証には 403 ではなく `GetCommunityError::CommunityNotFound`（404）を返す — 強制ポイント: usecases.rs `get_community_query`。詳細は [メンバーシップのルール](membership-rules.md)
- **一覧は PUBLIC のみ**: 認証の有無にかかわらず `?member=me` なしの `GET /communities` は PUBLIC だけを返す。`?member=me` は自分が ACTIVE メンバーのコミュニティ（PRIVATE 含む）— 強制ポイント: usecases.rs `list_communities_query` が渡す `visibility` を `CommunityRepository::find_all` が SQL に反映（旧 TS 実装は repository が条件を無視して非公開も返していたバグがあり、Rust 移行で修正された。[rewrite-in-rust](../../../decision-logs/arch-20261127-rewrite-in-rust.md)）
- **category / visibility の値域**: controller の `required_enum` + **DB CHECK 制約**（schema.sql）
- 作成成功で `CommunityCreated` イベント発行（usecases.rs。[ドメインイベント](../domain-events.md)）

## 関連

- [community (backend)](../../contexts/backend-community.md) / [用語集](../glossary.md)

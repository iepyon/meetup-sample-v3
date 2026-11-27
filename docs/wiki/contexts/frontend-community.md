---
type: context
title: community (frontend)
description: コミュニティ・イベント・メンバーの一覧/詳細/作成画面と共有 UI・ルート・proxy
tags: [frontend, community, event, leptos]
sources:
  - frontend/src/pages
  - frontend/src/components/mod.rs
  - frontend/src/main.rs
  - frontend/src/types.rs
  - frontend/src/utils.rs
  - frontend/Trunk.toml
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# community (frontend)

コミュニティの一覧・詳細・作成、イベント作成、メンバー管理の画面群。旧実装の `auth/` `community/` ディレクトリ分離は無く、画面は `frontend/src/pages/` に平置きされている。

## 画面（frontend/src/pages/、ルートは main.rs の `App`）

| ルート | コンポーネント | ファイル |
|---|---|---|
| `/` | `CommunityListPage`（一覧・カテゴリ絞り込み。`CommunityCard` / `load_communities` もここ） | `community_list.rs` |
| `/communities/new` | `CommunityCreatePage` | `community_create.rs` |
| `/communities/:id` | `CommunityDetailPage`（詳細 + メンバー + イベント、参加・脱退・承認・拒否） | `community_detail.rs` |
| `/communities/:id/events/new` | `EventCreatePage` | `event_create.rs` |
| `/my-communities` | `MyCommunitiesPage`（`/communities?member=me`） | `my_communities.rs` |
| `/login` `/register` | `LoginPage` / `RegisterPage` | `auth_pages.rs`（[auth (frontend)](frontend-auth.md)） |

## 共有コード

- `components/mod.rs` — `Card` / `Button`（`Variant::Primary` / `Danger`）/ `Input` / `ErrorAlert` / `CategoryBadge` / `VisibilityBadge` / `Layout`
- `types.rs` — `api_enum!` マクロで定義した `Category` / `Visibility` / `Role` / `MemberStatus` / `EventFormat` / `EventStatus`（`as_str` / `parse` / `ALL`）と `Community` / `CommunityMember` / `CommunityEvent`、Request/Response 型（サーバーの camelCase に対応する手書き）
- `utils.rs` — ラベル変換（`category_label` / `visibility_label` / `format_label`）・`filter_by_category`・`parse_capacity`。ブラウザ API に依存しない純粋関数で、ネイティブの `cargo test -p meetup-frontend` で検証する（単体テストは `utils/tests.rs` と `types/tests.rs` だけ）
- データ取得は各ページが `api::get` / `post` / … を `spawn_local` 内で呼ぶ（フック層は無い）

## dev proxy（frontend/Trunk.toml）

`trunk serve`（既定 `127.0.0.1:5173`。`d dev` は `--address 0.0.0.0`）が `/auth`, `/communities`, `/health` を `http://localhost:3000`（backend）へ転送する。イベント API は `/communities/{id}/events` 配下なので proxy 追加は不要。本番相当は backend が `FRONTEND_DIST` で `dist/` を同一オリジン配信する（[shared](backend-shared.md) の起動処理）。

## 関連

- backend 側: [community (backend)](backend-community.md) / [event (backend)](backend-event.md)
- ドメインルール: [domain/](../domain/) の各ページ（ロール・公開設定の意味）

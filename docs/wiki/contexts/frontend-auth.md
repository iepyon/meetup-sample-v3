---
type: context
title: auth (frontend)
description: ログイン・登録画面と認証状態管理（Auth コンテキスト / token / api クライアント）
tags: [frontend, auth, leptos]
sources:
  - frontend/src/auth.rs
  - frontend/src/token.rs
  - frontend/src/api.rs
  - frontend/src/pages/auth_pages.rs
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# auth (frontend)

ログイン・新規登録の画面と、アプリ全体の認証状態管理を担う。フロントエンドは Leptos 0.8（CSR / WebAssembly）+ Trunk + Tailwind CSS 4 で、`frontend/` は Cargo workspace のメンバー（クレート名 `meetup-frontend`）。

## 構成（frontend/src/）

- `auth.rs` — `Auth { user: RwSignal<Option<Account>>, loading: RwSignal<bool> }`。`provide_auth()` が `provide_context` で提供し、保存済みトークンがあれば `/auth/me` でユーザーを復元する（その間 `loading = true`）。`login` / `register`（`/auth/login`・`/auth/register` を叩きトークン保存 + user セット）/ `logout`（トークン削除）。失敗時はエラー文字列を返し、呼び出し側の画面がエラー表示してその画面に留まる
- `pages/auth_pages.rs` — `LoginPage` / `RegisterPage`（フォーム画面）。`expect_context::<Auth>()` で認証状態を使う
- `types.rs` — `Account` / `AccountResponse` / `AuthResponse` 等の DTO（サーバーの camelCase に対応する手書き型。backend とは独立）
- `token.rs` — `get_token` / `set_token` / `remove_token`。`web_sys` の localStorage、キーは `auth_token`
- `api.rs` — fetch ラッパー（`gloo-net`）。`get` / `post` / `post_empty` / `patch` / `delete`。token があれば `Authorization: Bearer` を付与。失敗は `Result<T, ApiFailure { status, message }>` で返す（message はサーバーの `{ code, message }` の message）。空ボディ（204 など）は `null` として扱う

## ルーティング上の位置（frontend/src/main.rs）

`App` コンポーネントが `auth::provide_auth()` を呼んだ後、`<Router>` の中で `<Layout>` が `<Routes>` を包む。ルートガードは無く、**各ページ・コンポーネント内で `Auth` を見て制御**する方式。

## 関連

- backend 側: [auth (backend)](backend-auth.md)
- テスト: `frontend/src/` に UI の単体テストは無い（純粋関数のみ `utils.rs` に `#[test]`）。画面は Rust 製の E2E（`e2e/`。chromiumoxide でヘッドレス Chromium を操作）でカバーする（`docs/conventions/testing.md`）

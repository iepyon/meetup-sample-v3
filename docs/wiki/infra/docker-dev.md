---
type: infra
title: Docker 開発環境
description: 単一常駐コンテナに exec する docker-dev.sh の仕組みと WITH_CLAUDE
tags: [docker, dev-environment]
sources:
  - scripts/docker-dev.sh
  - docker-compose.yml
  - docker-compose.claude.yml
  - Dockerfile
  - scripts/docker-entrypoint.sh
  - e2e/playwright.config.ts
  - backend/src/shared/middleware.rs
  - backend/src/main.rs
  - frontend/Trunk.toml
  - docs/decision-logs/arch-20260703-revert-to-docker-as-primary.md
  - docs/decision-logs/arch-20260703-remove-apple-container-fallback.md
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Docker 開発環境

ローカル開発は **Docker / Docker Compose 単一経路**。Apple `container` は 2026-07-03 に撤去された（経緯: [revert-to-docker](../../decision-logs/arch-20260703-revert-to-docker-as-primary.md) → [remove-fallback](../../decision-logs/arch-20260703-remove-apple-container-fallback.md)）。コマンド一覧と詳細解説は `docs/docker-usage.md`（早見表 §4.4 / レシピ §4.5）。

## 仕組みの要点

- **単一常駐コンテナ**: service `dev`（image `meetup-dev:local`）が `sleep infinity` で常駐し、`docker-dev.sh` の各サブコマンドは `docker compose exec -u devuser dev` で中に入る（catch-all で任意コマンドも可）
- **同一パス bind mount**: `${MOUNT_ROOT}:${MOUNT_ROOT}` でホストと同一パスを維持し、worktree の絶対パスも解決できる
- **named volume 4 本**（`node_modules_root` / `node_modules_e2e` / `cargo_target` / `cargo_registry`）: Linux の node_modules（E2E 用のみ）と Rust のビルド成果物 `target/`・取得済みクレートを macOS ホストから分離（`target/` は bind mount だと遅いため）。entrypoint（scripts/docker-entrypoint.sh）が `chown devuser` してから `gosu devuser` で実行
- **UID 合わせ**: Dockerfile が `USER_UID=501 / USER_GID=20` で devuser を作成し bind mount の権限問題を回避。ベースは `node:26-bookworm` + Playwright 依存 + Rust toolchain（stable / clippy / rustfmt / `wasm32-unknown-unknown`、`/opt/rustup`・`/opt/cargo`）+ `trunk` 0.22.2 + Claude Code CLI + lefthook。Node は E2E と Claude Code CLI 用で、アプリ本体は Rust
- ports: `BACKEND_PORT:-3000` / `FRONTEND_PORT:-5173`。env に `DATABASE_URL=file:./dev.db`、`JWT_SECRET`（既定値なし。`.env` に 32 バイト以上を設定しないと compose がエラー。backend も 32 バイト未満なら起動時に終了）、`PLAYWRIGHT_BROWSERS_PATH`、`ANTHROPIC_API_KEY`

## WITH_CLAUDE（コンテナ内 Claude Code）

- 既定では `~/.claude` を**マウントしない**（認証情報の露出回避）
- `WITH_CLAUDE=1 d up` で `docker-compose.claude.yml` が重なり `~/.claude` を読み取り専用（`:ro`）でマウント。コンテナからホストの設定や hook を書き換えられないようにするため、main でも rw にはしない
- **Gotcha: override はコンテナ生成時（up）のみ反映**。既存コンテナへの shell/exec には効かない

## 開発コマンドの中身

`d install` = `cargo fetch --locked` + `npm install`（e2e）+ Playwright ブラウザ、`d test` = `cargo test --workspace`、`d lint` = `cargo clippy -D warnings`（native + wasm32）、`d typecheck` = `cargo check`、`d review` = `cargo fmt --check` → lint → test、`d dev` = `cargo run -p meetup-backend`（:3000）+ `trunk serve --address 0.0.0.0`（:5173）、`d e2e` = e2e で `npm test`。

## 実行経路の判別

コンテナ内（`/.dockerenv` あり）ではラッパー不要で直接 `cargo test --workspace`。ホストでは必ず `./scripts/docker-dev.sh` 経由。`scripts/run-quality.sh` がこの判別を自動化している（[CI・品質ゲート](ci-quality-gates.md)）。

## d dev と d e2e の併用（Gotcha）

- `e2e/playwright.config.ts` は backend（`cargo run -p meetup-backend`、3000、`DATABASE_URL=file:./e2e-rust.db`）と frontend（`trunk serve`、5173）を `webServer` で自動起動し、どちらも `reuseExistingServer: true`。`d dev` の backend が起動中だとそれを再利用する
- `RATE_LIMIT_DISABLED=true` は Playwright が自分で backend を起動するときの `env` にしか入らない。再利用時はレート制限が有効なまま走り、公開 GET の上限（`shared/middleware.rs` の `PUBLIC_READ_LIMIT`、60 回/分）に当たって後半のテストが失敗する
- `d e2e` の前に dev サーバを止める（コマンドは `docs/docker-usage.md` §7）

## 関連

- [worktree 運用](worktree.md) / [AI hooks](ai-hooks.md)

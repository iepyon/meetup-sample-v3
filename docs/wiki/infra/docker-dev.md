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
  - e2e/src/server.rs
  - e2e/src/lib.rs
  - backend/src/main.rs
  - frontend/Trunk.toml
  - docs/decision-logs/arch-20260703-revert-to-docker-as-primary.md
  - docs/decision-logs/arch-20260703-remove-apple-container-fallback.md
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
  - docs/decision-logs/arch-20261127-e2e-in-rust.md
verified: 5891e84
---

# Docker 開発環境

ローカル開発は **Docker / Docker Compose 単一経路**。Apple `container` は 2026-07-03 に撤去された（経緯: [revert-to-docker](../../decision-logs/arch-20260703-revert-to-docker-as-primary.md) → [remove-fallback](../../decision-logs/arch-20260703-remove-apple-container-fallback.md)）。コマンド一覧と詳細解説は `docs/docker-usage.md`（早見表 §4.4 / レシピ §4.5）。

## 仕組みの要点

- **単一常駐コンテナ**: service `dev`（image `meetup-dev:local`）が `sleep infinity` で常駐し、`docker-dev.sh` の各サブコマンドは `docker compose exec -u devuser dev` で中に入る（catch-all で任意コマンドも可）
- **同一パス bind mount**: `${MOUNT_ROOT}:${MOUNT_ROOT}` でホストと同一パスを維持し、worktree の絶対パスも解決できる
- **named volume 2 本**（`cargo_target` / `cargo_registry`）: Rust のビルド成果物 `target/`・取得済みクレートを macOS ホストから分離（`target/` は bind mount だと遅いため）。npm を撤去したので node_modules 用の volume は無い。entrypoint（scripts/docker-entrypoint.sh）が `chown devuser` してから `gosu devuser` で実行
- **UID 合わせ**: Dockerfile が `USER_UID=501 / USER_GID=20` で devuser を作成し bind mount の権限問題を回避。ベースは `node:26-bookworm` + Debian の `chromium`（E2E 用。arm64 / amd64 両対応）+ Rust toolchain（stable / clippy / rustfmt / `wasm32-unknown-unknown`、`/opt/rustup`・`/opt/cargo`）+ `trunk` 0.22.2 + Claude Code CLI + lefthook。Node は Claude Code CLI 用のみで、アプリ本体と E2E は Rust
- ports: `BACKEND_PORT:-3000` / `FRONTEND_PORT:-5173`。env に `DATABASE_URL=file:./dev.db`、`JWT_SECRET`（既定値なし。`.env` に 32 バイト以上を設定しないと compose がエラー。backend も 32 バイト未満なら起動時に終了）、`ANTHROPIC_API_KEY`

## WITH_CLAUDE（コンテナ内 Claude Code）

- 既定では `~/.claude` を**マウントしない**（認証情報の露出回避）
- `WITH_CLAUDE=1 d up` で `docker-compose.claude.yml` が重なり `~/.claude` を読み取り専用（`:ro`）でマウント。コンテナからホストの設定や hook を書き換えられないようにするため、main でも rw にはしない
- **Gotcha: override はコンテナ生成時（up）のみ反映**。既存コンテナへの shell/exec には効かない

## 開発コマンドの中身

`d install` = `cargo fetch --locked` + `lefthook install`、`d test` = `cargo test --workspace`、`d lint` = `cargo clippy -D warnings`（native + wasm32）、`d typecheck` = `cargo check`、`d review` = `cargo fmt --check` → lint → test、`d dev` = `cargo run -p meetup-backend`（:3000）+ `trunk serve --address 0.0.0.0`（:5173）、`d e2e` = `frontend/` で `trunk build --dist ../target/e2e-dist` → `cargo test -p meetup-e2e -- --ignored`（追加引数は cargo test の `--` より前に渡る。例: `--test auth`）。

## 実行経路の判別

コンテナ内（`/.dockerenv` あり）ではラッパー不要で直接 `cargo test --workspace`。ホストでは必ず `./scripts/docker-dev.sh` 経由。`scripts/run-quality.sh` がこの判別を自動化している（[CI・品質ゲート](ci-quality-gates.md)）。

## d dev と d e2e の併用

- 同時に実行してよい。`e2e/src/server.rs` が `create_app_with_frontend` をインメモリ SQLite・`127.0.0.1:0`（空きポート）・レート制限無効でテストごとに同一プロセス起動し、`target/e2e-dist`（`E2E_FRONTEND_DIST` で上書き可）のビルド済み SPA を同一オリジンで配信する。3000 / 5173・`dev.db`・`frontend/dist` を共有しない
- レート制限はハーネスがアプリへ直接無効化を渡すので、`RATE_LIMIT_DISABLED` 環境変数は使わない。旧 Playwright 構成の「`d dev` の backend を再利用して 429 で落ちる」問題は構造上起きなくなった
- ブラウザは `chromium` / `google-chrome` などを自動検出し、テストごとに専用プロファイルで起動する。`CHROME=<実行ファイル>` で差し替えられる（詳細は `e2e/CLAUDE.md`）

## 関連

- [worktree 運用](worktree.md) / [AI hooks](ai-hooks.md)

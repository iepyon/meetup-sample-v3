---
type: infra
title: worktree 運用
description: worktree ごとに独立 Compose プロジェクト・独立ポートで main と同時起動する仕組み
tags: [docker, worktree, parallel-dev]
sources:
  - scripts/docker-worktree.sh
verified: 5891e84
---

# worktree 運用

`scripts/docker-worktree.sh <name> [command]` が各 git worktree を**独立した Compose プロジェクト**にし、main と同時起動できるようにする。使い方の手順は `docs/docker-usage.md` 6 章。

## 仕組み

- **プロジェクト分離**: `docker compose -p meetup-${SLUG}` でコンテナ・ネットワーク・volume が prefix 分離される。image `meetup-dev:local` は共有（N worktree でもビルド 1 回）
- **ポート割り当て**: PR 番号を `env PR` → `gh pr view` 自動取得 → ブランチ名の cksum ハッシュ、の順で決定し `SLOT = PR % 2000`、`BACKEND_PORT = 4000 + SLOT` / `FRONTEND_PORT = 6000 + SLOT`。env で上書き可。ハッシュ fallback は衝突可能性あり（warn 出力）
- **volume 分離**: `WORK_DIR` を worktree ディレクトリにすることで named volume（cargo の `target/`）のマウント先が worktree 配下になり、`-p` prefix と合わせて worktree 専用 volume が生成される。`cargo_registry`（`/opt/cargo/registry`）もプロジェクトごとの volume になる
- `up` 時に `lsof -iTCP` でポート空き確認。exec 系は起動前提を確認し未起動なら明示エラー
- `clean` は `down -v`（volume 削除）。**worktree 削除前に必須**
- `e2e` は `d e2e` と同じ処理（`trunk build --dist ../target/e2e-dist` → `cargo test -p meetup-e2e -- --ignored`）。E2E はテストごとに空きポートでサーバを立てるので、main や他の worktree と同時に実行できる

## Gotcha

- dev サーバはメインの 5173 ではなく**別ポート**で配信される（worktree のフロント変更を 5173 で確認しても反映されない）
- `WITH_CLAUDE=1` でのマウントは main・worktree とも `:ro`（[Docker 開発環境](docker-dev.md)）
- lefthook は worktree ごとに `lefthook install` が必要（`./scripts/docker-worktree.sh <name> install` が `cargo fetch --locked` に続けて実行する）

## 関連

- [Docker 開発環境](docker-dev.md) / [CI・品質ゲート](ci-quality-gates.md)

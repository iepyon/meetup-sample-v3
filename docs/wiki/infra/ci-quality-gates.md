---
type: infra
title: CI・品質ゲート
description: 3 層の品質ゲート（lefthook pre-commit / GitHub Actions / d review の中身）。cargo fmt / clippy / test が中核
tags: [ci, quality-gate, lefthook, cargo]
sources:
  - .github/workflows/ci.yml
  - lefthook.yml
  - scripts/run-quality.sh
  - scripts/docker-dev.sh
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# CI・品質ゲート

品質ゲートは 3 層: ① git pre-commit（lefthook）② CI（GitHub Actions）③ AI hooks（[別ページ](ai-hooks.md)）。`--no-verify` でローカルを迂回しても CI が最後の砦。
③ は ①② と**重複しない AI 固有のリスク**（出力のシークレット混入）に限定し、lint / 型 / テストは ①② に一元化する。

## lefthook pre-commit（lefthook.yml, parallel: false）

`**/*.rs` の変更に対して順に実行する: ① `cargo fmt --all -- --check` ② `cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings` ③ `cargo test --workspace`。すべて `scripts/run-quality.sh` 経由で、コンテナ内（`/.dockerenv`）/ worktree（docker-worktree.sh 委譲）/ メイン（docker-dev.sh 委譲）の 3 経路を自動判別する。prettier / eslint / tsc は撤去された（npm を撤去し TS のコードが無くなったため。`e2e/` も Rust で、上記の fmt / clippy の対象に入る）。

## GitHub Actions（.github/workflows/ci.yml）

- トリガー: push(main, `paths-ignore: **.md, docs/**`) と PR。`concurrency` で古い実行を自動キャンセル
- **quality ジョブ**（20 分, `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache`）: `cargo fmt --all -- --check` → `cargo clippy --workspace --all-targets -- -D warnings`（backend + native）→ `cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings`（frontend）→ `cargo test --workspace --locked`（backend 単体・HTTP 統合 + frontend 純粋関数）
- **e2e ジョブ**（PR のみ = private 無料枠節約, 30 分）: Rust（wasm32 ターゲット）+ `trunk` 0.22.2 インストール → `frontend/` で `trunk build --dist ../target/e2e-dist` → `cargo test -p meetup-e2e --locked -- --ignored`（`CHROME=/usr/bin/google-chrome` でランナーにプリインストールの Chrome を使う）。backend はテストごとに同一プロセスで起動するため、サーバの事前起動は不要。Node / npm は使わない。E2E は ignore 指定付き（`meetup-test-runner` の `.ignore()`）なので quality ジョブの `cargo test --workspace` では走らない

## `d review` の中身（scripts/docker-dev.sh の `review`）

`cargo fmt --all -- --check` → `d lint`（clippy `-D warnings`、native + wasm32）→ `d test`（`cargo test --workspace`）を順に実行する。警告はエラー扱い。

## 旧ゲートからの変更（未移植）

Rust 移行（[rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)）で次は**未移植**:

- 層依存・BC 間依存・循環依存の自動検査（旧 dependency-cruiser / madge）。現在はコードレビューと規約（controller → usecases → models / repository、event → community の一方向）で守る
- 複雑度・行数の自動検査（旧 ESLint ルール）。clippy の既定 lint のみ
- カバレッジ 80% ゲート（旧 vitest coverage）。`cargo llvm-cov` 等は未導入
- Leptos コンポーネントの単体テストは無い（UI は E2E 依存）

## 関連

- [AI hooks](ai-hooks.md) / [Docker 開発環境](docker-dev.md)

---
type: infra
title: AI hooks
description: Claude Code 用 hook は secret-leak-detector の 1 本のみ。発火タイミングと、lefthook / CI との役割分担
tags: [ai-harness, hooks, security]
sources:
  - .claude/hooks
  - .claude/settings.json
  - docs/decision-logs/arch-20260915-claude-code-only-harness.md
  - docs/decision-logs/arch-20261127-e2e-in-rust.md
verified: 52810be
---

# AI hooks

AI ハーネス用 hook は 1 本（`.claude/hooks/secret-leak-detector.sh`）。Claude Code 専用。

## 発火タイミング一覧

| hook | 発火タイミング | 動作 |
|------|----------------|------|
| `secret-leak-detector.sh` | PostToolUse: Bash | 出力に機密情報（JWT / AWS key / `sk-*` / `password=`）が含まれていれば block |

## hook に載せないもの（重複排除の方針）

lint（clippy）・整形チェック（`cargo fmt --check`）・型チェック・テストは **lefthook pre-commit と CI が担当**し、hook には載せない。

かつて `lint-guard.sh`（PostToolUse: Edit\|Write）と `tsc-check.sh`（Stop）が存在したが、lefthook / CI / `d review` と重複していたため撤去した。同時に GitHub Copilot CLI との併用対応（`.github/hooks/hooks.json` と、ハーネス差を吸収するシム `harness-lib.sh`）も撤去し、Claude Code 専用に一本化した（→ [decision-log](../../decision-logs/arch-20260915-claude-code-only-harness.md)）。

E2E の Rust 移行で npm を撤去した際（→ [e2e-in-rust](../../decision-logs/arch-20261127-e2e-in-rust.md)）、次の 2 本も撤去した。どちらも npm / TypeScript があって初めて守る対象があった hook である。

- `dep-install-guard.sh`（+ `dep-install-guard.test.sh`。PreToolUse: Bash）— `--save-dev` なしの `npm install <pkg>` を block する slopsquatting 対策だった。npm を使わなくなり、守る対象が無い
- `prettier-format.sh`（PostToolUse: Edit\|Write）— `e2e/` の TS と JSON / CSS を自動整形していた。対象ファイルが無くなり、Rust の整形は `cargo fmt` を lefthook / CI でチェックしている

hook に残すのは、他の層で代替できない **AI 固有のリスク**（コンテキストへのシークレット混入）だけ。hook はシムを介さず stdin の JSON を直接 `jq` で読む（空 stdin は「動いているつもり」バグを避けるため stderr に警告を出す）。

## hook の要点

- **secret-leak-detector.sh**（PostToolUse: Bash）— 出力に JWT（eyJ…）/ AWS key（AKIA…）/ `sk-`・`sk_live_` / `password=` 等を検出したら block し `/clear` を促す

## 登録場所

- `.claude/settings.json` — PostToolUse×1 の計 1 本（PreToolUse / Stop hook は無い）

## 関連

- [CI・品質ゲート](ci-quality-gates.md) — hook を迂回しても CI が最後の砦

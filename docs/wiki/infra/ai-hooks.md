---
type: infra
title: AI hooks
description: Claude Code 用 3 本の hook の発火タイミングと、lefthook / CI との役割分担
tags: [ai-harness, hooks, security]
sources:
  - .claude/hooks
  - .claude/settings.json
verified: 5891e84
---

# AI hooks

AI ハーネス用 hook は 3 本（`.claude/hooks/`）。Claude Code 専用。

## 発火タイミング一覧

| hook | 発火タイミング | 動作 |
|------|----------------|------|
| `dep-install-guard.sh` | PreToolUse: Bash | `--save-dev` なしの `npm install` を block（slopsquatting 対策） |
| `secret-leak-detector.sh` | PostToolUse: Bash | 出力に機密情報（JWT / AWS key / `sk-*` / `password=`）が含まれていれば block |
| `prettier-format.sh` | PostToolUse: Edit\|Write | 編集した `.ts/.tsx/.js/.jsx/.json/.css` を自動整形（block しない。`.rs` は対象外） |

## hook に載せないもの（重複排除の方針）

lint（clippy）・整形チェック（`cargo fmt --check`）・型チェック・テストは **lefthook pre-commit と CI が担当**し、hook には載せない。

かつて `lint-guard.sh`（PostToolUse: Edit\|Write）と `tsc-check.sh`（Stop）が存在したが、lefthook / CI / `d review` と重複していたため撤去した。同時に GitHub Copilot CLI との併用対応（`.github/hooks/hooks.json` と、ハーネス差を吸収するシム `harness-lib.sh`）も撤去し、Claude Code 専用に一本化した（→ [decision-log](../../decision-logs/arch-20260915-claude-code-only-harness.md)）。

Rust 移行（[rewrite-in-rust](../../decision-logs/arch-20261127-rewrite-in-rust.md)）後も hook は 3 本のまま。hook に残すのは、他の層で代替できない **AI 固有のリスク**（幻覚パッケージの依存追加・コンテキストへのシークレット混入）と、block しない整形の自動適用だけ。各 hook はシムを介さず stdin の JSON を直接 `jq` で読む（空 stdin は「動いているつもり」バグを避けるため stderr に警告を出す）。

## 各 hook の要点

- **dep-install-guard.sh**（PreToolUse: Bash）— `--save-dev` / `-D` なしの `npm install <pkg>` を block（slopsquatting 対策）。`;` `&` `|` でのセグメント分割・行末コメント除去・リダイレクト演算子検出で誤判定を回避。bare `npm install`（引数なし）は許可。`dep-install-guard.test.sh` は本体でなくテスト
- **secret-leak-detector.sh**（PostToolUse: Bash）— 出力に JWT（eyJ…）/ AWS key（AKIA…）/ `sk-`・`sk_live_` / `password=` 等を検出したら block し `/clear` を促す
- **prettier-format.sh**（PostToolUse: Edit|Write）— 編集ファイル（.ts/.tsx/.js/.jsx/.json/.css）を `prettier --write`。block しない。Rust 移行後、アプリ本体の `.rs` は対象拡張子に無く整形されない（Rust の整形は `cargo fmt` を lefthook / CI でチェック）。実質的な対象は `e2e/` の TS と JSON / CSS

## 登録場所

- `.claude/settings.json` — PreToolUse / PostToolUse を配列登録（Stop hook は無い）。登録は PreToolUse×1 / PostToolUse×2 の計 3 本

## 関連

- [CI・品質ゲート](ci-quality-gates.md) — hook を迂回しても CI が最後の砦

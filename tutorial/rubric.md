# AI 駆動開発ワークショップ — ルーブリック（3日間版）

## 概要

Claude Code を用いた AI 駆動開発の習熟度を **5 能力軸 × 4 段階** で評価する。

- 根拠: [Best Practices](https://code.claude.com/docs/en/best-practices) / [Common Workflows](https://code.claude.com/docs/en/common-workflows)
- 対象: プログラミング経験はあるが AI 駆動開発は初めて〜試し始めた方
- 目標: **全軸 L2〜L3**。L4 はワークショップ後の自律実践で到達。

---

## 3 日間ゴールと軸の対応

| 日 | 学習目標（画像より） | 主な軸 |
|----|---------------------|-------|
| **Day 1** | 既存構造の理解（オンボーディング）/ Plan モードで依頼 / 作成物のレビュー | 1・2・3 |
| **Day 2** | AI と密に対話しながら作り込む / ハーネス構築の入口 | 2・5 |
| **Day 3** | ハーネスを組み立て AI が働きやすい環境を作る / 今後の働き方 / 実施計画 | 4・5・3 |

> 注: 画像の重複付箋は文脈から振り分けて補完（推測）。旧・観点6（伴奏と委託）は5へ、旧・観点7（レビュー）は3へ合流済み。

---

## 評価軸

### 1. モードの理解と使い分け（Day 1）

> 根拠: [Plan Mode](https://code.claude.com/docs/en/common-workflows#use-plan-mode-for-safe-code-analysis) / [Auto Mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode)

| L1 | モードの違いを意識しない |
|----|----|
| **L2** | Plan / Normal の違いを説明し、調査→実装を実践 |
| **L3** | タスクに応じて意図的に切替（小修正は直接、複数ファイルは Plan→Implement） |
| **L4** | Auto Mode（分類器による安全チェック）・Worktree で自律実行を設計。`claude -p` を CI に組込 |

---

### 2. 意図の伝達（Day 1・Day 2）

> 根拠: [Provide specific context](https://code.claude.com/docs/en/best-practices#provide-specific-context-in-your-prompts) / [Let Claude interview you](https://code.claude.com/docs/en/best-practices#let-claude-interview-you)

| L1 | 曖昧な指示。期待結果を説明できない |
|----|----|
| **L2** | What を伝え、`@ファイル` や画像でコンテキストを与える |
| **L3** | What+Why+受入基準+既存パターンを伝える。ずれた原因を切り分け次に反映 |
| **L4** | AI にインタビューさせ仕様を SPEC.md 化→新セッションで実装。効いた構造を言語化 |

---

### 3. コードリーディング・軌道修正・レビュー（Day 1・Day 3）

> 根拠: [Give Claude a way to verify](https://code.claude.com/docs/en/best-practices#give-claude-a-way-to-verify-its-work) / [Course-correct early](https://code.claude.com/docs/en/best-practices#course-correct-early-and-often)

| L1 | 出力も計画もレビューせず承認 |
|----|----|
| **L2** | diff とテストで確認。明らかな問題を指摘。Plan を Ctrl+G で編集してから実行 |
| **L3** | 規約準拠を確認。Esc で即停止し具体指示、2 回直らなければ `/clear`。計画の抜けも指摘 |
| **L4** | 検証手段込みで依頼。`/rewind` で別案。レビューを subagent に委譲し CLAUDE.md/hooks に還元 |

---

### 4. AI 出力の安定化（Day 3）

> 根拠: [CLAUDE.md](https://code.claude.com/docs/en/best-practices#write-an-effective-claudemd) / [hooks](https://code.claude.com/docs/en/best-practices#set-up-hooks) / [skills](https://code.claude.com/docs/en/best-practices#create-skills)

| L1 | 設定ファイルを知らず毎回手動で伝える |
|----|----|
| **L2** | CLAUDE.md の役割を理解。`/init` で生成し既存ルールを説明できる |
| **L3** | 問題パターンからルールを追加・修正（不要なら削除）。hooks で lint/format 自動化 |
| **L4** | CLAUDE.md を階層設計。skills で肥大化を防ぎ subagent でレビュー・調査を自動化 |

---

### 5. 学習の深化と、伴奏・委託の使い分け（Day 2・Day 3）

> 根拠: [Ask codebase questions](https://code.claude.com/docs/en/best-practices#ask-codebase-questions) / [Use subagents for investigation](https://code.claude.com/docs/en/best-practices#use-subagents-for-investigation)

| L1 | AI を「答えを出す道具」としてのみ使い、意味を問わない |
|----|----|
| **L2** | 「この機能は何を解決?」「処理をトレースして」と質問。伴奏と委託の違いを理解 |
| **L3** | なぜ/トレードオフを掘り下げる。不慣れは伴奏・定型は委託。タスク間で `/clear` |
| **L4** | AI の説明を実コード・公式ドキュメントと照合。Writer/Reviewer や並列作業を設計 |

---

## 総合到達レベル

| レベル | 目安 | 対応 |
|--------|------|------|
| Beginner | 全軸 L1-L2 | Day 1 終了時 |
| **Intermediate** | 全軸 L2 以上・過半数 L3 | **Day 3 終了時の目標** |
| Advanced | 全軸 L3 以上・複数軸 L4 | ワークショップ後 |
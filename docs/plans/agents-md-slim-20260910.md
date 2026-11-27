# AGENTS.md 削減計画（2026-09-10）

## 背景

`/claude-api prompt-audit` によるプロンプト監査の後続タスクとして、常時ロードされる指示面の削減を検討した。本計画は調査結果と実施手順をまとめたもの。

**前提として「短くすること自体」は目的にしない。** 監査ガイドの基準どおり、害は「量」ではなく「二重管理によるドリフト」と「参照データが行動ルールを希釈すること」から来る。文字数を目標にすると、いちばん価値の高い文脈（Architecture の地図・Gotchas・制約の理由）から先に消える。

## 現状把握（調査結果）

### 常時ロードされる面のサイズ

| ファイル | 行数 | 文字数 | 読み込まれ方 |
|---|---|---|---|
| `CLAUDE.md` | 14 | 641 | 常時 |
| `CLAUDE.local.md` | 22 | 866 | 常時（gitignore 対象） |
| `AGENTS.md` | 247 | 11,119 | 常時 |
| `docs/conventions/coding.md` | 23 | 986 | 常時（`@import` で展開） |
| `docs/conventions/testing.md` | 25 | 1,245 | 常時（`@import` で展開） |
| **合計** | **331** | **14,857** | 毎リクエスト ≒ 4〜5k トークン |

### AGENTS.md のセクション別内訳

| セクション | 行数 | 性質 | 判定 |
|---|---|---|---|
| Commands | 102 (41%) | 参照データ | 圧縮 |
| Quality Gates | 30 | 参照データ + 行動ルール | 圧縮 |
| Architecture | 29 | 地図（文脈） | **維持** |
| Subagent Guidelines | 20 | 自動注入済みの複製 | 削除 |
| Core Patterns | 10 | 地図（文脈） | **維持** |
| AI Harness Compatibility | 10 | 参照データ | 圧縮 |
| Security / Gotchas / General Rules | 25 | 行動ルール | **維持** |
| Git / Coding / Testing / Context Mgmt | 17 | 行動ルール + import | **維持** |

### 削る根拠（3 点）

**① Subagent Guidelines（20 行）は検証可能に冗長。** 4 サブエージェントの `description` はハーネスが毎リクエスト自動注入している。`AGENTS.md` の同節はその要約で内容も一致しており、削っても AI 側の情報は減らない。加えて Copilot CLI にサブエージェント機構が無いため、共用ファイルに置く理由もない（Claude Code 固有 = `CLAUDE.md` の担当）。

**② Commands（102 行）は `docs/docker-usage.md` と二重管理で、すでにドリフトしている。** `docker-worktree.sh` の言及は `AGENTS.md` 8 箇所 / `docker-usage.md` 15 箇所。`AGENTS.md` 自身が 7 行目で「詳細は `docs/docker-usage.md`」と書いており、要約が本体より薄いまま並走している。本リポジトリでは同種のドリフト事故を既に 2 件検出済み（`render.py`/`serve.py`、質問数の矛盾）。

**③ Commands は参照データ。** モデルが毎ターン必要なのはルール（実行経路・完了前の `d review`）と表の在り処であって、レシピ全文ではない。

## 実施前に解消が必要な障害（調査で判明）

### 障害 1: 移動先が受け皿になっていない

`docs/docker-usage.md` §4.4 早見表に、品質系の第一級サブコマンドが**丸ごと欠落**している。

| `AGENTS.md` にある | §4.4 |
|---|---|
| `d lint` / `d typecheck` / `d review` | ❌ 表に無い |
| `d bash -c` 系レシピ（`lint:fix` / `test:coverage` / `format` / `format:check` / `db:migrate` / 単一テスト実行） | ❌ 無い |
| `docker compose down -v` | ❌ 無い |

`d review` は「AI が実装完了前に必ず実行すること」なので、**受け皿を作る前に削ると品質ゲートの入口が消える**。

### 障害 2: 4 ファイルが AGENTS.md を「コマンドの出典」として逆参照

```
README.md:34                     「AGENTS.md の Commands 節を参照」
docs/wiki/infra/docker-dev.md:19 「コマンド一覧は AGENTS.md」
docs/wiki/infra/worktree.md:13   「使い方の手順は AGENTS.md と docs/docker-usage.md 6 章」
docs/wiki/infra/ai-hooks.md:15   「発火タイミングの一覧表は AGENTS.md の Quality Gates 参照」← 循環参照
```

`ai-hooks.md` は AGENTS.md を指し、AGENTS.md はどこも指していない。hook 表は**削除ではなく `ai-hooks.md` への移設**が正解。

### 障害 3: 教材が「AGENTS.md に実行コマンドを書け」と教えている

`tutorial/` が 1,348 行あり、AGENTS.md を「AI への指示ファイルの実例」として教材化している。

```
tutorial/1day-tutorial.md:154  CLAUDE.md,AGENTS.md → …命名規約・実行コマンド・品質基準などを宣言
tutorial/1day-tutorial.md:406  [CLAUDE.md / AGENTS.md に書く] アーキテクチャ・品質ゲートコマンド・テスト実行の明示
tutorial/1day-tutorial.md:606  [CLAUDE.md/AGENTS.mdに書く] フロントの実行コマンド・テスト実行の明示
tutorial/1day-tutorial.md:789  rm CLAUDE.md AGENTS.md   ← 受講者が消して「無い状態」を体験する演習
```

**Commands を全部外に出すと教材の主張と実物が食い違う。** よって目標を「Commands 102 → 15 行」ではなく **「102 → 30 行」**（実行経路のルール + 最頻出コマンド表を残し、レシピ・worktree・Docker 内実行の 3 ブロックを移設）とする。教材の主張を満たしたまま重複の本体は消え、最終行数もほぼ変わらない。

## 目標

| | 現在 | 目標 |
|---|---|---|
| `AGENTS.md` | 247 行 | 約 130 行（-47%） |
| 常時ロード合計 | 331 行 | 約 215 行（-35%） |

参照方式は **通常リンク（オンデマンド読み込み）**。`@import` は増やさない — Copilot CLI が `@` を展開するかが未検証で、増やすと Copilot 側で内容が静かに失われるリスクがある。

## 実施計画（PR 2 本）

### PR 1: 受け皿を作る（追記のみ・`AGENTS.md` は触らない）

情報の追加と移設だけなので、単体でマージしても既存の参照は一切壊れない。

**Phase 1 — `docs/docker-usage.md` に不足を補う**

1. §4.4 の `docker-dev.sh` 表に `lint` / `typecheck` / `review` を追加（`review --strict` の挙動も明記）
2. §4.4 に「よく使うレシピ」小節を新設し、`d bash -c` 系 6 行 + 単一テスト実行 2 行 + `docker compose down -v` / `docker volume prune` を収容
3. `AGENTS.md` §「Docker 内で直接実行する場合」（82-105 行）相当を §3.1 として新設
4. `AGENTS.md` の `WITH_CLAUDE=1` ブロック（74-79 行）相当を §3.2 として新設（当初 §5 を想定していたが、§5 は「なぜこうなっているか」の解説節なので手順は §3 側に置く）

**Phase 2 — hook 表を wiki に移し循環参照を解消**

5. `AGENTS.md` の hook 発火タイミング表（179-185 行）を `docs/wiki/infra/ai-hooks.md` に転記
6. `ai-hooks.md:15` の「一覧表は AGENTS.md 参照」を削除し自己完結させる

### PR 2: `AGENTS.md` を削る + 参照を整合

**Phase 3 — 節の削除**

7. Subagent Guidelines（223-241 行）を削除。`CLAUDE.md` 側の 3 行が正しい置き場

**Phase 4 — 節の圧縮**

8. Commands 102 → 約 30 行（実行経路のルール + 最頻出 10 コマンド表 + `docker-usage.md` へのリンク）
9. Quality Gates 30 → 約 10 行（`d review` の義務・pre-commit の存在・hook 5 本の事実 + 2 リンク）
10. AI Harness Compatibility 10 → 4 行
11. Architecture / Core Patterns / Security / Gotchas / General Rules / conventions import は**変更しない**

**Phase 5 — 参照の整合（削除の完了条件）**

12. `README.md:34` の導線を `docs/docker-usage.md` 一本に
13. `docs/wiki/infra/docker-dev.md:19` の「コマンド一覧は AGENTS.md」を `docker-usage.md` へ
14. `docs/wiki/infra/worktree.md:13` を同様に修正
15. `docs/docker-usage.md:244` の AGENTS.md 説明（「プロジェクト共通のコマンドリファレンス」）を実態に合わせる
16. `tutorial/` は**変更しない**（Phase 4 で最頻出コマンド表を残すため主張と矛盾しない）

**Phase 6 — wiki 運用ルールの遵守（`docs/wiki/SCHEMA.md` 準拠）**

17. 変更した wiki ページの `verified` を HEAD SHA に更新、`index.md` の description 転記を更新、`log.md` に ingest エントリを追記

## 検証手順

各 PR 完了ごとに実施する。

1. **リンク切れ・孤立チェック** — `/wiki lint` + `grep -rn 'AGENTS\.md'` で逆参照 24 ファイルを再走査し、ダングリング参照が無いことを確認
2. **コマンド網羅性の確認** — 移設前の `AGENTS.md` に登場する全コマンド文字列が `docs/docker-usage.md` に存在することを機械照合
3. **オンデマンド読み込みの実効性 probe（最重要）** — リンク方式を選んだため、モデルが必要なときに実際に読むかを確認する。新規セッションで以下を投げ、`docs/docker-usage.md` を読みに行くか / 推測でコマンドを捏造するかを観察する:
   - 「backend のカバレッジを測って」（→ `d bash -c "cd backend && npm run test:coverage"` に到達できるか）
   - 「worktree を独立ポートで起動して」（→ `docker-worktree.sh <name> up` に到達できるか）
   - 「実装が終わったので完了前チェックをして」（→ `d review` に到達できるか）
   
   捏造した場合は、該当コマンドだけ `AGENTS.md` 本文に戻す。**削除は仮説であって結論ではない。**

### 検証結果（2026-09-10 実施）

フレッシュなコンテキストのエージェントで probe を実施。**3 問すべてドキュメントのみで到達し、コマンドの捏造は 0 件。**

| 質問 | 結果 | 到達経路 | ホップ |
|---|---|---|---|
| backend のカバレッジ計測 | 到達 | `AGENTS.md`（catch-all の案内）→ `docker-usage.md` §4.5 に逐語 | 2 |
| worktree の独立ポート起動 | 到達 | `AGENTS.md` → `docker-usage.md` §6 に逐語 + §4.4 のポート採番仕様 | 2 |
| 実装完了前の総合チェック | 到達 | `AGENTS.md` Quality Gates 節の冒頭で確定（`d review`） | 1 |

**結論: リンク方式（オンデマンド読み込み）は機能する。`AGENTS.md` に戻すべきコマンドはない。**

probe が 1 件のギャップを報告した — 「コンテナ内でカバレッジを測る形」は §3.1 のコンテナ内コマンド一覧に列挙がなく、`d bash -c` の内側を規則から導出したものだと明示していた（推測と逐語記載を正直に区別する望ましい振る舞い）。導出可能なので失敗ではないが、§3.1 に 1 行追記して導出ステップを削除した。

## ロールバック

PR 単位で revert 可能。PR 1 は追記のみなので revert 不要（残しても害がない）。PR 2 を revert すれば `AGENTS.md` は元に戻り、PR 1 で `docker-usage.md` に増えた内容だけが残る（重複が復活するだけで機能影響はない）。

## 対象外（今回やらない）

- `CLAUDE.md`（14 行）— 既に最小限
- `CLAUDE.local.md`（22 行）— superpowers 上書きと `snapshot/*` 禁止はいずれも load-bearing
- `docs/conventions/*.md` の `@import` — Copilot CLI の `@` 対応が未検証のため現状維持
- `eventstorming-facilitator/SKILL.md`（225 行）— `SCHEMA.md` の「1 ページ 150 行以内」基準を超えるが、常時ロードされないため優先度が低い。別タスクで分割を検討

## 関連

- `/claude-api prompt-audit` の監査結果 → PR #38（`claude/prompt-audit-fixes`）
- [`docs/wiki/SCHEMA.md`](../wiki/SCHEMA.md) — wiki 更新時の運用ルール
- [`docs/docker-usage.md`](../docker-usage.md) — 移設先の本体

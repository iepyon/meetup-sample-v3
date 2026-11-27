# AI ハーネスを Claude Code 専用に絞り、重複する hook と deny リストを撤去する

日付: 2026-09-15
種類: arch
ステータス: 承認済み

## コンテキスト

AI ハーネス（`.claude/` + `.github/hooks/`）に 3 種類の冗長性が蓄積していた。

- **多ハーネス対応の二重管理**: Claude Code と GitHub Copilot CLI の両方で動かすため、`harness-lib.sh`（72 行のシム。入力 JSON の `.hookEventName` 有無で `HARNESS_KIND` を判定し、block 出力を `{"decision":"block"}` / `{"permissionDecision":"deny"}` に切り替える）と `.github/hooks/hooks.json` を並行維持していた。しかし実運用は Claude Code 単独で、Copilot CLI 側の等価性は検証されていない
- **品質ゲートの三重実行**: `tsc-check.sh`（Stop 時の全体 `tsc --noEmit`、timeout 120 秒）は lefthook の typecheck・CI の quality ジョブ・`d review` と完全に重複する。`lint-guard.sh` も lefthook の workspace 全体 lint・CI・`d review` と粒度違いで重複し、AI セッション中に同一チェックが 2〜3 回走っていた
- **実効性のない防御層**: `.claude/settings.json` の `permissions.deny`（機密ファイルに対する cat/head/tail/grep/sed/awk/sort/jq の列挙、約 57 行）は `node -e` 等で容易に回避でき、AGENTS.md 自身が「一層にすぎない」と認めていた

加えて `.claude/agents/codebase-explorer.md` は、ハーネス組み込みの探索サブエージェント（Explore / general-purpose）と役割が重複していた。

## 決定

ハーネスに残すのは **lefthook / CI / `d review` では代替できない AI 固有のリスクだけ**とし、品質ゲート本体は AI 非依存の lefthook と CI に一元化する。

1. **Copilot CLI 対応を撤去**し Claude Code に一本化する。`.github/hooks/hooks.json` と `harness-lib.sh` を削除し、残る 3 hook はハーネス差の分岐を持たない形にインライン化する
2. **lefthook / CI と重複する `lint-guard.sh`・`tsc-check.sh` を撤去**する（hook 5 本 → 3 本: dep-install-guard / secret-leak-detector / prettier-format）
3. **`settings.json` の `permissions.deny` を撤去**する
4. **`codebase-explorer` サブエージェントを撤去**する（4 体 → 3 体）。固有価値だった「既存 wiki ページは `verified` SHA 以降の差分だけ裏取りする」という指示は `.claude/skills/wiki/SKILL.md` 側へ移す

`dep-install-guard.test.sh`（hook 唯一の回帰テスト）は**維持する**。`code-reviewer` / `security-reviewer` / `test-analyzer` と 4 スキルは、いずれもプロジェクト固有の規約を内容に持つため残す。

既存の decision log・plan・`docs/wiki/log.md` の過去エントリは時点の記録として改変せず、本ログで撤去を記録する形を取る（[arch-20260703-remove-apple-container-fallback.md](./arch-20260703-remove-apple-container-fallback.md) の前例に倣う）。

## 理由と代替案

**採用した理由:**

- 2 ハーネス併存は Apple `container` フォールバックと同じ構図で、等価性の維持コストが継続的に発生する一方、Copilot CLI 側は実運用されておらず drift の検出手段も無い
- hook が担うべきは「実行**前**に止める」「出力を検査する」といった他層で代替できない役割であり、lint / 型 / テストは commit 時と CI で押さえれば十分。同じ検査を 3 回走らせる分だけ AI セッションが遅くなっていた
- `harness-lib.sh` の存在理由（`HARNESS_KIND` 判定と block 形式の切替）が Copilot 撤去で 100% 消える。各 hook が lib から使う関数は 1〜2 個で、Claude 専用にすると 1 行の `jq` に潰れるため、抽象の元が取れない
- deny リストは回避可能で実効的な上積みが無い一方、読み取りコマンド 8 種 × 5 パターンの列挙という高い保守コストを払っていた

**検討した代替案:**

- **`harness-lib.sh` を Claude 専用 20 行に縮めて残す**: hook を将来 5〜6 本に増やす前提なら妥当だが、今回は逆方向（3 本へ削減）であり、最終状態のファイル数・総行数ともにインライン化の方が小さい。また lib を残すと「1 ハーネスしか無いのに分岐の残骸を持つシム」になり読み手を混乱させるため見送り
- **deny リストを残す**: 多層防御としての心理的な安心はあるが、`node -e` 等で回避できる以上「防げているつもり」の方が危険。実シークレットを作業ツリーに置かない運用と `secret-leak-detector.sh` の出口検査に集約する
- **`lint-guard.sh` だけ残す**: 実装中に即 lint が返る体験には価値があるが、単一ファイル ESLint では層依存違反・循環依存・複雑度は検出できず（それらは `d review` の dependency-cruiser / madge が担う）、timeout 30 秒 × 毎編集というコストに見合わないため見送り

## 影響

**ポジティブ:**

- 保守対象が `.claude/settings.json` 単独になり、hook 登録の二重管理が消える
- Stop hook 撤去でターン終了が最大 120 秒短縮される
- 各 hook が `source` 依存を持たず単体で読める・単体で実行できるようになる（手動テストが全 hook で可能）
- `settings.json` が約 160 行 → 約 95 行に縮小

**ネガティブ・リスク:**

- **Copilot CLI での動作保証を手放す**。再導入には `harness-lib.sh` 相当のシムと `hooks.json` の再実装が必要
- **実装中の即時 lint フィードバックが失われる**。違反の検出は commit 時の lefthook / `d review` / CI に後退する
- **deny 撤去により、ローカルに実 `.env` が存在すれば AI が読める**。防御は「実シークレットを作業ツリーに置かない」（`.env` は `.gitignore` 済み、共有するのは `.env.example` のみ）と `secret-leak-detector.sh` の出口検査の 2 層に依存する
- 研修教材 `tutorial/1day-tutorial.md` の「ハーネスありだと実装中に即座に lint が返る」という演出が弱まる（教材側は「ESLint hook を自分で新規追加する」演習へ差し替える）

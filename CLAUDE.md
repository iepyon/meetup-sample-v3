# CLAUDE.md

プロジェクト共通の指示は AGENTS.md に記載。本ファイルは Claude Code 固有の設定のみ。

@AGENTS.md

## Claude Code Specific

- コミットメッセージは `japanese-commit` スキルの形式で生成する
- `lefthook` の git pre-commit で品質ゲート（cargo fmt / clippy / test）が自動実行される
- AI hook は 3 個: dep-install-guard（npm install ガード）・secret-leak（Bash 出力監視）・prettier-format（Edit/Write 後の自動整形）。フォーマット・lint・型・テストは lefthook / CI / `d review` が担う
- code-reviewer + security-reviewer は、複数ファイルにまたがる実装・認証/認可/外部入力に触れる変更・PR 作成前に並列実行する。単一ファイルの軽微な修正では省略してよい
- test-analyzer は、テストを新規追加したときと受け入れ基準との対応を確認したいときに使う
- 複数ファイル・複数コンテキストに及ぶ調査はサブエージェントに委譲してメインコンテキストを保護する（1 ファイルの確認は直接読む）

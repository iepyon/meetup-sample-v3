# Apple `container` フォールバックを撤去し Docker 単一経路に統一する

日付: 2026-07-03
種類: arch
ステータス: 承認済み

## コンテキスト

同日の [arch-20260703-revert-to-docker-as-primary.md](./arch-20260703-revert-to-docker-as-primary.md) で正式経路を Docker / Docker Compose に戻し、Apple `container`（`./scripts/container-dev.sh` / `./scripts/container-worktree.sh`）はフォールバックとして残していた。

その後の Docker 構成レビューで、2 経路併存に以下の負債が確認された:

- **既に drift している**: Apple `container` 版のみ Playwright ブラウザを named volume 化、`~/.claude` を無条件 rw マウント、`container-worktree.sh` には `playwright install` が無い、など docker 版と挙動差が発生済み
- **worktree の並列起動に未対応**: `container-worktree.sh` は共有 `meetup-dev` コンテナに exec するだけで、Docker 版の「独立コンテナ・独立ポート」機能とは実装レベルが異なる
- **サブコマンド等価性の維持コスト**が継続的に発生し、片方だけ更新されると乖離する

Docker は Intel / Apple Silicon 両対応で、フォールバックが無くても環境の間口は塞がらない。

## 決定

Apple `container` 経路を**削除し、Docker / Docker Compose の単一経路に統一する**。

- 削除: `scripts/container-dev.sh`、`scripts/container-worktree.sh`、`docs/apple-container-usage.md`
- `scripts/run-quality.sh` から Apple ランタイム判定・分岐を除去し Docker 専用に簡素化
- `README.md` / `AGENTS.md` / `docs/docker-usage.md` から Apple `container` 記述とフォールバック導線を削除

[arch-20260620-adopt-apple-container.md](./arch-20260620-adopt-apple-container.md) と [arch-20260703-revert-to-docker-as-primary.md](./arch-20260703-revert-to-docker-as-primary.md) は時点の意思決定を記録した履歴のため改変せず、本ログで撤去を追記する形とする。

## 理由と代替案

**採用した理由:**
- 保守対象が 1 経路に減り、drift・サブコマンド等価性維持のコストが消える
- Docker は Intel / Apple Silicon 両対応のため、フォールバック撤去で塞がる環境は無い
- 実装差（Playwright volume・`~/.claude` の扱い・worktree 独立化の有無）に起因する事故要因を除去できる

**検討した代替案:**
- **フォールバックとして残置**: Apple 純正ランタイムの選択肢は残るが、drift 済みの 2 経路を等価に保つ保守コストと事故リスクに見合わないため見送り
- **Apple 版を docker 版と完全同期させて残す**: 独立化（worktree）まで含めた同期は実装負荷が高く、正式経路でない経路にその投資をする合理性が無いため見送り

## 影響

**ポジティブ:**
- コンテナ関連スクリプト・ドキュメントの保守対象が半減
- ランタイム判定分岐が消え、`run-quality.sh` などの経路が単純化
- 経路差に起因する不整合（`~/.claude` rw、Playwright 永続化差など）が解消

**ネガティブ・リスク:**
- Apple `container` を使っていた開発者は Docker（Docker Desktop / Colima 等）へ切り替える必要がある
- Apple 純正・OSS 完結というランタイム選択肢を将来にわたって手放す（再導入時は再実装が必要）

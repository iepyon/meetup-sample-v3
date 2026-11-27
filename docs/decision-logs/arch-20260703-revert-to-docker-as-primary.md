# ローカル開発の正式経路を Docker / Docker Compose に戻す

日付: 2026-07-03
種類: arch
ステータス: 承認済み

## コンテキスト

2026-06-20 に、ローカル開発のコンテナ実行を Colima + Docker Compose から Apple Silicon ネイティブの Apple `container`（`./scripts/container-dev.sh`）へ移行し、これを正式経路とした（[arch-20260620-adopt-apple-container.md](./arch-20260620-adopt-apple-container.md)）。PoC ではフルフロー（`up → install → test → e2e`）のパリティを実証済みだった。

一方で、Apple `container` を正式経路とする運用には以下の制約が残っていた:

- **Apple Silicon 専用**（Intel Mac 非対応）で、開発環境の間口が狭い
- Apple `container` v1.0.0 の成熟度に依存し、メモリがホストに返らない等の既知挙動が長時間運用で未解消
- `container system start` / カーネル導入など、Docker にはない初回セットアップ手順が必要
- CI や他ツールとの相互運用、ドキュメント・チームの共通言語としては Docker / Docker Compose のほうが標準的で摩擦が少ない

`docker-compose.yml` と `scripts/docker-dev.sh` は移行後もフォールバックとして削除せず保守しており、サブコマンド体系は両者同一で常に等価に動作する状態を維持できている。

## 決定

ローカル開発の**正式経路を Docker / Docker Compose（`./scripts/docker-dev.sh`）に戻す**。Apple `container`（`./scripts/container-dev.sh`）は削除せず、**フォールバック経路**として残存させる。

これに伴い、正式経路を示す各ドキュメントの位置づけを反転した:

- `README.md` — Docker を正式経路として記述（反転済み）
- `AGENTS.md` — コンテナランタイム・前提・alias・cleanup・worktree・General Rules を Docker 基準に更新
- `docs/docker-usage.md` — 「位置づけ」を Docker=正式経路 / Apple `container`=フォールバックに反転
- `docs/apple-container-usage.md` — 冒頭とフォールバック節を Docker 正式経路前提に更新

`docs/decision-logs/arch-20260620-adopt-apple-container.md` および `docs/plans/apple-container-poc.md` は時点の意思決定・検証を記録した履歴のため改変せず、本ログで判断の更新を追記する形とする。

## 理由と代替案

**採用した理由:**
- Intel Mac を含む幅広い環境で動作し、開発環境の間口が広い
- Docker / Docker Compose は業界標準で、CI・他ツール・チームの共通言語としての摩擦が少ない
- 追加のランタイム成熟度リスク（メモリ挙動など）や初回セットアップ手順を負わない
- `docker-compose.yml` / `docker-dev.sh` は保守継続しており、正式経路に戻す実コストがほぼゼロ

**検討した代替案:**
- **Apple `container` を正式経路のまま継続**: Apple 純正・OSS で完結する利点はあるが、Apple Silicon 専用の制約と v1.0.0 の成熟度リスクを正式経路として抱え続けることになるため見送り（フォールバックとしては残す）
- **両者を対等な「どちらでも可」として明記しない**: 正式経路が曖昧だと新規参加者やドキュメントの導線が定まらず、トラブル時の切り分け基準も失われるため見送り。正式経路とフォールバックを明示する方針を採用

## 影響

**ポジティブ:**
- Intel Mac を含む環境差を吸収でき、オンボーディングが容易
- 標準的な Docker ツールチェーンに揃い、CI・ドキュメント・外部知見との整合が取りやすい
- Apple `container` の成熟度リスクを正式経路から切り離せる

**ネガティブ・リスク:**
- Docker Desktop / Colima などサードパーティ・コンテナランタイムへの依存が正式経路に戻る（Apple 純正化のメリットは後退）
- Apple `container` はフォールバックとして残るため、2 経路のサブコマンド等価性を今後も維持する保守コストが継続する
- Apple `container` を使っていた開発者は alias を `./scripts/docker-dev.sh` に切り替える必要がある

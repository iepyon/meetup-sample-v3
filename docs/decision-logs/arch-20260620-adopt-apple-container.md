# Colima から Apple `container` へローカル開発環境を移行

日付: 2026-06-20
種類: arch
ステータス: 承認済み

## コンテキスト

ローカル開発は `./scripts/docker-dev.sh`（Docker Compose）を **Colima**（macOS Virtualization.Framework / runtime: docker）バックエンドで動かしていた。加えて Rancher Desktop 由来の docker CLI（`~/.rd/bin/docker`）が PATH 上に混在していた。

2026 年、Apple Silicon ネイティブの純正コンテナツール `container`（WWDC 2025 発表、2026-06-09 に v1.0.0）が利用可能になり、Docker Desktop / Colima 等のサードパーティ依存を排除できる見込みが立った。本プロジェクトは `docker-compose.yml` が **単一サービス（`dev`、DB は SQLite ファイル）** であり、Apple `container` が未対応の Compose オーケストレーションを必要としないため、移行の技術的障壁が低いと判断し PoC を実施した。

## 決定

ローカル開発のコンテナ実行を **Apple `container` に移行**する。`docker-dev.sh` 等価のヘルパー `scripts/container-dev.sh` を正式な実行経路とする。Colima / Rancher Desktop はフォールバックとして当面残す（`docker-dev.sh` も削除しない）。

PoC（ブランチ `feat/apple-container-poc`、`docs/plans/apple-container-poc.md`）で、`up → install → test → e2e` のフルフローが等価に通ることを実証済み（backend 284 / frontend 98 / E2E 11 すべて green）。

## 理由と代替案

**採用した理由:**
- 単一サービス構成のため Apple `container` の Compose 非対応が障害にならない
- Docker Desktop / Colima といったサードパーティ・ランタイム依存を排除でき、Apple 純正・OSS で完結
- PoC でフルフローのパリティを実証済み。移行に要した対処は 3 点のみ（下記）
- コンテナごとに軽量 VM・サブ秒起動で Apple Silicon に最適化

**検討した代替案:**
- **Colima を継続**: 既存ワークフローを変更不要だが、サードパーティ依存とランタイム混在（Rancher Desktop との二重 docker）が残る → 純正化のメリットを取れないため見送り
- **OrbStack へ移行**: Compose 完全互換で高速だが、これもサードパーティ依存。Docker 依存排除という主目的を満たさないため見送り（ただし移行が頓挫した場合の有力な代替として記録）
- **Apple `container` 単体 + Compose 風スクリプト/Container-Compose**: 複数サービス構成向け。本プロジェクトは単一サービスのため不要

## 影響

**ポジティブ:**
- サードパーティ・コンテナランタイム依存の排除（純正・OSS 化）
- Apple Silicon ネイティブによる高速起動・最適化

**ネガティブ・リスク:**
- **Apple Silicon 専用**（Intel Mac 非対応）。チームに Intel Mac があれば移行不可
- Apple `container` v1.0.0 の成熟度。特に **メモリがホストに返らない**（VM 再起動でのみ解放）挙動は長時間運用で未検証（R4）
- 移行で対処が必要だった差分（PoC で対処済み）:
  1. lefthook が x86_64 バイナリ → arm64 ネイティブで `Exec format error` → Dockerfile をアーキテクチャ自動判定に修正
  2. Playwright ブラウザがコンテナ書き込み層にあり `down`/`up` で消失 → named volume 化 + 自動 chown
  3. コンテナ既定メモリ 1GiB が不足し E2E がフレーキー → `--cpus 4 --memory 8g` を付与
- `container system start` とカーネル導入（`container system kernel set --recommended`）という初回セットアップ手順が増える

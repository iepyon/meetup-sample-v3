# 実装計画: Apple `container` への移行 PoC

ブランチ: `feat/apple-container-poc`
ステータス: ドラフト（PoC 検証中）

## 背景

現在この Mac は **Colima**（macOS Virtualization.Framework / runtime: docker / virtiofs）を Docker バックエンドに使用している。Apple が WWDC 2025 で発表し 2026-06-09 に v1.0.0 に到達した純正 `container` CLI への置き換えを検討する。

### 調査で確定した前提

- このプロジェクトの `docker-compose.yml` は **単一サービス `dev`** のみ。DB は別コンテナではなく **SQLite ファイル**（`DATABASE_URL=file:./prisma/dev.db`）。
  - → Apple `container` がネイティブ非対応の **Docker Compose オーケストレーション（複数コンテナ連携）は、このプロジェクトでは不要**。やることは単一サービスを 1 個の `container run` に翻訳するだけ。
- `container` CLI は移行に必要な機能を公式に備える（`command-reference` で確認済み）:
  - `-v/--volume`・`--mount`（バインドマウント）、`-p/--publish`、`-e/--env`、`--name`、`-d/--detach`、`container build --build-arg`
- 対応条件: **Apple Silicon 専用**（Intel 非対応）。

## ゴール

Colima を**いきなり捨てず**、Apple `container` 版のヘルパー `scripts/container-dev.sh` を `docker-dev.sh` と**並立**で作成し、以下の主要フローが等価に通ることを検証する。

```
up → install → dev → test → e2e → shell → down
```

通れば段階移行、通らなければ Colima 継続の判断材料とする。

## 未確定リスク（Phase 0 で検証済み）

| # | リスク | 影響 | 結果（2026-06-20 検証） |
|---|---|---|---|
| R1 | **名前付きボリューム**（node_modules_root/backend/frontend/e2e）の対応可否 | 高 | ✅ **解決**。`container volume create/ls/rm/inspect/prune` あり。`--mount type=volume,source=NAME,target=/path` で作成・書込・**別コンテナでの永続再読込**を確認 |
| R2 | **`container exec` のユーザ指定**（現状 `docker compose exec -u devuser`） | 中 | ✅ **解決**。`container exec -u/--user <name\|uid[:gid]>`・`--uid`・`--gid`・`-w` あり。`docker compose exec -u devuser` と等価 |
| R3 | `$REPO_PARENT` 同一パスのバインドマウント性能・権限（worktree 絶対パス互換） | 中 | ✅ **解決**。`-v host:container` 同一パスで双方向 read/write 確認。※コンテナ内書込ファイルの owner は実行ユーザ依存 → イメージの devuser(USER_UID/GID) もしくは `-u` で揃える |
| R4 | Apple container のメモリ非解放・成熟度（v1.0.0） | 低〜中 | 未検証（Phase 3 の長時間 test で観察） |

### Phase 0 検証ログ（環境）
- `container` 1.0.0（Homebrew bottle `1.0.0_1.arm64_tahoe`）/ macOS 26.5.1 / arm64
- `container system start` → 推奨カーネル（kata-static 3.28.0 arm64）を `container system kernel set --recommended` で導入
- スモーク: `container run --rm alpine echo ...` 成功。2 回目以降はイメージキャッシュで起動 1 秒未満

## フェーズ

### Phase 0: 環境準備とリスク潰し（先行）— ✅ 完了
- [x] `brew install container` → `container system start` → カーネル導入 → `container --version`
- [x] R1: 名前付きボリュームの作成・マウント構文を確認（OK）
- [x] R2: `container exec` のユーザ指定オプションを確認（OK）
- [x] R3: 同一パスバインドマウントの双方向 read/write を確認（OK）
- [x] R1/R2/R3 すべて対応あり → 代替案不要、本計画どおり進行可能

### Phase 1: イメージビルド — ✅ 完了
- [x] 既存 `Dockerfile` を `container build --build-arg USER_UID=$(id -u) --build-arg USER_GID=$(id -g) -t meetup-dev:poc .` でビルド（linux/arm64、成功）
- [x] `container image ls` で `meetup-dev:poc` 確認
- ⚠️ 注意点（Phase 3 で観察）:
  - Dockerfile 51 行目の lefthook が `Linux_x86_64` バイナリ。arm64 ネイティブ実行では起動しない可能性 → 実行時に問題なら arm64 版に差し替え
  - entrypoint は `gosu devuser` でユーザ切替 + named volume の owner 修正。`container run` でも CMD/ENTRYPOINT がそのまま効くか要確認（効かない場合は `container-dev.sh` 側で `exec -u devuser` 運用に寄せる）

### Phase 2: `scripts/container-dev.sh` 作成
- [ ] `docker-dev.sh` と同じサブコマンド（build/up/down/install/dev/test/e2e/shell + catch-all）を `container` CLI で実装
- [ ] compose の volumes/ports/env/working_dir を `container run` フラグに翻訳（本ドキュメントの翻訳表に従う）
- [ ] macOS/zsh 互換・POSIX 準拠（AGENTS.md のシェル規約遵守）

### Phase 3: フロー検証 — ✅ 完了（パリティ達成）
- [x] `up` → コンテナ起動（同一パスマウント・named volume・file マウント全て OK）
- [x] `install` → npm install + prisma generate + db:push + playwright install（全て成功）
- [x] `test` → **backend 284 / frontend 98 テスト全 green**
- [x] `e2e` → **Playwright 11/11 green（4.7s）**
- [x] `shell` / `down` 動作確認

#### Phase 3 で発見・対処した差分
1. **lefthook が x86_64 バイナリ** → arm64 ネイティブの Apple container で `Exec format error`。Dockerfile をアーキテクチャ自動判定に修正（Colima では qemu エミュレーションで動いていた）
2. **Playwright ブラウザ非永続** → `/opt/playwright-browsers` はコンテナ書き込み層にあり `down`/`up` で消失。named volume 化 + `up` 後の `chown devuser:<gid>` で対処（compose はコンテナを作り直さない運用で露見しなかった）
3. **メモリ既定 1GiB が不足** → e2e がタイムアウトでフレーキー。`--cpus 4 --memory 8g` 付与で安定（R4 の結論）。Colima は VM 全体メモリを使えるため顕在化していなかった

### Phase 4: 判断とドキュメント化
- [x] パリティ結果を本ドキュメントに追記
- [ ] 移行する場合: AGENTS.md / CLAUDE.md のコマンド節を更新、`decision-log` スキルで意思決定を記録
- [ ] 移行しない場合: 障壁を記録して Colima 継続

## 結論（PoC）

Apple `container` 1.0.0 で **`docker-dev.sh` 相当のフルフロー（up→install→test→e2e）を等価に再現できることを実証**。単一サービス構成のため Compose 非対応は障害にならなかった。移行に必要だったのは以下 3 点の対処のみ:

| 項目 | 対処 | 適用先 |
|---|---|---|
| lefthook アーキテクチャ | arch 自動判定に修正 | `Dockerfile`（既存にも有益） |
| Playwright ブラウザ永続化 | named volume + chown | `scripts/container-dev.sh` |
| リソース割当 | `--cpus 4 --memory 8g` | `scripts/container-dev.sh` |

残課題: R4 のメモリ非解放（長時間運用での挙動）は未観測。移行判断は上記トレードオフ（Colima/OrbStack の成熟度 vs Apple 純正・Docker 依存排除）を踏まえて決定する。

## 非対象（スコープ外）

- Colima / Rancher Desktop（`~/.rd/bin/docker`）の撤去 — PoC では既存環境を温存
- `container machine`（永続 VM 内 docker compose）方式 — 単一サービスの本プロジェクトでは過剰、別途検討
- CI / lefthook の Docker 化変更

## 翻訳リファレンス（compose → container run）

```bash
container build --build-arg USER_UID=$(id -u) --build-arg USER_GID=$(id -g) -t meetup-dev .

container run --detach --name meetup-dev \
  -v "$REPO_PARENT:$REPO_PARENT" \
  -v ~/.gitconfig:/home/devuser/.gitconfig:ro \
  -v ~/.claude:/home/devuser/.claude \
  -p 3000:3000 -p 5173:5173 \
  -e DATABASE_URL=file:./prisma/dev.db \
  -e JWT_SECRET=docker-dev-secret -e NODE_ENV=development \
  -e VITE_HOST=0.0.0.0 -e PLAYWRIGHT_BROWSERS_PATH=/opt/playwright-browsers \
  -w "$REPO_PARENT/$PROJECT_DIR_NAME" \
  meetup-dev
# node_modules_* の名前付きボリューム指定は R1 検証後に確定
```

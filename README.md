# Meetup Community Management

Meetup コミュニティとメンバーシップを管理するフルスタック Rust アプリケーション。

## Tech Stack

- **Backend**: Rust + axum + sqlx + SQLite（DDD の境界づけられたコンテキスト構成）
- **Frontend**: Rust + Leptos 0.8 (CSR / WebAssembly) + Trunk + Tailwind CSS
- **Testing**: `cargo test`（単体 + HTTP 統合テスト）+ E2E（Rust + chromiumoxide でヘッドレス Chromium を操作。npm は使わない）

## Getting Started

ローカル開発は **Docker / Docker Compose**（`./scripts/docker-dev.sh`）を経路とする（Intel / Apple Silicon 両対応）。詳しい手順・トラブルシュートは **[docs/docker-usage.md](docs/docker-usage.md)** を参照。

```bash
# 開発フロー
alias d="./scripts/docker-dev.sh"
d build     # イメージビルド（Dockerfile 変更時）
d up        # コンテナ起動
d install   # cargo fetch + lefthook install（git pre-commit の有効化）
d dev       # 開発サーバー起動 (backend:3000 + frontend:5173)
d test      # cargo test --workspace
d e2e       # E2E テスト（SPA をビルドし cargo test -p meetup-e2e -- --ignored）
d shell     # コンテナ内 bash
d down      # 停止 + 削除（named volume は保持）

# 任意コマンド（catch-all）— d <cmd> はコンテナ内でそのまま実行される
d lint                                  # cargo clippy（警告=エラー）
d review                                # fmt + clippy + test の総合確認
d bash -c "cargo test -p meetup-backend create_community"   # 単一テスト
```

> **コンテナ内で Claude Code を使う場合:** ホストの `~/.claude` は既定でマウントされません（認証情報の露出回避）。マウントはコンテナ生成時に反映されるため `WITH_CLAUDE=1 d up`（再生成）してから `d shell` してください（既存コンテナへの `d shell` 単独では効きません）。

> その他のレシピ（単一テスト実行・クリーンアップ等）は [docs/docker-usage.md](docs/docker-usage.md) §4.5 を参照。

## 並行開発（git worktree で独立起動）

worktree ごとに **独立した Docker コンテナ・独立したポート・独立した DB** を立てられます。main と複数の worktree を同時に起動して、それぞれ別ポートでブラウザ確認できます。

```bash
git worktree add .worktrees/feat-x -b feat-x        # worktree 作成
./scripts/docker-worktree.sh feat-x ports           # 割り当てポート確認
./scripts/docker-worktree.sh feat-x up              # 独立コンテナ起動
./scripts/docker-worktree.sh feat-x install         # 依存取得 + lefthook install
./scripts/docker-worktree.sh feat-x dev             # 開発サーバー起動（表示された別ポート）
./scripts/docker-worktree.sh feat-x test            # テスト
./scripts/docker-worktree.sh feat-x e2e             # E2E テスト（main と同時に実行できる）

./scripts/docker-worktree.sh feat-x clean           # コンテナ + volume 削除（削除前に必須）
git worktree remove .worktrees/feat-x               # worktree 削除
git branch -d feat-x                                # ブランチ削除（マージ済みなら）
```

**ポート採番**: PR 番号から slot を `SLOT = PR % 2000` で決め、`backend = 4000 + SLOT` / `frontend = 6000 + SLOT`。PR 番号は `gh` から自動取得し、取得できない場合はブランチ名のハッシュ値に fallback します。PR 番号が異なっても slot（mod 2000）が一致すると衝突しうるため、その場合は `PR=` か `BACKEND_PORT=/FRONTEND_PORT=` で上書きしてください（`up` 時にポート使用中を検知してエラーにします）。

```bash
PR=42 ./scripts/docker-worktree.sh feat-x up                          # PR 番号を明示 → 4042 / 6042
BACKEND_PORT=4100 FRONTEND_PORT=6100 ./scripts/docker-worktree.sh feat-x up   # ポート直接指定
```

> **注意:** `git worktree remove` は Docker のコンテナ・volume を掃除しません。worktree を捨てる前に必ず `clean`（`down -v`）を実行してください。詳細は [docs/docker-usage.md](docs/docker-usage.md) の「6. worktree 運用」を参照。

## Git Hooks セットアップ

コミット時に cargo fmt / clippy / test が自動実行されます。

**Docker 環境:** `d install` が `lefthook install` を実行します。追加手順は不要です。

**ホスト直接実行の場合:** lefthook バイナリが必要です。

```bash
# macOS
brew install lefthook

# Linux（GitHub Releases のバイナリを PATH に置く。Go があれば go install でもよい）
go install github.com/evilmartians/lefthook@v1.10.4   # Docker イメージと同じ版
```

インストール後、リポジトリ直下で `lefthook install` を一度実行します。

> **git worktree を使う場合:** worktree ごとに `./scripts/docker-worktree.sh <name> install`（`cargo fetch` → `lefthook install`）の実行が必要です。詳細は上記「並行開発（git worktree で独立起動）」を参照。

## Project Structure

```
backend/     axum API サーバー（auth / community / event / shared / infrastructure）
frontend/    Leptos SPA（WebAssembly。Trunk でビルド）
e2e/         E2E テスト（Rust。backend を同一プロセスで起動し、chromiumoxide でヘッドレス Chromium を操作）
Cargo.toml   Cargo workspace（backend + frontend + e2e）
```

## 本番相当の起動（単一バイナリ）

```bash
(cd frontend && trunk build --release)          # frontend/dist を生成
JWT_SECRET=<32 バイト以上のランダム値> FRONTEND_DIST=frontend/dist \
  cargo run --release -p meetup-backend          # API + SPA を :3000 で配信
```

環境変数: `JWT_SECRET`（必須・32 バイト以上。未設定または短ければ起動しない。開発は `.env` に書く）/ `DATABASE_URL`（既定 `file:./dev.db`）/ `PORT`（既定 3000）/ `RATE_LIMIT_DISABLED=true`（レート制限を無効化）/ `FRONTEND_DIST`（指定時のみ SPA を配信）。

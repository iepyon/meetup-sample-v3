# Docker / docker-compose 開発環境ガイド

Colima + Docker（`./scripts/docker-dev.sh`）を使ったローカル開発の解説資料です。各コマンドが何をするか、そしてこの構成がなぜこうなっているか（named volume / gosu / UID 合わせ）を1枚で理解できるようにまとめます。

> **位置づけ**: ローカル開発は本ガイドが扱う **Docker / Docker Compose**（`./scripts/docker-dev.sh`）の単一経路です（Intel / Apple Silicon 両対応）。かつて検証していた Apple `container` 経路は撤去しました。経緯は [`docs/decision-logs/arch-20260703-revert-to-docker-as-primary.md`](./decision-logs/arch-20260703-revert-to-docker-as-primary.md) と [`docs/decision-logs/arch-20260703-remove-apple-container-fallback.md`](./decision-logs/arch-20260703-remove-apple-container-fallback.md) を参照してください。

## 1. 前提

Docker Engine（`docker compose` サブコマンドが使える v2 系）が必要です。macOS では Colima か Docker Desktop のいずれかで Docker Engine を用意します。

```bash
# Colima を使う場合の一例（Docker Desktop の場合はアプリ起動でよい）
brew install colima docker
colima start --cpu 4 --memory 8            # Linux VM + Docker Engine を起動
docker compose version                     # v2 系が使えることを確認
```

以降、コマンドはすべて `./scripts/docker-dev.sh` ラッパー経由で実行します。エイリアスを張っておくと楽です。

```bash
alias d="./scripts/docker-dev.sh"
```

## 2. 全体アーキテクチャ

単一の常駐コンテナ `dev`（`sleep infinity` で起動しっぱなし）を立て、そこに `docker compose exec` で各種コマンドを流し込む「開発コンテナ方式」です。

```
┌─────────────────────────── ホスト (macOS) ───────────────────────────┐
│                                                                       │
│  リポジトリ実体 (ソースコード)  ~/.gitconfig(ro)  ~/.claude(opt-in)   │
│         │ bind mount                    │            │                │
│         │ (ホストと同一絶対パス)          │            │                │
│  ───────┼─────────────────────────────┼────────────┼──────────────  │
│         ▼                              ▼            ▼                  │
│  ┌──────────────────── dev コンテナ (node:26-bookworm) ───────────┐   │
│  │  ユーザー: devuser (UID 501 / GID 20 = macOS に合わせる)         │   │
│  │                                                                 │   │
│  │   backend (axum)     :3000  ──┐                                 │   │
│  │   frontend (trunk)   :5173  ──┼── ports で publish → localhost  │   │
│  │                               │                                 │   │
│  │   Linux バイナリ / ビルド成果物は named volume で分離:           │   │
│  │     cargo target / cargo registry                               │   │
│  │   Rust toolchain: /opt/rustup, /opt/cargo                       │   │
│  │   E2E 用ブラウザ: Debian の chromium                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                       │
└───────────────────────────────────────────────────────────────────┘
```

- ソースコードは **bind mount**（ホストで編集 → 即コンテナに反映。frontend は `trunk serve` が変更を検知して再ビルドする。backend は `cargo run` の再起動が必要）
- Rust のビルド成果物（`target/`）と cargo registry は **named volume**（ホストの macOS の成果物とコンテナの Linux のものを混在させず、bind mount より速い）
- backend `:3000` / frontend `:5173` を `localhost` に publish

アプリ本体は Rust（backend = axum + sqlx(SQLite)、frontend = Leptos CSR + trunk）です。E2E も Rust（`e2e/` クレート。chromiumoxide でヘッドレス Chromium を操作）で、npm は使いません。Node は Claude Code CLI のためだけにコンテナへ入っています。

## 3. 基本フロー

```bash
d build                  # イメージをビルド
d up                     # コンテナをバックグラウンド起動（--build 込み）
d install                # cargo fetch --locked + lefthook install（git pre-commit の有効化）
d dev                    # backend (cargo run) + frontend (trunk serve) を起動（detach）
                         #   → Backend  http://localhost:3000
                         #   → Frontend http://localhost:5173
d test                   # cargo test --workspace（backend 単体・HTTP 統合 + frontend 純粋関数）
d e2e                    # E2E テスト（trunk build → cargo test -p meetup-e2e -- --ignored。d dev と併用可）
d lint                   # cargo clippy -D warnings（backend + frontend/wasm32。対象を絞るなら d lint backend）
d typecheck              # cargo check（backend + frontend/wasm32）
d review                 # 実装完了前の総合チェック（cargo fmt --check → lint → test）
d shell                  # コンテナ内で bash に入る
d down                   # コンテナを停止・削除（named volume は保持）
```

任意コマンドはそのままコンテナ内で実行できます（catch-all）。

```bash
d bash -c "cargo fmt --all"                                  # 整形
d bash -c "cargo build --release -p meetup-backend"          # backend のリリースビルド
d bash -c "cd frontend && trunk build --release"             # frontend の本番ビルド（dist/ に出力）
```

よく使うレシピは §4.5、サブコマンドの一覧は §4.4 にあります。

### 3.1 コンテナ内で直接実行する場合（`/.dockerenv` 存在時）

`d shell` で入った状態や、コンテナ内で Claude Code を動かしている場合は、`docker-dev.sh` を経由せず直接実行します。コンテナ内かどうかは `/.dockerenv` の存在で判別できます。

```bash
# テスト
cargo test --workspace
cargo test -p meetup-backend           # backend のみ（単体 + tests/api.rs）
cargo test -p meetup-frontend          # frontend の純粋関数テスト

# Lint（警告=エラー）
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings

# 整形・型チェック
cargo fmt --all -- --check
cargo check -p meetup-backend --all-targets
cargo check -p meetup-frontend --target wasm32-unknown-unknown

# 単一テスト（テスト名の部分一致）
cargo test -p meetup-backend --test api 一覧は公開のみ

# E2E（#[ignore] 付きなので --ignored で明示的に実行する）
(cd frontend && trunk build --dist ../target/e2e-dist)
cargo test -p meetup-e2e -- --ignored
```

### 3.2 Docker 内で Claude Code を動かす（ホストから起動）

`~/.claude` は既定でマウントされません（認証情報の露出回避）。マウントは `up` 時に反映されるため、`WITH_CLAUDE=1` は `up` に付けます（既存コンテナへの `shell` / `exec` には効きません）。

```bash
WITH_CLAUDE=1 d up       # ~/.claude をマウントしてコンテナを（再）生成
d shell                  # コンテナに入る
claude                   # Claude Code 起動（API キーまたは ~/.claude の認証を使用）
```

## 4. 各構成ファイルの解説

### 4.1 `Dockerfile`

`node:26-bookworm` の単一ステージ。Node は Claude Code CLI 用のみで、アプリ本体と E2E は Rust です。中身は以下を積み上げています。

| 内容 | 目的 |
|---|---|
| `git` / `sqlite3` / `jq` / `curl` | 基本ツール（SQLite は DB の中身確認用） |
| `build-essential` / `pkg-config` / `libssl-dev` | Rust クレートのネイティブビルド用 |
| Rust toolchain（rustup を `/opt/rustup`・`/opt/cargo` に導入。stable、clippy / rustfmt、`wasm32-unknown-unknown` ターゲット） | backend のビルド、品質ゲート、Leptos（CSR）の wasm ビルド |
| `trunk` 0.22.2（`cargo install --locked`） | frontend のビルド・dev サーバ（`trunk serve`） |
| `chromium`（Debian パッケージ。arm64 / amd64 両対応） | E2E（`e2e/` の chromiumoxide）が CDP で操作するヘッドレスブラウザ |
| 非 root ユーザー `devuser`（`USER_UID=501` / `USER_GID=20`） | bind mount のファイル所有権を macOS 側と一致させる |
| `gosu` | エントリポイントで root → devuser に降格するため |
| `git config --system --add safe.directory '*'` | 所有者の異なる bind mount 上でも git を使えるようにする |
| `@anthropic-ai/claude-code`（グローバル） | コンテナ内で Claude Code を起動できる |
| `lefthook` v1.10.4 | git pre-commit の品質ゲート。`dpkg --print-architecture` で arm64/amd64 を判定しネイティブバイナリを取得 |

起動構成は `ENTRYPOINT ["docker-entrypoint.sh"]` + `CMD ["sleep", "infinity"]`。つまりコンテナは常駐し、実作業は `exec` で流し込みます。

### 4.2 `docker-compose.yml`

サービスは `dev` の1つだけです。

- **build args**: `USER_UID` / `USER_GID`（既定 `501` / `20`）。macOS の UID/GID に合わせる。
- **volumes**:
  - `${MOUNT_ROOT}:${MOUNT_ROOT}` — メインプロジェクトディレクトリを**ホストと同一の絶対パス**でマウント。worktree は `.worktrees/...` 配下にネストするので、これだけで worktree の絶対パスもコンテナ内で一致する。親ディレクトリ全体ではなくこのリポジトリ配下に露出を限定している。
  - `cargo_target`（`${WORK_DIR}/target`）/ `cargo_registry`（`/opt/cargo/registry`）— Rust のビルド成果物と取得済みクレート。bind mount だと遅いため named volume に置く。
  - マウント先パスは `${WORK_DIR}` で組み立てる（main は `docker-dev.sh` が project 直下を、worktree は `docker-worktree.sh` が worktree ディレクトリを export）。
  - `~/.gitconfig:...:ro` — ホストの git 設定を読み取り専用でマウント。
  - `~/.claude` — **既定ではマウントしない**。ホスト認証情報の露出を避けるため、コンテナ内で Claude Code を使うときだけ `WITH_CLAUDE=1` を付けて `docker-compose.claude.yml` を重ねる（main・worktree とも読み取り専用 `:ro`。コンテナからホストの設定や hook を書き換えられないようにするため）。マウントはコンテナ生成時にのみ反映されるので `WITH_CLAUDE=1 d up`（再生成）が必要で、既存コンテナへの `d shell`/`exec` 単独では効かない。
- **image**: `meetup-dev:local` — 共有イメージタグ。worktree ごとに別プロジェクトを立てても1つのビルドを再利用する（N 回ビルドを回避）。
- **ports**: `${BACKEND_PORT:-3000}:3000`（backend）/ `${FRONTEND_PORT:-5173}:5173`（frontend）。ホスト側ポートのみ可変。コンテナ内は常に 3000 / 5173 固定なので、`frontend/Trunk.toml` の proxy・backend の `PORT` は変更不要（E2E はこれらのポートを使わない）。
- **environment**:

  | 変数 | 値 | 用途 |
  |---|---|---|
  | `DATABASE_URL` | `file:./dev.db` | SQLite の DB ファイルパス（`WORK_DIR` 直下。worktree ごとに別ファイルで自然分離。backend がなければ作成し、起動時に `schema.sql` を冪等適用する） |
  | `JWT_SECRET` | `${JWT_SECRET:?…}`（必須・既定値なし） | JWT 署名鍵。リポジトリ直下の `.env` に 32 バイト以上のランダム値（例: `openssl rand -hex 32`）を書く。未設定なら compose 自体がエラーになり、backend も 32 バイト未満なら起動時に終了する（fail-fast） |
  | `ANTHROPIC_API_KEY` | `${ANTHROPIC_API_KEY:-}` | ホストの環境変数から引き渡し（未設定なら空） |

  > リポジトリルートの `.env` は Compose が自動ロードして展開します（`.env.example` をコピーして使う。`.env` は `.gitignore` 済み）。`JWT_SECRET` は既定値を持たないため、初回は `.env` を作ってから `d up` する。値を変えたら `d up` でコンテナを作り直す（既存コンテナの環境変数は更新されない）。`ANTHROPIC_API_KEY` を設定すると `docker inspect` から平文で見える点にも留意。
  >
  > backend は上記のほか `PORT`（既定 3000）・`RATE_LIMIT_DISABLED=true`（レート制限無効化。E2E はこの変数を使わず、ハーネスがアプリへ直接無効化を指定する）・`FRONTEND_DIST`（ビルド済み SPA を同一オリジンで配信する本番用）を読む。いずれも compose では設定しない。

- **tty / stdin_open**: 対話シェル（`d shell`）用。
- ネットワークは明示定義せず Compose のデフォルトネットワークを使用。

`${MOUNT_ROOT}` / `${WORK_DIR}` は `docker-dev.sh`（main）や `docker-worktree.sh`（worktree）が実行時に `export` する変数で、Compose がそれを展開してパスを組み立てます。`${BACKEND_PORT}` / `${FRONTEND_PORT}` / `${CLAUDE_MOUNT_SUFFIX}` は worktree 起動時のみ非デフォルト値が入ります。

### 4.3 `.dockerignore`

ビルドコンテキストから除外するもの: `target/` / `frontend/dist/` / `dist/` / `*.db` / `*.db-journal` / `.env`・`.env.*`（ただし `!.env.example` は含める）/ `coverage/` / `.DS_Store` / `*.log`。

### 4.4 サブコマンド早見表

**`scripts/docker-dev.sh`**（既定コマンドは `shell`）

| コマンド | 実行内容 |
|---|---|
| `build` | `docker compose build` |
| `up` | `docker compose up -d --build`（バックグラウンド起動＋ビルド） |
| `down` | `docker compose down`（named volume は残る） |
| `install` | `cargo fetch --locked` → `lefthook install`（git pre-commit の有効化） |
| `test` | `cargo test --workspace`（backend の単体・HTTP 統合テスト + frontend の純粋関数テスト） |
| `dev` | `cargo run -p meetup-backend` と `trunk serve --address 0.0.0.0`（frontend ディレクトリ）を各々 detach (`-d`) 起動 |
| `e2e` | frontend ディレクトリで `trunk build --dist ../target/e2e-dist` → `cargo test -p meetup-e2e -- --ignored`（追加引数は cargo test の `--` より前に渡る。例: `d e2e --test auth`）。backend はテストごとに同一プロセス・空きポートで起動するため、`d dev` と同時に実行できる |
| `lint` | `cargo clippy -D warnings`（backend は `--all-targets`、frontend は `wasm32-unknown-unknown`）。`d lint backend` / `d lint frontend` で対象を絞れる（省略時は両方） |
| `typecheck` | `cargo check`（backend は `--all-targets`、frontend は `wasm32-unknown-unknown`。対象指定可、省略時は両方） |
| `review` | `cargo fmt --all -- --check` → `lint` → `test` の総合確認 |
| `shell` | devuser で `bash` に入る |
| その他 | `docker compose exec -u devuser dev "$@"` として任意コマンドを実行 |

`lint` / `typecheck` / `review` は第一級サブコマンドです。**`review` は AI・人間ともに実装完了前の確認に使います**。CI の `quality` ジョブは `cargo fmt --check` / `cargo clippy`（native + wasm32）/ `cargo test --workspace --locked` を個別ステップで実行します。

`e2e` / `dev` の frontend 側は `-w "$PROJECT_DIR/frontend"` で作業ディレクトリを切り替えてから実行しています。

**`scripts/docker-worktree.sh`**（worktree ごとに**独立した Compose プロジェクト** `meetup-<slug>` を起動）

| コマンド | 実行内容 |
|---|---|
| `ports` | 割り当てられる PROJECT / PR / BACKEND_PORT / FRONTEND_PORT を表示（起動不要） |
| `up` | 独立コンテナを起動（ポート空き確認あり）。割り当てポートを表示 |
| `build` | このプロジェクトのイメージビルド |
| `down` | このプロジェクトのコンテナ停止・削除（named volume は残る） |
| `clean` | `down -v`。コンテナ + この worktree 専用の volume（cargo target / cargo registry）を削除 |
| `install` | worktree 内で `cargo fetch --locked` → `lefthook install`（worktree ごとに必要） |
| `dev` | `cargo run -p meetup-backend` と `trunk serve` を detach 起動。割り当てポートを表示 |
| `test` | worktree 内で `cargo test --workspace` |
| `e2e` | worktree 内で `d e2e` と同じ処理。ポートを共有しないため main や他の worktree と同時に実行できる |
| `shell` | worktree ディレクトリで `bash` |
| その他 | worktree 名を除いた残り引数をコンテナ内で実行 |

`docker-dev.sh` との違いは、`docker compose -p meetup-<slug>` で**別プロジェクト・別コンテナ・別ポート・別 volume** を立てる点です。`up` / `install` / `dev` / `test` / `e2e` / `shell` / 任意コマンドは対象コンテナが起動済みであること（`up` 済み）を要求し、未起動なら明示エラーになります。

**ポート採番**: PR 番号から slot を `SLOT = PR % 2000` で求め、`backend = 4000 + SLOT` / `frontend = 6000 + SLOT`。PR 番号は `gh pr view <branch>` で自動取得し、取得できない場合はブランチ名のハッシュ由来 slot に fallback します（警告表示）。PR 番号が異なっても slot（mod 2000）が一致すると別 worktree とポートが衝突しうるため、その場合は以下の env で上書きしてください（`up` はポート使用中を検知してエラーにします）。

```bash
PR=42 ./scripts/docker-worktree.sh feat-x up            # PR 番号を明示 → 4042 / 6042
BACKEND_PORT=4100 FRONTEND_PORT=6100 \
  ./scripts/docker-worktree.sh feat-x up               # ポートを直接指定
```

`docker-worktree.sh` も `lint` / `typecheck` / `review` を同じ名前で受け付けます（`./scripts/docker-worktree.sh feature lint`）。

### 4.5 よく使うレシピ

第一級サブコマンドで足りないものは catch-all でそのままコンテナ内に流します。

```bash
d bash -c "cargo fmt --all"                                 # 整形（review は --check のみ）
d bash -c "cargo clippy --workspace --all-targets --fix --allow-dirty"   # clippy の自動修正
d bash -c "cd frontend && trunk build --release"            # frontend の本番ビルド
d bash -c "rm -f dev.db"                                    # 開発 DB を作り直す（次回 backend 起動時に schema.sql を再適用）
```

単一テストの実行（テスト名の部分一致）:

```bash
d bash -c "cargo test -p meetup-backend --test api 同名コミュニティ"     # backend/tests/api.rs
d bash -c "cargo test -p meetup-backend 公開_GET_パス"                  # src 内の単体テスト（名前は describe!/test! の文字列を識別子にしたもの）
d bash -c "cargo test -p meetup-frontend カテゴリ"                      # frontend/src/utils.rs
```

クリーンアップは 3 段階あります。

```bash
d down                   # コンテナ停止・削除（named volume は保持）
docker compose down -v   # コンテナ + named volume を削除（次回 install / ビルドで cargo registry / target を再構築）
docker volume prune      # 未使用の named volume だけ削除
```

## 5. 仕組みの深掘り（なぜこうなっているか）

### 5.1 単一常駐コンテナ（`sleep infinity`）方式

コマンドごとにコンテナを起動・破棄するのではなく、`sleep infinity` で1つのコンテナを立てっぱなしにし、`docker compose exec` で作業を流し込みます。

- 起動オーバーヘッドがなく、`d test` / `d lint` などが即座に走る
- backend（`cargo run`）/ frontend（`trunk serve`）の dev サーバを同じコンテナ内で並行起動できる（`d dev` が両方を detach で立てる）
- コンテナ内に入って（`d shell`）Claude Code を動かすといった長時間セッションにも向く

### 5.2 named volume で `target/` を分離する理由

ソースコードは bind mount ですが、Rust のビルド成果物 `target/` と cargo registry は named volume に分けています。

- bind mount だと、ホスト（macOS）でビルドした `target/` の成果物（Linux 用にビルドされるべきもの）がコンテナに漏れ、実行時エラーになる
- named volume にしておけば、コンテナ内で作った Linux の成果物がホストの macOS のものと混ざらない
- `target/` は小さなファイルが大量に書かれるため、bind mount だと遅い。named volume に置くことでビルド時間を抑える

### 5.3 gosu + UID/GID 合わせで bind mount の権限を解決

- bind mount 上のファイルは、コンテナ内のユーザー UID とホストの UID が食い違うと「所有者が違う」扱いになり、書き込みや git 操作でトラブルが起きる
- そこで `devuser` を **macOS の UID 501 / GID 20** に合わせて作成し、権限のズレをなくしている
- named volume（`target/` / cargo registry）は root 所有で作られるため、`docker-entrypoint.sh` が起動時に `chown devuser` してから `gosu devuser "$@"` で devuser に降格して実際のコマンドを実行する

### 5.4 worktree を独立 Compose プロジェクトで動かす狙い

`${MOUNT_ROOT}`（メインプロジェクトディレクトリ）をホストと同じ絶対パスでマウントしているため、その配下 `.worktrees/feature` に作った worktree の絶対パスと、その `.git`（main の `.git/worktrees/<name>` を参照）がコンテナ内でもそのまま通用します。

その上で `docker-worktree.sh` は worktree ごとに `docker compose -p meetup-<slug>` で**独立したプロジェクト**を立てます。Compose はプロジェクト名でコンテナ・ネットワーク・named volume を名前空間分離するため、main と各 worktree が**別コンテナ・別 volume（cargo target / cargo registry）・別ホストポート**で同時に走ります。コンテナ内のポートは常に 3000 / 5173 のままで、ホスト公開ポートだけを PR 番号ベースでずらすので、アプリ側の設定変更は不要です。E2E はテストごとに空きポートでサーバを立てるため、どのプロジェクトでも同時に実行できます。

## 6. worktree 運用（独立コンテナ・独立ポート）

main とは別のコンテナ・別ポートで起動します。各 worktree で最初に `up` が必要です。

```bash
git worktree add .worktrees/feature -b feature      # worktree 作成
./scripts/docker-worktree.sh feature ports           # 割り当てポート確認（PR 番号ベース）
./scripts/docker-worktree.sh feature up              # 独立コンテナ起動
./scripts/docker-worktree.sh feature install         # 依存取得（cargo fetch。この worktree の volume に）+ lefthook install
./scripts/docker-worktree.sh feature dev             # dev サーバ起動 → 表示された別ポートで確認
./scripts/docker-worktree.sh feature test            # テスト
./scripts/docker-worktree.sh feature e2e             # E2E テスト（main と同時に実行してよい）
./scripts/docker-worktree.sh feature shell           # シェルに入る

./scripts/docker-worktree.sh feature clean           # コンテナ + volume を削除（先に実施）
git worktree remove .worktrees/feature               # 後片付け
git branch -d feature                                # ブランチ削除（マージ済みなら）
```

> `git worktree remove` は Docker のコンテナ・volume を掃除しません。worktree を捨てる前に必ず `clean`（`down -v`）を実行してください。

## 7. トラブルシューティング

| 症状 | 原因 / 対処 |
|---|---|
| `target/` の権限エラー、`EACCES`、`Permission denied` | named volume が root 所有のまま。`d down` → `d up` で再起動し entrypoint の `chown` を通す。UID/GID が 501/20 でない環境では build args `USER_UID` / `USER_GID` を自分の `id -u` / `id -g` に合わせて再ビルド |
| ポート 3000 / 5173 が使えない（`address already in use`） | ホスト側で別プロセスが使用中。既存プロセスを止めるか、`docker-compose.yml` の `ports` を変更（worktree なら `BACKEND_PORT=`/`FRONTEND_PORT=` で上書き） |
| worktree の `up` でポート衝突エラー | 同じ PR 番号 / ハッシュ slot に当たっている。`PR=<別番号>` か `BACKEND_PORT=`/`FRONTEND_PORT=` を指定して再実行 |
| worktree で品質ゲート（lefthook）が「コンテナが起動していません」で失敗 | その worktree の `./scripts/docker-worktree.sh <name> up` を先に実行しておく |
| worktree を削除したのに Docker のコンテナ / volume が残る | `git worktree remove` は Docker を掃除しない。削除前に `./scripts/docker-worktree.sh <name> clean` を実行する |
| `JWT_SECRET を .env に設定してください` で compose が失敗する / backend が `JWT_SECRET ...` で終了する | リポジトリ直下の `.env` に `JWT_SECRET=<openssl rand -hex 32 の出力>` を書き、`d up` でコンテナを作り直す。backend は 32 バイト未満の値でも起動しない |
| DB を作り直したい / スキーマ変更が反映されない | `backend/src/infrastructure/schema.sql` は `CREATE TABLE IF NOT EXISTS` で冪等適用されるだけで、既存テーブルの変更（マイグレーション）は行わない。`d bash -c "rm -f dev.db"` で SQLite ファイルを削除してから backend を再起動する |
| `d e2e` が「SPA が見つからない」/ ブラウザが見つからないで失敗する | SPA は `target/e2e-dist` に置く前提（`E2E_FRONTEND_DIST` で上書き可）。`d e2e` は先に `trunk build` するので、`cargo test -p meetup-e2e` を直接実行したときは `(cd frontend && trunk build --dist ../target/e2e-dist)` を先に通す。ブラウザはイメージの `chromium` を自動検出する。古いイメージなら `d build` → `d up` で作り直す。別のブラウザを使うなら `CHROME=<実行ファイル>` を指定する |
| `d dev` を起動したまま `d e2e` を実行してよいか | 問題ない。E2E は backend をテストごとに同一プロセス・空きポート・インメモリ SQLite で起動し、SPA も `target/e2e-dist` から配信するため、3000 / 5173・`dev.db`・`frontend/dist` のいずれも共有しない |
| dev サーバを立てたのにブラウザから繋がらない | `trunk serve` が `--address 0.0.0.0` で起動しているか（`d dev` は付与済み。`frontend/Trunk.toml` の既定は `127.0.0.1`）、ポート publish（`3000` / `5173`）を確認。初回は wasm ビルド完了まで待つ |
| backend のコード変更が反映されない | `cargo run` は自動再起動しない。backend を止めて `d dev` をやり直す（frontend は `trunk serve` が自動再ビルドする） |
| フルクリーンしたい | `d down` の後 `docker volume prune`（named volume を削除 → 次回 `d install` / ビルドで cargo registry / target を再構築） |

## 関連ドキュメント

- [`docs/decision-logs/arch-20260703-revert-to-docker-as-primary.md`](./decision-logs/arch-20260703-revert-to-docker-as-primary.md) — 正式経路を Docker に差し戻した意思決定記録
- [`docs/decision-logs/arch-20260703-remove-apple-container-fallback.md`](./decision-logs/arch-20260703-remove-apple-container-fallback.md) — Apple container フォールバックを撤去し Docker 単一経路にした意思決定記録
- [`docs/decision-logs/arch-20261127-e2e-in-rust.md`](./decision-logs/arch-20261127-e2e-in-rust.md) — E2E を Playwright（TypeScript）から Rust（chromiumoxide）へ移し、npm を撤去した意思決定記録
- [`AGENTS.md`](../AGENTS.md) — AI・人間共通のプロジェクト規約（実行経路のルールと最頻出コマンドのみ。詳細は本ドキュメント）

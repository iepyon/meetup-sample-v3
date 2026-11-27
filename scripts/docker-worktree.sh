#!/usr/bin/env bash
# Run each git worktree as an INDEPENDENT Docker Compose project with its own
# container, network, node_modules volumes, and host ports — so main and any
# number of worktrees can run their dev servers simultaneously.
# Usage: ./scripts/docker-worktree.sh <worktree-name> [command]
#
# Example:
#   git worktree add .worktrees/feat-x -b feat-x
#   ./scripts/docker-worktree.sh feat-x ports     # 割り当てポート確認
#   ./scripts/docker-worktree.sh feat-x up        # 独立コンテナ起動
#   ./scripts/docker-worktree.sh feat-x install
#   ./scripts/docker-worktree.sh feat-x dev       # 別ポートで dev サーバー起動
#   ./scripts/docker-worktree.sh feat-x clean     # コンテナ + volume 削除
#
# ポート採番: SLOT=PR%2000 として backend=4000+SLOT / frontend=6000+SLOT。
#   PR 番号は gh から自動取得（PR=<n> env で上書き可）。取得不可時はブランチ名の
#   ハッシュ由来 slot に fallback。BACKEND_PORT= / FRONTEND_PORT= で直接上書きも可。
set -euo pipefail

WORKTREE_NAME="${1:?Usage: $0 <worktree-name> [command]}"
COMMAND="${2:-shell}"

# Derive paths dynamically from script location
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# MOUNT_ROOT はメインプロジェクトディレクトリ。worktree は配下にネストするので
# これをホストと同一パスで bind すれば worktree の絶対パスも解決できる。
export MOUNT_ROOT="$PROJECT_DIR"

WORKTREE_DIR="${PROJECT_DIR}/.worktrees/${WORKTREE_NAME}"

if [ ! -d "$WORKTREE_DIR" ]; then
  echo "Error: ${WORKTREE_DIR} が存在しません"
  echo "先に worktree を作成してください: git worktree add .worktrees/${WORKTREE_NAME} -b ${WORKTREE_NAME}"
  exit 1
fi

# Compose プロジェクト名（[a-z0-9_-] にサニタイズ。大文字や '/' は不正なため）
SLUG=$(printf '%s' "$WORKTREE_NAME" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9_-' '-')
PROJECT="meetup-${SLUG}"

# PR 番号: env 上書き > gh 自動取得 > ハッシュ fallback
if [ -z "${PR:-}" ]; then
  BRANCH=$(git -C "$WORKTREE_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
  if [ -n "$BRANCH" ] && command -v gh >/dev/null 2>&1; then
    PR=$(gh pr view "$BRANCH" --json number -q .number 2>/dev/null || echo "")
  fi
fi
if [ -z "${PR:-}" ]; then
  PR=$(( $(printf '%s' "$SLUG" | cksum | awk '{print $1}') % 2000 ))
  echo "warn: PR 番号を取得できないためハッシュ由来 slot=${PR} を使用します（PR=<n> で上書き可）" >&2
fi
SLOT=$(( PR % 2000 ))

export WORK_DIR="$WORKTREE_DIR"
export BACKEND_PORT="${BACKEND_PORT:-$((4000 + SLOT))}"
export FRONTEND_PORT="${FRONTEND_PORT:-$((6000 + SLOT))}"

# WITH_CLAUDE=1 のときだけ ~/.claude マウント override を重ねる（worktree は :ro）
COMPOSE="docker compose -p ${PROJECT} -f ${PROJECT_DIR}/docker-compose.yml"
if [ "${WITH_CLAUDE:-}" = "1" ]; then
  COMPOSE="${COMPOSE} -f ${PROJECT_DIR}/docker-compose.claude.yml"
fi
EXEC="$COMPOSE exec -u devuser -w ${WORKTREE_DIR}"

# ホストポート空き確認（macOS ネイティブの lsof を使用）
port_free() { ! lsof -iTCP:"$1" -sTCP:LISTEN -n -P >/dev/null 2>&1; }

# exec 系コマンドの前提: このプロジェクトのコンテナが起動済みであること
require_running() {
  if [ -z "$($COMPOSE ps -q dev 2>/dev/null)" ]; then
    echo "Error: worktree コンテナ (${PROJECT}) が起動していません。" >&2
    echo "  先に './scripts/docker-worktree.sh ${WORKTREE_NAME} up' を実行してください。" >&2
    exit 1
  fi
}

case "${COMMAND}" in
  up)
    port_free "$BACKEND_PORT"  || { echo "Error: backend port ${BACKEND_PORT} は使用中です。BACKEND_PORT=<port> で上書きしてください" >&2; exit 1; }
    port_free "$FRONTEND_PORT" || { echo "Error: frontend port ${FRONTEND_PORT} は使用中です。FRONTEND_PORT=<port> で上書きしてください" >&2; exit 1; }
    $COMPOSE up -d
    echo "Backend:  http://localhost:${BACKEND_PORT}"
    echo "Frontend: http://localhost:${FRONTEND_PORT}"
    echo "Use './scripts/docker-worktree.sh ${WORKTREE_NAME} install' then 'dev'."
    ;;
  build)
    $COMPOSE build
    ;;
  down)
    $COMPOSE down
    ;;
  clean)
    # コンテナ + このworktree専用の volume（node_modules / cargo target）を削除
    $COMPOSE down -v
    ;;
  ports)
    echo "PROJECT=${PROJECT} PR=${PR} BACKEND_PORT=${BACKEND_PORT} FRONTEND_PORT=${FRONTEND_PORT}"
    ;;
  install)
    require_running
    $EXEC dev cargo fetch --locked
    $EXEC dev npm install
    $EXEC dev bash -c "cd e2e && npx playwright install chromium"
    ;;
  dev)
    require_running
    $COMPOSE exec -d -u devuser -w "${WORKTREE_DIR}" dev cargo run -p meetup-backend
    $COMPOSE exec -d -u devuser -w "${WORKTREE_DIR}/frontend" dev trunk serve --address 0.0.0.0
    echo "Backend:  http://localhost:${BACKEND_PORT}"
    echo "Frontend: http://localhost:${FRONTEND_PORT}"
    ;;
  test)
    require_running
    $EXEC dev cargo test --workspace
    ;;
  lint)
    require_running
    case "${3:-all}" in
      backend)  $EXEC dev cargo clippy -p meetup-backend --all-targets -- -D warnings ;;
      frontend) $EXEC dev cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings ;;
      *)        $EXEC dev cargo clippy --workspace --all-targets -- -D warnings \
                  && $EXEC dev cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings ;;
    esac
    ;;
  typecheck)
    require_running
    case "${3:-all}" in
      backend)  $EXEC dev cargo check -p meetup-backend --all-targets ;;
      frontend) $EXEC dev cargo check -p meetup-frontend --target wasm32-unknown-unknown ;;
      *)        $EXEC dev cargo check -p meetup-backend --all-targets \
                  && $EXEC dev cargo check -p meetup-frontend --target wasm32-unknown-unknown ;;
    esac
    ;;
  review)
    require_running
    $EXEC dev cargo fmt --all -- --check
    $EXEC dev cargo clippy --workspace --all-targets -- -D warnings
    $EXEC dev cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings
    $EXEC dev cargo test --workspace
    ;;
  shell)
    require_running
    $EXEC dev bash
    ;;
  *)
    require_running
    shift  # remove worktree name
    $EXEC dev "$@"
    ;;
esac

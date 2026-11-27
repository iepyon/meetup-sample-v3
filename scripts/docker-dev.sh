#!/usr/bin/env bash
# Docker development environment helper
# Usage: ./scripts/docker-dev.sh [command]
set -euo pipefail

# Derive paths dynamically from script location
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# MOUNT_ROOT はホストと同一パスで bind するメインプロジェクトディレクトリ。
# worktree は ${PROJECT_DIR}/.worktrees/ 配下にネストするため、これだけで足りる。
export MOUNT_ROOT="$PROJECT_DIR"
# WORK_DIR は compose の working_dir / cargo target マウント先。main はプロジェクト直下
export WORK_DIR="$PROJECT_DIR"

# compose ファイルを絶対パスで固定（cwd 非依存。ルートの .env も自動ロードされる）。
# WITH_CLAUDE=1 のときだけ ~/.claude マウント override を重ねる。
# 注: override はコンテナ生成時(up)にのみ反映される。既存コンテナへの exec/shell には
# 効かないため、~/.claude を使うには `WITH_CLAUDE=1 ./scripts/docker-dev.sh up` が必要。
COMPOSE="docker compose -f $PROJECT_DIR/docker-compose.yml"
if [ "${WITH_CLAUDE:-}" = "1" ]; then
  COMPOSE="$COMPOSE -f $PROJECT_DIR/docker-compose.claude.yml"
fi
EXEC="$COMPOSE exec -u devuser"

case "${1:-shell}" in
  build)
    $COMPOSE build
    ;;
  up)
    $COMPOSE up -d --build
    echo "Container running. Use './scripts/docker-dev.sh shell' to enter."
    ;;
  down)
    $COMPOSE down
    ;;
  install)
    # 依存はすべて cargo（npm は使わない）。git pre-commit（lefthook）もここで有効化する
    $EXEC dev cargo fetch --locked
    $EXEC dev lefthook install
    ;;
  test)
    $EXEC dev cargo test --workspace
    ;;
  lint)
    case "${2:-all}" in
      backend)  $EXEC dev cargo clippy -p meetup-backend --all-targets -- -D warnings ;;
      frontend) $EXEC dev cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings ;;
      *)        $EXEC dev cargo clippy --workspace --all-targets -- -D warnings \
                  && $EXEC dev cargo clippy -p meetup-frontend --target wasm32-unknown-unknown -- -D warnings ;;
    esac
    ;;
  typecheck)
    case "${2:-all}" in
      backend)  $EXEC dev cargo check -p meetup-backend --all-targets ;;
      frontend) $EXEC dev cargo check -p meetup-frontend --target wasm32-unknown-unknown ;;
      *)        $EXEC dev cargo check -p meetup-backend --all-targets \
                  && $EXEC dev cargo check -p meetup-frontend --target wasm32-unknown-unknown ;;
    esac
    ;;
  review)
    # フォーマット・lint (警告=エラー)・型・テストの総合確認
    $EXEC dev cargo fmt --all -- --check
    "$0" lint
    "$0" test
    ;;
  dev)
    $EXEC -d dev cargo run -p meetup-backend
    $EXEC -d -w "$PROJECT_DIR/frontend" dev trunk serve --address 0.0.0.0
    echo "Backend: http://localhost:3000"
    echo "Frontend: http://localhost:5173"
    ;;
  e2e)
    # SPA を E2E 専用の出力先にビルドし（d dev の trunk serve が使う frontend/dist とは分ける）、
    # backend を同一プロセス・空きポートで起動する Rust 製 E2E（e2e/）を実行する
    $EXEC -w "$PROJECT_DIR/frontend" dev trunk build --dist ../target/e2e-dist
    $EXEC dev cargo test -p meetup-e2e "${@:2}" -- --ignored
    ;;
  shell)
    $EXEC dev bash
    ;;
  *)
    $COMPOSE exec -u devuser dev "$@"
    ;;
esac

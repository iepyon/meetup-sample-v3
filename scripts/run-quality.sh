#!/bin/sh
# lefthook および AI hook 共通の品質チェックランナー
# Usage: run-quality.sh "cargo clippy --workspace --all-targets -- -D warnings"
# 呼び出し元は信頼済みの lefthook / AI hook のみを想定
#
# 実行経路（Docker 単一経路）:
#   1. コンテナ内（/.dockerenv が存在） → 直接実行
#   2. git worktree → worktree ごとの独立 Compose プロジェクトへ委譲
#   3. メインプロジェクト → docker-dev.sh 経由
CMD="$1"
if [ -z "$CMD" ]; then
  echo "Usage: $0 <command>" >&2
  exit 1
fi

# 1. コンテナ内: 直接実行
# 呼び出し元の cwd に依存しないよう、プロジェクトルートへ移動してから実行する
# （CMD は相対パスを含み得るため）
if [ -f "/.dockerenv" ]; then
  cd "${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "$0")/.." && pwd)}" && bash -c "$CMD"
  exit $?
fi

# プロジェクトディレクトリを特定
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "$0")/.." && pwd)}"

# 2. git worktree 検出
GIT_COMMON_DIR="$(git -C "$PROJECT_DIR" rev-parse --git-common-dir 2>/dev/null)"
GIT_DIR="$(git -C "$PROJECT_DIR" rev-parse --git-dir 2>/dev/null)"

if [ -n "$GIT_COMMON_DIR" ] && [ -n "$GIT_DIR" ] && [ "$GIT_COMMON_DIR" != "$GIT_DIR" ]; then
  # worktree ごとの独立 Compose プロジェクトに委譲する。
  # docker-worktree.sh がプロジェクト名・ポートを解決し、未 up 時は明示エラーを出す。
  MAIN_PROJECT_DIR="$(cd "$GIT_COMMON_DIR/.." && pwd)"
  WORKTREE_NAME="$(basename "$PROJECT_DIR")"
  "$MAIN_PROJECT_DIR/scripts/docker-worktree.sh" "$WORKTREE_NAME" bash -c "$CMD"
  exit $?
fi

# 3. メインプロジェクト: ラッパー経由
SCRIPT="$PROJECT_DIR/scripts/docker-dev.sh"
if [ -x "$SCRIPT" ]; then
  "$SCRIPT" bash -c "$CMD"
  exit $?
fi

echo "Error: docker-dev.sh が見つかりません" >&2
exit 1

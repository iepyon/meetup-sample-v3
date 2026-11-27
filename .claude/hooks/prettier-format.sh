#!/bin/sh
# PostToolUse hook: Edit/Write 後に変更ファイルへ prettier を自動適用
# block はしない（整形を適用するだけ）。lint / 型 / テストは lefthook と CI が担う

INPUT="$(cat)"
# 空入力はハーネス設定ミス (hook が stdin を受け取れていない) の兆候。
# 無音で no-op になると「動いているつもり」バグに気づけないため警告する。
[ -z "$INPUT" ] && echo "prettier-format: フック入力(stdin)が空です。ハーネス設定を確認してください" >&2

FILE="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // empty')"
[ -z "$FILE" ] && exit 0

case "$FILE" in
  *.ts|*.tsx|*.js|*.jsx|*.json|*.css) ;;
  *) exit 0 ;;
esac

PROJECT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
RUN_CMD="$PROJECT_DIR/scripts/run-quality.sh"

# ホストの絶対パス → プロジェクトルートからの相対パスへ変換
REL="$(echo "$FILE" | sed "s|^$PROJECT_DIR/||")"
[ "$REL" = "$FILE" ] && exit 0  # プロジェクト外はスキップ

"$RUN_CMD" "npx prettier --write \"$REL\"" > /dev/null 2>&1

exit 0

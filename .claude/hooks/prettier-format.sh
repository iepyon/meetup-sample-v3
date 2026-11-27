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

# ホストの絶対パス → プロジェクトルートからの相対パスへ変換（行単位の sed は改行入りパスで誤るため使わない）
REL="${FILE#"$PROJECT_DIR"/}"
[ "$REL" = "$FILE" ] && exit 0  # プロジェクト外はスキップ

# ファイル名は外部由来（AI が書いたパス）なので、文字列全体を安全な文字だけに限定する。
# 改行などの制御文字・先頭の -（オプション注入）・プロジェクト外へ出る .. のパス要素を拒否する
case "$REL" in
  -*|*[!A-Za-z0-9._/-]*|..|../*|*/../*|*/..)
    echo "prettier-format: 想定外の文字を含むパスのため整形をスキップしました" >&2
    exit 0
    ;;
esac

# 引数配列のまま渡し、シェルで再評価させない。-- 以降はファイル名としてだけ扱わせる。
# prettier はリポジトリで版を固定したものだけを使う
"$RUN_CMD" npx --no-install prettier --write -- "$REL" > /dev/null 2>&1

exit 0

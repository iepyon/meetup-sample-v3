#!/bin/bash
# PreToolUse hook: npm install の本番依存追加をブロック (slopsquatting 対策)
#
# 判定はコマンド文字列全体への部分一致ではなく、コマンド区切り (; & |) で分割した
# 「セグメント単位」で行う。これにより以下の誤判定を防ぐ:
#   - npm install evil && echo -D  → evil を正しく block（無関係な -D に惑わされない）
#   - npm install foo # コメント     → 行末コメントを除去して判定
#   - bash -c "npm install"         → 引用符を除去し bare install を許可
#   - npm  install foo (二重空白)   → npm とサブコマンド間の空白を可変に
INPUT="$(cat)"
# 空入力はハーネス設定ミス (hook が stdin を受け取れていない) の兆候。
# 無音で no-op になると「動いているつもり」バグに気づけないため警告する。
[ -z "$INPUT" ] && echo "dep-install-guard: フック入力(stdin)が空です。ハーネス設定を確認してください" >&2

COMMAND="$(printf '%s' "$INPUT" | jq -r '.tool_input.command // empty')"

[ -z "$COMMAND" ] && exit 0

OFFENDING=""

# コマンド区切り (; & |) を改行に変換してセグメント化する。
# && / || は & / | 各 1 文字が改行になるだけで分割目的には十分。
# COMMAND に元から含まれる改行も区切りとして扱われる（複数行コマンド対策）。
SEGMENTS="$(printf '%s' "$COMMAND" | tr ';&|' '\n\n\n')"

OLD_IFS="$IFS"
IFS='
'
for SEG in $SEGMENTS; do
  # 行末コメント ( #... ) を除去
  SEG="$(printf '%s' "$SEG" | sed -E 's/[[:space:]]#.*$//')"

  # npm install / npm add / npm i のセグメントか（npm は語頭境界、npm ci は非該当）
  echo "$SEG" | grep -qE '(^|[[:space:]])npm[[:space:]]+(install|add|i)([[:space:]]|$)' || continue

  # サブコマンドより後ろの引数部分を取り出す
  ARGS="$(printf '%s' "$SEG" | sed -E 's/^.*npm[[:space:]]+(install|add|i)[[:space:]]*//')"

  # 引数なし (bare install) は許可
  [ -z "$ARGS" ] && continue

  # --save-dev / -D があれば許可（-Dfoo のような密着形は許可しない）
  echo "$ARGS" | grep -qE '(^|[[:space:]])(--save-dev|-D)([[:space:]]|$)' && continue

  # 引用符を除去してトークン化し、実パッケージ名を収集する。
  # リダイレクト演算子 (>, <, 2>, 1> など) が現れたら、それ以降はコマンド引数
  # ではない (リダイレクト先ファイル名を含む) ため打ち切る。これにより
  # `npm install 2>&1`（& が区切りで消えて 2> が残る）や `npm install > log`
  # を誤ってパッケージ追加と判定しない。フラグ (-...) はパッケージではない。
  PKGS=""
  CLEAN="$(printf '%s' "$ARGS" | tr -d "\"'")"
  SAVED_IFS="$IFS"
  IFS=$' \t\n'
  for TOK in $CLEAN; do
    case "$TOK" in
      [0-9]'>'* | [0-9]'<'* | '>'* | '<'*) break ;;
      -* | '') continue ;;
    esac
    PKGS="$PKGS $TOK"
  done
  IFS="$SAVED_IFS"
  PKGS="$(printf '%s' "$PKGS" | sed -E 's/^[[:space:]]+//')"

  # フラグのみ（実パッケージなし）は許可（例: npm install --force）
  [ -z "$PKGS" ] && continue

  OFFENDING="$OFFENDING $PKGS"
done
IFS="$OLD_IFS"

OFFENDING="$(printf '%s' "$OFFENDING" | sed -E 's/^[[:space:]]+//')"
[ -z "$OFFENDING" ] && exit 0

MSG="$(printf '本番依存の追加を検出: %s\n--save-dev (-D) を付けるか、本当に本番依存が必要か確認してください。' "$OFFENDING")"
jq -nc --arg r "$MSG" '{decision:"block",reason:$r}'
exit 0

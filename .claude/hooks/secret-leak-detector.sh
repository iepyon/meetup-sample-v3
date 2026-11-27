#!/bin/bash
# PostToolUse hook: Bash 出力に機密情報パターンが含まれていないかチェック
INPUT="$(cat)"
# 空入力はハーネス設定ミス (hook が stdin を受け取れていない) の兆候。
# 無音で no-op になると「動いているつもり」バグに気づけないため警告する。
[ -z "$INPUT" ] && echo "secret-leak-detector: フック入力(stdin)が空です。ハーネス設定を確認してください" >&2

# Bash tool の tool_response は {stdout, stderr, ...} オブジェクト。
# 他ツールでは文字列のこともあるため型で分岐する。
OUTPUT="$(printf '%s' "$INPUT" | jq -r '
  if (.tool_response | type) == "object"
  then ((.tool_response.stdout // "" | tostring) + "\n" + (.tool_response.stderr // "" | tostring))
  elif (.tool_response | type) == "null" then empty
  else (.tool_response | tostring) end')"

[ -z "$OUTPUT" ] && exit 0

WARNINGS=""

# JWT トークン (eyJ で始まる Base64)
if echo "$OUTPUT" | grep -qE 'eyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}'; then
  WARNINGS="$WARNINGS\n- JWT トークンが出力に含まれています"
fi

# AWS アクセスキー
if echo "$OUTPUT" | grep -qE 'AKIA[0-9A-Z]{16}'; then
  WARNINGS="$WARNINGS\n- AWS アクセスキーが出力に含まれています"
fi

# 汎用 API キー/シークレットキーパターン
if echo "$OUTPUT" | grep -qE '(sk-[a-zA-Z0-9]{20,}|sk_live_[a-zA-Z0-9]+)'; then
  WARNINGS="$WARNINGS\n- API シークレットキーが出力に含まれています"
fi

# パスワード系 (key=value 形式)
# 値は「秘密情報らしい ASCII 文字列」に限定する。`[^[:space:]]{8,}` だと日本語は
# 語間に空白がないため散文が丸ごと値として一致し、この hook 自身の仕様を説明した
# 文（docs/wiki/infra/ai-hooks.md の `password=` を含む表など）を読むだけで誤 block する。
# 誤 block は「/clear を検討」という指示を AI に届けるため実害が大きい。
if echo "$OUTPUT" | grep -iqE '(password|passwd|secret|token)[[:space:]]*[=:][[:space:]]*[A-Za-z0-9+/_=.@-]{8,}'; then
  WARNINGS="$WARNINGS\n- パスワード/シークレットが key=value 形式で出力に含まれています"
fi

if [ -n "$WARNINGS" ]; then
  REASON="$(printf '機密情報リークの可能性:%b\nコンテキストに機密データが残っています。/clear の実行を検討してください。' "$WARNINGS")"
  jq -nc --arg r "$REASON" '{decision:"block",reason:$r}'
  exit 0
fi

exit 0

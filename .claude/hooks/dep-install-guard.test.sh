#!/bin/bash
# dep-install-guard.sh の回帰テスト
# 実行: bash .claude/hooks/dep-install-guard.test.sh
#
# hook は stdin で {"tool_input":{"command":"..."}} を受け取り、
# block 時のみ stdout に {"decision":"block",...} を出す。
# コマンドが BLOCK / ALLOW どちらになるかを検証する。

HOOK="$(dirname "$0")/dep-install-guard.sh"
pass=0
fail=0

# コマンド文字列を hook に渡し、block されたら "BLOCK"、されなければ "ALLOW"
run() {
  cmd="$1"
  json="$(printf '%s' "$cmd" | jq -R -s '{tool_input:{command:.}}')"
  out="$(printf '%s' "$json" | bash "$HOOK")"
  if printf '%s' "$out" | grep -q '"decision":"block"'; then
    echo "BLOCK"
  else
    echo "ALLOW"
  fi
}

assert() {
  desc="$1"
  expected="$2"
  cmd="$3"
  actual="$(run "$cmd")"
  if [ "$actual" = "$expected" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL: $desc"
    echo "  command : $cmd"
    echo "  expected: $expected / actual: $actual"
  fi
}

# --- 許可されるべき (ALLOW): 本番依存の新規追加ではない ---
assert "引数なし install" ALLOW "npm install"
assert "リダイレクト付き bare install" ALLOW "npm install 2>&1 | tail -30"
assert "ラッパー経由 bare install + redirect" ALLOW "./scripts/docker-dev.sh npm install 2>&1 | tail -30"
assert "出力リダイレクト" ALLOW "npm install > out.log"
assert "stderr リダイレクト単体" ALLOW "npm install 2> err.log"
assert "dev依存 --save-dev" ALLOW "npm install --save-dev foo"
assert "dev依存 -D" ALLOW "npm install -D foo"
assert "npm ci は対象外" ALLOW "npm ci"
assert "フラグのみ" ALLOW "npm install --force"
assert "引用符付き bare" ALLOW 'bash -c "npm install"'
assert "行末コメント付き bare" ALLOW "npm install # 依存を同期"

# --- block されるべき (BLOCK): 本番依存の新規追加 ---
assert "本番依存追加" BLOCK "npm install lodash"
assert "npm add" BLOCK "npm add lodash"
assert "npm i" BLOCK "npm i express"
assert "複数パッケージ" BLOCK "npm install foo bar"
assert "区切りの後ろの -D に惑わされない" BLOCK "npm install evil && echo -D"
assert "リダイレクト付きでも本番依存は検出" BLOCK "npm install foo 2>&1"
assert "リダイレクト先より前の依存を検出" BLOCK "npm install foo > out.log"

echo "----"
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ]

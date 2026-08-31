#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR

if [[ $# -ne 2 ]]; then
  printf 'usage: %s LYNX_CSSC DOM_UNITTEST_EXEC\n' "$0" >&2
  exit 2
fi

cssc="$1"
native_test="$2"
[[ -x "$cssc" && -x "$native_test" ]] || {
  printf 'test-lynx-cssc: expected two executable paths\n' >&2
  exit 2
}

temp_dir="$(mktemp -d)"
trap 'rm -rf -- "$temp_dir"' EXIT

rules="$temp_dir/rules.json"
payload="$temp_dir/style.lynxcss"
bad_json="$temp_dir/bad.json"
bad_rule="$temp_dir/bad-rule.json"
stylist_css="$temp_dir/stylist.css"
stylist_rules="$temp_dir/stylist-rules.json"
stylist_payload="$temp_dir/stylist.lynxcss"

cat >"$rules" <<'EOF'
[{"type":"StyleRule","selectorText":{"value":".compiled","loc":{"line":1,"column":1}},"style":[{"name":"width","value":"123px","keyLoc":{"line":1,"column":13}}],"variables":{}}]
EOF
printf '{' >"$bad_json"
printf 'width: 123px;\n' >"$stylist_css"

"$cssc" --input "$rules" --output "$payload"
[[ -s "$payload" ]] || {
  printf 'test-lynx-cssc: compiler produced an empty payload\n' >&2
  exit 1
}

if "$cssc" --input "$bad_json" --output "$temp_dir/bad.lynxcss"; then
  printf 'test-lynx-cssc: malformed JSON unexpectedly succeeded\n' >&2
  exit 1
fi
bad_rules=(
  '[]'
  '[1]'
  '[null]'
  '[{}]'
  '[{"type":"StyleRule","style":[],"variables":{}}]'
  '[{"type":"StyleRule","selectorText":{"value":".compiled"},"variables":{}}]'
  '[{"type":"StyleRule","selectorText":{"value":".compiled"},"style":[]}]'
  '[{"type":"StyleRule","selectorText":".compiled","style":[],"variables":{}}]'
  '[{"type":"StyleRule","selectorText":{"value":".compiled"},"style":[{}],"variables":{}}]'
  '[{"type":"StyleRule","selectorText":{"value":".compiled"},"style":[],"variables":{"--x":1}}]'
  '[{"type":"UnknownRule"}]'
)
for i in "${!bad_rules[@]}"; do
  printf '%s' "${bad_rules[$i]}" >"$bad_rule"
  bad_output="$temp_dir/bad-rule-$i.lynxcss"
  if "$cssc" --input "$bad_rule" --output "$bad_output"; then
    printf 'test-lynx-cssc: invalid ruleList case %s unexpectedly succeeded\n' "$i" >&2
    exit 1
  fi
  [[ ! -e "$bad_output" ]] || {
    printf 'test-lynx-cssc: invalid ruleList case %s produced output\n' "$i" >&2
    exit 1
  }
done
if "$cssc" --input "$rules" --output "$temp_dir/extra.lynxcss" extra; then
  printf 'test-lynx-cssc: positional argument unexpectedly succeeded\n' >&2
  exit 1
fi

LYNX_CSSC_TEST_FRAGMENT="$payload" \
  "$native_test" \
  --gtest_filter=NativeRendererApiTest.ImportsCompiledStyleSheetThroughFunctionTable

cargo run --locked --quiet --manifest-path "$ROOT_DIR/Cargo.toml" \
  -p stylist-lynx-cssc -- \
  --input "$stylist_css" --class compiled --output "$stylist_rules"
"$cssc" --input "$stylist_rules" --output "$stylist_payload"
LYNX_CSSC_TEST_FRAGMENT="$stylist_payload" \
  "$native_test" \
  --gtest_filter=NativeRendererApiTest.ImportsCompiledStyleSheetThroughFunctionTable

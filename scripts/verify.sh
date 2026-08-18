#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly YEW_SOURCE_DIR="$ROOT_DIR/.deps/yew"
readonly SCRIPTS=(
  "$ROOT_DIR/scripts/bootstrap-yew.sh"
  "$ROOT_DIR/scripts/verify.sh"
)

temp_dir=""

cleanup() {
  if [[ -n "$temp_dir" && -d "$temp_dir" ]]; then
    rm -rf -- "$temp_dir"
  fi
}

verify_yew_clean() {
  local checkout_status

  checkout_status="$(git -C "$YEW_SOURCE_DIR" status --porcelain=v1 --untracked-files=all)"
  if [[ -n "$checkout_status" ]]; then
    printf 'verify: patched Yew checkout is dirty:\n%s\n' "$checkout_status" >&2
    return 1
  fi
}

trap cleanup EXIT

printf '==> Checking shell scripts\n'
bash -n "${SCRIPTS[@]}"
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck "${SCRIPTS[@]}"
else
  printf 'shellcheck not found; skipping optional lint\n'
fi

printf '==> Bootstrapping pinned Yew checkout\n'
"$ROOT_DIR/scripts/bootstrap-yew.sh"
verify_yew_clean

printf '==> Checking project formatting\n'
cargo fmt --manifest-path "$ROOT_DIR/Cargo.toml" \
  --package yew-lynx-counter -- --check

printf '==> Checking project workspace\n'
cargo check --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --all-targets --locked

printf '==> Testing project workspace\n'
cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --locked

printf '==> Preparing isolated patched Yew test checkout\n'
temp_dir="$(mktemp -d "$ROOT_DIR/.deps/.yew-verify.XXXXXX")"
git clone --quiet --shared --no-checkout "$YEW_SOURCE_DIR" "$temp_dir/yew"
git -C "$temp_dir/yew" checkout --quiet --detach \
  "$(git -C "$YEW_SOURCE_DIR" rev-parse HEAD)"
YEW_TEST_DIR="$temp_dir/yew"

printf '==> Checking patched Yew clay feature\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo check --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew --features clay

printf '==> Testing patched Yew Clay renderer\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo test --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew --lib --features clay clay_renderer::tests

printf '==> Testing Yew macros with clay enabled\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo test --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew-macro --features clay --test html_macro_test html_macro -- --exact

printf '==> Testing Yew macros without clay\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo test --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew-macro --test html_macro_test html_macro -- --exact

printf '==> Confirming patched Yew checkout remained clean\n'
verify_yew_clean

printf '==> Verification complete\n'

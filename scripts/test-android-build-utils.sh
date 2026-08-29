#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=android-build-utils.sh
source "$ROOT_DIR/scripts/android-build-utils.sh"

temp_dir="$(mktemp -d)"
trap 'rm -rf -- "$temp_dir"' EXIT
ndk_dir="$temp_dir/ndk"
prebuilt_dir="$ndk_dir/toolchains/llvm/prebuilt"
mkdir -p -- \
  "$prebuilt_dir/darwin-x86_64" \
  "$prebuilt_dir/darwin-arm64" \
  "$prebuilt_dir/linux-x86_64"

assert_selected() {
  local expected="$1"
  local actual
  actual="$(resolve_android_ndk_prebuilt_dir "$ndk_dir")"
  [[ "$actual" == "$prebuilt_dir/$expected" ]] || {
    printf 'Expected %s, got %s\n' "$prebuilt_dir/$expected" "$actual" >&2
    exit 1
  }
}

ANDROID_NDK_HOST_TAG=darwin-x86_64 assert_selected darwin-x86_64
ANDROID_NDK_HOST_TAG=darwin-arm64 assert_selected darwin-arm64

FAKE_UNAME_S=Darwin
FAKE_UNAME_M=arm64
uname() {
  case "$1" in
    -s) printf '%s\n' "$FAKE_UNAME_S" ;;
    -m) printf '%s\n' "$FAKE_UNAME_M" ;;
  esac
}

unset ANDROID_NDK_HOST_TAG
assert_selected darwin-arm64
rm -rf -- "$prebuilt_dir/darwin-arm64"
assert_selected darwin-x86_64

FAKE_UNAME_S=Linux
assert_selected linux-x86_64

ANDROID_NDK_HOST_TAG=darwin-arm64
if error="$(resolve_android_ndk_prebuilt_dir "$ndk_dir" 2>&1)"; then
  printf 'Missing override unexpectedly resolved\n' >&2
  exit 1
fi
[[ "$error" == *"$prebuilt_dir/darwin-arm64"* ]] || {
  printf 'Failure did not list attempted path: %s\n' "$error" >&2
  exit 1
}

unset ANDROID_NDK_HOST_TAG
FAKE_UNAME_S=Darwin
rm -rf -- "$prebuilt_dir/darwin-x86_64"
if error="$(resolve_android_ndk_prebuilt_dir "$ndk_dir" 2>&1)"; then
  printf 'Missing Darwin prebuilts unexpectedly resolved\n' >&2
  exit 1
fi
for host_tag in darwin-arm64 darwin-x86_64; do
  [[ "$error" == *"$prebuilt_dir/$host_tag"* ]] || {
    printf 'Failure did not list attempted path for %s: %s\n' "$host_tag" "$error" >&2
    exit 1
  }
done

if shasum_path="$(command -v shasum 2>/dev/null)"; then
  fake_bin="$temp_dir/bin"
  mkdir -p -- "$fake_bin"
  {
    printf '#!%s\n' "$BASH"
    printf 'exec %q "$@"\n' "$shasum_path"
  } >"$fake_bin/shasum"
  chmod +x "$fake_bin/shasum"
  # shellcheck disable=SC2016 # $1 must expand in the child Bash process.
  fallback_checksum="$(
    PATH="$fake_bin" "$BASH" -c \
      'source "$1"; printf abc | sha256_checksum' \
      bash "$ROOT_DIR/scripts/android-build-utils.sh"
  )"
  [[ "${fallback_checksum%% *}" == \
      ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad ]] || {
    printf 'shasum fallback produced an unexpected digest: %s\n' "$fallback_checksum" >&2
    exit 1
  }
fi

printf 'Android build utility tests passed\n'

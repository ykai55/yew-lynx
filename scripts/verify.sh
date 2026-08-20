#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly YEW_SOURCE_DIR="$ROOT_DIR/.deps/yew"
readonly MTS_ADAPTER_DIR="$ROOT_DIR/adapters/mts"
readonly LYNX_SHA="0df14207cebb060f1bed8de12b64a1119dee8f06"
readonly LYNX_PATCH_DIR="$ROOT_DIR/patches/lynx"
readonly SCRIPTS=(
  "$ROOT_DIR/adapters/android/test/run-mock-checks.sh"
  "$ROOT_DIR/scripts/bootstrap-yew.sh"
  "$ROOT_DIR/scripts/build-android.sh"
  "$ROOT_DIR/scripts/prepare-flatc.sh"
  "$ROOT_DIR/scripts/prepare-hab.sh"
  "$ROOT_DIR/scripts/prepare-primjs.sh"
  "$ROOT_DIR/scripts/publish-lynx-maven.sh"
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

verify_android_metadata() {
  local actual_lynx_sha
  local gitlink
  local lynx_url
  local tracked_lynx_changes

  gitlink="$(git -C "$ROOT_DIR" ls-files --stage -- third_party/lynx)"
  [[ "$gitlink" == "160000 $LYNX_SHA 0"$'\t'"third_party/lynx" ]] || {
    printf 'verify: Lynx gitlink does not match %s\n' "$LYNX_SHA" >&2
    return 1
  }
  lynx_url="$(git config -f "$ROOT_DIR/.gitmodules" --get submodule.third_party/lynx.url)"
  [[ "$lynx_url" == "https://github.com/lynx-family/lynx.git" ]] || {
    printf 'verify: unexpected Lynx submodule URL: %s\n' "$lynx_url" >&2
    return 1
  }
  if [[ -d "$ROOT_DIR/third_party/lynx/.git" || -f "$ROOT_DIR/third_party/lynx/.git" ]]; then
    actual_lynx_sha="$(git -C "$ROOT_DIR/third_party/lynx" rev-parse HEAD)"
    [[ "$actual_lynx_sha" == "$LYNX_SHA" ]] || {
      printf 'verify: checked-out Lynx revision is %s\n' "$actual_lynx_sha" >&2
      return 1
    }
    tracked_lynx_changes="$(
      git -C "$ROOT_DIR/third_party/lynx" status --porcelain=v1 --untracked-files=no
    )"
    [[ -z "$tracked_lynx_changes" ]] || {
      printf 'verify: pinned Lynx submodule has tracked changes:\n%s\n' \
        "$tracked_lynx_changes" >&2
      return 1
    }
  fi
  for metadata in \
    "$ROOT_DIR/android/hab.lock" \
    "$ROOT_DIR/android/primjs.lock" \
    "$ROOT_DIR/examples/android/app/gradle.lockfile" \
    "$ROOT_DIR/examples/android/gradle/verification-metadata.xml"; do
    [[ -s "$metadata" ]] || {
      printf 'verify: missing Android lock metadata: %s\n' "$metadata" >&2
      return 1
    }
  done
  bash -c 'set -u; source "$1"; [[ "$HABITAT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ && "$HABITAT_SHA256" =~ ^[0-9a-f]{64}$ ]]' \
    bash "$ROOT_DIR/android/hab.lock"
  bash -c 'set -u; source "$1"; [[ "$PRIMJS_UNIQUE_VERSION" != *SNAPSHOT && "$PRIMJS_WASM_UNIQUE_VERSION" != *SNAPSHOT && "$PRIMJS_AAR_SHA256" =~ ^[0-9a-f]{64}$ && "$PRIMJS_POM_SHA256" =~ ^[0-9a-f]{64}$ && "$PRIMJS_WASM_AAR_SHA256" =~ ^[0-9a-f]{64}$ && "$PRIMJS_WASM_POM_SHA256" =~ ^[0-9a-f]{64}$ ]]' \
    bash "$ROOT_DIR/android/primjs.lock"
  grep -q '^org\.lynxsdk\.lynx:lynx:0\.0\.1-0df14207=' \
    "$ROOT_DIR/examples/android/app/gradle.lockfile"
  grep -q '<component group="com.android.tools.build" name="gradle" version="7.4.2">' \
    "$ROOT_DIR/examples/android/gradle/verification-metadata.xml"
}

verify_lynx_patches() {
  local patch_file
  local patch_name
  local -a patch_files=()

  [[ -s "$LYNX_PATCH_DIR/series" ]] || {
    printf 'verify: missing Lynx patch series\n' >&2
    return 1
  }
  while IFS= read -r patch_name || [[ -n "$patch_name" ]]; do
    patch_name="${patch_name%$'\r'}"
    case "$patch_name" in
      '' | \#*) continue ;;
    esac
    case "/$patch_name/" in
      */../* | */./*) return 1 ;;
    esac
    [[ "$patch_name" != /* ]] || return 1
    patch_file="$LYNX_PATCH_DIR/$patch_name"
    [[ -f "$patch_file" ]] || return 1
    git patch-id --stable < "$patch_file" | grep -Eq '^[0-9a-f]{40} [0-9a-f]{40}$'
    patch_files+=("$patch_file")
  done < "$LYNX_PATCH_DIR/series"
  ((${#patch_files[@]} > 0)) || return 1
  git -C "$ROOT_DIR/third_party/lynx" apply --check "${patch_files[@]}"
}

trap cleanup EXIT

printf '==> Checking shell scripts\n'
bash -n "${SCRIPTS[@]}"
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck -x -P SCRIPTDIR "${SCRIPTS[@]}"
else
  printf 'shellcheck not found; skipping optional lint\n'
fi

printf '==> Checking Android pins and lock metadata\n'
verify_android_metadata
printf '==> Checking pinned Lynx patch series\n'
verify_lynx_patches
python3 -c 'import pathlib, sys; compile(pathlib.Path(sys.argv[1]).read_bytes(), sys.argv[1], "exec")' \
  "$ROOT_DIR/scripts/android-device-acceptance.py"

printf '==> Regenerating protocol with locked flatc\n'
flatc="$("$ROOT_DIR/scripts/prepare-flatc.sh")"
FLATC="$flatc" node "$ROOT_DIR/scripts/generate-protocol.mjs"
git -C "$ROOT_DIR" diff --exit-code -- \
  protocol/schema \
  protocol/capabilities \
  protocol/generated

printf '==> Bootstrapping pinned Yew checkout\n'
"$ROOT_DIR/scripts/bootstrap-yew.sh"
verify_yew_clean

printf '==> Checking project formatting\n'
cargo fmt --manifest-path "$ROOT_DIR/Cargo.toml" --all -- --check

printf '==> Checking project workspace\n'
cargo check --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --all-targets --locked

printf '==> Testing project workspace\n'
cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --all-targets --locked

printf '==> Linting project workspace\n'
cargo clippy --manifest-path "$ROOT_DIR/Cargo.toml" \
  --workspace --all-targets --locked -- -D warnings

printf '==> Building and testing ordinary LepusNG/MTS template\n'
npm --prefix "$MTS_ADAPTER_DIR" ci
npm --prefix "$MTS_ADAPTER_DIR" run build
npm --prefix "$MTS_ADAPTER_DIR" run build:wasm
npm --prefix "$MTS_ADAPTER_DIR" test

printf '==> Testing Android Java/JNI adapter\n'
bash "$ROOT_DIR/adapters/android/test/run-mock-checks.sh"

printf '==> Building Android arm64 Rust static library\n'
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --locked --release \
  --target aarch64-linux-android --package yew-lynx-counter

printf '==> Preparing isolated patched Yew test checkout\n'
temp_dir="$(mktemp -d "$ROOT_DIR/.deps/.yew-verify.XXXXXX")"
git clone --quiet --shared --no-checkout "$YEW_SOURCE_DIR" "$temp_dir/yew"
git -C "$temp_dir/yew" checkout --quiet --detach \
  "$(git -C "$YEW_SOURCE_DIR" rev-parse HEAD)"
YEW_TEST_DIR="$temp_dir/yew"

printf '==> Checking patched Yew native_renderer feature\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo check --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew --features native_renderer

printf '==> Testing patched Yew native renderer\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo test --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew --lib --features native_renderer native_renderer::tests

printf '==> Testing Yew macros with native_renderer enabled\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo test --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew-macro --features native_renderer --test html_macro_test html_macro -- --exact

printf '==> Testing Yew macros without native_renderer\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo test --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew-macro --test html_macro_test html_macro -- --exact

printf '==> Confirming patched Yew checkout remained clean\n'
verify_yew_clean

printf '==> Verification complete\n'

#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
# shellcheck source=android-build-utils.sh
source "$ROOT_DIR/scripts/android-build-utils.sh"
readonly YEW_SOURCE_DIR="$ROOT_DIR/.deps/yew"
readonly LYNX_SHA="0df14207cebb060f1bed8de12b64a1119dee8f06"
readonly LYNX_PATCH_DIR="$ROOT_DIR/patches/lynx"
readonly LYNX_TOOLS_SHARED_SHA="ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea"
readonly LYNX_TOOLS_SHARED_PATCH_DIR="$ROOT_DIR/patches/lynx-tools-shared"
readonly WAMR_SHA="25bd7eb63e828e4bd242cc9b38d260b4b31c6605"
readonly SCRIPTS=(
  "$ROOT_DIR/adapters/android/test/run-mock-checks.sh"
  "$ROOT_DIR/scripts/android-build-utils.sh"
  "$ROOT_DIR/scripts/bootstrap-yew.sh"
  "$ROOT_DIR/scripts/build-android.sh"
  "$ROOT_DIR/scripts/prepare-hab.sh"
  "$ROOT_DIR/scripts/prepare-primjs.sh"
  "$ROOT_DIR/scripts/publish-lynx-maven.sh"
  "$ROOT_DIR/scripts/test-android-build-utils.sh"
  "$ROOT_DIR/scripts/verify.sh"
)

temp_dir=""
tools_shared_temp_dir=""

mkdir -p -- "$ROOT_DIR/.deps"

cleanup() {
  if [[ -n "$temp_dir" && -d "$temp_dir" ]]; then
    rm -rf -- "$temp_dir"
  fi
  if [[ -n "$tools_shared_temp_dir" && -d "$tools_shared_temp_dir" ]]; then
    rm -rf -- "$tools_shared_temp_dir"
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
  local wamr_gitlink
  local wamr_url

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
  wamr_gitlink="$(git -C "$ROOT_DIR" ls-files --stage -- third_party/wasm-micro-runtime)"
  [[ "$wamr_gitlink" == "160000 $WAMR_SHA 0"$'\t'"third_party/wasm-micro-runtime" ]] || {
    printf 'verify: WAMR gitlink does not match %s\n' "$WAMR_SHA" >&2
    return 1
  }
  wamr_url="$(git config -f "$ROOT_DIR/.gitmodules" --get submodule.third_party/wasm-micro-runtime.url)"
  [[ "$wamr_url" == "https://github.com/bytecodealliance/wasm-micro-runtime.git" ]] || {
    printf 'verify: unexpected WAMR submodule URL: %s\n' "$wamr_url" >&2
    return 1
  }
  if [[ -d "$ROOT_DIR/third_party/wasm-micro-runtime/.git" \
      || -f "$ROOT_DIR/third_party/wasm-micro-runtime/.git" ]]; then
    [[ "$(git -C "$ROOT_DIR/third_party/wasm-micro-runtime" rev-parse HEAD)" == "$WAMR_SHA" ]] || {
      printf 'verify: checked-out WAMR revision does not match %s\n' "$WAMR_SHA" >&2
      return 1
    }
  fi
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
  grep -q '^org\.lynxsdk\.lynx:lynx-native-renderer:0\.0\.1-0df14207=' \
    "$ROOT_DIR/examples/android/app/gradle.lockfile"
  if grep -Eq '^org\.lynxsdk\.lynx:(lynx|lynx-jssdk|primjs|primjsWasm):' \
      "$ROOT_DIR/examples/android/app/gradle.lockfile"; then
    printf 'verify: Android app lock contains a forbidden stock runtime dependency\n' >&2
    return 1
  fi
  grep -q '<trust group="org.lynxsdk.lynx" name="lynx-native-renderer"' \
    "$ROOT_DIR/examples/android/gradle/verification-metadata.xml"
  grep -q '<component group="com.android.tools.build" name="gradle" version="7.4.2">' \
    "$ROOT_DIR/examples/android/gradle/verification-metadata.xml"
  grep -q '<artifact name="aapt2-7.4.2-8841542-osx.jar">' \
    "$ROOT_DIR/examples/android/gradle/verification-metadata.xml"
  grep -q '1a69bd767bb6f8e71ca9faac52229e6a773814d36493b9423b4600a310028e5d' \
    "$ROOT_DIR/examples/android/gradle/verification-metadata.xml"
}

verify_lynx_tools_shared_patches() {
  local apply_status=0
  local checkout_status
  local i
  local patch_file
  local patch_name
  local tools_shared_dir="$ROOT_DIR/third_party/lynx/tools_shared"
  local -a applied_patch_files=()
  local -a patch_files=()

  if ! grep -q "commit.*$LYNX_TOOLS_SHARED_SHA" \
      "$ROOT_DIR/third_party/lynx/dependencies/DEPS"; then
    grep -q "^+.*commit.*$LYNX_TOOLS_SHARED_SHA" \
      "$LYNX_PATCH_DIR/0016-Pin-public-tools-shared-revision.patch"
  fi
  if [[ -d "$tools_shared_dir/.git" || -f "$tools_shared_dir/.git" ]]; then
    checkout_status="$(
      git -C "$tools_shared_dir" status --porcelain=v1 --untracked-files=all
    )"
    [[ -z "$checkout_status" ]] || {
      printf 'verify: Lynx tools_shared checkout has tracked or non-ignored untracked changes:\n%s\n' \
        "$checkout_status" >&2
      return 1
    }
  fi
  if [[ ! -d "$tools_shared_dir/.git" && ! -f "$tools_shared_dir/.git" ]] \
      || [[ "$(git -C "$tools_shared_dir" rev-parse HEAD)" != "$LYNX_TOOLS_SHARED_SHA" ]]; then
    tools_shared_temp_dir="$(mktemp -d "$ROOT_DIR/.deps/.tools-shared-verify.XXXXXX")"
    tools_shared_dir="$tools_shared_temp_dir/tools_shared"
    git clone --quiet --no-checkout \
      https://github.com/lynx-family/tools-shared.git "$tools_shared_dir"
    git -C "$tools_shared_dir" checkout --quiet --detach "$LYNX_TOOLS_SHARED_SHA"
  fi
  [[ "$(git -C "$tools_shared_dir" rev-parse HEAD)" == "$LYNX_TOOLS_SHARED_SHA" ]] || {
    printf 'verify: Lynx tools_shared checkout is not at %s\n' \
      "$LYNX_TOOLS_SHARED_SHA" >&2
    return 1
  }
  checkout_status="$(git -C "$tools_shared_dir" status --porcelain=v1 --untracked-files=all)"
  [[ -z "$checkout_status" ]] || {
    printf 'verify: Lynx tools_shared checkout has tracked or non-ignored untracked changes:\n%s\n' \
      "$checkout_status" >&2
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
    patch_file="$LYNX_TOOLS_SHARED_PATCH_DIR/$patch_name"
    [[ -f "$patch_file" ]] || return 1
    git patch-id --stable < "$patch_file" | grep -Eq '^[0-9a-f]{40} [0-9a-f]{40}$'
    patch_files+=("$patch_file")
  done < "$LYNX_TOOLS_SHARED_PATCH_DIR/series"
  ((${#patch_files[@]} > 0)) || return 1

  for patch_file in "${patch_files[@]}"; do
    if ! git -C "$tools_shared_dir" apply --check "$patch_file" \
        || ! git -C "$tools_shared_dir" apply "$patch_file"; then
      apply_status=1
      break
    fi
    applied_patch_files+=("$patch_file")
  done
  if ((apply_status == 0)); then
    if ! python3 -c 'import pathlib, sys; [compile(pathlib.Path(path).read_bytes(), path, "exec") for path in sys.argv[1:]]' \
        "$tools_shared_dir/jni_generator/generate_and_register_jni_files.py" \
        "$tools_shared_dir/jni_generator/jni_generator.py"; then
      apply_status=1
    fi
  fi
  for ((i = ${#applied_patch_files[@]} - 1; i >= 0; --i)); do
    if ! git -C "$tools_shared_dir" apply --reverse \
        "${applied_patch_files[$i]}"; then
      printf 'verify: failed to remove temporary Lynx tools_shared patch %s\n' \
        "${applied_patch_files[$i]}" >&2
      apply_status=1
    fi
  done
  [[ -z "$(git -C "$tools_shared_dir" status --porcelain=v1 --untracked-files=no)" ]] || {
    printf 'verify: failed to restore Lynx tools_shared checkout\n' >&2
    return 1
  }
  ((apply_status == 0))
}

verify_lynx_patches() {
  local apply_status=0
  local verification_status=0
  local i
  local patch_file
  local patch_name
  local -a applied_patch_files=()
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
  for patch_file in "${patch_files[@]}"; do
    if ! git -C "$ROOT_DIR/third_party/lynx" apply --check "$patch_file" \
        || ! git -C "$ROOT_DIR/third_party/lynx" apply "$patch_file"; then
      apply_status=1
      break
    fi
    applied_patch_files+=("$patch_file")
  done
  if ((apply_status == 0)) &&
      ! cmp -s \
        "$ROOT_DIR/third_party/lynx/core/public/lynx_native_renderer.h" \
        "$ROOT_DIR/include/lynx_native_renderer.h"; then
    printf 'verify: patched Lynx public C header differs from include/lynx_native_renderer.h\n' \
      >&2
    verification_status=1
  fi
  for ((i = ${#applied_patch_files[@]} - 1; i >= 0; --i)); do
    if ! git -C "$ROOT_DIR/third_party/lynx" apply --reverse \
        "${applied_patch_files[$i]}"; then
      printf 'verify: failed to remove temporary Lynx patch %s\n' \
        "${applied_patch_files[$i]}" >&2
      apply_status=1
    fi
  done
  if ((apply_status != 0 || verification_status != 0)); then
    return 1
  fi
}

verify_removed_transport() {
  local path
  local references

  for path in \
    adapters/mts \
    crates/element-bridge-wire \
    protocol \
    docs/oss-lynx-gap.md \
    include/lynx_element_bridge.h \
    scripts/generate-protocol.mjs \
    scripts/prepare-flatc.sh; do
    [[ ! -e "$ROOT_DIR/$path" ]] || {
      printf 'verify: removed transport path still exists: %s\n' "$path" >&2
      return 1
    }
  done

  references="$(
    git -C "$ROOT_DIR" grep --untracked -n -E \
      'LynxElementBridgeModule|LEB2|element-bridge-wire|prepare-flatc|generate-protocol|adapters/mts|ResultSlot|ResponseBatch|CapabilityRequest|InvokeCapability|yew_lynx_(mount|dispatch|complete|destroy|buffer_free)' \
      -- . \
      ':(exclude)third_party/lynx' \
      ':(exclude)adapters/android/test/run-mock-checks.sh' \
      ':(exclude)scripts/verify.sh' || true
  )"
  [[ -z "$references" ]] || {
    printf 'verify: removed transport references remain:\n%s\n' "$references" >&2
    return 1
  }

  references="$(
    git -C "$ROOT_DIR" grep --untracked -ni -E \
      'flatbuffers|flatc|protocol[- ]v2|wire buffer|wire transport' \
      -- . \
      ':(exclude)third_party/lynx' \
      ':(exclude)examples/android/gradle/verification-metadata.xml' \
      ':(exclude)adapters/android/test/run-mock-checks.sh' \
      ':(exclude)scripts/verify.sh' || true
  )"
  [[ -z "$references" ]] || {
    printf 'verify: removed serialization references remain:\n%s\n' "$references" >&2
    return 1
  }
}

trap cleanup EXIT

printf '==> Checking shell scripts\n'
bash -n "${SCRIPTS[@]}"
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck -x -P SCRIPTDIR "${SCRIPTS[@]}"
else
  printf 'shellcheck not found; skipping optional lint\n'
fi
if command -v actionlint >/dev/null 2>&1; then
  actionlint "$ROOT_DIR"/.github/workflows/*.yml
else
  printf 'actionlint not found; skipping optional workflow lint\n'
fi
git -C "$ROOT_DIR" diff --check

printf '==> Checking Android pins and lock metadata\n'
verify_android_metadata
printf '==> Checking removed transport gates\n'
verify_removed_transport
printf '==> Checking pinned Lynx patch series\n'
verify_lynx_patches
printf '==> Checking pinned Lynx tools_shared patch series\n'
verify_lynx_tools_shared_patches
python3 -c 'import pathlib, sys; compile(pathlib.Path(sys.argv[1]).read_bytes(), sys.argv[1], "exec")' \
  "$ROOT_DIR/scripts/android-device-acceptance.py"
python3 "$ROOT_DIR/scripts/test_android_device_acceptance.py"
python3 "$ROOT_DIR/scripts/test_build_android.py"

printf '==> Bootstrapping pinned Yew checkout\n'
"$ROOT_DIR/scripts/bootstrap-yew.sh"
verify_yew_clean

printf '==> Checking project formatting\n'
cargo fmt --manifest-path "$ROOT_DIR/Cargo.toml" --all -- --check

printf '==> Checking project workspace\n'
cargo check --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --all-targets --locked

printf '==> Testing project workspace\n'
cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --all-targets --locked

printf '==> Testing real WAMR lifecycle\n'
cargo test -p lynx-element-bridge-wamr-host --features wamr -- --test-threads=1

printf '==> Linting project workspace\n'
cargo clippy --manifest-path "$ROOT_DIR/Cargo.toml" \
  --workspace --all-targets --locked -- -D warnings

printf '==> Testing Android Java/JNI adapter\n'
bash "$ROOT_DIR/adapters/android/test/run-mock-checks.sh"

ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$ANDROID_HOME" || ! -d "$ANDROID_HOME" ]]; then
  printf 'verify: ANDROID_HOME must point to an installed Android SDK\n' >&2
  exit 1
fi
android_llvm_bin="$(resolve_android_ndk_prebuilt_dir \
  "$ANDROID_HOME/ndk/25.2.9519653")/bin"
for tool in aarch64-linux-android24-clang llvm-ar; do
  [[ -x "$android_llvm_bin/$tool" ]] || {
    printf 'verify: missing Android NDK tool: %s\n' "$android_llvm_bin/$tool" >&2
    exit 1
  }
done
export CC_aarch64_linux_android="$android_llvm_bin/aarch64-linux-android24-clang"
export AR_aarch64_linux_android="$android_llvm_bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"

printf '==> Building Yew and Dioxus Android arm64 Rust static libraries\n'
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --locked --release \
  --target aarch64-linux-android \
  --package yew-lynx-counter \
  --package lynx-element-bridge-dioxus-counter

printf '==> Building WAMR Android arm64 host and framework wasm32-wasip1 guests\n'
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --locked --release \
  --target aarch64-linux-android \
  --package lynx-element-bridge-wamr-host --features wamr
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --locked --release \
  --target wasm32-wasip1 \
  --package lynx-element-bridge-dioxus-counter \
  --package yew-lynx-counter

printf '==> Preparing isolated patched Yew test source tree\n'
temp_dir="$(mktemp -d "$ROOT_DIR/.deps/.yew-verify.XXXXXX")"
mkdir -p -- "$temp_dir/yew"
git -C "$YEW_SOURCE_DIR" archive --format=tar HEAD |
  tar -xf - -C "$temp_dir/yew"
YEW_TEST_DIR="$temp_dir/yew"

printf '==> Checking patched Yew native_renderer feature\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew" \
  cargo check --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  -p yew --features native_renderer

printf '==> Checking patched Yew native_renderer feature for wasm32-wasip1\n'
CARGO_TARGET_DIR="$ROOT_DIR/target/yew-wasi" \
  cargo check --locked --manifest-path "$YEW_TEST_DIR/Cargo.toml" \
  --target wasm32-wasip1 -p yew --features native_renderer

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

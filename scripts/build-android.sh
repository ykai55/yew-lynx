#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly YEW_SOURCE_DIR="$ROOT_DIR/.deps/yew"
readonly LYNX_SOURCE_DIR="$ROOT_DIR/third_party/lynx"
readonly LYNX_SHA="0df14207cebb060f1bed8de12b64a1119dee8f06"
readonly LYNX_VERSION="0.0.1-0df14207"
readonly HAB_EXECUTABLE="$ROOT_DIR/.deps/android/hab/hab.pex"
readonly PRIMJS_REPOSITORY="$ROOT_DIR/.deps/android/primjs-maven"
readonly LYNX_REPOSITORY="$LYNX_SOURCE_DIR/platform/android/build/release/$LYNX_VERSION"
readonly ANDROID_PROJECT="$ROOT_DIR/examples/android"
readonly APK="$ANDROID_PROJECT/app/build/outputs/apk/debug/app-debug.apk"
readonly EVIDENCE="$ROOT_DIR/.deps/android/build-evidence.txt"
readonly CACHE_KEY_FILE="$ROOT_DIR/.deps/android/build-inputs.sha256"
readonly LYNX_GRADLE_TASKS=(
  :ServiceAPI:assembleNoasanRelease
  :LynxBase:assembleNoasanRelease
  :LynxGfx:assembleNoasanRelease
  :LynxTrace:assembleNoasanRelease
  :LynxJSSDK:assembleNoasanRelease
  :LynxAndroid:assembleNoasanRelease
)
clean=0
offline=0

usage() {
  printf 'Usage: %s [--clean] [--offline]\n' "${0##*/}"
}

remove_generated_android_outputs() {
  rm -rf -- \
    "$ROOT_DIR/.deps/android" \
    "$ROOT_DIR/adapters/mts/dist" \
    "$ROOT_DIR/target/android-libs" \
    "$ROOT_DIR/target/aarch64-linux-android/release" \
    "$ANDROID_PROJECT/.gradle" \
    "$ANDROID_PROJECT/build" \
    "$ANDROID_PROJECT/app/.cxx" \
    "$ANDROID_PROJECT/app/build" \
    "$LYNX_SOURCE_DIR/platform/android/build" \
    "$LYNX_SOURCE_DIR/platform/android/lynx_android/build" \
    "$LYNX_SOURCE_DIR/platform/android/lynx_js_sdk/build" \
    "$LYNX_SOURCE_DIR/platform/android/service_api/build" \
    "$LYNX_SOURCE_DIR/base/platform/android/build" \
    "$LYNX_SOURCE_DIR/base/trace/android/build" \
    "$LYNX_SOURCE_DIR/gfx/platform/android/build"
}

calculate_cache_key() {
  {
    printf 'lynx_sha=%s\nlynx_patches=none\n' "$LYNX_SHA"
    git -C "$ROOT_DIR" ls-files -co --exclude-standard -- \
      .gitmodules Cargo.lock Cargo.toml rust-toolchain.toml \
      adapters/android adapters/mts/package.json adapters/mts/package-lock.json \
      adapters/mts/scripts adapters/mts/src adapters/mts/template android \
      crates examples/android examples/counter patches/yew \
      scripts/bootstrap-yew.sh scripts/build-android.sh scripts/prepare-hab.sh \
      scripts/prepare-primjs.sh scripts/publish-lynx-maven.sh \
      | sort -u \
      | while IFS= read -r input; do
          [[ -f "$ROOT_DIR/$input" ]] || continue
          printf '%s=' "$input"
          sha256sum "$ROOT_DIR/$input" | cut -d ' ' -f 1
        done
  } | sha256sum | cut -d ' ' -f 1
}

while (($#)); do
  case "$1" in
    --clean)
      clean=1
      ;;
    --offline)
      offline=1
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage >&2
      printf 'Unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

if ((clean && offline)); then
  printf -- '--clean and --offline cannot be used together\n' >&2
  exit 2
fi

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

for command in awk cargo curl cut git grep head java node npm python3 rustc sha256sum sort unzip; do
  require_command "$command"
done

if ((offline)); then
  if [[ ! -d "$LYNX_SOURCE_DIR/.git" && ! -f "$LYNX_SOURCE_DIR/.git" ]]; then
    printf -- '--offline requires the initialized Lynx submodule\n' >&2
    exit 1
  fi
  if [[ ! -d "$YEW_SOURCE_DIR/.git" && ! -f "$YEW_SOURCE_DIR/.git" ]]; then
    printf -- '--offline requires the prepared patched Yew checkout\n' >&2
    exit 1
  fi
else
  printf '==> Initializing pinned Lynx submodule\n'
  git -C "$ROOT_DIR" submodule update --init --recursive -- third_party/lynx
fi

if [[ ! -d "$LYNX_SOURCE_DIR/.git" && ! -f "$LYNX_SOURCE_DIR/.git" ]]; then
  printf 'Missing Lynx submodule. Run: git submodule update --init --recursive\n' >&2
  exit 1
fi

printf '==> Preparing pinned patched Yew checkout\n'
"$ROOT_DIR/scripts/bootstrap-yew.sh"
actual_lynx_sha="$(git -C "$LYNX_SOURCE_DIR" rev-parse HEAD)"
if [[ "$actual_lynx_sha" != "$LYNX_SHA" ]]; then
  printf 'Lynx submodule mismatch: expected %s, got %s\n' \
    "$LYNX_SHA" "$actual_lynx_sha" >&2
  exit 1
fi
tracked_lynx_changes="$(git -C "$LYNX_SOURCE_DIR" status --porcelain=v1 --untracked-files=no)"
if [[ -n "$tracked_lynx_changes" ]]; then
  printf 'Pinned Lynx submodule has tracked source changes:\n%s\n' \
    "$tracked_lynx_changes" >&2
  exit 1
fi

java_major="$(java -version 2>&1 | awk -F '[".]' '/version/ { print $2; exit }')"
if [[ "$java_major" != 11 ]]; then
  printf 'Lynx Android requires JDK 11; current java is:\n' >&2
  java -version >&2
  exit 1
fi

ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$ANDROID_HOME" || ! -d "$ANDROID_HOME" ]]; then
  printf 'ANDROID_HOME must point to an installed Android SDK\n' >&2
  exit 1
fi
export ANDROID_HOME
export ANDROID_SDK_ROOT="$ANDROID_HOME"

for component in \
  "platforms/android-33/android.jar" \
  "build-tools/33.0.1/aapt2" \
  "ndk/21.1.6352462/build/cmake/android.toolchain.cmake" \
  "ndk/25.2.9519653/build/cmake/android.toolchain.cmake" \
  "cmake/3.22.1/bin/cmake"; do
  if [[ ! -e "$ANDROID_HOME/$component" ]]; then
    printf 'Missing Android SDK component: %s\n' "$ANDROID_HOME/$component" >&2
    exit 1
  fi
done

if ((clean)); then
  printf '==> Removing generated Android integration outputs\n'
  remove_generated_android_outputs
fi

mkdir -p -- "$ROOT_DIR/.deps/android"
cache_key="$(calculate_cache_key)"
if [[ -f "$CACHE_KEY_FILE" ]]; then
  cached_key="$(<"$CACHE_KEY_FILE")"
  if [[ "$cached_key" != "$cache_key" ]]; then
    if ((offline)); then
      printf -- '--offline cache does not match current Android build inputs\n' >&2
      exit 1
    fi
    printf '==> Android build inputs changed; invalidating generated outputs\n'
    remove_generated_android_outputs
    mkdir -p -- "$ROOT_DIR/.deps/android"
  fi
elif ((offline)); then
  printf -- '--offline requires an Android cache prepared by an online build\n' >&2
  exit 1
fi

if ((offline)); then
  if [[ ! -d "$PRIMJS_REPOSITORY" || ! -d "$LYNX_REPOSITORY" ]]; then
    printf -- '--offline requires prepared PrimJS and Lynx Maven repositories\n' >&2
    exit 1
  fi
else
  printf '==> Materializing locked Habitat executable\n'
  "$ROOT_DIR/scripts/prepare-hab.sh" "$HAB_EXECUTABLE"

  printf '==> Materializing locked PrimJS snapshots\n'
  "$ROOT_DIR/scripts/prepare-primjs.sh" "$PRIMJS_REPOSITORY"

  printf '==> Synchronizing pinned public Lynx dependencies\n'
  (
    cd -- "$LYNX_SOURCE_DIR"
    set +u
    # shellcheck disable=SC1091
    source tools/envsetup.sh
    set -u
    "$HAB_EXECUTABLE" sync .
  )

  printf '==> Building pinned Lynx Android AARs\n'
  (
    cd -- "$LYNX_SOURCE_DIR"
    set +u
    # shellcheck disable=SC1091
    source tools/envsetup.sh
    set -u
    export YEW_LYNX_PRIMJS_REPO="$PRIMJS_REPOSITORY"
    platform/android/gradlew -p platform/android \
      --init-script "$ROOT_DIR/android/lynx-repositories.init.gradle" \
      "${LYNX_GRADLE_TASKS[@]}" \
      -Pversion="$LYNX_VERSION" \
      -PVERSION="$LYNX_VERSION" \
      -Penable_trace=none \
      -PabiList=arm64-v8a \
      -PbuildLynxDebugSo \
      --no-daemon
  )
  "$ROOT_DIR/scripts/publish-lynx-maven.sh" "$LYNX_VERSION" "$LYNX_REPOSITORY"
fi

for artifact in lynx lynx-base lynx-gfx lynx-trace lynx-jssdk service-api; do
  if [[ ! -f "$LYNX_REPOSITORY/org/lynxsdk/lynx/$artifact/$LYNX_VERSION/$artifact-$LYNX_VERSION.aar" ]]; then
    printf 'Missing published Lynx artifact: %s\n' "$artifact" >&2
    exit 1
  fi
done
if ! unzip -Z1 \
    "$LYNX_REPOSITORY/org/lynxsdk/lynx/lynx-jssdk/$LYNX_VERSION/lynx-jssdk-$LYNX_VERSION.aar" \
    | grep '^assets/lynx_core.js$' >/dev/null; then
  printf 'Published lynx-jssdk AAR is missing assets/lynx_core.js\n' >&2
  exit 1
fi

printf '==> Assembling standalone Android application\n'
gradle_arguments=(
  --project-dir "$ANDROID_PROJECT"
  :app:assembleDebug
  --no-daemon
  --dependency-verification strict
)
if ((offline)); then
  gradle_arguments+=(--offline -PyewLynxOffline=true)
fi
"$ANDROID_PROJECT/gradlew" "${gradle_arguments[@]}"

if ((!offline)); then
  printf '==> Reassembling application offline\n'
  "$ANDROID_PROJECT/gradlew" "${gradle_arguments[@]}" --offline -PyewLynxOffline=true
fi

if [[ ! -f "$APK" ]]; then
  printf 'Expected APK was not produced: %s\n' "$APK" >&2
  exit 1
fi
apk_entries="$(unzip -Z1 "$APK")"
for library in \
  libyew_lynx_bridge.so \
  liblynx.so \
  liblynxbase.so \
  liblynxgfx.so \
  liblynxtrace.so \
  libquick.so; do
  if ! grep -q "^lib/arm64-v8a/$library$" <<<"$apk_entries"; then
    printf 'APK is missing arm64-v8a/%s\n' "$library" >&2
    exit 1
  fi
done
if grep -Eq '^lib/(armeabi|armeabi-v7a|x86|x86_64)/' <<<"$apk_entries"; then
  printf 'APK contains an unsupported ABI\n' >&2
  exit 1
fi

mkdir -p -- "$(dirname -- "$EVIDENCE")"
cat >"$EVIDENCE" <<EOF
lynx_sha=$LYNX_SHA
lynx_version=$LYNX_VERSION
lynx_patches=none
hab_lock_sha256=$(sha256sum "$ROOT_DIR/android/hab.lock" | cut -d ' ' -f 1)
primjs_lock_sha256=$(sha256sum "$ROOT_DIR/android/primjs.lock" | cut -d ' ' -f 1)
yew_head=$(git -C "$YEW_SOURCE_DIR" rev-parse HEAD)
yew_patch_series_sha256=$(sha256sum "$ROOT_DIR/patches/yew/series" | cut -d ' ' -f 1)
npm_lock_sha256=$(sha256sum "$ROOT_DIR/adapters/mts/package-lock.json" | cut -d ' ' -f 1)
rustc=$(rustc --version)
node=$(node --version)
java=$(java -version 2>&1 | head -n 1)
lynx_gradle=6.7.1
app_gradle=7.6.4
lynx_android_ndk=21.1.6352462
app_android_ndk=25.2.9519653
android_platform=33
apk_sha256=$(sha256sum "$APK" | cut -d ' ' -f 1)
apk=$APK
android_input_cache_key=$cache_key
EOF
printf '%s\n' "$cache_key" >"$CACHE_KEY_FILE"

printf 'Android APK: %s\n' "$APK"
printf 'Build evidence: %s\n' "$EVIDENCE"

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
REPO_ROOT=$(cd "$ROOT_DIR/../.." && pwd)
BUILD_DIR=$(mktemp -d "$ROOT_DIR/test/.mock-build.XXXXXX")
JAVA_HOME=${JAVA_HOME:-$(dirname "$(dirname "$(readlink -f "$(command -v javac)")")")}
APP_ACTIVITY="$REPO_ROOT/examples/android/app/src/main/java/com/yew/lynx/example/MainActivity.kt"
APP_GRADLE="$REPO_ROOT/examples/android/app/build.gradle.kts"

mkdir -p "$BUILD_DIR/classes" "$BUILD_DIR/native"
trap 'rm -rf "$BUILD_DIR"' EXIT

grep -q 'registerNativeRendererHost()' "$APP_ACTIVITY"
grep -q 'rendererHost.mount(hostToken)' "$APP_ACTIVITY"
grep -Fq 'rendererHost?.destroy()' "$APP_ACTIVITY"
grep -Fq 'view.unregisterNativeRendererHost(hostToken)' "$APP_ACTIVITY"
grep -Fq 'var view: LynxView? = null' "$APP_ACTIVITY"
grep -Fq 'var hostRegistered = false' "$APP_ACTIVITY"
grep -Fq 'var rustMounted = false' "$APP_ACTIVITY"
grep -Fq 'rendererHost?.abandon()' "$APP_ACTIVITY"
grep -Fq 'error.addSuppressed(cleanupError)' "$APP_ACTIVITY"
grep -Fq 'throw lifecycleFailure' "$APP_ACTIVITY"
grep -Fq 'lynxView?.onEnterForeground()' "$APP_ACTIVITY"
grep -Fq 'lynxView?.onEnterBackground()' "$APP_ACTIVITY"
grep -Fq 'readWasmAsset(BuildConfig.LYNX_ELEMENT_BRIDGE_WASM_INITIAL_ASSET)' "$APP_ACTIVITY"
grep -Fq 'nativeRendererHost?.replace(moduleBytes)' "$APP_ACTIVITY"
grep -Fq 'readWasmAsset(BuildConfig.LYNX_ELEMENT_BRIDGE_WASM_REPLACEMENT_ASSET)' "$APP_ACTIVITY"
grep -Fq 'assets.open(assetName)' "$APP_ACTIVITY"
grep -Fq 'wasm-dioxus' "$APP_GRADLE"
grep -Fq 'assets.srcDir(generatedAssetsDirectory)' "$APP_GRADLE"
grep -Fq 'replacement-fixture' "$APP_GRADLE"
grep -Fq 'buildInitialWasmGuest' "$APP_GRADLE"
grep -Fq 'buildReplacementWasmGuest' "$APP_GRADLE"
if grep -Eq 'renderTemplate|setEnableMTSModule|registerModule|\.lynx\.bundle' \
    "$APP_ACTIVITY"; then
  printf 'MainActivity still references the dormant MTS/template path\n' >&2
  exit 1
fi
if grep -Eq 'mtsAdapter|LynxElementBridgeTemplate|\.lynx\.bundle' "$APP_GRADLE"; then
  printf 'Android Gradle still builds or stages the dormant MTS template\n' >&2
  exit 1
fi

javac -d "$BUILD_DIR/classes" \
  "$ROOT_DIR/src/main/java/com/lynx/elementbridge/LynxNativeRendererHost.java" \
  "$ROOT_DIR/test/java/com/lynx/elementbridge/LynxNativeRendererHostTest.java" \
  "$ROOT_DIR/test/java/com/lynx/elementbridge/DlsymFailureTest.java" \
  "$ROOT_DIR/test/java/com/lynx/elementbridge/JniIntegrationTest.java"

g++ -std=c++17 -Wall -Wextra -Werror -fPIC -shared \
  -I"$REPO_ROOT/include" \
  -I"$JAVA_HOME/include" \
  -I"$JAVA_HOME/include/linux" \
  "$ROOT_DIR/src/main/cpp/lynx_element_bridge_jni.cc" \
  "$ROOT_DIR/test/cpp/mock_lynx_native_application.cc" \
  -ldl \
  -o "$BUILD_DIR/native/liblynx_element_bridge.so"

g++ -std=c++17 -Wall -Wextra -Werror -fPIC -shared \
  -Wl,-soname,liblynx_native_renderer.so \
  -I"$REPO_ROOT/include" \
  "$ROOT_DIR/test/cpp/mock_lynx_native_renderer.cc" \
  -o "$BUILD_DIR/native/liblynx_native_renderer.so"

g++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only \
  -I"$REPO_ROOT/include" \
  -I"$JAVA_HOME/include" \
  -I"$JAVA_HOME/include/linux" \
  "$ROOT_DIR/src/main/cpp/lynx_element_bridge_jni.cc"

java -cp "$BUILD_DIR/classes" com.lynx.elementbridge.LynxNativeRendererHostTest
java -Djava.library.path="$BUILD_DIR/native" \
  -cp "$BUILD_DIR/classes" com.lynx.elementbridge.JniIntegrationTest
java -Djava.library.path="$BUILD_DIR/native" \
  -cp "$BUILD_DIR/classes" com.lynx.elementbridge.DlsymFailureTest

mock_symbols="$(nm -D --defined-only "$BUILD_DIR/native/liblynx_element_bridge.so")"
for symbol in \
  Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeMount \
  Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeMountWasm \
  Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeReplaceWasm \
  Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeDestroySession \
  Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeAbandonSession \
  Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeBackend; do
  grep -Eq "[[:space:]]$symbol$" <<<"$mock_symbols"
done
if grep -Eq 'Java_com_lynx_elementbridge_LynxElementBridgeModule_' <<<"$mock_symbols"; then
  printf 'Mock JNI library still exports the removed module transport\n' >&2
  exit 1
fi

cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --locked --release \
  --package yew-lynx-counter --package lynx-element-bridge-dioxus-counter
for backend in yew dioxus; do
  if [[ "$backend" == yew ]]; then
    archive="$REPO_ROOT/target/release/libyew_lynx_counter.a"
  else
    archive="$REPO_ROOT/target/release/liblynx_element_bridge_dioxus_counter.a"
  fi
  archive_symbols="$(nm -g --defined-only "$archive")"
  for symbol in \
    lynx_element_bridge_native_mount \
    lynx_element_bridge_native_destroy_session \
    lynx_element_bridge_native_abandon_session \
    lynx_element_bridge_backend \
    lynx_element_bridge_backend_marker; do
    grep -Eq "[[:space:]]$symbol$" <<<"$archive_symbols"
  done
  if grep -Eq '[[:space:]](lynx_element_bridge_mount|lynx_element_bridge_dispatch_event|lynx_element_bridge_complete_batch|lynx_element_bridge_destroy_session|lynx_element_bridge_buffer_free|yew_lynx_mount|yew_lynx_dispatch|yew_lynx_complete|yew_lynx_destroy|yew_lynx_buffer_free)$' \
      <<<"$archive_symbols"; then
    printf 'Rust %s static library still exports a removed transport symbol\n' "$backend" >&2
    exit 1
  fi
  cc -std=c11 -Wall -Wextra -Werror \
    -DEXPECTED_BACKEND=\"$backend\" \
    -DEXPECTED_BACKEND_MARKER=\"lynx-element-bridge-backend:$backend\" \
    -I"$REPO_ROOT/include" \
    "$ROOT_DIR/test/cpp/real_staticlib_smoke.c" \
    "$archive" \
    -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc \
    -o "$BUILD_DIR/native/real_staticlib_smoke_$backend"
  "$BUILD_DIR/native/real_staticlib_smoke_$backend"
done

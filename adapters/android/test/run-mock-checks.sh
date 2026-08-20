#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
REPO_ROOT=$(cd "$ROOT_DIR/../.." && pwd)
BUILD_DIR=$(mktemp -d "$ROOT_DIR/test/.mock-build.XXXXXX")
JAVA_HOME=${JAVA_HOME:-$(dirname "$(dirname "$(readlink -f "$(command -v javac)")")")}

mkdir -p "$BUILD_DIR/classes" "$BUILD_DIR/native"
trap 'rm -rf "$BUILD_DIR"' EXIT

javac -d "$BUILD_DIR/classes" \
  "$ROOT_DIR/test/java/android/content/Context.java" \
  "$ROOT_DIR/test/java/com/lynx/jsbridge/LynxMethod.java" \
  "$ROOT_DIR/test/java/com/lynx/jsbridge/LynxModule.java" \
  "$ROOT_DIR/src/main/java/com/lynx/elementbridge/LynxElementBridgeModule.java" \
  "$ROOT_DIR/test/java/com/lynx/elementbridge/LynxElementBridgeModuleTest.java" \
  "$ROOT_DIR/test/java/com/lynx/elementbridge/JniIntegrationTest.java"

g++ -std=c++17 -Wall -Wextra -Werror -fPIC -shared \
  -I"$REPO_ROOT/include" \
  -I"$JAVA_HOME/include" \
  -I"$JAVA_HOME/include/linux" \
  "$ROOT_DIR/src/main/cpp/lynx_element_bridge_jni.cc" \
  "$ROOT_DIR/test/cpp/mock_lynx_element_bridge.cc" \
  -o "$BUILD_DIR/native/liblynx_element_bridge.so"

g++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only \
  -I"$REPO_ROOT/include" \
  -I"$JAVA_HOME/include" \
  -I"$JAVA_HOME/include/linux" \
  "$ROOT_DIR/src/main/cpp/lynx_element_bridge_jni.cc"

java -cp "$BUILD_DIR/classes" com.lynx.elementbridge.LynxElementBridgeModuleTest
java -Djava.library.path="$BUILD_DIR/native" \
  -cp "$BUILD_DIR/classes" com.lynx.elementbridge.JniIntegrationTest

cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --locked --release \
  --package yew-lynx-counter --package lynx-element-bridge-dioxus-counter
for backend in yew dioxus; do
  if [[ "$backend" == yew ]]; then
    archive="$REPO_ROOT/target/release/libyew_lynx_counter.a"
  else
    archive="$REPO_ROOT/target/release/liblynx_element_bridge_dioxus_counter.a"
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

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
  "$ROOT_DIR/src/main/java/com/yew/lynx/YewLynxModule.java" \
  "$ROOT_DIR/test/java/com/yew/lynx/YewLynxModuleTest.java" \
  "$ROOT_DIR/test/java/com/yew/lynx/JniIntegrationTest.java"

g++ -std=c++17 -Wall -Wextra -Werror -fPIC -shared \
  -I"$ROOT_DIR/test/include" \
  -I"$JAVA_HOME/include" \
  -I"$JAVA_HOME/include/linux" \
  "$ROOT_DIR/src/main/cpp/yew_lynx_jni.cc" \
  "$ROOT_DIR/test/cpp/mock_yew_lynx.cc" \
  -o "$BUILD_DIR/native/libyew_lynx_bridge.so"

g++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only \
  -I"$ROOT_DIR/../../examples/counter/include" \
  -I"$JAVA_HOME/include" \
  -I"$JAVA_HOME/include/linux" \
  "$ROOT_DIR/src/main/cpp/yew_lynx_jni.cc"

java -cp "$BUILD_DIR/classes" com.yew.lynx.YewLynxModuleTest
java -Djava.library.path="$BUILD_DIR/native" \
  -cp "$BUILD_DIR/classes" com.yew.lynx.JniIntegrationTest

cargo build --manifest-path "$REPO_ROOT/Cargo.toml" \
  --locked --release --package yew-lynx-counter
cc -std=c11 -Wall -Wextra -Werror \
  -I"$REPO_ROOT/examples/counter/include" \
  "$ROOT_DIR/test/cpp/real_staticlib_smoke.c" \
  "$REPO_ROOT/target/release/libyew_lynx_counter.a" \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc \
  -o "$BUILD_DIR/native/real_staticlib_smoke"
"$BUILD_DIR/native/real_staticlib_smoke"

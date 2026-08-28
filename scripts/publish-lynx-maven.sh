#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly LYNX_SOURCE_DIR="$ROOT_DIR/third_party/lynx"
readonly VERSION="${1:?usage: publish-lynx-maven.sh VERSION [OUTPUT_DIR] [stock|native]}"
readonly OUTPUT_DIR="${2:-$LYNX_SOURCE_DIR/platform/android/build/release/$VERSION}"
readonly PRODUCT="${3:-stock}"

publish_aar() {
  local artifact="$1"
  local source="$2"
  local dependencies="$3"
  local artifact_dir="$OUTPUT_DIR/org/lynxsdk/lynx/$artifact/$VERSION"

  if [[ ! -f "$source" ]]; then
    printf 'Missing Lynx AAR for %s: %s\n' "$artifact" "$source" >&2
    return 1
  fi
  mkdir -p -- "$artifact_dir"
  cp -- "$source" "$artifact_dir/$artifact-$VERSION.aar"
  cat >"$artifact_dir/$artifact-$VERSION.pom" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>org.lynxsdk.lynx</groupId>
  <artifactId>$artifact</artifactId>
  <version>$VERSION</version>
  <packaging>aar</packaging>
  <name>$artifact</name>
  <description>Public Lynx Android artifact built from 0df14207cebb060f1bed8de12b64a1119dee8f06.</description>
  <url>https://github.com/lynx-family/lynx</url>
  <licenses>
    <license>
      <name>The Apache License, Version 2.0</name>
      <url>https://www.apache.org/licenses/LICENSE-2.0.txt</url>
    </license>
  </licenses>
  <dependencies>
$dependencies
  </dependencies>
</project>
EOF
}

dependency() {
  local group="$1"
  local artifact="$2"
  local version="$3"
  local scope="$4"

  cat <<EOF
    <dependency>
      <groupId>$group</groupId>
      <artifactId>$artifact</artifactId>
      <version>$version</version>
      <scope>$scope</scope>
    </dependency>
EOF
}

case "$PRODUCT" in
  stock|native) ;;
  *)
    printf 'Unknown Lynx product: %s\n' "$PRODUCT" >&2
    exit 2
    ;;
esac

if [[ "$PRODUCT" == stock ]]; then
  rm -rf -- "$OUTPUT_DIR"

  publish_aar \
    service-api \
    "$LYNX_SOURCE_DIR/platform/android/service_api/build/outputs/aar/ServiceAPI-noasan-release.aar" \
    "$(dependency androidx.annotation annotation 1.0.0 runtime)"

  publish_aar \
    lynx-base \
    "$LYNX_SOURCE_DIR/base/platform/android/build/outputs/aar/LynxBase-noasan-release.aar" \
    "$(dependency org.lynxsdk.lynx service-api "$VERSION" compile)"

  publish_aar \
    lynx-gfx \
    "$LYNX_SOURCE_DIR/gfx/platform/android/build/outputs/aar/LynxGfx-noasan-release.aar" \
    "$(dependency org.lynxsdk.lynx lynx-base "$VERSION" runtime)"

  publish_aar \
    lynx-trace \
    "$LYNX_SOURCE_DIR/base/trace/android/build/outputs/aar/LynxTrace-noasan-release.aar" \
    "$(dependency org.lynxsdk.lynx lynx-base "$VERSION" runtime)"

  publish_aar \
    lynx-jssdk \
    "$LYNX_SOURCE_DIR/platform/android/lynx_js_sdk/build/outputs/aar/LynxJSSDK-noasan-release.aar" \
    ""

  lynx_dependencies="$({
    dependency org.lynxsdk.lynx lynx-base "$VERSION" runtime
    dependency org.lynxsdk.lynx lynx-gfx "$VERSION" runtime
    dependency org.lynxsdk.lynx lynx-trace "$VERSION" runtime
    dependency androidx.core core 1.1.0 runtime
    dependency org.lynxsdk.lynx primjs 4.2.0-alpha.0-SNAPSHOT runtime
    dependency org.lynxsdk.lynx primjsWasm 4.2.0-alpha.0-SNAPSHOT runtime
    dependency org.lynxsdk.lynx lynx-jssdk "$VERSION" compile
    dependency org.lynxsdk.lynx service-api "$VERSION" compile
  })"
  publish_aar \
    lynx \
    "$LYNX_SOURCE_DIR/platform/android/lynx_android/build/outputs/aar/LynxAndroid-noasan-release.aar" \
    "$lynx_dependencies"
else
  [[ -d "$OUTPUT_DIR/org/lynxsdk/lynx" ]] || {
    printf 'Publish the stock Lynx product before the native renderer product\n' >&2
    exit 1
  }
  native_dependencies="$({
    dependency org.lynxsdk.lynx lynx-base "$VERSION" runtime
    dependency org.lynxsdk.lynx lynx-gfx "$VERSION" runtime
    dependency org.lynxsdk.lynx lynx-trace "$VERSION" runtime
    dependency androidx.core core 1.1.0 runtime
    dependency org.lynxsdk.lynx service-api "$VERSION" compile
  })"
  publish_aar \
    lynx-native-renderer \
    "$LYNX_SOURCE_DIR/platform/android/lynx_android/build/outputs/aar/LynxAndroid-noasan-release.aar" \
    "$native_dependencies"
fi

printf 'Published pinned Lynx %s Maven product: %s\n' "$PRODUCT" "$OUTPUT_DIR"

#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly LOCK_FILE="$ROOT_DIR/android/primjs.lock"
readonly REPOSITORY_DIR="${1:-$ROOT_DIR/.deps/android/primjs-maven}"
readonly REMOTE_BASE="https://central.sonatype.com/repository/maven-snapshots/org/lynxsdk/lynx"

# shellcheck source=../android/primjs.lock
source "$LOCK_FILE"

download_locked() {
  local url="$1"
  local output="$2"
  local expected_sha256="$3"
  local actual_sha256
  local temporary

  if [[ -f "$output" ]]; then
    actual_sha256="$(sha256sum "$output" | cut -d ' ' -f 1)"
    if [[ "$actual_sha256" == "$expected_sha256" ]]; then
      return
    fi
    rm -f -- "$output"
  fi

  temporary="$(mktemp "${output}.tmp.XXXXXX")"
  if ! curl --fail --location --retry 3 --output "$temporary" "$url"; then
    rm -f -- "$temporary"
    return 1
  fi
  actual_sha256="$(sha256sum "$temporary" | cut -d ' ' -f 1)"
  if [[ "$actual_sha256" != "$expected_sha256" ]]; then
    rm -f -- "$temporary"
    printf 'PrimJS checksum mismatch for %s: expected %s, got %s\n' \
      "$url" "$expected_sha256" "$actual_sha256" >&2
    return 1
  fi
  mv -- "$temporary" "$output"
}

materialize_module() {
  local artifact="$1"
  local version="$2"
  local unique_version="$3"
  local aar_sha256="$4"
  local pom_sha256="$5"
  local module_dir="$REPOSITORY_DIR/org/lynxsdk/lynx/$artifact/$version"
  local remote_dir="$REMOTE_BASE/$artifact/$version"

  mkdir -p -- "$module_dir"
  download_locked \
    "$remote_dir/$artifact-$unique_version.aar" \
    "$module_dir/$artifact-$unique_version.aar" \
    "$aar_sha256"
  download_locked \
    "$remote_dir/$artifact-$unique_version.pom" \
    "$module_dir/$artifact-$unique_version.pom" \
    "$pom_sha256"

  cat >"$module_dir/maven-metadata.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<metadata modelVersion="1.1.0">
  <groupId>org.lynxsdk.lynx</groupId>
  <artifactId>$artifact</artifactId>
  <version>$version</version>
  <versioning>
    <snapshot>
      <timestamp>20260731.091808</timestamp>
      <buildNumber>1</buildNumber>
    </snapshot>
    <lastUpdated>20260731091808</lastUpdated>
    <snapshotVersions>
      <snapshotVersion>
        <extension>pom</extension>
        <value>$unique_version</value>
        <updated>20260731091808</updated>
      </snapshotVersion>
      <snapshotVersion>
        <extension>aar</extension>
        <value>$unique_version</value>
        <updated>20260731091808</updated>
      </snapshotVersion>
    </snapshotVersions>
  </versioning>
</metadata>
EOF
}

materialize_module \
  primjs "$PRIMJS_VERSION" "$PRIMJS_UNIQUE_VERSION" \
  "$PRIMJS_AAR_SHA256" "$PRIMJS_POM_SHA256"
materialize_module \
  primjsWasm "$PRIMJS_WASM_VERSION" "$PRIMJS_WASM_UNIQUE_VERSION" \
  "$PRIMJS_WASM_AAR_SHA256" "$PRIMJS_WASM_POM_SHA256"

printf 'Locked PrimJS Maven repository: %s\n' "$REPOSITORY_DIR"

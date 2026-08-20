#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
source "$ROOT_DIR/protocol/flatc.lock"

readonly INSTALL_DIR="$ROOT_DIR/.deps/flatbuffers-$FLATC_VERSION"
readonly ARCHIVE="$INSTALL_DIR/$FLATC_LINUX_ASSET"
readonly FLATC="$INSTALL_DIR/flatc"
readonly URL="https://github.com/google/flatbuffers/releases/download/v$FLATC_VERSION/${FLATC_LINUX_ASSET//+/%2B}"

if [[ -x "$FLATC" && "$($FLATC --version)" == "flatc version $FLATC_VERSION" ]]; then
  printf '%s\n' "$FLATC"
  exit 0
fi

mkdir -p "$INSTALL_DIR"
curl --fail --location --retry 3 --output "$ARCHIVE" "$URL"
printf '%s  %s\n' "$FLATC_LINUX_SHA256" "$ARCHIVE" | sha256sum --check --status
unzip -oq "$ARCHIVE" -d "$INSTALL_DIR"
[[ "$($FLATC --version)" == "flatc version $FLATC_VERSION" ]]
printf '%s\n' "$FLATC"

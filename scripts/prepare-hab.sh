#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly LOCK_FILE="$ROOT_DIR/android/hab.lock"
readonly OUTPUT="${1:-$ROOT_DIR/.deps/android/hab/hab.pex}"

# shellcheck source=android/hab.lock
source "$LOCK_FILE"
readonly URL="https://github.com/lynx-family/habitat/releases/download/$HABITAT_VERSION/hab.pex"

mkdir -p -- "$(dirname -- "$OUTPUT")"
if [[ -f "$OUTPUT" ]] &&
  [[ "$(sha256sum "$OUTPUT" | cut -d ' ' -f 1)" == "$HABITAT_SHA256" ]]; then
  chmod +x "$OUTPUT"
  printf 'Locked Habitat executable: %s\n' "$OUTPUT"
  exit 0
fi

temporary="$(mktemp "${OUTPUT}.tmp.XXXXXX")"
trap 'rm -f -- "$temporary"' EXIT
curl --fail --location --retry 3 --output "$temporary" "$URL"
actual_sha256="$(sha256sum "$temporary" | cut -d ' ' -f 1)"
if [[ "$actual_sha256" != "$HABITAT_SHA256" ]]; then
  printf 'Habitat checksum mismatch: expected %s, got %s\n' \
    "$HABITAT_SHA256" "$actual_sha256" >&2
  exit 1
fi
chmod +x "$temporary"
mv -- "$temporary" "$OUTPUT"
trap - EXIT

printf 'Locked Habitat executable: %s\n' "$OUTPUT"

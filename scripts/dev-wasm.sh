#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR

exec cargo run --locked --manifest-path "$ROOT_DIR/Cargo.toml" \
  --package yew-lynx-dev-server -- "$@"

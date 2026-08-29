#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly OUTPUT_DIR="$ROOT_DIR/target/wasm32-wasip1/release"
readonly CARGO_WATCH_VERSION="8.5.3"
readonly CARGO_WATCH_ROOT="$ROOT_DIR/target/cargo-tools"
readonly CARGO_WATCH_BIN="$CARGO_WATCH_ROOT/bin/cargo-watch"

backend=all
port=8000
server_pid=""

usage() {
  printf 'Usage: %s [--backend yew|dioxus|all] [--port PORT]\n' "${0##*/}"
}

cleanup() {
  local exit_status=$?

  trap - EXIT
  if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  exit "$exit_status"
}

while (($#)); do
  case "$1" in
    --backend)
      (($# >= 2)) || {
        usage >&2
        exit 2
      }
      backend="$2"
      shift 2
      ;;
    --port)
      (($# >= 2)) || {
        usage >&2
        exit 2
      }
      port="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$backend" in
  yew)
    packages=(--package yew-lynx-counter)
    wasm_files=(yew_lynx_counter.wasm)
    ;;
  dioxus)
    packages=(--package lynx-element-bridge-dioxus-counter)
    wasm_files=(lynx_element_bridge_dioxus_counter.wasm)
    ;;
  all)
    packages=(
      --package yew-lynx-counter
      --package lynx-element-bridge-dioxus-counter
    )
    wasm_files=(
      yew_lynx_counter.wasm
      lynx_element_bridge_dioxus_counter.wasm
    )
    ;;
  *)
    printf 'Unsupported backend: %s\n' "$backend" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ ! "$port" =~ ^[0-9]+$ ]] || ((port < 1 || port > 65535)); then
  printf 'Port must be an integer between 1 and 65535: %s\n' "$port" >&2
  exit 2
fi

command -v python3 >/dev/null 2>&1 || {
  printf 'python3 is required to serve the WASM files\n' >&2
  exit 1
}
if [[ ! -x "$CARGO_WATCH_BIN" ]] ||
    [[ "$($CARGO_WATCH_BIN --version 2>/dev/null || true)" != "cargo-watch $CARGO_WATCH_VERSION" ]]; then
  printf 'Installing cargo-watch %s in %s\n' "$CARGO_WATCH_VERSION" "$CARGO_WATCH_ROOT"
  cargo install cargo-watch --locked --version "$CARGO_WATCH_VERSION" \
    --root "$CARGO_WATCH_ROOT" --force
fi

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

printf 'Building %s WASM guest(s)\n' "$backend"
cargo build --locked --release --target wasm32-wasip1 "${packages[@]}"

printf 'Serving %s on http://0.0.0.0:%s\n' "$OUTPUT_DIR" "$port"
for wasm_file in "${wasm_files[@]}"; do
  printf '  http://127.0.0.1:%s/%s\n' "$port" "$wasm_file"
done
printf 'For a USB-connected Android device, run: adb reverse tcp:%s tcp:%s\n' \
  "$port" "$port"

python3 -m http.server "$port" --bind 0.0.0.0 --directory "$OUTPUT_DIR" &
server_pid=$!
sleep 0.2
if ! kill -0 "$server_pid" 2>/dev/null; then
  wait "$server_pid"
fi

watch_command='build --locked --release --target wasm32-wasip1'
for ((i = 0; i < ${#packages[@]}; i += 2)); do
  watch_command+=" --package ${packages[$((i + 1))]}"
done
"$CARGO_WATCH_BIN" watch --postpone --watch "$ROOT_DIR" --ignore target \
  -x "$watch_command"

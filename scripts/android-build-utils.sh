#!/usr/bin/env bash

resolve_android_ndk_prebuilt_dir() {
  local ndk_dir="$1"
  local host_tag
  local os
  local arch
  local candidate
  local -a host_tags=()
  local -a attempted=()

  if [[ -n "${ANDROID_NDK_HOST_TAG:-}" ]]; then
    host_tags=("$ANDROID_NDK_HOST_TAG")
  else
    os="$(uname -s)"
    case "$os" in
      Linux)
        host_tags=(linux-x86_64)
        ;;
      Darwin)
        arch="$(uname -m)"
        case "$arch" in
          arm64|aarch64) host_tags=(darwin-arm64 darwin-x86_64) ;;
          *) host_tags=(darwin-x86_64 darwin-arm64) ;;
        esac
        ;;
      *)
        printf 'Unsupported Android NDK host OS: %s\n' "$os" >&2
        return 1
        ;;
    esac
  fi

  for host_tag in "${host_tags[@]}"; do
    candidate="$ndk_dir/toolchains/llvm/prebuilt/$host_tag"
    attempted+=("$candidate")
    if [[ -d "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return
    fi
  done

  printf 'Unable to locate Android NDK prebuilt host directory. Tried:\n' >&2
  printf '  %s\n' "${attempted[@]}" >&2
  return 1
}

sha256_checksum() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$@"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$@"
  else
    printf 'Missing required command: sha256sum or shasum\n' >&2
    return 1
  fi
}

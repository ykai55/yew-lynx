#!/usr/bin/env bash

set -euo pipefail

readonly YEW_UPSTREAM_URL="https://github.com/yewstack/yew.git"
readonly YEW_BASE_REVISION="0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3"
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT_DIR
readonly DEPS_DIR="$ROOT_DIR/.deps"
readonly YEW_DIR="$DEPS_DIR/yew"
readonly PATCH_DIR="$ROOT_DIR/patches/yew"
readonly SERIES_FILE="$PATCH_DIR/series"

temp_dir=""

fail() {
  printf 'bootstrap-yew: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  if [[ -n "$temp_dir" && -d "$temp_dir" ]]; then
    rm -rf -- "$temp_dir"
  fi
}

trap cleanup EXIT

[[ -f "$SERIES_FILE" ]] || fail "missing patch series: $SERIES_FILE"

patch_files=()
patch_ids=()
while IFS= read -r patch_name || [[ -n "$patch_name" ]]; do
  patch_name="${patch_name%$'\r'}"
  case "$patch_name" in
    '' | \#*) continue ;;
  esac

  case "/$patch_name/" in
    */../* | */./*) fail "unsafe path in patch series: $patch_name" ;;
  esac
  [[ "$patch_name" != /* ]] || fail "absolute path in patch series: $patch_name"

  patch_file="$PATCH_DIR/$patch_name"
  [[ -f "$patch_file" ]] || fail "patch listed in series does not exist: $patch_name"

  mapfile -t patch_id_lines < <(git patch-id --stable < "$patch_file")
  ((${#patch_id_lines[@]} == 1)) || fail "expected one commit in patch: $patch_name"
  read -r patch_id _ <<< "${patch_id_lines[0]}"
  [[ "$patch_id" =~ ^[0-9a-f]{40}$ ]] || fail "could not identify patch: $patch_name"

  patch_files+=("$patch_file")
  patch_ids+=("$patch_id")
done < "$SERIES_FILE"

((${#patch_files[@]} > 0)) || fail "patch series is empty: $SERIES_FILE"

prepare_checkout() {
  local checkout_dir="$1"
  local checkout_status
  local commit
  local commit_patch_id
  local expected_parent="$YEW_BASE_REVISION"
  local git_dir
  local index
  local origin_url
  local patch_id_line
  local pre_apply_head
  local -a commit_info=()
  local -a commits=()

  [[ ! -L "$checkout_dir" ]] || fail "refusing to modify symlink: $checkout_dir"
  [[ "$(git -C "$checkout_dir" rev-parse --is-inside-work-tree 2>/dev/null)" == "true" ]] ||
    fail "existing path is not a Git checkout: $checkout_dir"
  git_dir="$(git -C "$checkout_dir" rev-parse --absolute-git-dir)" ||
    fail "could not resolve Git state directory: $checkout_dir"

  origin_url="$(git -C "$checkout_dir" remote get-url origin 2>/dev/null)" ||
    fail "Yew checkout has no origin remote: $checkout_dir"
  [[ "$origin_url" == "$YEW_UPSTREAM_URL" ]] ||
    fail "unexpected Yew origin '$origin_url'; expected '$YEW_UPSTREAM_URL'"

  if git -C "$checkout_dir" symbolic-ref -q HEAD >/dev/null; then
    fail "Yew checkout is on a branch; use a separate detached checkout"
  fi

  [[ ! -d "$git_dir/rebase-apply" && ! -d "$git_dir/rebase-merge" ]] ||
    fail "Yew checkout has an interrupted rebase or git am operation"

  checkout_status="$(git -C "$checkout_dir" status --porcelain=v1 --untracked-files=all)"
  [[ -z "$checkout_status" ]] ||
    fail "Yew checkout has local changes; refusing to reset or overwrite them"

  if ! git -C "$checkout_dir" cat-file -e "$YEW_BASE_REVISION^{commit}" 2>/dev/null; then
    git -C "$checkout_dir" fetch --depth 1 origin "$YEW_BASE_REVISION"
  fi
  [[ "$(git -C "$checkout_dir" rev-parse "$YEW_BASE_REVISION^{commit}")" == "$YEW_BASE_REVISION" ]] ||
    fail "could not resolve exact Yew base revision"
  git -C "$checkout_dir" merge-base --is-ancestor "$YEW_BASE_REVISION" HEAD ||
    fail "Yew HEAD is not based on $YEW_BASE_REVISION"

  mapfile -t commits < <(git -C "$checkout_dir" rev-list --reverse "$YEW_BASE_REVISION..HEAD")
  ((${#commits[@]} <= ${#patch_files[@]})) ||
    fail "Yew checkout contains commits beyond the declared patch series"

  for index in "${!commits[@]}"; do
    commit="${commits[$index]}"
    read -r -a commit_info <<< "$(git -C "$checkout_dir" rev-list --parents -n 1 "$commit")"
    ((${#commit_info[@]} == 2)) || fail "Yew patch history is not linear at $commit"
    [[ "${commit_info[1]}" == "$expected_parent" ]] ||
      fail "Yew patch history contains an unexpected commit at $commit"

    mapfile -t commit_patch_ids < <(
      git -C "$checkout_dir" format-patch --stdout -1 "$commit" | git patch-id --stable
    )
    ((${#commit_patch_ids[@]} == 1)) || fail "could not identify Yew commit: $commit"
    read -r commit_patch_id _ <<< "${commit_patch_ids[0]}"
    patch_id_line="${patch_ids[$index]}"
    [[ "$commit_patch_id" == "$patch_id_line" ]] ||
      fail "Yew commit $commit does not match ${patch_files[$index]##*/}"

    expected_parent="$commit"
  done

  if ((${#commits[@]} < ${#patch_files[@]})); then
    pre_apply_head="$(git -C "$checkout_dir" rev-parse HEAD)"
    if ! git -c user.name='yew-lynx bootstrap' \
      -c user.email='yew-lynx-bootstrap@users.noreply.github.com' \
      -C "$checkout_dir" am "${patch_files[@]:${#commits[@]}}"; then
      if ! git -C "$checkout_dir" am --abort; then
        fail "failed to apply Yew patch series and git am --abort failed; checkout restoration was not verified"
      fi

      checkout_status="$(git -C "$checkout_dir" status --porcelain=v1 --untracked-files=all)"
      if [[ "$(git -C "$checkout_dir" rev-parse HEAD)" != "$pre_apply_head" ||
        -n "$checkout_status" || -d "$git_dir/rebase-apply" ||
        -d "$git_dir/rebase-merge" ]]; then
        fail "failed to apply Yew patch series; git am --abort did not restore the verified pre-apply state"
      fi
      fail "failed to apply Yew patch series; git am --abort restored the verified clean pre-apply checkout"
    fi
  fi

  checkout_status="$(git -C "$checkout_dir" status --porcelain=v1 --untracked-files=all)"
  [[ -z "$checkout_status" ]] || fail "Yew checkout is dirty after patch application"
  [[ "$(git -C "$checkout_dir" rev-list --count "$YEW_BASE_REVISION..HEAD")" == "${#patch_files[@]}" ]] ||
    fail "Yew checkout does not contain the complete patch series"
}

if [[ -e "$YEW_DIR" || -L "$YEW_DIR" ]]; then
  prepare_checkout "$YEW_DIR"
else
  mkdir -p -- "$DEPS_DIR"
  temp_dir="$(mktemp -d "$DEPS_DIR/.yew-bootstrap.XXXXXX")"
  git clone --filter=blob:none --no-checkout --depth 1 \
    "$YEW_UPSTREAM_URL" "$temp_dir/checkout"
  git -C "$temp_dir/checkout" fetch --depth 1 origin "$YEW_BASE_REVISION"
  git -C "$temp_dir/checkout" checkout --detach "$YEW_BASE_REVISION"
  prepare_checkout "$temp_dir/checkout"

  [[ ! -e "$YEW_DIR" && ! -L "$YEW_DIR" ]] ||
    fail "destination appeared during bootstrap: $YEW_DIR"
  mv -- "$temp_dir/checkout" "$YEW_DIR"
fi

printf 'Yew checkout ready: %s\n' "$YEW_DIR"
printf 'Base revision: %s\n' "$YEW_BASE_REVISION"
printf 'Applied patches: %d\n' "${#patch_files[@]}"
printf 'Patched HEAD: %s\n' "$(git -C "$YEW_DIR" rev-parse HEAD)"

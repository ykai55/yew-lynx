#!/usr/bin/env bash
set -euo pipefail

expected="flatc version 25.2.10"
actual="$(flatc --version)"
if [[ "$actual" != "$expected" ]]; then
  printf 'expected %s, found %s\n' "$expected" "$actual" >&2
  exit 1
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
flatc --rust --filename-suffix _generated -o "$root/src" \
  "$root/schema/guest_protocol.fbs"
perl -0pi -e 's/\n+\z/\n/' "$root/src/guest_protocol_generated.rs"

#!/usr/bin/env bash
# build-libetpan.sh rebuilds stale caches and shows build errors.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
src=$root/reference/libetpan
[ -d "$src/.git" ] || { echo "SKIP build-libetpan: reference/libetpan is not checked out"; exit 0; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

mkdir -p "$work/scripts" "$work/engines/libetpan/lib" "$work/reference" "$work/tests/engines"
cp "$root/scripts/build-libetpan.sh" "$work/scripts/"
cp "$root/tests/engines/libetpan.sh" "$work/tests/engines/"
ln -s "$src" "$work/reference/libetpan"
git -C "$src" rev-parse HEAD >"$work/engines/libetpan/PIN"
touch "$work/engines/libetpan/lib/libetpan.a"
: >"$work/engines/libetpan/BUILD-config.h"

if OPENSSL_DIR=/nonexistent "$work/scripts/build-libetpan.sh" >"$work/out" 2>&1; then
  fail "a cached build without TLS was reused"
fi
grep -q "no OpenSSL headers under /nonexistent" "$work/out" || fail "the OpenSSL error was not shown"

if OPENSSL_DIR=/nonexistent "$work/tests/engines/libetpan.sh" >"$work/out" 2>&1; then
  fail "the probe passed on a TLS-less cached build"
fi
grep -q "no OpenSSL headers under /nonexistent" "$work/out" || fail "the probe hid the build error"

printf '#define HAVE_OPENSSL 1\n#define HAVE_JSON 1\n' >"$work/engines/libetpan/BUILD-config.h"
OPENSSL_DIR=/nonexistent "$work/scripts/build-libetpan.sh" >"$work/out" 2>&1 || fail "a valid cache was not reused"
grep -q "^ok    libetpan" "$work/out" || fail "a valid cache did not print ok"

echo "build-libetpan: stale caches are rebuilt, errors reach stderr, valid caches are reused"

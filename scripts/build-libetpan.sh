#!/usr/bin/env bash
# Builds the pinned libetpan into engines/libetpan.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
src=$root/reference/libetpan
prefix=$root/engines/libetpan
work=$root/engines/libetpan-src
pin=$(git -C "$src" rev-parse HEAD)

if [ "$(cat "$prefix/PIN" 2>/dev/null)" = "$pin" ] && [ -f "$prefix/lib/libetpan.a" ] &&
  grep -q "^#define HAVE_OPENSSL 1" "$prefix/BUILD-config.h" 2>/dev/null &&
  grep -q "^#define HAVE_JSON 1" "$prefix/BUILD-config.h" 2>/dev/null; then
  echo "ok    libetpan $pin"
  exit 0
fi

ssl=${OPENSSL_DIR:-$(brew --prefix openssl@3)} || {
  echo "build-libetpan.sh: set OPENSSL_DIR to an OpenSSL 3 prefix" >&2
  exit 1
}
[ -f "$ssl/include/openssl/ssl.h" ] || {
  echo "build-libetpan.sh: no OpenSSL headers under $ssl" >&2
  exit 1
}
jobs=$(sysctl -n hw.ncpu 2>/dev/null || nproc)
rm -rf "$work" "$prefix"
rsync -a --exclude .git "$src/" "$work/"
cd "$work"
./autogen.sh --prefix="$prefix" --enable-static --disable-shared \
  --with-openssl="$ssl" --with-json --disable-db
for feature in HAVE_OPENSSL HAVE_JSON; do
  grep -q "^#define $feature 1" config.h || {
    echo "build-libetpan.sh: configure left out $feature" >&2
    exit 1
  }
done
make -j"$jobs"
make install
cp config.h "$prefix/BUILD-config.h"
echo "$pin" >"$prefix/PIN"
echo "built libetpan $pin"

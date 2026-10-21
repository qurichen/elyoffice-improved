#!/usr/bin/env bash
# Pinned libetpan has every header MailCore 2 includes.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
prefix=$root/engines/libetpan
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

"$root/scripts/build-libetpan.sh" >"$work/build.log" 2>&1 || {
  cat "$work/build.log" >&2
  fail "build failed"
}
[ -s "$prefix/lib/libetpan.a" ] || fail "no libetpan.a"
config=$prefix/BUILD-config.h
grep -q '^#define HAVE_OPENSSL 1' "$config" || fail "libetpan built without TLS"
grep -q '^#define HAVE_JSON 1' "$config" || fail "libetpan built without json-c"

headers=$(grep -rhoE '#[[:space:]]*include[[:space:]]*<libetpan/[A-Za-z_0-9]+\.h>' "$root/reference/mailcore2/src" |
  sed -E 's/.*<(libetpan\/[^>]+)>.*/\1/' | sort -u)
[ -n "$headers" ] || fail "MailCore 2 sources include no libetpan header"
for header in $headers; do
  [ -f "$prefix/include/$header" ] || fail "pinned libetpan lacks $header"
done
echo "libetpan: $(wc -l <<<"$headers" | tr -d ' ') headers MailCore 2 includes are present"

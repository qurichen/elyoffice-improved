#!/bin/sh
# Copy Lucide icons: scripts/icons.sh name...
set -eu
VERSION=1.48.0
DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
(cd "$DIR" && npm pack "lucide-static@$VERSION" --silent >/dev/null && tar xzf "lucide-static-$VERSION.tgz")
cp "$DIR/package/LICENSE" assets/icons/LICENSE
for name in "$@"; do
  cp "$DIR/package/icons/$name.svg" "assets/icons/$name.svg"
done

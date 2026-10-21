#!/usr/bin/env bash
# Builds demo content, then records all six apps.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
bash -c "source '$here/lib.sh'; trap - EXIT; content"
for app in doc table show pdf tree mark; do
  bash "$here/$app.sh"
done

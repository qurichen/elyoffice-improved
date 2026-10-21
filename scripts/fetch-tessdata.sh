#!/usr/bin/env bash
# OCR language data, tessdata_fast at one commit, into engines/tessdata.
set -euo pipefail

out="$(cd "$(dirname "$0")/.." && pwd)/engines/tessdata"
rev=87416418657359cb625c412a48b6e1d6d41c29bd
mkdir -p "$out"
while read -r lang sum; do
  file="$out/$lang.traineddata"
  [ "$lang" = LICENSE ] && file=$out/LICENSE
  if [ -f "$file" ] && [ "$(shasum -a 256 "$file" | cut -d' ' -f1)" = "$sum" ]; then
    echo "ok    $lang"
    continue
  fi
  curl -fsSL -o "$file.part" "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$rev/${file##*/}"
  got=$(shasum -a 256 "$file.part" | cut -d' ' -f1)
  if [ "$got" != "$sum" ]; then
    rm -f "$file.part"
    echo "fetch-tessdata: $lang hash $got, want $sum" >&2
    exit 1
  fi
  mv "$file.part" "$file"
  echo "got   $lang"
done <<'LIST'
LICENSE cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30
eng 7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2
chi_sim a5fcb6f0db1e1d6d8522f39db4e848f05984669172e584e8d76b6b3141e1f730
chi_tra 529c5b5797d64b126065cd55f2bb4c7fd7b15790798091b1ff259941a829330b
LIST

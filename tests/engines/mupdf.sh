#!/usr/bin/env bash
# Builds MuPDF, renders a PDF page, extracts its text.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
out=$root/engines/mupdf-build
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

"$root/scripts/build-mupdf.sh" >"$work/build.log" 2>&1 || fail "build failed, see $work/build.log"
python3 - "$work/hello.pdf" <<'PY'
import sys
stream = b"BT /F1 24 Tf 72 700 Td (Ely Office contract) Tj ET"
objs = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    b"<< /Length %d >>\nstream\n%s\nendstream" % (len(stream), stream),
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
body, offsets = b"%PDF-1.4\n", []
for n, obj in enumerate(objs, 1):
    offsets.append(len(body))
    body += b"%d 0 obj\n%s\nendobj\n" % (n, obj)
xref = len(body)
body += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
body += b"".join(b"%010d 00000 n \n" % o for o in offsets)
body += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
open(sys.argv[1], "wb").write(body)
PY
clang -O1 -I "$root/reference/mupdf/include" "$(dirname "$0")/mupdf_probe.c" \
  "$out/libmupdf.a" "$out/libmupdf-third.a" -lm -o "$work/probe" || fail "probe does not link"
result=$("$work/probe" "$work/hello.pdf") || fail "probe failed: $result"

grep -qx 'pages=1' <<<"$result" || fail "page count: $result"
grep -qx 'bounds=612x792' <<<"$result" || fail "page bounds: $result"
grep -qx 'pixmap=1224x1584' <<<"$result" || fail "pixmap size: $result"
grep -qE '^dark=[1-9][0-9]*$' <<<"$result" || fail "page renders blank: $result"
grep -qx 'text=Ely Office contract' <<<"$result" || fail "text extraction: $result"
nm -u "$out/libmupdf-pkcs7.a" 2>/dev/null | grep -q 'PKCS7_' || fail "signing helper is a stub: no OpenSSL PKCS7 symbols"
echo "mupdf: render, text and OpenSSL signing helper pass"

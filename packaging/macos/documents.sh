#!/usr/bin/env bash
# Prints the app's CFBundleDocumentTypes. Usage: documents.sh <slug>
set -euo pipefail

# role, then the content types it opens.
case ${1:?app slug} in
  ely-doc) set -- Editor org.openxmlformats.wordprocessingml.document org.oasis-open.opendocument.text \
    com.microsoft.word.doc public.rtf ;;
  ely-table) set -- Editor org.openxmlformats.spreadsheetml.sheet org.oasis-open.opendocument.spreadsheet \
    com.microsoft.excel.xls public.comma-separated-values-text public.tab-separated-values-text ;;
  ely-show) set -- Editor org.openxmlformats.presentationml.presentation \
    org.oasis-open.opendocument.presentation com.microsoft.powerpoint.ppt ;;
  ely-pdf) set -- Editor com.adobe.pdf ;;
  ely-mark) set -- Editor net.daringfireball.markdown ;;
  ely-tree) set -- Viewer public.folder ;;
  *) exit 0 ;;
esac
role=$1
shift
echo "  <key>CFBundleDocumentTypes</key><array><dict>"
echo "    <key>CFBundleTypeRole</key><string>$role</string>"
echo "    <key>LSHandlerRank</key><string>Alternate</string>"
echo "    <key>LSItemContentTypes</key><array>"
for type in "$@"; do echo "      <string>$type</string>"; done
echo "    </array>"
echo "  </dict></array>"

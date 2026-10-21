#!/usr/bin/env python3
"""Creates explicit and omitted compression mode fixtures."""

import sys
import zipfile
from pathlib import Path
from defusedxml.minidom import parseString

W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
MODES = ['doNotCompress', 'compressPunctuation', 'compressPunctuationAndJapaneseKana', 'absent']


def main(source, folder):
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=True)
    for mode in MODES:
        with zipfile.ZipFile(source) as original, zipfile.ZipFile(folder / f'compression-{mode}.docx', 'w') as output:
            for item in original.infolist():
                data = original.read(item)
                if item.filename == 'word/settings.xml':
                    doc = parseString(data)
                    for node in list(doc.getElementsByTagNameNS(W, 'characterSpacingControl')):
                        node.parentNode.removeChild(node)
                    if mode != 'absent':
                        node = doc.createElementNS(W, 'w:characterSpacingControl')
                        node.setAttributeNS(W, 'w:val', mode)
                        following = {'footnotePr', 'endnotePr', 'compat', 'docVars', 'rsids', 'themeFontLang'}
                        anchor = next((child for child in doc.documentElement.childNodes
                                       if child.namespaceURI == W and child.localName in following), None)
                        doc.documentElement.insertBefore(node, anchor)
                    data = doc.toxml(encoding='utf-8')
                output.writestr(item, data)


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit('usage: compression-fixture.py base.docx output-folder')
    main(sys.argv[1], sys.argv[2])

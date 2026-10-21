#!/usr/bin/env python3
"""Adds shared script mappings to spreadsheet and slide fixtures."""

from pathlib import Path
import sys
from zipfile import ZipFile
from defusedxml.minidom import parseString

A = 'http://schemas.openxmlformats.org/drawingml/2006/main'


def main(folder):
    folder = Path(folder)
    with ZipFile(folder / 'theme-fonts.docx') as archive:
        theme = parseString(archive.read('word/theme/theme1.xml'))
    for source, target, prefix in [('chart-anchors.xlsx', 'theme-fonts.xlsx', 'xl/theme/'),
                                   ('hyperlink-colors.pptx', 'theme-fonts.pptx', 'ppt/theme/')]:
        changed = 0
        with ZipFile(folder / source) as original, ZipFile(folder / target, 'w') as output:
            for item in original.infolist():
                data = original.read(item)
                if item.filename.startswith(prefix) and item.filename.endswith('.xml'):
                    doc = parseString(data)
                    for label in ('majorFont', 'minorFont'):
                        group = doc.getElementsByTagNameNS(A, label)[0]
                        for font in list(group.getElementsByTagNameNS(A, 'font')):
                            group.removeChild(font)
                        for font in theme.getElementsByTagNameNS(A, label)[0].getElementsByTagNameNS(A, 'font'):
                            group.appendChild(doc.importNode(font, True))
                    data = doc.toxml(encoding='utf-8')
                    changed += 1
                output.writestr(item, data)
        assert changed, (source, 'no theme found')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit('usage: theme-matrix-fixture.py fixture-folder')
    main(sys.argv[1])

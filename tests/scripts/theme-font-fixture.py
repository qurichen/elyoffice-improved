#!/usr/bin/env python3
"""Adds distinct major and minor script mappings."""

import sys
from zipfile import ZipFile
from defusedxml.minidom import parseString
from openpyxl.writer.theme import theme_xml

A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
R = 'http://schemas.openxmlformats.org/package/2006/relationships'
T = 'http://schemas.openxmlformats.org/package/2006/content-types'
MAJOR = {'Hans': 'SimSun', 'Hant': 'PMingLiU', 'Jpan': 'MS Mincho', 'Hang': 'Batang',
         'Arab': 'Times New Roman', 'Hebr': 'David', 'Thai': 'Angsana New', 'Deva': 'Mangal'}
MINOR = {'Hans': 'Microsoft YaHei', 'Hant': 'Microsoft JhengHei', 'Jpan': 'Meiryo', 'Hang': 'Malgun Gothic',
         'Arab': 'Arial', 'Hebr': 'Arial', 'Thai': 'Cordia New', 'Deva': 'Nirmala UI'}


def main(source, target):
    theme = parseString(theme_xml)
    for label, fonts in [('majorFont', MAJOR), ('minorFont', MINOR)]:
        parent = theme.getElementsByTagNameNS(A, label)[0]
        for node in list(parent.getElementsByTagNameNS(A, 'font')):
            parent.removeChild(node)
        for script, face in fonts.items():
            node = theme.createElementNS(A, 'a:font')
            node.setAttribute('script', script)
            node.setAttribute('typeface', face)
            parent.appendChild(node)
    with ZipFile(source) as original, ZipFile(target, 'w') as output:
        assert 'word/theme/theme1.xml' not in original.namelist(), 'base already has a theme'
        for item in original.infolist():
            data = original.read(item)
            if item.filename == 'word/_rels/document.xml.rels':
                doc = parseString(data)
                node = doc.createElementNS(R, 'Relationship')
                node.setAttribute('Id', 'rIdThemeRegression')
                node.setAttribute('Type', 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme')
                node.setAttribute('Target', 'theme/theme1.xml')
                doc.documentElement.appendChild(node)
                data = doc.toxml(encoding='utf-8')
            elif item.filename == '[Content_Types].xml':
                doc = parseString(data)
                node = doc.createElementNS(T, 'Override')
                node.setAttribute('PartName', '/word/theme/theme1.xml')
                node.setAttribute('ContentType', 'application/vnd.openxmlformats-officedocument.theme+xml')
                doc.documentElement.appendChild(node)
                data = doc.toxml(encoding='utf-8')
            elif item.filename == 'word/styles.xml':
                doc = parseString(data)
                default = doc.getElementsByTagNameNS(W, 'rPrDefault')[0]
                fonts = default.getElementsByTagNameNS(W, 'rFonts')[0]
                fonts.setAttributeNS(W, 'w:eastAsiaTheme', 'minorEastAsia')
                fonts.setAttributeNS(W, 'w:cstheme', 'minorBidi')
                data = doc.toxml(encoding='utf-8')
            output.writestr(item, data)
        output.writestr('word/theme/theme1.xml', theme.toxml(encoding='utf-8'))


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit('usage: theme-font-fixture.py base.docx output.docx')
    main(*sys.argv[1:])

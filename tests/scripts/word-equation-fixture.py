#!/usr/bin/env python3
"""Generate synthetic legacy-EQ and native-OMML regression cases."""

from pathlib import Path
import sys
from xml.sax.saxutils import escape
from zipfile import ZipFile, ZipInfo

W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
M = 'http://schemas.openxmlformats.org/officeDocument/2006/math'


def run(content):
    return '<w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/>' + \
           '<w:b w:val="0"/><w:i w:val="0"/><w:color w:val="000000"/>' + \
           '<w:sz w:val="22"/></w:rPr>' + content + '</w:r>'


def field(code, cached=False):
    result = run('<w:fldChar w:fldCharType="begin"/>')
    result += run('<w:instrText xml:space="preserve">' + escape(code) + '</w:instrText>')
    if cached:
        result += run('<w:fldChar w:fldCharType="separate"/>')
        result += run('<w:t>CACHE_MUST_NOT_DUPLICATE</w:t>')
    return result + run('<w:fldChar w:fldCharType="end"/>')


def fixture():
    cases = [
        ('EQ_BASIC', field(r' EQ \f(1,2) ', True)),
        ('EQ_NESTED', field(r' EQ \f(\f(3,4),5) ')),
        ('EQ_ESCAPED', field(r' EQ \f(a\,b,c\(d\)\\e) ')),
        ('EQ_ROOT', field(r' EQ \r(,z) ')),
        ('OMML_NATIVE', '<m:oMath><m:f><m:num><m:r><m:t>9</m:t></m:r></m:num>'
         '<m:den><m:r><m:t>10</m:t></m:r></m:den></m:f></m:oMath>'),
    ]
    body = ''.join('<w:p>' + run('<w:t xml:space="preserve">' + label + ': </w:t>') +
                   formula + '</w:p><w:p>' + run('<w:t>AFTER_' + label + '</w:t>') +
                   '</w:p>' for label, formula in cases)
    return {
        '[Content_Types].xml': '<?xml version="1.0"?><Types '
        'xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Override PartName="/word/document.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
        '<Override PartName="/word/styles.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
        '</Types>',
        '_rels/.rels': '<Relationships '
        'xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" '
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
        'Target="word/document.xml"/></Relationships>',
        'word/_rels/document.xml.rels': '<Relationships '
        'xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rIdStyles" '
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" '
        'Target="styles.xml"/></Relationships>',
        'word/document.xml': f'<w:document xmlns:w="{W}" xmlns:m="{M}"><w:body>' + body +
        '<w:sectPr><w:pgSz w:w="12240" w:h="15840"/>'
        '<w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440"/>'
        '</w:sectPr></w:body></w:document>',
        'word/styles.xml': f'<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault>'
        '<w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:sz w:val="22"/>'
        '</w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" '
        'w:default="1" w:styleId="Normal"><w:name w:val="Normal"/>'
        '<w:rPr><w:b/><w:i/><w:color w:val="FF0000"/></w:rPr></w:style></w:styles>',
    }


def main(output):
    with ZipFile(output, 'w') as archive:
        for name, content in fixture().items():
            info = ZipInfo(name, date_time=(2026, 10, 21, 0, 0, 0))
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content.encode('utf-8'))


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit('usage: word-equation-fixture.py output.docx')
    main(Path(sys.argv[1]))

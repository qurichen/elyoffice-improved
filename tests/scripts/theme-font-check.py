#!/usr/bin/env python3
"""Checks both supplemental theme-font lists after save."""

import posixpath
import sys
from xml.etree import ElementTree as ET
from zipfile import ZipFile

A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
P = '{http://schemas.openxmlformats.org/package/2006/relationships}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
SLIDE = '{http://schemas.openxmlformats.org/presentationml/2006/main}'
EXPECTED = {
    'majorFont': {'Hans': 'SimSun', 'Hant': 'PMingLiU', 'Jpan': 'MS Mincho', 'Hang': 'Batang',
                  'Arab': 'Times New Roman', 'Hebr': 'David', 'Thai': 'Angsana New', 'Deva': 'Mangal'},
    'minorFont': {'Hans': 'Microsoft YaHei', 'Hant': 'Microsoft JhengHei', 'Jpan': 'Meiryo', 'Hang': 'Malgun Gothic',
                  'Arab': 'Arial', 'Hebr': 'Arial', 'Thai': 'Cordia New', 'Deva': 'Nirmala UI'}}


def related(archive, source, kind, identifiers=None):
    folder, name = posixpath.split(source)
    rels = ET.fromstring(archive.read(posixpath.join(folder, '_rels', name + '.rels')))
    links = [link for link in rels.findall(P + 'Relationship')
             if link.get('Type') == 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/' + kind
             and (identifiers is None or link.get('Id') in identifiers)]
    if identifiers is not None:
        assert len(links) == len(identifiers) and {link.get('Id') for link in links} == identifiers, kind
    result = []
    for link in links:
        assert link.get('TargetMode') != 'External', (kind, 'external relationship')
        target = link.get('Target')
        result.append(target.lstrip('/') if target.startswith('/') else posixpath.normpath(posixpath.join(folder, target)))
    return result


def check(path):
    with ZipFile(path) as archive:
        names = archive.namelist()
        if 'word/document.xml' in names:
            sources = ['word/document.xml']
            kind = 'word'
        elif 'xl/workbook.xml' in names:
            sources = ['xl/workbook.xml']
            kind = 'excel'
        else:
            presentation = ET.fromstring(archive.read('ppt/presentation.xml'))
            ids = {item.get(R + 'id') for item in presentation.findall(SLIDE + 'sldMasterIdLst/' + SLIDE + 'sldMasterId')}
            assert ids, 'no slide master'
            sources = related(archive, 'ppt/presentation.xml', 'slideMaster', ids)
            kind = 'powerpoint'
        targets = []
        for source in sources:
            themes = related(archive, source, 'theme')
            assert len(themes) == 1, (source, 'theme link missing or duplicated')
            targets.extend(themes)
        for target in set(targets):
            check_theme(ET.fromstring(archive.read(target)))
        if kind == 'word':
            body = ET.fromstring(archive.read('word/document.xml'))
            texts = [''.join(node.text or '' for node in para.iter(W + 't'))
                     for para in body.find(W + 'body').findall(W + 'p')]
            assert texts == ['THEME_FONT_MARKER', '中文主题字体：正文与标题。', '日本語：テーマと句読点。',
                             '한국어 테마 글꼴', 'العربية والخطوط', 'ภาษาไทยและแบบอักษร'], 'body text changed'
        elif kind == 'excel':
            book = ET.fromstring(archive.read('xl/workbook.xml'))
            assert [sheet.get('name') for sheet in book.find(S + 'sheets')] == [
                'CellFixed', 'CellResize', 'DefaultResize', 'PageFixed', 'TwoCellPage', 'TwoCellFixed'], 'sheets changed'
        else:
            slides = related(archive, 'ppt/presentation.xml', 'slide')
            assert len(slides) == 1, 'slides changed'
            slide = ET.fromstring(archive.read(slides[0]))
            text = ''.join(node.text or '' for node in slide.iter(A + 't'))
            for marker in ('KEEP_BLACK', 'KEEP_RED', 'THEME_BLUE', 'HLINK_MODE', 'UNKNOWN_EXTENSION'):
                assert text.count(marker) == 1, ('slide text changed', marker)


def check_theme(theme):
    for label, expected in EXPECTED.items():
        group = theme.findall('.//' + A + label)
        assert len(group) == 1, (label, 'font collection missing or duplicated')
        fonts = group[0].findall(A + 'font')
        assert len(fonts) == len(expected), (label, 'supplemental list changed')
        assert {font.get('script'): font.get('typeface') for font in fonts} == expected, label
        assert [node.tag for node in group[0]][:3] == [A + 'latin', A + 'ea', A + 'cs'], label


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit('usage: theme-font-check.py saved.docx|saved.xlsx|saved.pptx')
    check(sys.argv[1])
    print('sixteen supplemental theme fonts and multilingual body: correct')

#!/usr/bin/env python3
"""Checks chart attachment modes and workbook calculations independently."""

import posixpath
import sys
from xml.etree import ElementTree as ET
from zipfile import ZipFile

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
P = '{http://schemas.openxmlformats.org/package/2006/relationships}'
XDR = '{http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing}'
C = '{http://schemas.openxmlformats.org/drawingml/2006/chart}'
OFFICE = '{urn:oasis:names:tc:opendocument:xmlns:office:1.0}'
TABLE = '{urn:oasis:names:tc:opendocument:xmlns:table:1.0}'
DRAW = '{urn:oasis:names:tc:opendocument:xmlns:drawing:1.0}'
MODES = {'CellFixed': 'oneCell', 'CellResize': 'twoCell', 'DefaultResize': 'twoCell',
         'PageFixed': 'absolute', 'TwoCellPage': 'absolute', 'TwoCellFixed': 'oneCell'}


def target(archive, source, ident):
    folder, name = posixpath.split(source)
    rels = ET.fromstring(archive.read(posixpath.join(folder, '_rels', name + '.rels')))
    matches = [item for item in rels.findall(P + 'Relationship') if item.get('Id') == ident]
    assert len(matches) == 1 and matches[0].get('TargetMode') != 'External', (source, ident)
    path = matches[0].get('Target')
    return path.lstrip('/') if path.startswith('/') else posixpath.normpath(posixpath.join(folder, path))


def flat(path):
    spreadsheet = ET.parse(path).find('.//' + OFFICE + 'spreadsheet')
    tables = spreadsheet.findall(TABLE + 'table')
    assert [table.get(TABLE + 'name') for table in tables] == list(MODES), 'model sheets changed'
    result = {}
    for table in tables:
        name = table.get(TABLE + 'name')
        frames = [frame for frame in table.iter(DRAW + 'frame') if frame.find(DRAW + 'object') is not None]
        assert len(frames) == 1, (name, 'missing or duplicated model chart')
        frame = frames[0]
        if frame in table.findall(TABLE + 'shapes/' + DRAW + 'frame'):
            mode = 'absolute'
        else:
            assert frame in table.findall(TABLE + 'table-row/' + TABLE + 'table-cell/' + DRAW + 'frame'), name
            mode = 'twoCell' if frame.get(TABLE + 'end-cell-address') else 'oneCell'
        result[name] = mode
    return result


def workbook(path):
    result = {}
    with ZipFile(path) as archive:
        book = ET.fromstring(archive.read('xl/workbook.xml'))
        sheets = book.find(S + 'sheets')
        assert [sheet.get('name') for sheet in sheets] == list(MODES), 'saved sheets changed'
        for sheet in sheets:
            name = sheet.get('name')
            sheet_path = target(archive, 'xl/workbook.xml', sheet.get(R + 'id'))
            content = ET.fromstring(archive.read(sheet_path))
            cells = {cell.get('r'): cell for cell in content.iter(S + 'c')}
            for index in range(2, 5):
                cell = cells.get(f'A{index}')
                assert cell is not None and cell.get('t', 'n') == 'n', (name, 'source cell lost')
                assert float(cell.findtext(S + 'v')) == index - 1, (name, 'source value changed')
            assert cells['B2'].findtext(S + 'f') == 'SUM(A2:A4)', (name, 'formula changed')
            assert float(cells['B2'].findtext(S + 'v')) == 6, (name, 'formula cache changed')
            drawings = content.findall(S + 'drawing')
            assert len(drawings) == 1, (name, 'drawing missing or duplicated')
            drawing_path = target(archive, sheet_path, drawings[0].get(R + 'id'))
            drawing = ET.fromstring(archive.read(drawing_path))
            assert len(drawing) == 1, (name, 'chart anchor missing or duplicated')
            anchor = drawing[0]
            if anchor.tag == XDR + 'twoCellAnchor':
                mode = anchor.get('editAs', 'twoCell')
            else:
                mode = {XDR + 'oneCellAnchor': 'oneCell', XDR + 'absoluteAnchor': 'absolute'}[anchor.tag]
            charts = anchor.findall('.//' + C + 'chart')
            assert len(charts) == 1, (name, 'chart reference missing or duplicated')
            chart_path = target(archive, drawing_path, charts[0].get(R + 'id'))
            chart = ET.fromstring(archive.read(chart_path))
            series = chart.findall('.//' + C + 'barChart/' + C + 'ser')
            assert len(series) == 1, (name, 'chart series changed')
            values = series[0].find(C + 'val/' + C + 'numRef')
            assert values is not None and values.findtext(C + 'f').replace("'", '') == f'{name}!$A$2:$A$4', name
            cache = values.find(C + 'numCache')
            assert cache.find(C + 'ptCount').get('val') == '3', (name, 'chart cache count changed')
            points = {point.get('idx'): float(point.findtext(C + 'v')) for point in cache.findall(C + 'pt')}
            assert points == {'0': 1, '1': 2, '2': 3}, (name, 'chart values changed')
            result[name] = mode
    return result


def check(imported, saved, reopened):
    for label, actual in [('import', flat(imported)), ('export', workbook(saved)), ('reopen', flat(reopened))]:
        assert actual == MODES, (label, actual, MODES)


if __name__ == '__main__':
    if len(sys.argv) != 4:
        sys.exit('usage: chart-anchor-check.py imported.fods saved.xlsx reopened.fods')
    check(*sys.argv[1:])
    print('six chart attachments, series caches and formulas: correct')

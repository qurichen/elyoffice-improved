#!/usr/bin/env python3
"""Checks effective point colors, series and values independently."""

import posixpath
import sys
from xml.etree import ElementTree as ET
from zipfile import ZipFile

C = '{http://schemas.openxmlformats.org/drawingml/2006/chart}'
A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
P = '{http://schemas.openxmlformats.org/package/2006/relationships}'
OFFICE = '{urn:oasis:names:tc:opendocument:xmlns:office:1.0}'
CHART = '{urn:oasis:names:tc:opendocument:xmlns:chart:1.0}'
STYLE = '{urn:oasis:names:tc:opendocument:xmlns:style:1.0}'
TEXT = '{urn:oasis:names:tc:opendocument:xmlns:text:1.0}'
DRAW = '{urn:oasis:names:tc:opendocument:xmlns:drawing:1.0}'
PALETTE = ['4F81BD', 'C0504D', '9BBB59']
EXPECTED = {
    'BarOmitted': (True, [PALETTE]),
    'BarFalse': (False, [['4F81BD'] * 3]),
    'BarTrue': (True, [PALETTE]),
    'BarStyled': (True, [PALETTE]),
    'BarFill': (False, [['7030A0'] * 3]),
    'PieOmitted': (True, [PALETTE]),
    'PieFalse': (False, [['4F81BD'] * 3]),
    'MultiOmitted': (False, [['4F81BD'] * 3, ['C0504D'] * 3]),
}


def expectations(legacy):
    result = EXPECTED.copy()
    if legacy:
        for label in ('BarOmitted', 'BarStyled', 'PieOmitted'):
            result[label] = (False, [['4F81BD'] * 3])
    return result


def target(archive, source, ident):
    folder, name = posixpath.split(source)
    rels = ET.fromstring(archive.read(posixpath.join(folder, '_rels', name + '.rels')))
    matches = [item for item in rels.findall(P + 'Relationship') if item.get('Id') == ident]
    assert len(matches) == 1 and matches[0].get('TargetMode') != 'External', (source, ident)
    path = matches[0].get('Target')
    return path.lstrip('/') if path.startswith('/') else posixpath.normpath(posixpath.join(folder, path))


def fill_color(node):
    color = node.find(C + 'spPr/' + A + 'solidFill/' + A + 'srgbClr')
    return color.get('val').upper() if color is not None else None


def cached_strings(reference):
    assert reference is not None, 'missing string reference'
    cache = reference.find(C + 'strCache')
    assert cache is not None, 'missing string cache'
    points = cache.findall(C + 'pt')
    count = cache.find(C + 'ptCount')
    assert count is not None and int(count.get('val')) == len(points), 'string cache count changed'
    values = {int(point.get('idx')): point.findtext(C + 'v', '') for point in points}
    assert len(values) == len(points) and set(values) == set(range(len(points))), 'invalid string indices'
    return [values[index] for index in range(len(points))]


def chart_colors(root, label, legacy=False):
    group = root.find(C + 'chart/' + C + 'plotArea/' + C + ('pieChart' if label.startswith('Pie') else 'barChart'))
    assert group is not None, (label, 'chart type changed')
    vary = group.find(C + 'varyColors')
    assert vary is not None and vary.get('val') in ('0', '1', 'false', 'true'), (label, 'missing color mode')
    actual_mode = vary.get('val') in ('1', 'true')
    expected_mode, expected_colors = expectations(legacy)[label]
    assert actual_mode == expected_mode, (label, 'color mode', actual_mode, expected_mode)
    series = group.findall(C + 'ser')
    assert len(series) == len(expected_colors), (label, 'series count changed')
    colors = []
    for number, item in enumerate(series):
        title = item.find(C + 'tx/' + C + 'strRef')
        assert title is not None, (label, 'missing series title reference')
        letter = 'B' if number == 0 else 'C'
        title_reference = title.findtext(C + 'f').replace("'", '').replace('$', '')
        assert title_reference == f'{label}!{letter}1', (label, 'title reference changed')
        assert cached_strings(title) == ['Measure' if number == 0 else 'Second'], (label, 'series title changed')
        categories = item.find(C + 'cat/' + C + 'strRef')
        assert categories is not None, (label, 'missing categories')
        assert categories.findtext(C + 'f').replace("'", '') == f'{label}!$A$2:$A$4', (label, 'category reference changed')
        assert cached_strings(categories) == ['Item 1', 'Item 2', 'Item 3'], (label, 'categories changed')
        values = item.find(C + 'val/' + C + 'numRef')
        assert values is not None, (label, 'missing values')
        reference = values.findtext(C + 'f').replace("'", '')
        assert reference == f'{label}!${letter}$2:${letter}$4', (label, reference)
        cache = values.find(C + 'numCache')
        assert cache is not None and cache.find(C + 'ptCount').get('val') == '3', (label, 'missing value cache')
        data = {int(point.get('idx')): float(point.findtext(C + 'v')) for point in cache.findall(C + 'pt')}
        assert data == {index: (index + 1) * (2 if number == 0 else 3) for index in range(3)}, (label, data)
        result = [fill_color(item)] * 3
        seen = set()
        for point in item.findall(C + 'dPt'):
            index = int(point.find(C + 'idx').get('val'))
            assert index in range(3) and index not in seen, (label, 'invalid point index')
            seen.add(index)
            color = fill_color(point)
            if color is not None:
                result[index] = color
        assert result == expected_colors[number], (label, 'point colors', result, expected_colors[number])
        colors.append(result)
    return colors


def sheet_data(root, strings, label):
    cells = {cell.get('r'): cell for cell in root.iter(S + 'c')}
    expected = {'A1': 'Category', 'B1': 'Measure', 'C1': 'Second'}
    for index in range(1, 4):
        expected.update({f'A{index + 1}': f'Item {index}',
                         f'B{index + 1}': index * 2, f'C{index + 1}': index * 3})
    for address, value in expected.items():
        cell = cells.get(address)
        assert cell is not None, (label, address, 'source cell missing')
        kind = cell.get('t', 'n')
        if kind == 's':
            actual = strings[int(cell.findtext(S + 'v'))]
        elif kind == 'inlineStr':
            actual = ''.join(node.text or '' for node in cell.iter(S + 't'))
        else:
            assert kind == 'n', (label, address, kind)
            actual = float(cell.findtext(S + 'v'))
        assert actual == value, (label, address, 'source changed', actual, value)


def workbook(path, legacy=False):
    result = {}
    with ZipFile(path) as archive:
        strings = []
        if 'xl/sharedStrings.xml' in archive.namelist():
            shared = ET.fromstring(archive.read('xl/sharedStrings.xml'))
            strings = [''.join(node.text or '' for node in item.iter(S + 't')) for item in shared.findall(S + 'si')]
        book = ET.fromstring(archive.read('xl/workbook.xml'))
        sheets = book.findall(S + 'sheets/' + S + 'sheet')
        assert [sheet.get('name') for sheet in sheets] == list(EXPECTED), 'sheets changed'
        for sheet in sheets:
            label = sheet.get('name')
            part = target(archive, 'xl/workbook.xml', sheet.get(R + 'id'))
            content = ET.fromstring(archive.read(part))
            sheet_data(content, strings, label)
            drawings = content.findall(S + 'drawing')
            assert len(drawings) == 1, (label, 'drawing changed')
            part = target(archive, part, drawings[0].get(R + 'id'))
            drawing = ET.fromstring(archive.read(part))
            charts = drawing.findall('.//' + C + 'chart')
            assert len(charts) == 1, (label, 'chart changed')
            part = target(archive, part, charts[0].get(R + 'id'))
            result[label] = chart_colors(ET.fromstring(archive.read(part)), label, legacy)
    return result


def style_color(styles, name):
    style = styles.get(name)
    if style is None:
        return None
    graphic = style.find(STYLE + 'graphic-properties')
    color = graphic.get(DRAW + 'fill-color') if graphic is not None else None
    return color.lstrip('#').upper() if color is not None else None


def flat(path, legacy=False):
    result = {}
    for document in ET.parse(path).iter(OFFICE + 'document'):
        chart = document.find(OFFICE + 'body/' + OFFICE + 'chart/' + CHART + 'chart')
        if chart is None:
            continue
        title = chart.find(CHART + 'title')
        assert title is not None, 'model title missing'
        label = ''.join(title.itertext()).strip()
        assert label in EXPECTED and label not in result, (label, 'unexpected model chart')
        styles = {item.get(STYLE + 'name'): item for item in document.iter(STYLE + 'style')}
        series = chart.findall(CHART + 'plot-area/' + CHART + 'series')
        expected = expectations(legacy)[label][1]
        assert len(series) == len(expected), (label, 'model series changed')
        colors = []
        for number, item in enumerate(series):
            base = style_color(styles, item.get(CHART + 'style-name'))
            points = []
            for point in item.findall(CHART + 'data-point'):
                count = int(point.get(CHART + 'repeated', '1'))
                assert 0 < count <= 3, (label, count)
                color = style_color(styles, point.get(CHART + 'style-name')) or base
                points.extend([color] * count)
            assert points == expected[number], (label, 'model colors', points, expected[number])
            colors.append(points)
        result[label] = colors
    assert set(result) == set(EXPECTED), ('missing model charts', result)
    return result


def check(imported, saved, reopened, legacy=False):
    flat(imported, legacy)
    workbook(saved, legacy)
    flat(reopened, legacy)


if __name__ == '__main__':
    arguments = sys.argv[1:]
    legacy = arguments[:1] == ['--office-2007']
    if legacy:
        arguments = arguments[1:]
    if len(arguments) != 3:
        sys.exit('usage: chart-appearance-check.py imported.fods saved.xlsx reopened.fods')
    check(*arguments, legacy=legacy)
    print('chart point colors, explicit false/fill and multiple-series controls: correct')

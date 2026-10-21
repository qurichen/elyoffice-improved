#!/usr/bin/env python3
"""Checks row geometry, hidden state and formula caches."""

import math
import sys
import zipfile
from xml.etree import ElementTree as ET

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
HEIGHTS = [23.2, 0.5, 12.75, 14.2, 29.8, 409.0, 23.2]


def on(value):
    return value in ('1', 'true')


def close(actual, expected, label):
    assert actual is not None and math.isclose(float(actual), expected, abs_tol=0.025, rel_tol=0), (label, actual, expected)


def formulas(sheet, rows, offset=0):
    cells = {cell.get('r'): cell for cell in sheet.iter(S + 'c')}
    for row in rows:
        value = row + offset
        cell = cells[f'B{row}']
        assert cell.findtext(S + 'f') == f'A{row}*2', (row, 'formula lost')
        close(cell.findtext(S + 'v'), value * 2, f'B{row}')
        close(cells[f'A{row}'].findtext(S + 'v'), value, f'A{row}')


def check(path):
    with zipfile.ZipFile(path) as archive:
        book = ET.fromstring(archive.read('xl/workbook.xml'))
        assert [sheet.get('name') for sheet in book.find(S + 'sheets')] == ['Rows', 'DefaultCustom']
        rows = ET.fromstring(archive.read('xl/worksheets/sheet1.xml'))
        models = {int(row.get('r')): row for row in rows.iter(S + 'row')}
        for index, height in enumerate(HEIGHTS, 1):
            model = models[index]
            close(model.get('ht'), height, f'row {index}')
            assert on(model.get('customHeight')), (index, 'manual height lost')
            assert on(model.get('hidden')) == (index == 7), (index, 'hidden state changed')
        formulas(rows, range(1, 8))
        defaults = ET.fromstring(archive.read('xl/worksheets/sheet2.xml'))
        fmt = defaults.find(S + 'sheetFormatPr')
        close(fmt.get('defaultRowHeight'), 23.2, 'manual default')
        assert on(fmt.get('customHeight')), 'manual default lost'
        models = {int(row.get('r')): row for row in defaults.iter(S + 'row')}
        for index in (1, 2):
            close(models[index].get('ht', fmt.get('defaultRowHeight')), 23.2, f'default row {index}')
        close(models[3].get('ht'), 12.75, 'explicit default-sheet override')
        assert on(models[3].get('customHeight')), 'explicit override lost'
        formulas(defaults, range(1, 3), offset=8)


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit('usage: row-height-check.py saved.xlsx')
    check(sys.argv[1])
    print('manual row heights, hidden state and nine formulas: correct')

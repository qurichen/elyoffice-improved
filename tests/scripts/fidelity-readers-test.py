#!/usr/bin/env python3
"""Mutation tests for independent fidelity result readers."""

import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
from xml.etree import ElementTree as ET
from zipfile import ZipFile


def reader(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ROW = reader('row-height-check')
COMPRESSION = reader('compression-check')
S, W, CONFIG = ROW.S, COMPRESSION.W, COMPRESSION.CONFIG


def archive(parts):
    result = io.BytesIO()
    with ZipFile(result, 'w') as target:
        for name, root in parts.items():
            target.writestr(name, ET.tostring(root))
    result.seek(0)
    return result


def cells(row, index, value):
    cell = ET.SubElement(row, S + 'c', r=f'A{index}')
    ET.SubElement(cell, S + 'v').text = str(value)
    cell = ET.SubElement(row, S + 'c', r=f'B{index}')
    ET.SubElement(cell, S + 'f').text = f'A{index}*2'
    ET.SubElement(cell, S + 'v').text = str(value * 2)


def workbook():
    book = ET.Element(S + 'workbook')
    sheets = ET.SubElement(book, S + 'sheets')
    for name in ('Rows', 'DefaultCustom'):
        ET.SubElement(sheets, S + 'sheet', name=name)
    rows = ET.Element(S + 'worksheet')
    data = ET.SubElement(rows, S + 'sheetData')
    for index, height in enumerate(ROW.HEIGHTS, 1):
        row = ET.SubElement(data, S + 'row', r=str(index), ht=str(height), customHeight='1')
        if index == 7:
            row.set('hidden', '1')
        cells(row, index, index)
    defaults = ET.Element(S + 'worksheet')
    ET.SubElement(defaults, S + 'sheetFormatPr', defaultRowHeight='23.2', customHeight='1')
    data = ET.SubElement(defaults, S + 'sheetData')
    for index in (1, 2):
        cells(ET.SubElement(data, S + 'row', r=str(index)), index, index + 8)
    ET.SubElement(data, S + 'row', r='3', ht='12.75', customHeight='1')
    return {'xl/workbook.xml': book, 'xl/worksheets/sheet1.xml': rows,
            'xl/worksheets/sheet2.xml': defaults}


class RowReaderTests(unittest.TestCase):
    def test_complete_result(self):
        ROW.check(archive(workbook()))

    def test_rejects_mutations(self):
        for mutation in ('missing_override', 'height', 'manual_flag', 'hidden',
                         'formula', 'formula_cache', 'default_height'):
            with self.subTest(mutation=mutation):
                parts = workbook()
                rows = parts['xl/worksheets/sheet1.xml']
                first = rows.find(S + 'sheetData/' + S + 'row')
                defaults = parts['xl/worksheets/sheet2.xml']
                if mutation == 'missing_override':
                    data = defaults.find(S + 'sheetData')
                    data.remove(data[-1])
                elif mutation == 'height':
                    first.set('ht', '22.5')
                elif mutation == 'manual_flag':
                    first.set('customHeight', 'false')
                elif mutation == 'hidden':
                    first.set('hidden', 'true')
                elif mutation == 'formula':
                    first.find(S + 'c/' + S + 'f').text = 'A1*3'
                elif mutation == 'formula_cache':
                    first.findall(S + 'c')[1].find(S + 'v').text = '3'
                else:
                    defaults.find(S + 'sheetFormatPr').set('defaultRowHeight', '22.5')
                with self.assertRaises((AssertionError, KeyError)):
                    ROW.check(archive(parts))


class CompressionReaderTests(unittest.TestCase):
    def check_model(self, mode, mutation=None):
        value = COMPRESSION.MODES[mode]
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ('imported.fodt', 'saved.docx', 'reopened.fodt')]
            for index in (0, 2):
                root = ET.Element('document')
                item = ET.SubElement(root, CONFIG + 'config-item', {CONFIG + 'name': 'CharacterCompressionType'})
                item.text = str(0 if mutation == ('import' if index == 0 else 'reopen') else value)
                ET.ElementTree(root).write(paths[index])
            settings = ET.Element(W + 'settings')
            if mode != 'absent':
                ET.SubElement(settings, W + 'characterSpacingControl',
                              {W + 'val': 'doNotCompress' if mutation == 'export' else mode})
            body = ET.Element(W + 'document')
            ET.SubElement(body, W + 't').text = 'LOST' if mutation == 'body' else 'COMPRESSION_MARKER'
            paths[1].write_bytes(archive({'word/settings.xml': settings, 'word/document.xml': body}).getvalue())
            COMPRESSION.check(mode, *paths)

    def test_all_modes(self):
        for mode in COMPRESSION.MODES:
            with self.subTest(mode=mode):
                self.check_model(mode)

    def test_rejects_mutations(self):
        for mutation in ('import', 'export', 'reopen', 'body'):
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                self.check_model('compressPunctuation', mutation)


if __name__ == '__main__':
    unittest.main(verbosity=2)

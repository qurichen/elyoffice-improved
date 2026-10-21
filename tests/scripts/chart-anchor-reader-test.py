#!/usr/bin/env python3
"""Mutation tests for chart-placement result inspection."""

import importlib.util
import io
from pathlib import Path
import unittest
from xml.etree import ElementTree as ET
from zipfile import ZipFile

spec = importlib.util.spec_from_file_location('anchors', Path(__file__).with_name('chart-anchor-check.py'))
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)
S, C, XDR = reader.S, reader.C, reader.XDR
TABLE, DRAW, OFFICE = reader.TABLE, reader.DRAW, reader.OFFICE


def result_parts():
    fixture = Path(__file__).resolve().parents[1] / 'fixtures/office/chart-anchors.xlsx'
    with ZipFile(fixture) as archive:
        parts = {item.filename: archive.read(item) for item in archive.infolist()}
    for name, data in list(parts.items()):
        if name.startswith('xl/worksheets/sheet') and name.endswith('.xml'):
            root = ET.fromstring(data)
            cell = next(cell for cell in root.iter(S + 'c') if cell.get('r') == 'B2')
            cell.find(S + 'v').text = '6'
            parts[name] = ET.tostring(root)
        elif name.startswith('xl/charts/chart') and name.endswith('.xml'):
            root = ET.fromstring(data)
            reference = root.find('.//' + C + 'val/' + C + 'numRef')
            cache = ET.SubElement(reference, C + 'numCache')
            ET.SubElement(cache, C + 'ptCount', val='3')
            for index in range(3):
                point = ET.SubElement(cache, C + 'pt', idx=str(index))
                ET.SubElement(point, C + 'v').text = str(index + 1)
            parts[name] = ET.tostring(root)
    return parts


def archive(parts):
    result = io.BytesIO()
    with ZipFile(result, 'w') as output:
        for name, data in parts.items():
            output.writestr(name, data)
    result.seek(0)
    return result


def model():
    doc = ET.Element(OFFICE + 'document')
    sheet = ET.SubElement(ET.SubElement(doc, OFFICE + 'body'), OFFICE + 'spreadsheet')
    for name, mode in reader.MODES.items():
        table = ET.SubElement(sheet, TABLE + 'table', {TABLE + 'name': name})
        parent = ET.SubElement(table, TABLE + 'shapes') if mode == 'absolute' else ET.SubElement(
            ET.SubElement(table, TABLE + 'table-row'), TABLE + 'table-cell')
        frame = ET.SubElement(parent, DRAW + 'frame')
        if mode == 'twoCell':
            frame.set(TABLE + 'end-cell-address', name + '.I13')
        ET.SubElement(frame, DRAW + 'object')
    return doc


class WorkbookReaderTests(unittest.TestCase):
    def test_all_six_encodings(self):
        self.assertEqual(reader.workbook(archive(result_parts())), reader.MODES)

    def test_rejects_mode_changes(self):
        for number, name in enumerate(reader.MODES, 1):
            with self.subTest(name=name):
                parts = result_parts()
                part = f'xl/drawings/drawing{number}.xml'
                root = ET.fromstring(parts[part])
                anchor = root[0]
                anchor.tag = XDR + 'twoCellAnchor'
                anchor.set('editAs', 'absolute' if reader.MODES[name] != 'absolute' else 'oneCell')
                parts[part] = ET.tostring(root)
                self.assertNotEqual(reader.workbook(archive(parts)), reader.MODES)

    def test_rejects_lost_content(self):
        for change in ('formula', 'formula_cache', 'chart_value', 'series_reference',
                       'chart_reference', 'anchor', 'source_missing', 'source_value'):
            with self.subTest(change=change):
                parts = result_parts()
                if change in ('formula', 'formula_cache', 'source_missing', 'source_value'):
                    part = 'xl/worksheets/sheet1.xml'
                    root = ET.fromstring(parts[part])
                    reference = 'A2' if change.startswith('source_') else 'B2'
                    cell = next(cell for cell in root.iter(S + 'c') if cell.get('r') == reference)
                    if change == 'source_missing':
                        row = root.find(S + 'sheetData/' + S + 'row' + "[@r='2']")
                        row.remove(cell)
                    else:
                        cell.find(S + ('f' if change == 'formula' else 'v')).text = '7'
                elif change in ('chart_value', 'series_reference'):
                    part = 'xl/charts/chart1.xml'
                    root = ET.fromstring(parts[part])
                    reference = root.find('.//' + C + 'val/' + C + 'numRef')
                    if change == 'chart_value':
                        reference.find(C + 'numCache/' + C + 'pt/' + C + 'v').text = '99'
                    else:
                        reference.find(C + 'f').text = 'CellFixed!$A$2:$A$3'
                else:
                    part = 'xl/drawings/drawing1.xml'
                    root = ET.fromstring(parts[part])
                    if change == 'anchor':
                        root.remove(root[0])
                    else:
                        chart = root.find('.//' + C + 'chart')
                        chart.set(reader.R + 'id', 'missing')
                parts[part] = ET.tostring(root)
                with self.assertRaises(AssertionError):
                    reader.workbook(archive(parts))


class ModelReaderTests(unittest.TestCase):
    def test_complete_model(self):
        self.assertEqual(reader.flat(io.BytesIO(ET.tostring(model()))), reader.MODES)

    def test_mode_loss(self):
        for name in ('CellResize', 'DefaultResize', 'PageFixed', 'TwoCellPage'):
            with self.subTest(name=name):
                root = model()
                table = next(item for item in root.iter(TABLE + 'table') if item.get(TABLE + 'name') == name)
                frame = next(table.iter(DRAW + 'frame'))
                if name in ('CellResize', 'DefaultResize'):
                    del frame.attrib[TABLE + 'end-cell-address']
                else:
                    table.find(TABLE + 'shapes').remove(frame)
                    cell = ET.SubElement(ET.SubElement(table, TABLE + 'table-row'), TABLE + 'table-cell')
                    cell.append(frame)
                self.assertNotEqual(reader.flat(io.BytesIO(ET.tostring(root))), reader.MODES)


if __name__ == '__main__':
    unittest.main(verbosity=2)

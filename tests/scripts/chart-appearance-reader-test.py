#!/usr/bin/env python3
"""Mutation tests for chart point-color inspection."""

import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
from xml.etree import ElementTree as ET
from zipfile import ZipFile

spec = importlib.util.spec_from_file_location('appearance', Path(__file__).with_name('chart-appearance-check.py'))
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)
spec = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('chart-appearance-fixture.py'))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
A, C, S, OFFICE = reader.A, reader.C, reader.S, reader.OFFICE
STYLE, CHART, TEXT, DRAW = reader.STYLE, reader.CHART, reader.TEXT, reader.DRAW
FIXTURE = Path(__file__).resolve().parents[1] / 'fixtures/office/chart-appearance.xlsx'


def parts():
    with ZipFile(FIXTURE) as archive:
        return {item.filename: archive.read(item) for item in archive.infolist()}


def archive(content):
    result = io.BytesIO()
    with ZipFile(result, 'w') as output:
        for name, data in content.items():
            output.writestr(name, data)
    result.seek(0)
    return result


def add_fill(parent, color):
    shape = parent.find(C + 'spPr')
    if shape is None:
        shape = ET.SubElement(parent, C + 'spPr')
    fill = shape.find(A + 'solidFill')
    if fill is None:
        fill = ET.SubElement(shape, A + 'solidFill')
    else:
        fill.clear()
    ET.SubElement(fill, A + 'srgbClr', val=color)


def result_parts(legacy=False):
    result = parts()
    for number, (label, (vary, colors)) in enumerate(reader.expectations(legacy).items(), 1):
        name = f'xl/charts/chart{number}.xml'
        root = ET.fromstring(result[name])
        group = root.find(C + 'chart/' + C + 'plotArea/' + C + ('pieChart' if label.startswith('Pie') else 'barChart'))
        mode = group.find(C + 'varyColors')
        if mode is None:
            mode = ET.SubElement(group, C + 'varyColors')
        mode.set('val', str(int(vary)))
        for index, series in enumerate(group.findall(C + 'ser')):
            title = series.find(C + 'tx/' + C + 'strRef')
            title.find(C + 'f').text = f"{label}!${'B' if index == 0 else 'C'}$1"
            cache = ET.SubElement(title, C + 'strCache')
            ET.SubElement(cache, C + 'ptCount', val='1')
            ET.SubElement(ET.SubElement(cache, C + 'pt', idx='0'), C + 'v').text = 'Measure' if index == 0 else 'Second'
            categories = series.find(C + 'cat')
            categories.clear()
            reference = ET.SubElement(categories, C + 'strRef')
            ET.SubElement(reference, C + 'f').text = f'{label}!$A$2:$A$4'
            cache = ET.SubElement(reference, C + 'strCache')
            ET.SubElement(cache, C + 'ptCount', val='3')
            for point_index in range(3):
                point = ET.SubElement(cache, C + 'pt', idx=str(point_index))
                ET.SubElement(point, C + 'v').text = f'Item {point_index + 1}'
            add_fill(series, colors[index][0])
            for point_index, color in enumerate(colors[index]):
                point = ET.SubElement(series, C + 'dPt')
                ET.SubElement(point, C + 'idx', val=str(point_index))
                add_fill(point, color)
            values = series.find(C + 'val/' + C + 'numRef')
            cache = ET.SubElement(values, C + 'numCache')
            ET.SubElement(cache, C + 'ptCount', val='3')
            for point_index in range(3):
                point = ET.SubElement(cache, C + 'pt', idx=str(point_index))
                ET.SubElement(point, C + 'v').text = str((point_index + 1) * (2 if index == 0 else 3))
        result[name] = ET.tostring(root)
    return result


def model(legacy=False):
    root = ET.Element(OFFICE + 'document')
    for label, (_, series_colors) in reader.expectations(legacy).items():
        document = ET.SubElement(root, OFFICE + 'document')
        styles = ET.SubElement(document, OFFICE + 'automatic-styles')
        chart = ET.SubElement(ET.SubElement(ET.SubElement(document, OFFICE + 'body'),
                                           OFFICE + 'chart'), CHART + 'chart')
        ET.SubElement(ET.SubElement(chart, CHART + 'title'), TEXT + 'p').text = label
        plot = ET.SubElement(chart, CHART + 'plot-area')
        for number, colors in enumerate(series_colors):
            series = ET.SubElement(plot, CHART + 'series')
            for index, color in enumerate(colors):
                name = f'p{number}_{index}'
                style = ET.SubElement(styles, STYLE + 'style', {STYLE + 'name': name})
                ET.SubElement(style, STYLE + 'graphic-properties', {DRAW + 'fill-color': '#' + color})
                ET.SubElement(series, CHART + 'data-point', {CHART + 'style-name': name})
    return root


class FixtureTests(unittest.TestCase):
    def test_source_encodes_absent_and_explicit_modes(self):
        content = parts()
        for number, label in enumerate(reader.EXPECTED, 1):
            root = ET.fromstring(content[f'xl/charts/chart{number}.xml'])
            group = root.find(C + 'chart/' + C + 'plotArea/' + C + ('pieChart' if label.startswith('Pie') else 'barChart'))
            mode = group.find(C + 'varyColors')
            if label in ('BarOmitted', 'BarStyled', 'PieOmitted', 'MultiOmitted'):
                self.assertIsNone(mode, label)
            else:
                self.assertEqual(mode.get('val'), '1' if label == 'BarTrue' else '0')

    def test_office_2007_copy_only_changes_application_metadata(self):
        with tempfile.TemporaryDirectory() as folder:
            destination = Path(folder) / 'legacy.xlsx'
            fixture.legacy_copy(FIXTURE, destination)
            with ZipFile(destination) as output:
                original = parts()
                self.assertEqual(set(output.namelist()), set(original))
                for name, data in original.items():
                    if name != 'docProps/app.xml':
                        self.assertEqual(output.read(name), data, name)
                props = ET.fromstring(output.read('docProps/app.xml'))
                self.assertEqual(props.findtext(fixture.EP + 'Application'), 'Microsoft Excel')
                self.assertEqual(props.findtext(fixture.EP + 'AppVersion'), '12.0000')
            with self.assertRaises(AssertionError):
                fixture.legacy_copy(FIXTURE, destination)


class WorkbookReaderTests(unittest.TestCase):
    def test_complete_outputs_for_both_compatibility_branches(self):
        for legacy in (False, True):
            actual = reader.workbook(archive(result_parts(legacy)), legacy)
            self.assertEqual(actual, {label: value[1] for label, value in reader.expectations(legacy).items()})

    def test_rejects_changed_color_modes(self):
        for number, label in enumerate(reader.EXPECTED, 1):
            with self.subTest(label=label):
                content = result_parts()
                name = f'xl/charts/chart{number}.xml'
                root = ET.fromstring(content[name])
                mode = next(root.iter(C + 'varyColors'))
                mode.set('val', '0' if reader.EXPECTED[label][0] else '1')
                content[name] = ET.tostring(root)
                with self.assertRaises(AssertionError):
                    reader.workbook(archive(content))

    def test_rejects_lost_colors_values_and_relationships(self):
        for change in ('color', 'point_index', 'value', 'reference', 'series', 'drawing',
                       'source', 'source_missing', 'category', 'category_reference', 'label'):
            with self.subTest(change=change):
                content = result_parts()
                name = 'xl/charts/chart1.xml'
                root = ET.fromstring(content[name])
                series = root.find('.//' + C + 'barChart/' + C + 'ser')
                if change == 'color':
                    series.find(C + 'dPt/' + C + 'spPr/' + A + 'solidFill/' + A + 'srgbClr').set('val', 'FF00FF')
                elif change == 'point_index':
                    series.find(C + 'dPt/' + C + 'idx').set('val', '3')
                elif change == 'value':
                    series.find(C + 'val/' + C + 'numRef/' + C + 'numCache/' + C + 'pt/' + C + 'v').text = '99'
                elif change == 'reference':
                    series.find(C + 'val/' + C + 'numRef/' + C + 'f').text = 'Other!$B$2:$B$4'
                elif change == 'category':
                    series.find(C + 'cat/' + C + 'strRef/' + C + 'strCache/' + C + 'pt/' + C + 'v').text = 'Wrong'
                elif change == 'category_reference':
                    series.find(C + 'cat/' + C + 'strRef/' + C + 'f').text = 'Other!$A$2:$A$4'
                elif change == 'label':
                    series.find(C + 'tx/' + C + 'strRef/' + C + 'strCache/' + C + 'pt/' + C + 'v').text = 'Wrong'
                elif change in ('source', 'source_missing'):
                    name = 'xl/worksheets/sheet1.xml'
                    root = ET.fromstring(content[name])
                    cell = next(cell for cell in root.iter(S + 'c') if cell.get('r') == 'B2')
                    if change == 'source':
                        cell.find(S + 'v').text = '99'
                    else:
                        next(row for row in root.iter(S + 'row') if cell in row).remove(cell)
                elif change == 'series':
                    root.find('.//' + C + 'barChart').remove(series)
                else:
                    name = 'xl/drawings/_rels/drawing1.xml.rels'
                    root = ET.fromstring(content[name])
                    root[0].set('TargetMode', 'External')
                content[name] = ET.tostring(root)
                with self.assertRaises(AssertionError):
                    reader.workbook(archive(content))


class ModelReaderTests(unittest.TestCase):
    def test_complete_models_for_both_branches(self):
        for legacy in (False, True):
            actual = reader.flat(io.BytesIO(ET.tostring(model(legacy))), legacy)
            self.assertEqual(actual, {label: value[1] for label, value in reader.expectations(legacy).items()})

    def test_rejects_missing_duplicate_or_incomplete_model(self):
        for change in ('missing', 'duplicate', 'color', 'points', 'series'):
            with self.subTest(change=change):
                root = model()
                if change == 'missing':
                    root.remove(root[0])
                elif change == 'duplicate':
                    root.append(ET.fromstring(ET.tostring(root[0])))
                elif change == 'color':
                    next(root.iter(STYLE + 'graphic-properties')).set(DRAW + 'fill-color', '#FF00FF')
                else:
                    plot = next(root.iter(CHART + 'plot-area'))
                    if change == 'series':
                        plot.remove(plot[0])
                    else:
                        plot[0][0].set(CHART + 'repeated', '3')
                with self.assertRaises(AssertionError):
                    reader.flat(io.BytesIO(ET.tostring(root)))


if __name__ == '__main__':
    unittest.main(verbosity=2)

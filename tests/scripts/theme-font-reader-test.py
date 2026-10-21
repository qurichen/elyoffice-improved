#!/usr/bin/env python3
"""Mutation tests for supplemental theme-font checks."""

import importlib.util
import io
from pathlib import Path
import unittest
from xml.etree import ElementTree as ET
from zipfile import ZipFile

spec = importlib.util.spec_from_file_location('theme', Path(__file__).with_name('theme-font-check.py'))
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)
A, W = reader.A, reader.W


def parts(extension='docx'):
    fixture = Path(__file__).resolve().parents[1] / f'fixtures/office/theme-fonts.{extension}'
    with ZipFile(fixture) as original:
        return {name: original.read(name) for name in original.namelist()}


def archive(content):
    target = io.BytesIO()
    with ZipFile(target, 'w') as output:
        for name, data in content.items():
            output.writestr(name, data)
    target.seek(0)
    return target


class ThemeReaderTests(unittest.TestCase):
    def test_complete(self):
        reader.check(archive(parts()))

    def test_split_runs(self):
        content = parts()
        doc = ET.fromstring(content['word/document.xml'])
        run = doc.find(W + 'body/' + W + 'p/' + W + 'r')
        text = run.find(W + 't')
        value = text.text
        text.text = value[:5]
        ET.SubElement(run, W + 't').text = value[5:]
        content['word/document.xml'] = ET.tostring(doc)
        reader.check(archive(content))

    def test_missing_or_changed_fonts(self):
        for label in reader.EXPECTED:
            for change in ('missing', 'value', 'duplicate', 'swapped'):
                with self.subTest(label=label, change=change):
                    content = parts()
                    theme = ET.fromstring(content['word/theme/theme1.xml'])
                    group = theme.find('.//' + A + label)
                    fonts = group.findall(A + 'font')
                    if change == 'missing':
                        group.remove(fonts[0])
                    elif change == 'value':
                        fonts[0].set('typeface', 'Changed')
                    elif change == 'duplicate':
                        group.append(ET.fromstring(ET.tostring(fonts[0])))
                    else:
                        other = 'minorFont' if label == 'majorFont' else 'majorFont'
                        fonts[0].set('typeface', reader.EXPECTED[other][fonts[0].get('script')])
                    content['word/theme/theme1.xml'] = ET.tostring(theme)
                    with self.assertRaises(AssertionError):
                        reader.check(archive(content))

    def test_missing_body(self):
        content = parts()
        root = ET.fromstring(content['word/document.xml'])
        root.find(W + 'body/' + W + 'p/' + W + 'r/' + W + 't').text = 'Wrong'
        content['word/document.xml'] = ET.tostring(root)
        with self.assertRaises(AssertionError):
            reader.check(archive(content))

    def test_external_theme(self):
        content = parts()
        rels = ET.fromstring(content['word/_rels/document.xml.rels'])
        link = next(link for link in rels if link.get('Type').endswith('/theme'))
        link.set('TargetMode', 'External')
        content['word/_rels/document.xml.rels'] = ET.tostring(rels)
        with self.assertRaises(AssertionError):
            reader.check(archive(content))

    def test_other_formats(self):
        for extension in ('xlsx', 'pptx'):
            with self.subTest(extension=extension):
                reader.check(archive(parts(extension)))

    def test_theme_path_variants(self):
        for target in ('/word/theme/moved.xml', './theme/moved.xml'):
            with self.subTest(target=target):
                content = parts()
                content['word/theme/moved.xml'] = content.pop('word/theme/theme1.xml')
                rels = ET.fromstring(content['word/_rels/document.xml.rels'])
                link = next(link for link in rels if link.get('Type').endswith('/theme'))
                link.set('Target', target)
                content['word/_rels/document.xml.rels'] = ET.tostring(rels)
                reader.check(archive(content))

    def test_every_master_theme(self):
        content = parts('pptx')
        presentation = ET.fromstring(content['ppt/presentation.xml'])
        ET.SubElement(presentation.find(reader.SLIDE + 'sldMasterIdLst'), reader.SLIDE + 'sldMasterId',
                      {'id': '2147483649', reader.R + 'id': 'secondMaster'})
        content['ppt/presentation.xml'] = ET.tostring(presentation)
        rels = ET.fromstring(content['ppt/_rels/presentation.xml.rels'])
        ET.SubElement(rels, reader.P + 'Relationship', Id='secondMaster',
                      Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster',
                      Target='slideMasters/second.xml')
        content['ppt/_rels/presentation.xml.rels'] = ET.tostring(rels)
        content['ppt/slideMasters/second.xml'] = content['ppt/slideMasters/slideMaster1.xml']
        rels = ET.fromstring(content['ppt/slideMasters/_rels/slideMaster1.xml.rels'])
        link = next(link for link in rels if link.get('Type').endswith('/theme'))
        link.set('Target', '../theme/second.xml')
        content['ppt/slideMasters/_rels/second.xml.rels'] = ET.tostring(rels)
        content['ppt/theme/second.xml'] = content['ppt/theme/theme1.xml']
        reader.check(archive(content))
        theme = ET.fromstring(content['ppt/theme/second.xml'])
        group = theme.find('.//' + A + 'minorFont')
        group.remove(group.find(A + 'font'))
        content['ppt/theme/second.xml'] = ET.tostring(theme)
        with self.assertRaises(AssertionError):
            reader.check(archive(content))

    def test_missing_master_relationship(self):
        content = parts('pptx')
        rels = ET.fromstring(content['ppt/_rels/presentation.xml.rels'])
        rels.remove(next(link for link in rels if link.get('Type').endswith('/slideMaster')))
        content['ppt/_rels/presentation.xml.rels'] = ET.tostring(rels)
        with self.assertRaises(AssertionError):
            reader.check(archive(content))

    def test_excel_mapping_loss(self):
        for label in reader.EXPECTED:
            with self.subTest(label=label):
                content = parts('xlsx')
                theme = ET.fromstring(content['xl/theme/theme1.xml'])
                group = theme.find('.//' + A + label)
                group.remove(group.find(A + 'font'))
                content['xl/theme/theme1.xml'] = ET.tostring(theme)
                with self.assertRaises(AssertionError):
                    reader.check(archive(content))

    def test_presentation_external_theme(self):
        content = parts('pptx')
        rels = ET.fromstring(content['ppt/slideMasters/_rels/slideMaster1.xml.rels'])
        link = next(link for link in rels if link.get('Type').endswith('/theme'))
        link.set('TargetMode', 'External')
        content['ppt/slideMasters/_rels/slideMaster1.xml.rels'] = ET.tostring(rels)
        with self.assertRaises(AssertionError):
            reader.check(archive(content))


if __name__ == '__main__':
    unittest.main(verbosity=2)

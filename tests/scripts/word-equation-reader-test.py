#!/usr/bin/env python3
"""Mutation tests for the legacy-EQ reader, not engine execution."""

import importlib.util
import io
from pathlib import Path
import unittest
from xml.etree import ElementTree as ET
from zipfile import ZipFile


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


CHECK = module('word-equation-check')
FIXTURE = module('word-equation-fixture')
W, M = CHECK.W, CHECK.M


def run(content):
    node = ET.Element(W + 'r')
    node.append(content)
    return node


def good_document():
    root = ET.fromstring(FIXTURE.fixture()['word/document.xml'])
    for marker in CHECK.EXPECTED:
        p = CHECK.paragraph(root, marker)
        nodes = list(p.iter(W + 'fldChar'))
        if len(nodes) == 2:
            end = next(r for r in p if r.find(W + 'fldChar') is nodes[-1])
            at = list(p).index(end)
            separator = ET.Element(W + 'fldChar', {W + 'fldCharType': 'separate'})
            p.insert(at, run(separator))
            text = ET.Element(W + 't')
            text.text = 'cached current value'
            p.insert(at + 1, run(text))
    for node in root.iter(W + 't'):
        if node.text == 'CACHE_MUST_NOT_DUPLICATE':
            node.text = '(1)/(2)'
    p = CHECK.paragraph(root, 'EQ_ROOT')
    for child in list(p)[1:]:
        p.remove(child)
    math = ET.SubElement(p, M + 'oMath')
    rad = ET.SubElement(math, M + 'rad')
    base = ET.SubElement(rad, M + 'e')
    ET.SubElement(ET.SubElement(base, M + 'r'), M + 't').text = 'z'
    return root


def archive(root):
    target = io.BytesIO()
    with ZipFile(target, 'w') as output:
        output.writestr('word/document.xml', ET.tostring(root))
    target.seek(0)
    return target


def flat_document():
    root = ET.Element('flat')
    for marker in [*CHECK.EXPECTED, 'EQ_ROOT', 'OMML_NATIVE']:
        p = ET.SubElement(root, CHECK.TEXT + 'p')
        span = ET.SubElement(p, CHECK.TEXT + 'span')
        ET.SubElement(span, CHECK.TEXT + 'span').text = marker + ': '
        frame = ET.SubElement(p, CHECK.DRAW + 'frame')
        math = ET.SubElement(ET.SubElement(frame, CHECK.DRAW + 'object'), CHECK.ODF_MATH + 'math')
        semantics = ET.SubElement(math, CHECK.ODF_MATH + 'semantics')
        ET.SubElement(semantics, CHECK.ODF_MATH + 'annotation', encoding='StarMath 5.0').text = \
            '{"1"} over {"2"}'
        if marker != 'OMML_NATIVE':
            for suffix, value in [('', 'true'), ('-font', 'Arial'), ('-size', '11')]:
                ET.SubElement(semantics, CHECK.ODF_MATH + 'annotation',
                              encoding=CHECK.ORIGIN + suffix).text = value
    return root


def odt_document(font='Liberation Serif', height='12'):
    content = flat_document()
    target = io.BytesIO()
    with ZipFile(target, 'w') as output:
        for index, marker in enumerate([*CHECK.EXPECTED, 'EQ_ROOT', 'OMML_NATIVE'], 1):
            p = CHECK.flat_paragraph(content, marker)
            next(p.iter(CHECK.DRAW + 'object')).set(CHECK.XLINK + 'href', f'./Object{index}')
            settings = ET.Element('settings')
            for name, value in [('FontNameText', font), ('BaseFontHeight', height)]:
                ET.SubElement(settings, CHECK.CONFIG + 'config-item',
                              {CHECK.CONFIG + 'name': name}).text = value
            output.writestr(f'Object{index}/settings.xml', ET.tostring(settings))
        output.writestr('content.xml', ET.tostring(content))
    target.seek(0)
    return target


def namespaced_flat():
    body = ''.join(ET.tostring(child, encoding='unicode') for child in flat_document())
    return ('<?xml version="1.0" encoding="UTF-8"?>'
            '<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
            'xmlns:ooo="http://openoffice.org/2004/office" '
            'xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0" '
            'office:mimetype="application/vnd.oasis.opendocument.text">'
            '<office:settings><config:config-item-set config:name="ooo:view-settings"/>'
            '</office:settings><office:body><office:text>' + body +
            '</office:text></office:body></office:document>').encode('utf-8')


class EquationReaderTests(unittest.TestCase):
    def test_complete_document(self):
        CHECK.check_docx(archive(good_document()))

    def test_rejects_field_mutations(self):
        for mutation in ['missing', 'duplicate', 'value', 'size', 'bold', 'italic', 'color',
                         'separator', 'cache', 'wrong_omml', 'following_text']:
            with self.subTest(mutation=mutation):
                root = good_document()
                p = CHECK.paragraph(root, 'EQ_BASIC')
                if mutation == 'missing':
                    root.find(W + 'body').remove(p)
                elif mutation == 'duplicate':
                    root.find(W + 'body').append(ET.fromstring(ET.tostring(p)))
                elif mutation == 'value':
                    next(p.iter(W + 'instrText')).text = r' EQ \f(8,2) '
                elif mutation in ['size', 'bold', 'italic', 'color']:
                    props = next(r for r in p if r.find(W + 'instrText') is not None).find(W + 'rPr')
                    tag = {'size': 'sz', 'bold': 'b', 'italic': 'i', 'color': 'color'}[mutation]
                    props.find(W + tag).set(W + 'val', 'FF0000' if mutation == 'color' else '1')
                elif mutation == 'separator':
                    next(p.iter(W + 'fldChar')).set(W + 'fldCharType', 'end')
                elif mutation == 'cache':
                    next(p.iter(W + 't')).text += 'CACHE_MUST_NOT_DUPLICATE'
                elif mutation == 'wrong_omml':
                    ET.SubElement(p, M + 'oMath')
                else:
                    next(n for n in root.iter(W + 't') if n.text == 'AFTER_EQ_BASIC').text = ''
                with self.assertRaises((AssertionError, AttributeError)):
                    CHECK.check_docx(archive(root))

    def test_rejects_native_and_unsupported_content_loss(self):
        for marker, tag in [('EQ_ROOT', 't'), ('OMML_NATIVE', 'num'), ('OMML_NATIVE', 'den')]:
            with self.subTest(marker=marker, tag=tag):
                root = good_document()
                p = CHECK.paragraph(root, marker)
                target = next(p.iter(M + tag))
                if tag != 't':
                    target = next(target.iter(M + 't'))
                target.text = 'wrong'
                with self.assertRaises(AssertionError):
                    CHECK.check_docx(archive(root))

    def test_current_edited_values_are_required(self):
        root = good_document()
        code = next(CHECK.paragraph(root, 'EQ_BASIC').iter(W + 'instrText'))
        with self.assertRaises(AssertionError):
            CHECK.check_docx(archive(root), 'edited')
        code.text = r' EQ \f(7,9) '
        CHECK.check_docx(archive(root), 'edited')

    def test_literal_escapes_and_nested_fraction(self):
        self.assertEqual(CHECK.Parser(r' EQ \f(a\,b,c\(d\)\\e) ').parse(),
                         ('frac', 'a,b', 'c(d)\\e'))
        self.assertEqual(CHECK.Parser(r' EQ \f(\f(3,4),5) ').parse(),
                         ('frac', ('frac', '3', '4'), '5'))

    def test_rejects_truncated_and_unexpected_commands(self):
        for code in [r' EQ \f(1)', r' EQ \f(1,2', r' EQ \g(1)', r' EQ \f(1,2)TAIL']:
            with self.subTest(code=code), self.assertRaises((AssertionError, IndexError)):
                CHECK.Parser(code).parse()

    def test_flat_span_markers_and_current_model_edit(self):
        root = flat_document()
        CHECK.check_flat(io.BytesIO(ET.tostring(root)))
        output = io.BytesIO()
        CHECK.edit_flat(io.BytesIO(ET.tostring(root)), output)
        output.seek(0)
        edited = ET.parse(output).getroot()
        p = CHECK.flat_paragraph(edited, 'EQ_BASIC')
        annotation = next(n for n in p.iter(CHECK.ODF_MATH + 'annotation')
                          if n.get('encoding') == 'StarMath 5.0')
        self.assertEqual(annotation.text, '{"7"} over {"9"}')

    def test_flat_markers_cannot_be_found_inside_math_objects(self):
        root = flat_document()
        p = CHECK.flat_paragraph(root, 'EQ_BASIC')
        p.find('.//' + CHECK.TEXT + 'span/' + CHECK.TEXT + 'span').text = 'unrelated: '
        next(p.iter(CHECK.ODF_MATH + 'annotation')).text = 'EQ_BASIC: not a visible marker'
        with self.assertRaisesRegex(AssertionError, 'EQ_BASIC: expected one visible flat marker'):
            CHECK.flat_paragraph(root, 'EQ_BASIC')

    def test_flat_rejects_lost_or_misattributed_origin(self):
        for mutation in ['missing', 'wrong', 'font', 'size']:
            root = flat_document()
            p = CHECK.flat_paragraph(root, 'EQ_BASIC')
            annotations = list(p.iter(CHECK.ODF_MATH + 'annotation'))
            if mutation == 'missing':
                next(a for a in annotations if a.get('encoding') == CHECK.ORIGIN).text = 'false'
            else:
                suffix = {'wrong': '', 'font': '-font', 'size': '-size'}[mutation]
                next(a for a in annotations if a.get('encoding') == CHECK.ORIGIN + suffix).text = 'wrong'
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                CHECK.check_flat(io.BytesIO(ET.tostring(root)))

    def test_unmarked_and_false_metadata_controls(self):
        for variant in ['plain', 'absent', 'false']:
            result = io.BytesIO()
            CHECK.poison_flat(io.BytesIO(ET.tostring(flat_document())), result, variant)
            result.seek(0)
            root = ET.parse(result).getroot()
            annotations = list(root.iter(CHECK.ODF_MATH + 'annotation'))
            origins = [n for n in annotations if n.get('encoding') == CHECK.ORIGIN]
            self.assertFalse(any(n.text == 'true' for n in origins))
            self.assertEqual(len(origins), 4 if variant == 'false' else 0)

    def test_flat_mutations_preserve_detection_and_qname_declarations(self):
        source = namespaced_flat()
        before = [(n.tag, n.attrib, n.text, n.tail) for n in ET.fromstring(source).iter()
                  if n.tag != CHECK.ODF_MATH + 'annotation']
        cases = [('edit', v) for v in ['edited', 'spacing', 'mixed', 'leading', 'blank', 'quote']]
        cases += [('poison', v) for v in ['plain', 'absent', 'false']]
        for mode, variant in cases:
            with self.subTest(mode=mode, variant=variant):
                result = io.BytesIO()
                getattr(CHECK, mode + '_flat')(io.BytesIO(source), result, variant)
                payload = result.getvalue()
                self.assertIn(b'office:mimetype="application/vnd.oasis.opendocument.text"', payload[:4000])
                self.assertIn(b'xmlns:ooo="http://openoffice.org/2004/office"', payload[:4000])
                self.assertIn(b'config:name="ooo:view-settings"', payload)
                after = [(n.tag, n.attrib, n.text, n.tail) for n in ET.fromstring(payload).iter()
                         if n.tag != CHECK.ODF_MATH + 'annotation']
                self.assertEqual(after, before)

    def test_unmarked_styles_compare_current_model_properties(self):
        CHECK.check_unmarked_styles(odt_document(), odt_document())
        for font, height in [('Poisoned Candidate Font', '12'), ('Liberation Serif', '19')]:
            with self.subTest(font=font, height=height), self.assertRaises(AssertionError):
                CHECK.check_unmarked_styles(odt_document(), odt_document(font, height))

    def test_refused_models_keep_their_actual_omml_content(self):
        cases = {'spacing': 'AB', 'mixed': '12', 'leading': '  A', 'blank': '   ', 'quote': 'A"B'}
        for variant, value in cases.items():
            root = good_document()
            p = CHECK.paragraph(root, 'EQ_BASIC')
            for child in list(p)[1:]:
                p.remove(child)
            math = ET.SubElement(p, M + 'oMath')
            text = ET.SubElement(ET.SubElement(math, M + 'r'), M + 't')
            text.text = value
            CHECK.check_docx(archive(root), variant)
            text.text = 'lost or changed'
            with self.subTest(variant=variant), self.assertRaises(AssertionError):
                CHECK.check_docx(archive(root), variant)


if __name__ == '__main__':
    unittest.main()

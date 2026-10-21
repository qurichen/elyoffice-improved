#!/usr/bin/env python3
"""Independent legacy-EQ AST and source-kind roundtrip assertions."""

import sys
import posixpath
from pathlib import Path
from xml.dom import minidom
from xml.etree import ElementTree as ET
from zipfile import ZipFile

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
M = '{http://schemas.openxmlformats.org/officeDocument/2006/math}'
ODF_MATH = '{http://www.w3.org/1998/Math/MathML}'
TEXT = '{urn:oasis:names:tc:opendocument:xmlns:text:1.0}'
DRAW = '{urn:oasis:names:tc:opendocument:xmlns:drawing:1.0}'
CONFIG = '{urn:oasis:names:tc:opendocument:xmlns:config:1.0}'
XLINK = '{http://www.w3.org/1999/xlink}'
ORIGIN = 'application/vnd.elyoffice.word-eq'
EXPECTED = {
    'EQ_BASIC': ('frac', '1', '2'),
    'EQ_NESTED': ('frac', ('frac', '3', '4'), '5'),
    'EQ_ESCAPED': ('frac', 'a,b', 'c(d)\\e'),
}


class Parser:
    def __init__(self, text):
        assert text.lstrip().upper().startswith('EQ '), 'not an EQ field'
        self.text = text.lstrip()[2:].lstrip()
        self.at = 0

    def argument(self):
        parts = []
        literal = ''
        while self.at < len(self.text):
            char = self.text[self.at]
            if char in ',)':
                break
            if self.text.startswith('\\f(', self.at):
                if literal:
                    parts.append(literal)
                    literal = ''
                self.at += 3
                numerator = self.argument()
                assert self.text[self.at] == ',', 'missing fraction separator'
                self.at += 1
                denominator = self.argument()
                assert self.text[self.at] == ')', 'missing fraction end'
                self.at += 1
                parts.append(('frac', numerator, denominator))
                continue
            if char == '\\':
                self.at += 1
                assert self.at < len(self.text), 'truncated escape'
                char = self.text[self.at]
                assert char in '\\(),', 'unsupported command'
            literal += char
            self.at += 1
        if literal.strip():
            parts.append(literal)
        assert len(parts) == 1, f'unexpected expression: {parts!r}'
        return parts[0]

    def parse(self):
        result = self.argument()
        assert self.at == len(self.text), 'trailing field data'
        return result


def document(source):
    with ZipFile(source) as archive:
        root = ET.fromstring(archive.read('word/document.xml'))
    return root


def paragraph(root, marker):
    matches = [p for p in root.iter(W + 'p')
               if ''.join(t.text or '' for t in p.iter(W + 't')).startswith(marker + ':')]
    assert len(matches) == 1, f'{marker}: missing or duplicated paragraph'
    return matches[0]


def check_docx(source, variant='normal'):
    root = document(source)
    body = ''.join(t.text or '' for t in root.iter(W + 't'))
    assert 'CACHE_MUST_NOT_DUPLICATE' not in body, 'cached result duplicated'
    for marker, expected in EXPECTED.items():
        p = paragraph(root, marker)
        if marker == 'EQ_BASIC' and variant in ['spacing', 'mixed', 'leading', 'blank', 'quote']:
            assert len(list(p.iter(M + 'oMath'))) == 1, 'lossy edited AST was exported as EQ'
            assert not list(p.iter(W + 'instrText')), 'unsupported edit kept EQ'
            actual = ''.join(n.text or '' for n in p.iter(M + 't'))
            if variant == 'mixed':
                assert actual == '12', 'mixed-size operands lost'
            else:
                expected_text = {'spacing': 'AB', 'leading': '  A', 'blank': '   ', 'quote': 'A"B'}
                assert actual == expected_text[variant], 'unsupported edit text changed'
            continue
        kinds = [n.get(W + 'fldCharType') for n in p.iter(W + 'fldChar')]
        assert kinds == ['begin', 'separate', 'end'], (marker, kinds)
        assert not list(p.iter(M + 'oMath')), marker + ': still changed to OMML'
        codes = list(p.iter(W + 'instrText'))
        assert codes, marker + ': no editable instruction'
        code = ''.join(t.text or '' for t in codes)
        if variant == 'edited' and marker == 'EQ_BASIC':
            expected = ('frac', '7', '9')
        assert Parser(code).parse() == expected, (marker, code)
        for run in p.iter(W + 'r'):
            if run.find(W + 'instrText') is not None:
                size = run.find(W + 'rPr/' + W + 'sz')
                assert size is not None and size.get(W + 'val') == '22', 'font size lost'
                props = run.find(W + 'rPr')
                assert props.find(W + 'b').get(W + 'val') == '0', 'inherited bold leaks'
                assert props.find(W + 'i').get(W + 'val') == '0', 'inherited italic leaks'
                assert props.find(W + 'color').get(W + 'val') in ['auto', '000000'], 'color leaks'
    for marker in ['EQ_ROOT', 'OMML_NATIVE']:
        p = paragraph(root, marker)
        assert len(list(p.iter(M + 'oMath'))) == 1, marker + ': native OMML was rewritten'
        assert not list(p.iter(W + 'instrText')), marker + ': unsupported EQ export'
        if marker == 'EQ_ROOT':
            rad = p.find('.//' + M + 'rad')
            assert rad is not None and ''.join(n.text or '' for n in rad.iter(M + 't')) == 'z'
        else:
            fraction = p.find('.//' + M + 'f')
            assert fraction is not None
            assert ''.join(n.text or '' for n in fraction.find(M + 'num').iter(M + 't')) == '9'
            assert ''.join(n.text or '' for n in fraction.find(M + 'den').iter(M + 't')) == '10'
    for marker in [*EXPECTED, 'EQ_ROOT', 'OMML_NATIVE']:
        assert body.count('AFTER_' + marker) == 1, 'missing or repeated following text'


def visible_prefix(paragraph):
    pieces = []

    def visit(node):
        if node.tag in [DRAW + 'frame', DRAW + 'object', ODF_MATH + 'math']:
            return False
        if node.text:
            pieces.append(node.text)
        for child in node:
            if not visit(child):
                return False
            if child.tail:
                pieces.append(child.tail)
        return True

    visit(paragraph)
    return ''.join(pieces)


def flat_paragraph(root, marker):
    matches = [p for p in root.iter(TEXT + 'p') if visible_prefix(p).startswith(marker + ':')]
    assert len(matches) == 1, f'{marker}: expected one visible flat marker, found {len(matches)}'
    return matches[0]


def check_flat(source):
    root = ET.parse(source).getroot()
    body = ''.join(root.itertext())
    assert 'CACHE_MUST_NOT_DUPLICATE' not in body, 'cached result duplicated'
    markers = [n for n in root.iter(ODF_MATH + 'annotation') if n.get('encoding') == ORIGIN]
    assert len(markers) == 4 and all(n.text == 'true' for n in markers), 'source-kind marker lost'
    for marker in [*EXPECTED, 'EQ_ROOT', 'OMML_NATIVE']:
        annotations = list(flat_paragraph(root, marker).iter(ODF_MATH + 'annotation'))
        origins = [a for a in annotations if a.get('encoding') == ORIGIN]
        if marker == 'OMML_NATIVE':
            assert not origins, 'ordinary OMML acquired an EQ marker'
        else:
            assert len(origins) == 1 and origins[0].text == 'true'
            for suffix, value in [('-font', 'Arial'), ('-size', '11')]:
                props = [a for a in annotations if a.get('encoding') == ORIGIN + suffix]
                assert len(props) == 1 and props[0].text == value, 'flat model style lost'


def editable_flat(source):
    document = minidom.parse(source if hasattr(source, 'read') else str(source))
    root = ET.fromstring(document.toxml())
    annotations = list(root.iter(ODF_MATH + 'annotation'))
    nodes = list(document.getElementsByTagNameNS(ODF_MATH[1:-1], 'annotation'))
    assert len(annotations) == len(nodes), 'flat annotation mapping changed'
    return root, document, dict(zip(annotations, nodes))


def set_annotation(document, node, value):
    for child in list(node.childNodes):
        node.removeChild(child)
    node.appendChild(document.createTextNode(value))


def write_flat(document, target):
    payload = document.toxml(encoding='utf-8')
    if hasattr(target, 'write'):
        target.write(payload)
    else:
        Path(target).write_bytes(payload)


def edit_flat(source, target, variant='edited'):
    root, document, nodes = editable_flat(source)
    p = flat_paragraph(root, 'EQ_BASIC')
    annotation = next(n for n in p.iter(ODF_MATH + 'annotation')
                      if n.get('encoding', '').startswith('StarMath'))
    assert '&quot;' not in annotation.text, 'annotation must be decoded'
    assert '"1"' in annotation.text and '"2"' in annotation.text
    if variant == 'edited':
        annotation.text = annotation.text.replace('"1"', '"7"').replace('"2"', '"9"')
    elif variant == 'spacing':
        annotation.text = '"A" "B"'
    elif variant == 'mixed':
        annotation.text = '{size 18 {"1"}} over {"2"}'
    elif variant == 'leading':
        annotation.text = '"  A"'
    elif variant == 'blank':
        annotation.text = '"   "'
    elif variant == 'quote':
        annotation.text = '"A\\"B"'
    else:
        raise ValueError('unknown edit variant')
    set_annotation(document, nodes[annotation], annotation.text)
    write_flat(document, target)


def poison_flat(source, target, variant):
    assert variant in ['plain', 'absent', 'false'], 'unknown metadata control'
    root, document, nodes = editable_flat(source)
    for parent in root.iter():
        for child in list(parent):
            if child.tag != ODF_MATH + 'annotation':
                continue
            encoding = child.get('encoding', '')
            if encoding == ORIGIN:
                if variant == 'false':
                    set_annotation(document, nodes[child], 'false')
                else:
                    nodes[child].parentNode.removeChild(nodes[child])
            elif encoding in [ORIGIN + '-font', ORIGIN + '-size']:
                if variant == 'plain':
                    nodes[child].parentNode.removeChild(nodes[child])
                else:
                    value = 'Poisoned Candidate Font' if encoding.endswith('-font') else '19'
                    set_annotation(document, nodes[child], value)
    write_flat(document, target)


def odt_styles(source):
    result = {}
    with ZipFile(source) as archive:
        content = ET.fromstring(archive.read('content.xml'))
        for marker in [*EXPECTED, 'EQ_ROOT', 'OMML_NATIVE']:
            paragraph = flat_paragraph(content, marker)
            objects = list(paragraph.iter(DRAW + 'object'))
            assert len(objects) == 1, marker + ': expected one math object'
            href = objects[0].get(XLINK + 'href', '')
            part = posixpath.normpath(href)
            assert part and ':' not in part and not part.startswith(('/', '..')), 'external math object'
            settings = ET.fromstring(archive.read(part + '/settings.xml'))
            values = {}
            for name in ['FontNameText', 'BaseFontHeight']:
                items = [n for n in settings.iter(CONFIG + 'config-item')
                         if n.get(CONFIG + 'name') == name]
                assert len(items) == 1 and items[0].text, 'missing current math property'
                values[name] = items[0].text
            result[marker] = values
    return result


def check_unmarked_styles(baseline, candidate):
    expected = odt_styles(baseline)
    actual = odt_styles(candidate)
    assert actual == expected, 'unmarked private metadata changed the current math model'
    assert all(value['FontNameText'] != 'Poisoned Candidate Font' for value in actual.values())


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == 'docx':
        check_docx(sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else 'normal')
    elif mode == 'flat':
        check_flat(sys.argv[2])
    elif mode == 'edit':
        edit_flat(*sys.argv[2:])
    elif mode == 'poison':
        poison_flat(*sys.argv[2:])
    elif mode == 'odf-styles':
        check_unmarked_styles(*sys.argv[2:])
    else:
        raise SystemExit('usage: word-equation-check.py docx|flat|edit paths...')
    print('PASS:', mode)

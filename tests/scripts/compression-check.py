#!/usr/bin/env python3
"""Checks actual compression settings before and after DOCX."""

import sys
import zipfile
from xml.etree import ElementTree as ET

CONFIG = '{urn:oasis:names:tc:opendocument:xmlns:config:1.0}'
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
MODES = {'doNotCompress': 0, 'compressPunctuation': 1,
         'compressPunctuationAndJapaneseKana': 2, 'absent': 0}


def flat(path, expected):
    settings = [item for item in ET.parse(path).iter(CONFIG + 'config-item')
                if item.get(CONFIG + 'name') == 'CharacterCompressionType']
    assert len(settings) == 1 and int(settings[0].text) == expected, (path, 'compression model differs')


def check(mode, imported, saved, reopened):
    expected = MODES[mode]
    flat(imported, expected)
    with zipfile.ZipFile(saved) as archive:
        settings = ET.fromstring(archive.read('word/settings.xml'))
        setting = settings.find(W + 'characterSpacingControl')
        actual = setting.get(W + 'val') if setting is not None else 'doNotCompress'
        assert actual == ('doNotCompress' if mode == 'absent' else mode), ('DOCX compression differs', actual, mode)
        body = ET.fromstring(archive.read('word/document.xml'))
        assert 'COMPRESSION_MARKER' in ''.join(body.itertext()), 'body lost'
    flat(reopened, expected)


if __name__ == '__main__':
    if len(sys.argv) != 5 or sys.argv[1] not in MODES:
        sys.exit('usage: compression-check.py mode imported.fodt saved.docx reopened.fodt')
    check(*sys.argv[1:])
    print('compression import, DOCX mode and reopened model: correct')

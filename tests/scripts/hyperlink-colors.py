#!/usr/bin/env python3
"""Checks imported and exported hyperlink colors independently."""

import sys
import zipfile
import xml.etree.ElementTree as ET

A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
AHYP = "{http://schemas.microsoft.com/office/drawing/2018/hyperlinkcolor}"
TEXT = "{urn:oasis:names:tc:opendocument:xmlns:text:1.0}"
STYLE = "{urn:oasis:names:tc:opendocument:xmlns:style:1.0}"
FO = "{urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0}"
EXPECTED = {"KEEP_BLACK": "000000", "KEEP_RED": "CC0000",
            "THEME_BLUE": "0000FF", "HLINK_MODE": "0000FF",
            "UNKNOWN_EXTENSION": "0000FF"}


def flat(path):
    tree = ET.parse(path)
    styles = {node.get(STYLE + "name"): node
              for node in tree.iter(STYLE + "style")}
    found = {}
    for span in tree.iter(TEXT + "span"):
        marker = "".join(span.itertext())
        if marker not in EXPECTED:
            continue
        style = styles[span.get(TEXT + "style-name")]
        props = style.find(STYLE + "text-properties")
        found[marker] = props.get(FO + "color").lstrip("#").upper()
    assert found == EXPECTED, (path, found, EXPECTED)


def pptx(path):
    found = {}
    with zipfile.ZipFile(path) as archive:
        tree = ET.fromstring(archive.read("ppt/slides/slide1.xml"))
        for run in tree.iter(A + "r"):
            marker = run.findtext(A + "t")
            if marker not in EXPECTED:
                continue
            props = run.find(A + "rPr")
            link = props.find(A + "hlinkClick")
            assert link is not None, marker
            color = props.find(A + "solidFill/" + A + "srgbClr")
            scheme = props.find(A + "solidFill/" + A + "schemeClr")
            if marker in ("THEME_BLUE", "HLINK_MODE", "UNKNOWN_EXTENSION"):
                assert scheme is not None and scheme.get("val") == "hlink", marker
            else:
                assert color is not None, marker
            found[marker] = (color.get("val").upper() if color is not None
                             else {"hlink": "0000FF"}[scheme.get("val")])
            mode = link.find(".//" + AHYP + "hlinkClr")
            assert mode is not None and mode.get("val") == "tx", marker
    assert found == EXPECTED, (path, found, EXPECTED)


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit("usage: hyperlink-colors.py imported.fodp saved.pptx reopened.fodp")
    flat(sys.argv[1])
    pptx(sys.argv[2])
    flat(sys.argv[3])
    print("hyperlink colors: import, export and reopen agree")

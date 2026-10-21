#!/usr/bin/env python3
"""Adds color modes to the synthetic PowerPoint fixture."""

import sys
import zipfile
from defusedxml.minidom import parseString

A = "http://schemas.openxmlformats.org/drawingml/2006/main"
URI = "{A12FA001-AC4F-418D-AE19-62706E023703}"
AHYP = "http://schemas.microsoft.com/office/drawing/2018/hyperlinkcolor"


def prepared(name, data):
    if name == "ppt/slides/slide1.xml":
        doc = parseString(data)
        for run in doc.getElementsByTagNameNS(A, "r"):
            text = run.getElementsByTagNameNS(A, "t")[0].firstChild.data
            link = run.getElementsByTagNameNS(A, "hlinkClick")[0]
            for child in list(link.childNodes):
                link.removeChild(child)
            if text == "THEME_BLUE":
                continue
            extensions = doc.createElementNS(A, "a:extLst")
            extension = doc.createElementNS(A, "a:ext")
            extension.setAttribute("uri", URI)
            extensions.appendChild(extension)
            if text == "UNKNOWN_EXTENSION":
                color = doc.createElementNS("urn:ely:qa", "qa:unknown")
                color.setAttribute("xmlns:qa", "urn:ely:qa")
            else:
                color = doc.createElementNS(AHYP, "ahyp:hlinkClr")
                color.setAttribute("xmlns:ahyp", AHYP)
                color.setAttribute("val", "hlink" if text == "HLINK_MODE" else "tx")
            extension.appendChild(color)
            link.appendChild(extensions)
        return doc.toxml(encoding="utf-8")
    if name.startswith("ppt/theme/") and name.endswith(".xml"):
        doc = parseString(data)
        for color in doc.getElementsByTagNameNS(A, "hlink"):
            value = color.getElementsByTagNameNS(A, "srgbClr")[0]
            value.setAttribute("val", "0000FF")
        return doc.toxml(encoding="utf-8")
    return data


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] == sys.argv[2]:
        sys.exit("usage: hyperlink-fixture.py base.pptx output.pptx")
    with zipfile.ZipFile(sys.argv[1]) as source, zipfile.ZipFile(sys.argv[2], "w") as output:
        for item in source.infolist():
            output.writestr(item, prepared(item.filename, source.read(item)))

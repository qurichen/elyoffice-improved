#!/usr/bin/env python3
"""Writes the demo DOCX, XLSX, PPTX and Markdown into argv[1]."""
import sys
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.util import Emu, Inches, Pt

OUT = Path(sys.argv[1])
OUT.mkdir(parents=True, exist_ok=True)

RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
DOC_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
S = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"


def package(path, parts):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, text in parts.items():
            archive.writestr(name, text)


def types(overrides):
    rows = "".join(f'<Override PartName="{p}" ContentType="{t}"/>' for p, t in overrides)
    return ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            f'<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            f'<Default Extension="xml" ContentType="application/xml"/>{rows}</Types>')


def rels(rows):
    body = "".join(f'<Relationship Id="{i}" Type="{DOC_REL}/{t}" Target="{target}"/>' for i, t, target in rows)
    return f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}">{body}</Relationships>'


# DOCX: a one-page project brief.
def run(text, bold=False, color=None):
    props = ("<w:b/>" if bold else "") + (f'<w:color w:val="{color}"/>' if color else "")
    return f'<w:r><w:rPr>{props}</w:rPr><w:t xml:space="preserve">{escape(text)}</w:t></w:r>'


def para(text, style=None, runs=None):
    pstyle = f'<w:pPr><w:pStyle w:val="{style}"/></w:pPr>' if style else ""
    return f"<w:p>{pstyle}{runs or run(text)}</w:p>"


def cell(text, width, header=False):
    shade = '<w:shd w:val="clear" w:color="auto" w:fill="EEF2F0"/>' if header else ""
    return (f'<w:tc><w:tcPr><w:tcW w:w="{width}" w:type="dxa"/>{shade}</w:tcPr>'
            f'<w:p><w:pPr><w:spacing w:before="60" w:after="60"/></w:pPr>{run(text, bold=header)}</w:p></w:tc>')


def table(rows, widths):
    border = "".join(f'<w:{side} w:val="single" w:sz="4" w:space="0" w:color="B8C4BE"/>'
                     for side in ("top", "left", "bottom", "right", "insideH", "insideV"))
    grid = "".join(f'<w:gridCol w:w="{w}"/>' for w in widths)
    body = "".join("<w:tr>" + "".join(cell(t, w, i == 0) for t, w in zip(row, widths)) + "</w:tr>"
                   for i, row in enumerate(rows))
    return (f'<w:tbl><w:tblPr><w:tblW w:w="{sum(widths)}" w:type="dxa"/><w:tblBorders>{border}</w:tblBorders>'
            f'<w:tblCellMar><w:left w:w="120" w:type="dxa"/><w:right w:w="120" w:type="dxa"/></w:tblCellMar>'
            f'</w:tblPr><w:tblGrid>{grid}</w:tblGrid>{body}</w:tbl>')


def style(sid, name, font, size, color="1F2A26", bold=False, before=0, after=160, based=None, level=None):
    b = "<w:b/>" if bold else ""
    outline = f'<w:keepNext/><w:outlineLvl w:val="{level}"/>' if level is not None else ""
    base = f'<w:basedOn w:val="{based}"/>' if based else ""
    return (f'<w:style w:type="paragraph" w:styleId="{sid}"><w:name w:val="{name}"/>{base}<w:qFormat/>'
            f'<w:pPr><w:spacing w:before="{before}" w:after="{after}" w:line="300" w:lineRule="auto"/>{outline}</w:pPr>'
            f'<w:rPr><w:rFonts w:ascii="{font}" w:hAnsi="{font}" w:cs="{font}"/>{b}'
            f'<w:color w:val="{color}"/><w:sz w:val="{size}"/></w:rPr></w:style>')


def docx(path):
    styles = (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{W}">'
              '<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Georgia" w:hAnsi="Georgia" w:cs="Georgia"/>'
              '<w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault></w:docDefaults>'
              + style("Normal", "Normal", "Georgia", 22, "2B3330")
              + style("Title", "Title", "Helvetica Neue", 52, "1F3D35", True, 0, 80)
              + style("Subtitle", "Subtitle", "Helvetica Neue", 24, "5E6E68", False, 0, 360)
              + style("Heading1", "heading 1", "Helvetica Neue", 30, "2F6B5B", True, 320, 120, level=0)
              + "</w:styles>")
    body = "".join([
        para("Coastal Trails Field Guide", "Title"),
        para("Project brief · Spring edition · Draft 3", "Subtitle"),
        para("Purpose", "Heading1"),
        para("The field guide maps forty walks along the northern coast, from short harbor loops to "
             "full-day cliff routes. Each walk gets a hand-drawn map, honest timings, tide notes and "
             "a short story about the place. The spring edition adds twelve new routes and a fold-out "
             "overview map."),
        para("Goals for this edition", "Heading1"),
        para("", runs=run("Accurate. ", bold=True) + run("Every route is walked twice before it goes to print.")),
        para("", runs=run("Readable. ", bold=True) + run("One walk per spread, with the map and the notes side by side.")),
        para("", runs=run("Durable. ", bold=True) + run("Water-resistant paper and a lay-flat binding for the trail.")),
        para("Schedule", "Heading1"),
        table([["Phase", "Owner", "Due"],
               ["Route surveys", "Maya Lind", "12 March"],
               ["Maps and illustrations", "Theo Brandt", "2 April"],
               ["Copy edit and proofs", "Ines Okafor", "23 April"],
               ["Print and distribution", "Sam Reyes", "14 May"]],
              [3600, 2800, 2600]),
        para(""),
        para("Notes: ", runs=run("Next review: ", bold=True) + run("Thursday, with the first six route maps.")),
    ])
    sect = ('<w:sectPr><w:pgSz w:w="11906" w:h="16838"/>'
            '<w:pgMar w:top="1300" w:right="1300" w:bottom="1300" w:left="1300" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr>')
    document = (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                f'<w:document xmlns:w="{W}"><w:body>{body}{sect}</w:body></w:document>')
    main = "application/vnd.openxmlformats-officedocument.wordprocessingml"
    package(path, {
        "[Content_Types].xml": types([("/word/document.xml", f"{main}.document.main+xml"),
                                      ("/word/styles.xml", f"{main}.styles+xml")]),
        "_rels/.rels": rels([("r1", "officeDocument", "word/document.xml")]),
        "word/_rels/document.xml.rels": rels([("r1", "styles", "styles.xml")]),
        "word/document.xml": document,
        "word/styles.xml": styles,
    })


# XLSX: half-year sales by product line.
MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]
SALES = {
    "Trail Maps": [4820, 5140, 6380, 7120, 7950, 8610],
    "Guidebooks": [6150, 5980, 7210, 8040, 8820, 9470],
    "Prints": [2310, 2475, 2890, 3120, 3460, 3780],
    "Workshops": [1800, 2100, 2650, 3400, 3900, 4350],
    "Postcards": [940, 870, 1120, 1360, 1580, 1710],
    "Gift Cards": [1250, 980, 760, 820, 1430, 2050],
}


def col(i):
    return "ABCDEFGH"[i]


def xlsx(path):
    def text(ref, value, s):
        return f'<c r="{ref}" t="inlineStr" s="{s}"><is><t>{escape(value)}</t></is></c>'

    def num(ref, value, s):
        return f'<c r="{ref}" s="{s}"><v>{value}</v></c>'

    rows = ['<row r="1" ht="22" customHeight="1">' + text("A1", "Product", 1)
            + "".join(text(f"{col(i + 1)}1", m, 2) for i, m in enumerate(MONTHS)) + text("H1", "Total", 2) + "</row>"]
    for r, (line, values) in enumerate(SALES.items(), start=2):
        cells = text(f"A{r}", line, 3) + "".join(num(f"{col(i + 1)}{r}", v, 4) for i, v in enumerate(values))
        cells += f'<c r="H{r}" s="5"><f>SUM(B{r}:G{r})</f><v>{sum(values)}</v></c>'
        rows.append(f'<row r="{r}" ht="20" customHeight="1">{cells}</row>')
    rows.append('<row r="9" ht="22" customHeight="1">' + text("A9", "All lines", 1)
                + "".join(f'<c r="{col(i + 1)}9" s="5"/>' for i in range(7)) + "</row>")
    sheet = (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="{S}">'
             '<sheetViews><sheetView workbookViewId="0" showGridLines="1"/></sheetViews>'
             '<sheetFormatPr defaultRowHeight="20"/>'
             '<cols><col min="1" max="1" width="16" customWidth="1"/><col min="2" max="8" width="12" customWidth="1"/></cols>'
             f'<sheetData>{"".join(rows)}</sheetData></worksheet>')
    styles = (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="{S}">'
              '<numFmts count="1"><numFmt numFmtId="164" formatCode="&quot;$&quot;#,##0"/></numFmts>'
              '<fonts count="3"><font><sz val="12"/><name val="Helvetica Neue"/></font>'
              '<font><b/><sz val="12"/><color rgb="FF1F3D35"/><name val="Helvetica Neue"/></font>'
              '<font><b/><sz val="12"/><name val="Helvetica Neue"/></font></fonts>'
              '<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>'
              '<fill><patternFill patternType="solid"><fgColor rgb="FFE4EEE9"/></patternFill></fill></fills>'
              '<borders count="1"><border/></borders>'
              '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>'
              '<cellXfs count="6"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>'
              '<xf numFmtId="0" fontId="1" fillId="2" borderId="0" applyFont="1" applyFill="1"/>'
              '<xf numFmtId="0" fontId="1" fillId="2" borderId="0" applyFont="1" applyFill="1" applyAlignment="1"><alignment horizontal="right"/></xf>'
              '<xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>'
              '<xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>'
              '<xf numFmtId="164" fontId="2" fillId="0" borderId="0" applyNumberFormat="1" applyFont="1"/></cellXfs>'
              '</styleSheet>')
    workbook = (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="{S}" xmlns:r="{DOC_REL}">'
                '<sheets><sheet name="Sales" sheetId="1" r:id="r1"/></sheets><calcPr fullCalcOnLoad="1"/></workbook>')
    main = "application/vnd.openxmlformats-officedocument.spreadsheetml"
    package(path, {
        "[Content_Types].xml": types([("/xl/workbook.xml", f"{main}.sheet.main+xml"),
                                      ("/xl/worksheets/sheet1.xml", f"{main}.worksheet+xml"),
                                      ("/xl/styles.xml", f"{main}.styles+xml")]),
        "_rels/.rels": rels([("r1", "officeDocument", "xl/workbook.xml")]),
        "xl/_rels/workbook.xml.rels": rels([("r1", "worksheet", "worksheets/sheet1.xml"),
                                            ("r2", "styles", "styles.xml")]),
        "xl/workbook.xml": workbook,
        "xl/worksheets/sheet1.xml": sheet,
        "xl/styles.xml": styles,
    })


# PPTX: four simple slides.
def pptx(path):
    deck = Presentation()
    deck.slide_width, deck.slide_height = Emu(12192000), Emu(6858000)
    ink, accent = RGBColor(0x1F, 0x3D, 0x35), RGBColor(0x2F, 0x6B, 0x5B)

    def paint(frame, size, color, bold=False):
        for p in frame.paragraphs:
            for r in p.runs:
                r.font.size, r.font.color.rgb, r.font.bold, r.font.name = Pt(size), color, bold, "Helvetica Neue"

    def place(shape, left, top, width, height):
        shape.left, shape.top, shape.width, shape.height = Inches(left), Inches(top), Inches(width), Inches(height)

    def bar(slide, left, top, width, height):
        shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(left), Inches(top), Inches(width), Inches(height))
        shape.fill.solid()
        shape.fill.fore_color.rgb = accent
        shape.line.fill.background()

    title = deck.slides.add_slide(deck.slide_layouts[0])
    title.shapes.title.text = "Coastal Trails 2027"
    title.placeholders[1].text = "Spring edition · Planning review"
    place(title.shapes.title, 1.0, 2.2, 11.33, 1.4)
    place(title.placeholders[1], 1.0, 3.7, 11.33, 0.8)
    paint(title.shapes.title.text_frame, 54, ink, True)
    paint(title.placeholders[1].text_frame, 24, accent)
    bar(title, 0, 6.9, 13.333, 0.6)
    slides = [
        ("What is new", ["Twelve new routes on the northern cliffs", "A fold-out overview map",
                          "Tide tables for every harbor walk", "Water-resistant, lay-flat binding"]),
        ("Half-year sales", ["Guidebooks lead at $45,670", "Workshops more than doubled since January",
                              "Trail Maps up 79% since January", "Prints steady at about $3,000 a month"]),
        ("Next steps", ["Finish route surveys by 12 March", "Maps and illustrations by 2 April",
                        "Proofs to the printer on 23 April", "Launch walk on 14 May"]),
    ]
    for heading, bullets in slides:
        slide = deck.slides.add_slide(deck.slide_layouts[1])
        slide.shapes.title.text = heading
        place(slide.shapes.title, 1.3, 0.6, 10.73, 1.1)
        paint(slide.shapes.title.text_frame, 40, ink, True)
        bar(slide, 1.0, 0.75, 0.12, 0.8)
        body = slide.placeholders[1]
        place(body, 1.3, 2.0, 10.73, 4.6)
        body.text_frame.text = bullets[0]
        for line in bullets[1:]:
            body.text_frame.add_paragraph().text = line
        paint(body.text_frame, 28, RGBColor(0x2B, 0x33, 0x30))
    deck.core_properties.author = "Coastal Trails"
    deck.save(path)


MARKDOWN = r"""# Trail notes: Harbor Loop

A short walk around the old harbor, best at low tide. The path follows the sea wall,
climbs to the lighthouse and comes back through the dunes.

## Before you go

- [x] Check the tide table
- [x] Pack water and a windproof layer
- [ ] Download the offline map
- [ ] Tell someone your route

## Route at a glance

| Section          | Distance | Time   |
| ---------------- | -------- | ------ |
| Sea wall         | 1.2 km   | 15 min |
| Lighthouse climb | 0.8 km   | 20 min |
| Dune path        | 2.1 km   | 30 min |

## Pace

Walking time is distance over pace, plus a minute for every ten meters of climb:

$$
t = \frac{d}{v} + \frac{h}{10}
$$

## Plan

```mermaid
flowchart LR
  A[Harbor] --> B[Sea wall]
  B --> C[Lighthouse]
  C --> D[Dunes]
  D --> A
```
"""

docx(OUT / "Field Guide Brief.docx")
xlsx(OUT / "Sales 2027.xlsx")
pptx(OUT / "Coastal Trails 2027.pptx")
(OUT / "harbor-loop.md").write_text(MARKDOWN)

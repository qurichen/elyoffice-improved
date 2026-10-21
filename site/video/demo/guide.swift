// Writes a four-page sample chapter PDF to argv[1].
import AppKit
import CoreText

let out = URL(fileURLWithPath: CommandLine.arguments[1])
var page = CGRect(x: 0, y: 0, width: 595, height: 842)
let info = [kCGPDFContextTitle: "Coastal Trails Field Guide", kCGPDFContextCreator: "Coastal Trails"] as CFDictionary
let pdf = CGContext(out as CFURL, mediaBox: &page, info)!

let ink = NSColor(red: 0.12, green: 0.24, blue: 0.21, alpha: 1)
let accent = NSColor(red: 0.18, green: 0.42, blue: 0.36, alpha: 1)
let body = NSColor(red: 0.17, green: 0.2, blue: 0.19, alpha: 1)
let sea = NSColor(red: 0.84, green: 0.91, blue: 0.94, alpha: 1)
let land = NSColor(red: 0.93, green: 0.94, blue: 0.89, alpha: 1)

func style(_ font: String, _ size: CGFloat, _ color: NSColor, line: CGFloat = 1.0) -> [NSAttributedString.Key: Any] {
    let p = NSMutableParagraphStyle()
    p.lineHeightMultiple = line
    p.paragraphSpacing = size * 0.6
    return [.font: NSFont(name: font, size: size)!, .foregroundColor: color, .paragraphStyle: p]
}

// Draws text in a box measured from the page top.
func text(_ s: String, _ attrs: [NSAttributedString.Key: Any], x: CGFloat, top: CGFloat, w: CGFloat, h: CGFloat) {
    let frame = CTFramesetterCreateFrame(
        CTFramesetterCreateWithAttributedString(NSAttributedString(string: s, attributes: attrs)),
        CFRange(location: 0, length: 0), CGPath(rect: CGRect(x: x, y: 842 - top - h, width: w, height: h), transform: nil), nil)
    CTFrameDraw(frame, pdf)
}

// A simple hand-drawn style route map.
func map(top: CGFloat, seed: CGFloat) {
    let box = CGRect(x: 56, y: 842 - top - 230, width: 483, height: 230)
    pdf.setFillColor(sea.cgColor)
    pdf.fill(box)
    pdf.setFillColor(land.cgColor)
    pdf.move(to: CGPoint(x: box.minX, y: box.minY))
    for i in 0...24 {
        let x = box.minX + CGFloat(i) / 24 * box.width
        pdf.addLine(to: CGPoint(x: x, y: box.minY + 120 + 34 * sin(CGFloat(i) * 0.55 + seed)))
    }
    pdf.addLine(to: CGPoint(x: box.maxX, y: box.minY))
    pdf.fillPath()
    pdf.setStrokeColor(accent.cgColor)
    pdf.setLineWidth(3)
    pdf.setLineDash(phase: 0, lengths: [8, 5])
    pdf.move(to: CGPoint(x: box.minX + 40, y: box.minY + 40))
    for i in 1...8 {
        let x = box.minX + 40 + CGFloat(i) * 52
        pdf.addLine(to: CGPoint(x: x, y: box.minY + 60 + 40 * sin(CGFloat(i) * 0.9 + seed)))
    }
    pdf.strokePath()
    pdf.setLineDash(phase: 0, lengths: [])
    pdf.setFillColor(ink.cgColor)
    for x in [box.minX + 40, box.minX + 456] {
        pdf.fillEllipse(in: CGRect(x: x - 5, y: box.minY + 35, width: 10, height: 10))
    }
}

let routes = [
    ("Harbor Loop", "4.1 km · 1 h 05 min · Easy",
     "A gentle circuit around the old harbor, best walked at low tide. The path follows the sea wall past the fish market, climbs the steps to the lighthouse and returns through the dunes behind the beach.\nAt the lighthouse, take the narrow path on the seaward side for the best view of the bay. In winter the dune path can flood after heavy rain; the road above it is a dry alternative.\nRefreshments at the harbor café, open daily from eight."),
    ("Cliff Path North", "9.6 km · 3 h 10 min · Moderate",
     "The classic walk of the coast. From the car park at Gull Point the path climbs steadily onto the headland and keeps close to the cliff edge for most of its length.\nThere are two steep descents into coves with stepping stones; check the tide table before you set out, because the second crossing is cut off for two hours either side of high water.\nReturn by the inland bridleway, or catch the coast bus from the village green."),
    ("Dune Meadows", "5.3 km · 1 h 30 min · Easy",
     "A quiet walk through protected dune grassland, rich in orchids in early summer. Keep to the marked boardwalks: the meadows are grazed by ponies and nesting birds use the open ground.\nThe bird hide at the far end of the reserve looks out over the salt marsh and is worth a long stop at dawn or dusk."),
    ("Lighthouse to Lantern Bay", "7.2 km · 2 h 20 min · Moderate",
     "A linear walk between two lighthouses, with a long sandy bay in the middle. The first half is on firm clifftop turf; the second half crosses the bay on the beach and climbs a flight of old coastguard steps.\nThe steps are uneven and can be slippery when wet. Walkers with dogs should use the ramp at the north end of the bay."),
]

for (n, route) in routes.enumerated() {
    pdf.beginPDFPage(nil)
    pdf.setFillColor(NSColor.white.cgColor)
    pdf.fill(page)
    NSGraphicsContext.current = NSGraphicsContext(cgContext: pdf, flipped: false)
    text("COASTAL TRAILS FIELD GUIDE · ROUTE \(n + 1)", style("HelveticaNeue-Medium", 9, accent), x: 56, top: 48, w: 483, h: 20)
    text(route.0, style("HelveticaNeue-Bold", 30, ink), x: 56, top: 70, w: 483, h: 44)
    text(route.1, style("HelveticaNeue", 13, accent), x: 56, top: 116, w: 483, h: 22)
    map(top: 150, seed: CGFloat(n) * 1.7)
    text(route.2, style("Georgia", 12, body, line: 1.25), x: 56, top: 404, w: 483, h: 360)
    text("\(n + 1)", style("HelveticaNeue", 9, accent), x: 290, top: 800, w: 30, h: 16)
    pdf.endPDFPage()
}
pdf.closePDF()

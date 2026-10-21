// Drags the left button from x1 y1 to x2 y2.
import CoreGraphics
import Foundation
let a = CommandLine.arguments.dropFirst().map { Double($0)! }
let (from, to) = (CGPoint(x: a[0], y: a[1]), CGPoint(x: a[2], y: a[3]))
func post(_ kind: CGEventType, _ at: CGPoint) {
    CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: at, mouseButton: .left)!.post(tap: .cghidEventTap)
    usleep(30_000)
}
post(.mouseMoved, from)
post(.leftMouseDown, from)
for step in 1...20 {
    let t = Double(step) / 20
    post(.leftMouseDragged, CGPoint(x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t))
}
post(.leftMouseUp, to)

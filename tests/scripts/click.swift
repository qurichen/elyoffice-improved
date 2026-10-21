// Clicks the left button at x y in screen points.
import CoreGraphics
import Foundation

let point = CGPoint(x: Double(CommandLine.arguments[1])!, y: Double(CommandLine.arguments[2])!)
for kind in [CGEventType.leftMouseDown, .leftMouseUp] {
    CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: point, mouseButton: .left)!
        .post(tap: .cghidEventTap)
    usleep(60_000)
}

// Human-paced mouse: move, click, double, drag, path, scroll.
import CoreGraphics
import Foundation

let args = Array(CommandLine.arguments.dropFirst())
let verb = args[0]
let n = args.dropFirst().map { Double($0)! }

func now() -> CGPoint { CGEvent(source: nil)!.location }

func post(_ kind: CGEventType, _ at: CGPoint, clicks: Int64 = 1) {
    let e = CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: at, mouseButton: .left)!
    e.setIntegerValueField(.mouseEventClickState, value: clicks)
    e.post(tap: .cghidEventTap)
}

// Eased glide; dragging posts drag events instead.
func glide(to: CGPoint, seconds: Double, dragging: Bool = false) {
    let from = now()
    let steps = max(2, Int(seconds * 120))
    for i in 1...steps {
        let t = Double(i) / Double(steps)
        let e = t < 0.5 ? 4 * t * t * t : 1 - pow(-2 * t + 2, 3) / 2
        let p = CGPoint(x: from.x + (to.x - from.x) * e, y: from.y + (to.y - from.y) * e)
        post(dragging ? .leftMouseDragged : .mouseMoved, p)
        usleep(useconds_t(seconds / Double(steps) * 1_000_000))
    }
}

func pace(_ a: CGPoint) -> Double {
    let d = hypot(a.x - now().x, a.y - now().y)
    return min(0.9, max(0.25, d / 1400))
}

switch verb {
case "move":
    let p = CGPoint(x: n[0], y: n[1])
    glide(to: p, seconds: n.count > 2 ? n[2] : pace(p))
case "click", "double":
    let p = CGPoint(x: n[0], y: n[1])
    glide(to: p, seconds: pace(p))
    usleep(120_000)
    for c in 1...(verb == "double" ? 2 : 1) {
        post(.leftMouseDown, p, clicks: Int64(c))
        usleep(70_000)
        post(.leftMouseUp, p, clicks: Int64(c))
        usleep(90_000)
    }
case "drag":
    let a = CGPoint(x: n[0], y: n[1]), b = CGPoint(x: n[2], y: n[3])
    glide(to: a, seconds: pace(a))
    usleep(150_000)
    post(.leftMouseDown, a)
    usleep(120_000)
    glide(to: b, seconds: n.count > 4 ? n[4] : 0.7, dragging: true)
    usleep(120_000)
    post(.leftMouseUp, b)
case "path":
    // Seconds, then x y pairs; drags through them.
    let seconds = n[0]
    let points = stride(from: 1, to: n.count - 1, by: 2).map { CGPoint(x: n[$0], y: n[$0 + 1]) }
    glide(to: points[0], seconds: pace(points[0]))
    usleep(150_000)
    post(.leftMouseDown, points[0])
    for p in points.dropFirst() {
        post(.leftMouseDragged, p)
        usleep(useconds_t(seconds / Double(points.count) * 1_000_000))
    }
    usleep(100_000)
    post(.leftMouseUp, points[points.count - 1])
case "scroll":
    // Pixel deltas, spread over time; negative scrolls down.
    let total = n[0], seconds = n.count > 1 ? n[1] : 1.0
    let steps = max(1, Int(seconds * 60))
    for _ in 0..<steps {
        CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1,
                wheel1: Int32(total / Double(steps)), wheel2: 0, wheel3: 0)!.post(tap: .cghidEventTap)
        usleep(useconds_t(seconds / Double(steps) * 1_000_000))
    }
default:
    FileHandle.standardError.write("unknown verb \(verb)\n".data(using: .utf8)!)
    exit(2)
}

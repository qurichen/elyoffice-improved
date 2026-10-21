// Drags a file from its window onto x, y.
import AppKit

let args = CommandLine.arguments
let file = URL(fileURLWithPath: args[1])
let target = CGPoint(x: Double(args[2])!, y: Double(args[3])!)
let start = CGPoint(x: 60, y: 60)

final class Source: NSView, NSDraggingSource {
    func draggingSession(_: NSDraggingSession, sourceOperationMaskFor _: NSDraggingContext) -> NSDragOperation {
        .copy
    }

    func draggingSession(_: NSDraggingSession, endedAt _: NSPoint, operation: NSDragOperation) {
        print(operation.isEmpty ? "refused" : "dropped")
        exit(0)
    }

    override func acceptsFirstMouse(for _: NSEvent?) -> Bool {
        true
    }

    override func mouseDown(with event: NSEvent) {
        let item = NSDraggingItem(pasteboardWriter: file as NSURL)
        item.setDraggingFrame(bounds, contents: NSImage(size: bounds.size))
        beginDraggingSession(with: [item], event: event, source: self)
    }
}

func post(_ type: CGEventType, _ at: CGPoint) {
    CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: at, mouseButton: .left)?
        .post(tap: .cghidEventTap)
    usleep(40000)
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let height = NSScreen.screens[0].frame.height
let frame = NSRect(x: start.x - 20, y: height - start.y - 20, width: 40, height: 40)
let window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
window.contentView = Source(frame: NSRect(x: 0, y: 0, width: 40, height: 40))
window.level = .floating
window.orderFrontRegardless()
DispatchQueue.global().asyncAfter(deadline: .now() + 0.5) {
    post(.mouseMoved, start)
    post(.leftMouseDown, start)
    for step in 1...30 {
        let t = Double(step) / 30
        post(.leftMouseDragged, CGPoint(x: start.x + (target.x - start.x) * t, y: start.y + (target.y - start.y) * t))
    }
    usleep(300000)
    post(.leftMouseDragged, target)
    post(.leftMouseUp, target)
}
DispatchQueue.main.asyncAfter(deadline: .now() + 10) {
    print("timeout")
    exit(1)
}
app.run()

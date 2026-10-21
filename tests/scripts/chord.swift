// Posts Command plus a key to one process, flag included.
import CoreGraphics
import Foundation

// chord <pid> <virtual key code>
let pid = pid_t(CommandLine.arguments[1])!
let code = CGKeyCode(CommandLine.arguments[2])!
let source = CGEventSource(stateID: .privateState)
for down in [true, false] {
    let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)!
    event.flags = .maskCommand
    event.postToPid(pid)
    usleep(30_000)
}

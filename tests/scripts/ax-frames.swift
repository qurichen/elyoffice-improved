// Prints each window element's frame and parent as JSON.
import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
    return value
}

// The element's frame in screen points.
func frame(_ element: AXUIElement) -> CGRect? {
    var origin = CGPoint.zero, size = CGSize.zero
    guard let position = attribute(element, kAXPositionAttribute),
          let extent = attribute(element, kAXSizeAttribute),
          AXValueGetValue(position as! AXValue, .cgPoint, &origin),
          AXValueGetValue(extent as! AXValue, .cgSize, &size) else { return nil }
    return CGRect(origin: origin, size: size)
}

var next = 0
func walk(_ element: AXUIElement, _ parent: Int, _ depth: Int) {
    if depth > 60 { return }
    next += 1
    let id = next
    let role = attribute(element, kAXRoleAttribute) as? String ?? ""
    let name = [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute]
        .compactMap { attribute(element, $0) as? String }.first { !$0.isEmpty } ?? ""
    var row: [String: Any] = ["id": id, "parent": parent, "role": role, "name": name]
    if let rect = frame(element) {
        row["frame"] = [rect.minX, rect.minY, rect.width, rect.height]
    }
    let data = try! JSONSerialization.data(withJSONObject: row)
    print(String(data: data, encoding: .utf8)!)
    for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
        walk(child, id, depth + 1)
    }
}

guard AXIsProcessTrusted() else {
    print("untrusted")
    exit(2)
}
let app = AXUIElementCreateApplication(pid_t(CommandLine.arguments[1])!)
for window in attribute(app, kAXWindowsAttribute) as? [AXUIElement] ?? [] {
    walk(window, 0, 0)
}

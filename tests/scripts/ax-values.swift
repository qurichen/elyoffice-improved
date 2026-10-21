// Prints role, value, title and center of each AX element.
import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
    return value
}

// The element's center in screen points, or empty.
func center(_ element: AXUIElement) -> String {
    var origin = CGPoint.zero, size = CGSize.zero
    guard let position = attribute(element, kAXPositionAttribute),
          let extent = attribute(element, kAXSizeAttribute),
          AXValueGetValue(position as! AXValue, .cgPoint, &origin),
          AXValueGetValue(extent as! AXValue, .cgSize, &size) else { return "" }
    return "\(Int(origin.x + size.width / 2)),\(Int(origin.y + size.height / 2))"
}

func walk(_ element: AXUIElement, _ depth: Int) {
    if depth > 40 { return }
    let role = attribute(element, kAXRoleAttribute) as? String ?? ""
    let value = attribute(element, kAXValueAttribute) as? String ?? ""
    let title = attribute(element, kAXTitleAttribute) as? String ?? ""
    let label = attribute(element, kAXDescriptionAttribute) as? String ?? ""
    let help = attribute(element, kAXHelpAttribute) as? String ?? ""
    print("\(role)\tvalue=\(value)\ttitle=\(title)\tlabel=\(label)\thelp=\(help)\tat=\(center(element))")
    for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
        walk(child, depth + 1)
    }
}

guard AXIsProcessTrusted() else {
    print("untrusted")
    exit(2)
}
walk(AXUIElementCreateApplication(pid_t(CommandLine.arguments[1])!), 0)

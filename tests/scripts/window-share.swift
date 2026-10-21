// The share of a window's pixels near one colour.
import CoreGraphics
import Foundation
import ImageIO

let args = CommandLine.arguments
let pid = Int32(args[1])!
let want = args[2].split(separator: ",").map { Int($0)! }
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
guard let window = windows.first(where: {
    ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int) == 0
}) else {
    print("none")
    exit(0)
}
let id = window[kCGWindowNumber as String] as! Int
let file = NSTemporaryDirectory() + "ely-share-\(pid).png"
let shot = Process()
shot.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
shot.arguments = ["-x", "-o", "-l", "\(id)", file]
try shot.run()
shot.waitUntilExit()
guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: file) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    print("denied")
    exit(0)
}
let (width, height) = (image.width, image.height)
var pixels = [UInt8](repeating: 0, count: width * height * 4)
let context = CGContext(
    data: &pixels, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
    space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
var near = 0
for at in stride(from: 0, to: pixels.count, by: 4) {
    let off = abs(Int(pixels[at]) - want[0]) + abs(Int(pixels[at + 1]) - want[1]) + abs(Int(pixels[at + 2]) - want[2])
    if off < 40 { near += 1 }
}
print(String(format: "%.4f", Double(near) / Double(width * height)))

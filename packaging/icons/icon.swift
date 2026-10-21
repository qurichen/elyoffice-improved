// Renders SVGs to transparent PNGs. Usage: icon <px> <in.svg> <out.png>...
import AppKit

let args = CommandLine.arguments
guard args.count >= 4, args.count % 2 == 0, let size = Int(args[1]) else {
  FileHandle.standardError.write("usage: icon <px> <in.svg> <out.png>...\n".data(using: .utf8)!)
  exit(2)
}
for pair in stride(from: 2, to: args.count, by: 2) {
  guard let svg = NSImage(contentsOfFile: args[pair]) else {
    FileHandle.standardError.write("icon: cannot read \(args[pair])\n".data(using: .utf8)!)
    exit(1)
  }
  let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
  NSGraphicsContext.saveGraphicsState()
  NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
  svg.draw(in: NSRect(x: 0, y: 0, width: size, height: size))
  NSGraphicsContext.restoreGraphicsState()
  try rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: args[pair + 1]))
}

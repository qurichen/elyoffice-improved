"""Packs PNGs into a Windows .ico. Usage: ico.py out.ico in.png..."""
import struct
import sys

out, pngs = sys.argv[1], sys.argv[2:]
images = [open(path, "rb").read() for path in pngs]
header = struct.pack("<HHH", 0, 1, len(images))
offset = len(header) + 16 * len(images)
entries = b""
for data in images:
    # Size from the PNG header; 256 is written as 0.
    width, height = struct.unpack(">II", data[16:24])
    entries += struct.pack("<BBBBHHII", width % 256, height % 256, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
with open(out, "wb") as ico:
    ico.write(header + entries + b"".join(images))

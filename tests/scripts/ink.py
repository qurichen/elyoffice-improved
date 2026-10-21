#!/usr/bin/env python3
"""Counts bright pixels in a PNG region."""
import struct
import sys
import zlib


def read_png(path):
    with open(path, "rb") as png:
        data = png.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        sys.exit(f"FAIL: {path} is not a PNG")
    pos, width, height, color, interlace, idat = 8, 0, 0, 0, 0, b""
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos : pos + 8])
        body = data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or color not in (2, 6) or interlace != 0:
                sys.exit(f"FAIL: {path} needs 8-bit RGB/RGBA, not depth {depth} color {color} interlace {interlace}")
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
        pos += 12 + length
    return width, height, 4 if color == 6 else 3, zlib.decompress(idat)


def unfilter(width, height, channels, raw):
    stride = width * channels
    out, previous = bytearray(), bytearray(stride)
    pos = 0
    for _ in range(height):
        kind = raw[pos]
        pos += 1
        line = bytearray(raw[pos : pos + stride])
        pos += stride
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            up_left = previous[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 0xFF
            elif kind == 4:
                p = left + up - up_left
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - up_left)
                line[i] = (line[i] + (left if pa <= pb and pa <= pc else up if pb <= pc else up_left)) & 0xFF
        out += line
        previous = line
    return out


def region_pixels(path, x0, y0, x1, y1):
    width, height, channels, raw = read_png(path)
    pixels = unfilter(width, height, channels, raw)
    xs, xe = int(width * x0), int(width * x1)
    ys, ye = int(height * y0), int(height * y1)
    out = bytearray()
    for y in range(ys, ye):
        for x in range(xs, xe):
            at = (y * width + x) * channels
            out += pixels[at : at + 3]
    return xe - xs, ye - ys, bytes(out)


def region_at(path, xs, ys, width, height):
    shot_w, shot_h, channels, raw = read_png(path)
    pixels = unfilter(shot_w, shot_h, channels, raw)
    out = bytearray()
    for y in range(ys, ys + height):
        for x in range(xs, xs + width):
            at = (y * shot_w + x) * channels
            out += pixels[at : at + 3]
    return bytes(out)


def main():
    if sys.argv[1] == "dist":
        print(bin(int(sys.argv[2], 16) ^ int(sys.argv[3], 16)).count("1"))
        return
    if sys.argv[1] == "dump":
        shot, out, x0, y0, x1, y1 = sys.argv[2], sys.argv[3], *map(float, sys.argv[4:8])
        width, height, rgb = region_pixels(shot, x0, y0, x1, y1)
        head = struct.pack("<4sII4f", b"RGBZ", width, height, x0, y0, x1, y1)
        with open(out, "wb") as fixture:
            fixture.write(head + zlib.compress(rgb, 9))
        return
    if sys.argv[1] == "cmp":
        shot, fixture_path = sys.argv[2], sys.argv[3]
        with open(fixture_path, "rb") as fixture:
            blob = fixture.read()
        magic, width, height, x0, y0, x1, y1 = struct.unpack("<4sII4f", blob[:28])
        if magic != b"RGBZ":
            sys.exit(f"FAIL: {fixture_path} is not an RGBZ fixture")
        shot_w, shot_h, _, _ = read_png(shot)
        mine = region_at(shot, int(shot_w * x0), int(shot_h * y0), width, height)
        theirs = zlib.decompress(blob[28:])
        if len(mine) != len(theirs):
            sys.exit(f"FAIL: {fixture_path} region is {len(theirs)} bytes, the shot's is {len(mine)}")
        print(sum(a != b for a, b in zip(mine, theirs, strict=True)))
        return
    if sys.argv[1] == "hash":
        path, x0, y0, x1, y1 = sys.argv[2], *map(float, sys.argv[3:7])
        width, height, channels, raw = read_png(path)
        pixels = unfilter(width, height, channels, raw)
        cells = []
        for gy in range(16):
            for gx in range(16):
                total = count = 0
                for y in range(int(height * (y0 + (y1 - y0) * gy / 16)), int(height * (y0 + (y1 - y0) * (gy + 1) / 16))):
                    for x in range(int(width * (x0 + (x1 - x0) * gx / 16)), int(width * (x0 + (x1 - x0) * (gx + 1) / 16))):
                        at = (y * width + x) * channels
                        total += (pixels[at] + pixels[at + 1] + pixels[at + 2]) / 3
                        count += 1
                cells.append(total / max(count, 1))
        mean = sum(cells) / 256
        bits = 0
        for cell in cells:
            bits = (bits << 1) | (cell >= mean)
        print(f"{bits:064x}")
        return
    path, x0, y0, x1, y1 = sys.argv[1], *map(float, sys.argv[2:6])
    width, height, channels, raw = read_png(path)
    pixels = unfilter(width, height, channels, raw)
    ink = 0
    for y in range(int(height * y0), int(height * y1)):
        for x in range(int(width * x0), int(width * x1)):
            at = (y * width + x) * channels
            if pixels[at] + pixels[at + 1] + pixels[at + 2] > 380:
                ink += 1
    print(ink)


main()

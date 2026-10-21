#!/usr/bin/env python3
"""Prints a looping signature stroke as x y pairs."""
import math
import sys

cx, cy, w = float(sys.argv[1]), float(sys.argv[2]), float(sys.argv[3])
turns = 6
a = 0.8 * w / (turns * 2 * math.pi)
b = 1.6 * a
left = cx - w / 2
points = []
for i in range(301):
    t = i / 300 * turns * 2 * math.pi
    lift = (2.6 if i < 50 else 1.4) * (1 + 0.35 * math.sin(t / 3))
    rise = lift * b * (1 - math.cos(t))
    points.append((left + a * t - b * math.sin(t) + 0.35 * rise, cy - rise + 0.04 * w * (i / 300)))
end_x, end_y = points[-1]
for i in range(1, 61):
    s = i / 60
    points.append((end_x - s * 0.95 * w * 0.9, end_y + 0.07 * w * math.sin(s * math.pi) + 0.03 * w * s))
print(" ".join(f"{x:.0f} {y:.0f}" for x, y in points))

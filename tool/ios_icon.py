"""Checks the iPhone app icon against the app's icon design.

    python3 tool/ios_icon.py

The iOS build takes its icon from
`ios/Runner/Assets.xcassets/AppIcon.appiconset/`, and App Store Connect
shows the 1024 × 1024 one on the product page. Until 0.5.1 that set
still held Flutter's default logo, while every other platform had the
"?!" icon drawn from `play-developer/app-icon.svg`. This check fails when
a file of the set is missing, not at the size `Contents.json` names, not
an opaque 8-bit RGB PNG (App Store Connect refuses an alpha channel), or
when the 1024 one does not look like `play-developer/app-icon-512.png`,
the same design rendered for Google Play.

The icons are rendered from the SVG (any SVG renderer, at each size,
saved as RGB without alpha); the standard library alone reads them here.
"""
import json
import os
import struct
import sys
import zlib

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
ICONS = os.path.join(ROOT, "ios", "Runner", "Assets.xcassets", "AppIcon.appiconset")
REFERENCE = os.path.join(ROOT, "play-developer", "app-icon-512.png")

# The mean difference per channel, on a 0-255 scale, that still counts as
# the same picture: edges differ between renderers by a few levels.
MAX_MEAN_DIFF = 4.0


def read_png(path):
    """(width, height, colour type, rows of RGB(A) bytes) of an 8-bit,
    non-interlaced PNG of colour type 2 (RGB) or 6 (RGBA)."""
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    pos, idat, header = 8, b"", None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    width, height, depth, colour, _, _, interlace = header
    if depth != 8 or colour not in (2, 6) or interlace:
        return width, height, colour, None
    channels = 3 if colour == 2 else 4
    stride = width * channels
    raw = zlib.decompress(idat)
    rows, previous = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        for x in range(stride):
            left = line[x - channels] if x >= channels else 0
            up = previous[x]
            corner = previous[x - channels] if x >= channels else 0
            if kind == 1:
                line[x] = (line[x] + left) & 255
            elif kind == 2:
                line[x] = (line[x] + up) & 255
            elif kind == 3:
                line[x] = (line[x] + (left + up) // 2) & 255
            elif kind == 4:
                p = left + up - corner
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
                pred = left if pa <= pb and pa <= pc else (up if pb <= pc else corner)
                line[x] = (line[x] + pred) & 255
        rows.append(line)
        previous = line
    return width, height, colour, rows


def pixel(rows, colour, x, y):
    channels = 3 if colour == 2 else 4
    return tuple(rows[y][x * channels:x * channels + 3])


def mean_difference(icon, reference, samples=64):
    """The mean difference per channel between [icon] and [reference],
    both (width, height, colour, rows), at samples × samples points."""
    total = 0
    for j in range(samples):
        for i in range(samples):
            a = pixel(icon[3], icon[2], (2 * i + 1) * icon[0] // (2 * samples), (2 * j + 1) * icon[1] // (2 * samples))
            b = pixel(reference[3], reference[2], (2 * i + 1) * reference[0] // (2 * samples), (2 * j + 1) * reference[1] // (2 * samples))
            total += sum(abs(p - q) for p, q in zip(a, b))
    return total / (samples * samples * 3)


def failures(icons=ICONS, reference=REFERENCE):
    out = []
    contents = json.load(open(os.path.join(icons, "Contents.json")))
    largest = None
    for entry in contents["images"]:
        name = entry["filename"]
        path = os.path.join(icons, name)
        if not os.path.exists(path):
            out.append(f"{name}: missing")
            continue
        size = round(float(entry["size"].split("x")[0]) * int(entry["scale"].rstrip("x")))
        width, height, colour, rows = read_png(path)
        if (width, height) != (size, size):
            out.append(f"{name}: {width} × {height}, not {size} × {size}")
        if colour != 2 or rows is None:
            out.append(f"{name}: not an opaque 8-bit RGB PNG")
        elif entry.get("idiom") == "ios-marketing":
            largest = (width, height, colour, rows)
    if largest is None:
        out.append("no 1024 × 1024 ios-marketing icon to compare")
    else:
        diff = mean_difference(largest, read_png(reference))
        if diff > MAX_MEAN_DIFF:
            out.append(f"the App Store icon differs from {os.path.relpath(reference, ROOT)} (mean {diff:.1f} per channel)")
    return out


if __name__ == "__main__":
    found = failures()
    for line in found:
        print(line)
    if found:
        sys.exit(1)
    print("iPhone app icon: every size present, opaque, and the app's own design.")

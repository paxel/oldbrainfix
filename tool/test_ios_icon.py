import json
import os
import shutil
import struct
import tempfile
import unittest
import zlib

from ios_icon import ICONS, failures, read_png


def png(path, width, height, colour, fill):
    """Writes a PNG of one colour; colour type 2 (RGB) or 6 (RGBA)."""
    row = bytes(fill) * width
    raw = b"".join(b"\x00" + row for _ in range(height))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, colour, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw)))
        f.write(chunk(b"IEND", b""))


class IosIconTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.icons = os.path.join(self.dir, "AppIcon.appiconset")
        os.makedirs(self.icons)
        json.dump(
            {"images": [
                {"size": "20x20", "idiom": "iphone", "filename": "small.png", "scale": "2x"},
                {"size": "64x64", "idiom": "ios-marketing", "filename": "large.png", "scale": "1x"},
            ]},
            open(os.path.join(self.icons, "Contents.json"), "w"),
        )
        self.reference = os.path.join(self.dir, "reference.png")
        png(self.reference, 32, 32, 6, (38, 41, 44, 255))

    def tearDown(self):
        shutil.rmtree(self.dir)

    def test_the_icons_in_the_repo_pass(self):
        self.assertEqual(failures(), [])

    def test_the_icons_in_the_repo_are_read_whole(self):
        width, height, colour, rows = read_png(os.path.join(ICONS, "Icon-App-1024x1024@1x.png"))
        self.assertEqual((width, height, colour, len(rows)), (1024, 1024, 2, 1024))

    def test_matching_icons_pass(self):
        png(os.path.join(self.icons, "small.png"), 40, 40, 2, (38, 41, 44))
        png(os.path.join(self.icons, "large.png"), 64, 64, 2, (38, 41, 44))
        self.assertEqual(failures(self.icons, self.reference), [])

    def test_another_picture_fails(self):
        png(os.path.join(self.icons, "small.png"), 40, 40, 2, (38, 41, 44))
        png(os.path.join(self.icons, "large.png"), 64, 64, 2, (255, 255, 255))
        self.assertIn("differs", failures(self.icons, self.reference)[0])

    def test_a_wrong_size_a_missing_file_and_an_alpha_channel_fail(self):
        png(os.path.join(self.icons, "large.png"), 64, 64, 6, (38, 41, 44, 255))
        found = failures(self.icons, self.reference)
        self.assertIn("small.png: missing", found)
        self.assertIn("large.png: not an opaque 8-bit RGB PNG", found)
        png(os.path.join(self.icons, "small.png"), 20, 20, 2, (38, 41, 44))
        self.assertIn("small.png: 20 × 20, not 40 × 40", failures(self.icons, self.reference))


if __name__ == "__main__":
    unittest.main()

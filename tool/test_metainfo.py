import unittest
import xml.etree.ElementTree as ET

from metainfo import render

TEMPLATE = """<component>
  <image>https://github.com/paxel/oldbrainfix/releases/download/v@VERSION@/a.png</image>
  <releases>
    <!-- @RELEASES@ -->
  </releases>
</component>
"""

CHANGELOG = """# Changelog

## [0.3.0]

### Added
- Not released yet.

## [0.2.0]

### Added
- Runs `oldbrainfix capture` & more.

### Fixed
- A <bug>.

## [0.1.0]

### Added
- First release.

## [0.0.9]

### Added
- Never tagged.

---
""".splitlines(keepends=True)

TAGS = {"0.1.0": "2026-11-02"}


class MetainfoTest(unittest.TestCase):
    def render(self, version="0.2.0"):
        return render(TEMPLATE, CHANGELOG, version, "2026-12-01", dated=TAGS.get)

    def test_the_screenshots_point_at_the_versions_release(self):
        self.assertIn("/releases/download/v0.2.0/a.png", self.render())

    def test_releases_up_to_the_version_newest_first_dated_by_tags(self):
        root = ET.fromstring(self.render())
        releases = root.findall("./releases/release")
        self.assertEqual(
            [(r.get("version"), r.get("date")) for r in releases],
            [("0.2.0", "2026-12-01"), ("0.1.0", "2026-11-02")],
        )

    def test_the_notes_are_the_entries_escaped_without_backticks(self):
        root = ET.fromstring(self.render())
        items = [li.text for li in root.findall("./releases/release[1]/description/ul/li")]
        self.assertEqual(items, ["Runs oldbrainfix capture & more.", "A <bug>."])

    def test_a_version_without_a_section_is_refused(self):
        with self.assertRaises(SystemExit):
            self.render("0.2.5")


if __name__ == "__main__":
    unittest.main()

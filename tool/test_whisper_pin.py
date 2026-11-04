import unittest

from whisper_pin import licence_tag, readme_tag, report

PINNED = "d1be6fde11ac6e0407606b4e42fe72d34add8037"
NEWER = "1111111111111111111111111111111111111111"


def warnings(lines):
    return [line for line in lines if line.startswith("::warning::")]


class WhisperPinTest(unittest.TestCase):
    def test_the_tag_of_the_whisper_row(self):
        text = (
            "| Submodule   | Pinned   |\n"
            "|-------------|----------|\n"
            "| `whisper.cpp` | `v1.9.5` |\n"
        )
        self.assertEqual(readme_tag(text), "v1.9.5")
        self.assertIsNone(readme_tag("| other | `v2.0.0` |\n"))

    def test_the_release_the_licence_names(self):
        line = "whisper.cpp and its ggml library, https://github.com/ggml-org/whisper.cpp, release v1.9.5, the CPU parts"
        self.assertEqual(licence_tag(line), "v1.9.5")
        self.assertIsNone(licence_tag("whisper.cpp"))

    def test_the_latest_release_pinned_everywhere_warns_of_nothing(self):
        lines = report(PINNED, "v1.9.5", PINNED, "v1.9.5", PINNED, "v1.9.5")
        self.assertEqual(warnings(lines), [])
        self.assertIn("whisper.cpp: v1.9.5 is upstream's latest release", lines)

    def test_a_newer_release_upstream_warns(self):
        lines = report(PINNED, "v1.9.5", PINNED, "v1.9.6", NEWER, "v1.9.5")
        self.assertEqual(len(warnings(lines)), 1)
        self.assertIn("behind: upstream's latest release is v1.9.6", warnings(lines)[0])

    def test_a_submodule_off_its_named_tag_warns(self):
        # As after a Dependabot update that left third_party/README.md.
        lines = report(NEWER, "v1.9.5", PINNED, "v1.9.6", NEWER, "v1.9.5")
        self.assertEqual(len(warnings(lines)), 1)
        self.assertIn(f"at {NEWER}, but v1.9.5", warnings(lines)[0])

    def test_a_licence_naming_another_release_warns(self):
        lines = report(PINNED, "v1.9.5", PINNED, "v1.9.5", PINNED, "v1.9.4")
        self.assertEqual(len(warnings(lines)), 1)
        self.assertIn("licenses/common.txt names whisper.cpp release v1.9.4", warnings(lines)[0])


if __name__ == "__main__":
    unittest.main()

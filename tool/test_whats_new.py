import os
import tempfile
import unittest

from whats_new import BUNDLE, LIMIT, ROOT, bundle, bundle_text, whats_new, write_bundle

NOTES = """# Release notes 0.2.0

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Production release" -->
```
Added
• First thing.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Production release" -->
```
Neu
• Erstes Ding.
```

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="Play internal and closed testing" -->
```
What to test
```
""".splitlines(keepends=True)


class WhatsNewTest(unittest.TestCase):
    def test_the_block_of_the_language_word_for_word(self):
        self.assertEqual(whats_new("en", NOTES), "Added\n• First thing.")
        self.assertEqual(whats_new("de", NOTES), "Neu\n• Erstes Ding.")

    def test_a_missing_language_fails(self):
        with self.assertRaises(ValueError):
            whats_new("fr", NOTES)

    def test_an_empty_block_fails(self):
        empty = ["<!-- block id=play_notes_en lang=en -->\n", "```\n", "```\n"]
        with self.assertRaises(ValueError):
            whats_new("en", empty)

    def test_a_block_over_the_limit_fails(self):
        long = ["<!-- block id=play_notes_en lang=en -->\n", "```\n", "x" * (LIMIT + 1) + "\n", "```\n"]
        with self.assertRaises(ValueError):
            whats_new("en", long)


if __name__ == "__main__":
    unittest.main()


class BundleTest(unittest.TestCase):
    """The What's new both apps bundle as Notes (#180)."""

    def folder(self, files):
        docs = tempfile.TemporaryDirectory()
        self.addCleanup(docs.cleanup)
        for name, text in files.items():
            with open(os.path.join(docs.name, name), "w", encoding="utf-8") as f:
                f.write(text)
        return docs.name

    def test_every_release_after_0_3_1_with_both_languages(self):
        text = "".join(NOTES)
        docs = self.folder({
            "release-notes-0.3.1.md": text,
            "release-notes-0.4.0.md": text,
            "release-notes-0.10.0.md": text,
            "hardware-buttons.md": "# Buttons",
        })
        notes = {"en": "Added\n• First thing.", "de": "Neu\n• Erstes Ding."}
        self.assertEqual(bundle(docs), {"0.4.0": notes, "0.10.0": notes})

    def test_a_release_without_a_language_fails(self):
        english = "".join(NOTES).replace("play_notes_de", "play_notes_fr")
        docs = self.folder({"release-notes-0.4.0.md": english})
        with self.assertRaises(ValueError):
            bundle(docs)

    def test_a_required_version_without_notes_fails(self):
        docs = self.folder({"release-notes-0.4.0.md": "".join(NOTES)})
        out = os.path.join(docs, "out", "whats_new.json")
        write_bundle("0.4.0", docs=docs, out=out)
        with self.assertRaises(ValueError):
            write_bundle("0.4.1", docs=docs, out=out)

    def test_the_bundled_file_is_current(self):
        with open(BUNDLE, encoding="utf-8") as f:
            held = f.read()
        self.assertEqual(
            held,
            bundle_text(bundle(os.path.join(ROOT, "docs"))),
            "run python3 tool/whats_new.py --bundle",
        )

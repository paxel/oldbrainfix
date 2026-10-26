import unittest

from whats_new import LIMIT, whats_new

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

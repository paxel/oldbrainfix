import os
import tempfile
import unittest

from changelog_check import LIMIT, check, current_entries, problems

CHANGELOG = """# Changelog

## [0.6.1]

### Added

- First thing that changed ([obf-1012](https://github.com/paxel/oldbrainfix/issues/319)).
- A second thing
  that wraps onto a next line.

---

## [0.6.0] - 2026-11-14

- An older entry that is never checked, however long it gets, because it is released already and stays as it was written.
"""


class ChangelogCheckTest(unittest.TestCase):
    def test_only_the_current_section_is_read_and_wrapped_lines_join(self):
        self.assertEqual(
            current_entries(CHANGELOG),
            [
                "First thing that changed ([obf-1012](https://github.com/paxel/oldbrainfix/issues/319)).",
                "A second thing that wraps onto a next line.",
            ],
        )

    def test_a_short_sentence_with_its_link_passes(self):
        self.assertEqual(problems("Copy in the right-click menu reaches the clipboard ([obf-1012](https://x/319))."), [])
        self.assertEqual(problems("Fixed the cursor (#312)."), [])

    def test_the_link_is_not_counted(self):
        words = " ".join(["word"] * LIMIT)
        self.assertEqual(problems(words + " ([obf-1000](https://github.com/paxel/oldbrainfix/issues/307))."), [])

    def test_too_many_words_fail(self):
        words = " ".join(["word"] * (LIMIT + 1))
        self.assertEqual(problems(words + "."), [f"{LIMIT + 1} words, at most {LIMIT}"])

    def test_a_second_sentence_fails(self):
        self.assertEqual(problems("One thing changed. Another sentence follows."), ["more than one sentence"])
        self.assertEqual(problems("Ein Satz. Noch einer."), ["more than one sentence"])

    def test_numbers_abbreviations_in_words_and_quotes_pass(self):
        self.assertEqual(problems("A weight of 1.5 kg and v1.2 keep one sentence."), [])
        self.assertEqual(problems('The button reads "Add to Note…" now.'), [])

    def test_a_file_reports_each_broken_entry(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "CHANGELOG.md")
            with open(path, "w", encoding="utf-8") as file:
                file.write("# Changelog\n\n## [1.0.0]\n\n- Fine.\n- Bad. Two sentences.\n")
            self.assertEqual(len(check(path)), 1)


if __name__ == "__main__":
    unittest.main()

import os
import tempfile
import unittest

from appstore_listing import LOCALES, check_review, check_text, doc_block, failures_in


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


class AppStoreListingTest(unittest.TestCase):
    def test_the_listing_in_the_repo_passes(self):
        self.assertEqual(failures_in(), [])

    def test_a_text_over_its_limit_fails(self):
        self.assertEqual(check_text("en-US/subtitle.txt", "x" * 30), [])
        self.assertIn("over 30", check_text("en-US/subtitle.txt", "x" * 31)[0])
        self.assertIn("over 170", check_text("en-US/promotional_text.txt", "x" * 171)[0])
        self.assertIn("over 4000", check_text("en-US/description.txt", "x" * 4001)[0])

    def test_keywords_count_bytes_not_characters(self):
        self.assertEqual(check_text("de-DE/keywords.txt", "abc," * 24 + "abc"), [])
        umlauts = "äöü" * 17
        self.assertEqual(len(umlauts), 51)
        self.assertIn("102 bytes", check_text("de-DE/keywords.txt", umlauts)[0])

    def test_keywords_carry_no_space_after_a_comma_and_no_short_word(self):
        self.assertIn("space after a comma", " ".join(check_text("en-US/keywords.txt", "notes, alarm")))
        self.assertIn("two characters", " ".join(check_text("en-US/keywords.txt", "notes,ok")))

    def test_another_platform_or_testflight_fails(self):
        for word in ("Android", "Google Play", "Play Store", "F-Droid", "TestFlight", "testing", "beta test"):
            self.assertTrue(check_text("en-US/description.txt", f"Works like on {word}."), word)
        self.assertEqual(check_text("en-US/description.txt", "Androids are robots in books."), [])

    def test_a_url_must_be_https(self):
        self.assertEqual(check_text("en-US/support_url.txt", "https://example.org/"), [])
        self.assertTrue(check_text("en-US/support_url.txt", "http://example.org/"))
        self.assertTrue(check_text("en-US/privacy_url.txt", "https://a.org/ https://b.org/"))

    def test_a_missing_or_empty_text_fails(self):
        self.assertEqual(check_text("en-US/name.txt", None), ["en-US/name.txt is missing"])
        self.assertEqual(check_text("en-US/name.txt", "  "), ["en-US/name.txt is empty"])

    def test_the_review_notes_are_the_documents_block(self):
        doc = "# Notes\n\nPaste this:\n\n```text\nLine one.\n\nLine two.\n```\n\nMore.\n"
        self.assertEqual(doc_block(doc), "Line one.\n\nLine two.")
        self.assertEqual(check_review("Line one.\n\nLine two.", doc), [])
        self.assertIn("differs", check_review("Line one.", doc)[0])
        self.assertIn("over 4000", " ".join(check_review("é" * 2001, "```text\n" + "é" * 2001 + "\n```")))

    def test_every_locale_needs_every_file_and_release_notes_may_be_empty(self):
        with tempfile.TemporaryDirectory() as root:
            metadata = os.path.join(root, "ios")
            for locale in LOCALES:
                for name, text in (
                    ("name.txt", "Old Brain Fix"),
                    ("subtitle.txt", "Notes"),
                    ("promotional_text.txt", "Promo."),
                    ("keywords.txt", "notes,alarm"),
                    ("description.txt", "Text."),
                    ("support_url.txt", "https://example.org/"),
                    ("marketing_url.txt", "https://example.org/"),
                    ("privacy_url.txt", "https://example.org/privacy"),
                    ("release_notes.txt", ""),
                ):
                    write(os.path.join(metadata, locale, name), text)
            notes = os.path.join(metadata, "review_information", "notes.txt")
            doc = os.path.join(root, "notes.md")
            write(notes, "Notes.\n")
            write(doc, "```text\nNotes.\n```\n")
            self.assertEqual(failures_in(metadata, notes, doc), [])
            os.remove(os.path.join(metadata, "de-DE", "keywords.txt"))
            os.remove(os.path.join(metadata, "en-US", "release_notes.txt"))
            self.assertEqual(
                failures_in(metadata, notes, doc),
                ["en-US/release_notes.txt is missing", "de-DE/keywords.txt is missing"],
            )


if __name__ == "__main__":
    unittest.main()

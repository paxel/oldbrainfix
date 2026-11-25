import os
import subprocess
import tempfile
import unittest

from store_release import appstore, block, changed, frames, named

RECORD = """# Release notes 9.9.9

<!-- block id=appstore_whatsnew_en limit=4000 lang=en title="What's New" where="x" -->
```
New things.
```

<!-- block id=appstore_whatsnew_de limit=4000 lang=de title="Neu" where="x" -->
```
Neue Dinge.
```

## Play Console, in order

1. Store listing: replace phone screenshots 1, 3 and 8 with the frames above; the rest stay.

## App Store Connect, in order

1. Replace frames none by hand in App Store Connect.
""".splitlines(keepends=True)


def git(root, *args):
    subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)


class StoreReleaseTest(unittest.TestCase):
    def test_a_block_is_read_word_for_word_and_a_missing_one_fails(self):
        self.assertEqual(block(RECORD, "appstore_whatsnew_de"), "Neue Dinge.")
        with self.assertRaises(ValueError):
            block(RECORD, "appstore_whatsnew_fr")

    def test_the_app_stores_whats_new_is_written_then_checked(self):
        with tempfile.TemporaryDirectory() as root:
            for locale in ("en-US", "de-DE"):
                os.makedirs(os.path.join(root, "fastlane/metadata/ios", locale))
                open(os.path.join(root, "fastlane/metadata/ios", locale, "release_notes.txt"), "w").close()
            self.assertEqual(len(appstore(RECORD, root, check=True)), 2)
            self.assertEqual(appstore(RECORD, root), [])
            self.assertEqual(appstore(RECORD, root, check=True), [])
            with open(os.path.join(root, "fastlane/metadata/ios/en-US/release_notes.txt"), encoding="utf-8") as f:
                self.assertEqual(f.read(), "New things.\n")

    def test_the_console_steps_name_their_frames(self):
        self.assertEqual(named(RECORD, "## Play Console, in order", "replace phone screenshots"), {1, 3, 8})
        self.assertEqual(named(RECORD, "## App Store Connect, in order", "Replace frames"), set())
        with self.assertRaises(ValueError):
            named(RECORD, "## Nowhere", "Replace frames")

    def test_the_steps_must_name_exactly_the_frames_that_changed(self):
        with tempfile.TemporaryDirectory() as root:
            git(root, "init", "-q")
            git(root, "config", "user.email", "t@t")
            git(root, "config", "user.name", "t")
            play = os.path.join(root, "fastlane/metadata/android/en-US/images/phoneScreenshots")
            ios = os.path.join(root, "fastlane/metadata/ios/en-US/screenshots/iphone-6.9")
            os.makedirs(play)
            os.makedirs(ios)
            for n in range(1, 9):
                for folder in (play, ios):
                    with open(os.path.join(folder, f"{n}-x.png"), "w") as f:
                        f.write("old")
            git(root, "add", ".")
            git(root, "commit", "-qm", "base")
            git(root, "tag", "v1")
            for n in (1, 3):
                with open(os.path.join(play, f"{n}-x.png"), "w") as f:
                    f.write("new")
            git(root, "commit", "-qam", "frames")
            self.assertEqual(changed("v1", "fastlane/metadata/android", root), {1, 3})
            problems = frames(RECORD, "v1", root)
            self.assertEqual(len(problems), 1)
            self.assertIn("names [1, 3, 8], but [1, 3] changed", problems[0])


if __name__ == "__main__":
    unittest.main()

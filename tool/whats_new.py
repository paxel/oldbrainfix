"""Prints a version's "What's new" for Google Play and F-Droid in one
language, from the release notes record the store kit writes.

    python3 tool/whats_new.py docs/release-notes-0.1.0.md en > whatsnew-en-US

The text is the `play_notes_<lang>` block of that file, word for word.
Both stores cut at 500 characters, so a missing block or a longer one
fails instead of shipping a release without notes or with cut ones.
"""
import sys

LIMIT = 500


def section(version, lines):
    """The lines of a version's CHANGELOG section, without its heading."""
    grab = False
    for line in lines:
        if line.startswith(f"## [{version}]"):
            grab = True
            continue
        if grab and (line.startswith("## [") or line.startswith("---")):
            return
        if grab:
            yield line.rstrip("\n")


def whats_new(lang, lines):
    """The `play_notes_<lang>` block's text; ValueError when it is missing
    or over the limit."""
    marker = f"<!-- block id=play_notes_{lang} "
    lines = [line.rstrip("\n") for line in lines]
    for i, line in enumerate(lines):
        if not line.startswith(marker):
            continue
        rest = lines[i + 1 :]
        if not rest or not rest[0].startswith("```"):
            break
        try:
            end = rest.index("```", 1)
        except ValueError:
            break
        text = "\n".join(rest[1:end]).strip()
        if not text:
            break
        if len(text) > LIMIT:
            raise ValueError(f"play_notes_{lang} has {len(text)} characters, over {LIMIT}")
        return text
    raise ValueError(f"no play_notes_{lang} block")


if __name__ == "__main__":
    try:
        with open(sys.argv[1], encoding="utf-8") as f:
            print(whats_new(sys.argv[2], f.readlines()))
    except (OSError, ValueError) as e:
        sys.exit(f"{sys.argv[1]}: {e}")

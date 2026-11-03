"""Prints a version's "What's new" for Google Play and F-Droid in one
language, from the release notes record the store kit writes.

    python3 tool/whats_new.py docs/release-notes-0.1.0.md en > whatsnew-en-US

The text is the `play_notes_<lang>` block of that file, word for word.
Both stores cut at 500 characters, so a missing block or a longer one
fails instead of shipping a release without notes or with cut ones.

The same notes become the app's own "What's new in X.Y.Z" Notes (#180):

    python3 tool/whats_new.py --bundle [--require 0.4.0]

writes `assets/help/whats_new.json`, every release after 0.3.1 with its
English and German notes, which both apps bundle. With `--require` it
fails when that version has no notes, as the tag pipeline runs it, so no
release ships without its What's new.
"""
import json
import os
import re
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


# The help Notes start after this version: an update from it passed
# every version bundled.
SINCE = (0, 3, 1)
LANGS = ("en", "de")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BUNDLE = os.path.join(ROOT, "assets", "help", "whats_new.json")
NOTES_FILE = re.compile(r"^release-notes-(\d+)\.(\d+)\.(\d+)\.md$")


def bundle(docs):
    """Every release after SINCE in the folder `docs`, by version, with
    its notes by language; ValueError when one lacks a language."""
    found = {}
    for name in os.listdir(docs):
        m = NOTES_FILE.match(name)
        if not m or tuple(int(p) for p in m.groups()) <= SINCE:
            continue
        with open(os.path.join(docs, name), encoding="utf-8") as f:
            lines = f.readlines()
        version = ".".join(m.groups())
        try:
            found[version] = {lang: whats_new(lang, lines) for lang in LANGS}
        except ValueError as e:
            raise ValueError(f"{name}: {e}") from e
    return found


def bundle_text(found):
    """The bundle as it is written: sorted, two spaces, a final newline."""
    return json.dumps(found, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def write_bundle(require=None, docs=None, out=BUNDLE):
    docs = docs or os.path.join(ROOT, "docs")
    found = bundle(docs)
    if require is not None and require not in found:
        raise ValueError(
            f"no What's new for {require}: docs/release-notes-{require}.md "
            "needs its play_notes_en and play_notes_de blocks"
        )
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        f.write(bundle_text(found))


if __name__ == "__main__" and sys.argv[1:2] == ["--bundle"]:
    args = sys.argv[2:]
    required = args[1] if args[:1] == ["--require"] and len(args) == 2 else None
    if args and required is None:
        sys.exit("usage: whats_new.py --bundle [--require <version>]")
    try:
        write_bundle(required)
    except (OSError, ValueError) as e:
        sys.exit(str(e))
elif __name__ == "__main__":
    try:
        with open(sys.argv[1], encoding="utf-8") as f:
            print(whats_new(sys.argv[2], f.readlines()))
    except (OSError, ValueError) as e:
        sys.exit(f"{sys.argv[1]}: {e}")

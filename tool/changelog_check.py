#!/usr/bin/env python3
"""Checks the current section of each changelog: one short sentence per entry.

Every entry under the first `## [x.y.z]` heading is a single sentence of at
most LIMIT words, not counting its ticket link at the end
(`([obf-1012](…))`, `(#319)`). What changed for the user goes into the
changelog; how and why go into the guide, an ADR or the spec.

    python3 tool/changelog_check.py CHANGELOG.md site/changelog.de.md

Exits 1 and names each entry that breaks a rule.
"""

import re
import sys

LIMIT = 25

# A ticket link at the end of an entry, in either form, possibly several.
LINK = re.compile(r"(\s*\((\[[a-z]+-\d+\]\([^)]*\)|#\d+)(,\s*(\[[a-z]+-\d+\]\([^)]*\)|#\d+))*\))+\.?\s*$")
# A sentence end followed by the start of another sentence.
SENTENCE_BREAK = re.compile(r"[.!?…][\"'”»)]*\s+[\"'„«(]*[A-ZÄÖÜ0-9]")


def current_entries(text):
    """The entries of the first version section, each joined to one line."""
    entries = []
    inside = False
    for line in text.splitlines():
        if line.startswith("## "):
            if inside:
                break
            inside = True
            continue
        if not inside:
            continue
        if line.startswith("---"):
            break
        if line.startswith("- "):
            entries.append(line[2:].strip())
        elif line.startswith("  ") and entries and line.strip():
            entries[-1] += " " + line.strip()
    return entries


def problems(entry):
    """What is wrong with one entry; empty when it is fine."""
    body = LINK.sub("", entry).strip()
    found = []
    words = len(body.split())
    if words > LIMIT:
        found.append(f"{words} words, at most {LIMIT}")
    if SENTENCE_BREAK.search(body):
        found.append("more than one sentence")
    return found


def check(path):
    with open(path, encoding="utf-8") as file:
        entries = current_entries(file.read())
    failures = []
    for entry in entries:
        for problem in problems(entry):
            failures.append(f"{path}: {problem}: {entry[:80]}…")
    return failures


def main(paths):
    if not paths:
        print("usage: changelog_check.py CHANGELOG.md [more changelogs]", file=sys.stderr)
        return 2
    failures = [failure for path in paths for failure in check(path)]
    for failure in failures:
        print(f"::error::{failure}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

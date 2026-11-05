"""Checks the App Store listing texts (#245) that fastlane deliver uploads,
for CI, the release gate and the store kit.

    python3 tool/appstore_listing.py

Reads fastlane/metadata/ios/<locale>/ for en-US and de-DE and
fastlane/metadata/ios/review_information/notes.txt, and exits 1, with
GitHub errors, when a text is missing or empty, is over App Store
Connect's limit (bytes for the keywords and the review notes), carries a
space after a comma or a keyword of two characters or fewer, names
another mobile platform or its store (Apple rejects those, guideline
2.3.10), TestFlight or a word fastlane precheck takes for test content,
or when a URL is not https. The App Review notes must also be
the block in docs/agents/app-review-notes.md, word for word, so the
pasted text and the uploaded one are the same.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
METADATA = os.path.join(ROOT, "fastlane", "metadata", "ios")
REVIEW_NOTES = os.path.join(METADATA, "review_information", "notes.txt")
REVIEW_DOC = os.path.join(ROOT, "docs", "agents", "app-review-notes.md")
LOCALES = ("en-US", "de-DE")

# App Store Connect's limits: characters, but bytes for the keywords and
# the App Review notes (developer.apple.com/help/app-store-connect/
# reference/app-information/platform-version-information and
# .../app-information, read 2026-11-05).
LIMITS = {
    "name.txt": 30,
    "subtitle.txt": 30,
    "promotional_text.txt": 170,
    "description.txt": 4000,
}
KEYWORD_BYTES = 100
URLS = ("support_url.txt", "marketing_url.txt", "privacy_url.txt")
# What's New, written for each release; empty until then, which deliver skips.
OPTIONAL = ("release_notes.txt",)
REVIEW_BYTES = 4000
# Apple's guideline 2.3.10, and the words fastlane precheck refuses
# before a submission (its other-platforms and test-words rules).
FORBIDDEN = re.compile(
    r"\bAndroid\b|\bGoogle\b|Play Store|F-Droid|TestFlight|\btesting\b|just a test|alpha test|beta test",
    re.I,
)


def read(path):
    """The file's text without its final newline, or None when missing."""
    if not os.path.isfile(path):
        return None
    with open(path, encoding="utf-8") as f:
        return f.read().rstrip("\n")


def check_text(name, text):
    """The failures of one listing text."""
    failures = []
    if text is None:
        return [f"{name} is missing"]
    if not text.strip():
        return [f"{name} is empty"]
    limit = LIMITS.get(os.path.basename(name))
    if limit is not None and len(text) > limit:
        failures.append(f"{name} has {len(text)} characters, over {limit}")
    if os.path.basename(name) == "keywords.txt":
        size = len(text.encode("utf-8"))
        if size > KEYWORD_BYTES:
            failures.append(f"{name} has {size} bytes, over {KEYWORD_BYTES}")
        short = [word for word in text.split(",") if len(word.strip()) <= 2]
        if short:
            failures.append(f"{name} has keywords of two characters or fewer: {short}")
        if ", " in text:
            failures.append(f"{name} carries a space after a comma")
        if "\n" in text:
            failures.append(f"{name} spans more than one line")
    if os.path.basename(name) in URLS and not re.fullmatch(r"https://\S+", text):
        failures.append(f"{name} is not one https URL")
    match = FORBIDDEN.search(text)
    if match:
        failures.append(f"{name} names {match.group(0)!r}")
    return failures


def doc_block(text):
    """The first fenced block of the App Review notes document, or None."""
    match = re.search(r"^```text\n(.*?)\n```$", text, re.M | re.S)
    return match.group(1) if match else None


def check_review(notes, doc):
    """The failures of the App Review notes and their document."""
    failures = check_text("review_information/notes.txt", notes)
    if notes is not None and len(notes.encode("utf-8")) > REVIEW_BYTES:
        size = len(notes.encode("utf-8"))
        failures.append(f"review_information/notes.txt has {size} bytes, over {REVIEW_BYTES}")
    if doc is None:
        failures.append("docs/agents/app-review-notes.md is missing")
    elif notes is not None and doc_block(doc) != notes:
        failures.append("docs/agents/app-review-notes.md's notes block differs from review_information/notes.txt")
    return failures


def failures_in(metadata=METADATA, review_notes=REVIEW_NOTES, review_doc=REVIEW_DOC):
    """Every failure of the listing on disk."""
    failures = []
    for locale in LOCALES:
        for name in list(LIMITS) + ["keywords.txt"] + list(URLS):
            failures += check_text(f"{locale}/{name}", read(os.path.join(metadata, locale, name)))
        for name in OPTIONAL:
            if read(os.path.join(metadata, locale, name)) is None:
                failures.append(f"{locale}/{name} is missing")
    failures += check_review(read(review_notes), read(review_doc))
    return failures


def main():
    failures = failures_in()
    for failure in failures:
        print(f"::error::App Store listing: {failure}")
    if not failures:
        print("App Store listing: every text within its limit.")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

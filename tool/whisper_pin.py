"""Checks the pin of the submodule third_party/whisper.cpp (#247), for CI,
the release gate and verify-release.

    python3 tool/whisper_pin.py

Exits 1, with GitHub errors, when the pin is not one: when the commit
checked out in third_party/whisper.cpp is not the commit of the tag
third_party/README.md names, or when licenses/common.txt names another
release than that tag (as a Dependabot update leaves them), or when that
cannot be checked. Whether the tag is upstream's latest release is only a
GitHub warning, never a failure, because an update needs the Transcripts
checked first. Asks GitHub's API, with $GITHUB_TOKEN or $GH_TOKEN when set.
"""

import json
import os
import re
import subprocess
import sys
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SUBMODULE = os.path.join(ROOT, "third_party", "whisper.cpp")
README = os.path.join(ROOT, "third_party", "README.md")
LICENCES = os.path.join(ROOT, "licenses", "common.txt")
API = "https://api.github.com/repos/ggml-org/whisper.cpp"


def readme_tag(text):
    """The tag the whisper.cpp row of third_party/README.md pins, or None."""
    match = re.search(r"^\| `?whisper\.cpp`? .*?`(v[0-9][0-9.]*)`", text, re.MULTILINE)
    return match.group(1) if match else None


def licence_tag(text):
    """The release licenses/common.txt names for whisper.cpp, or None."""
    match = re.search(r"github\.com/ggml-org/whisper\.cpp, release (v[0-9][0-9.]*)", text)
    return match.group(1) if match else None


def report(pinned, tag, tag_commit, latest, latest_commit, licence):
    """The lines to print; a line that starts with ::error:: fails the
    check, one that starts with ::warning:: is only reported. latest is
    None when upstream's latest release could not be asked for."""
    lines = [f"whisper.cpp: third_party/whisper.cpp is at {pinned}, third_party/README.md pins {tag}"]
    if tag_commit != pinned:
        lines.append(
            f"::error::third_party/whisper.cpp is at {pinned}, but {tag} in "
            f"third_party/README.md is {tag_commit}: name the tag the submodule is at"
        )
    if licence != tag:
        lines.append(
            f"::error::licenses/common.txt names whisper.cpp release {licence}, "
            f"third_party/README.md {tag}"
        )
    if latest is None:
        lines.append("::warning::whisper.cpp: could not ask for upstream's latest release")
    elif latest_commit == pinned:
        lines.append(f"whisper.cpp: {latest} is upstream's latest release")
    else:
        lines.append(
            f"::warning::whisper.cpp is behind: upstream's latest release is {latest} "
            f"({latest_commit}). Update the submodule once the Transcripts of the "
            "test recordings are checked (docs/adr/0045)"
        )
    return lines


def failed(lines):
    """Whether the report holds an error."""
    return any(line.startswith("::error::") for line in lines)


def github(path):
    """GitHub's API answer for a path under whisper.cpp's repository."""
    request = urllib.request.Request(API + path, headers={"Accept": "application/vnd.github+json"})
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def main():
    errors = (OSError, ValueError, KeyError, subprocess.CalledProcessError)
    try:
        # Not checked out, git would answer for this repository instead.
        if not os.path.exists(os.path.join(SUBMODULE, ".git")):
            raise OSError("third_party/whisper.cpp is not checked out")
        pinned = subprocess.run(
            ["git", "-C", SUBMODULE, "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        with open(README, encoding="utf-8") as f:
            tag = readme_tag(f.read())
        with open(LICENCES, encoding="utf-8") as f:
            licence = licence_tag(f.read())
        if not tag:
            raise ValueError("third_party/README.md names no tag for whisper.cpp")
        tag_commit = github(f"/commits/{tag}")["sha"]
    except errors as error:
        print(f"::error::whisper.cpp: could not check the pin: {error}")
        return 1
    try:
        latest = github("/releases/latest")["tag_name"]
        latest_commit = github(f"/commits/{latest}")["sha"]
    except errors:
        latest = latest_commit = None
    lines = report(pinned, tag, tag_commit, latest, latest_commit, licence)
    for line in lines:
        print(line)
    return 1 if failed(lines) else 0


if __name__ == "__main__":
    sys.exit(main())

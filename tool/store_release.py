"""Keeps a release's store files in step with its release notes record.

    python3 tool/store_release.py appstore docs/release-notes-0.9.0.md [--check]

writes the App Store's What's New, the `appstore_whatsnew_en` and
`appstore_whatsnew_de` blocks word for word, into
`fastlane/metadata/ios/<locale>/release_notes.txt`, where `fastlane
deliver` reads it on a submission. With `--check` it writes nothing and
fails when a file differs, so a tag never carries an empty or stale
What's New.

    python3 tool/store_release.py frames docs/release-notes-0.9.0.md v0.8.0

fails unless the console steps of the record name exactly the store
frames that changed since that tag: the Play steps' "replace phone
screenshots …" the phone frames under `fastlane/metadata/android/`, the
App Store steps' "Replace frames …" the iPhone frames under
`fastlane/metadata/ios/`. "none" names no frame.
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOCALES = {"en": "en-US", "de": "de-DE"}
PLAY = "fastlane/metadata/android"
IOS = "fastlane/metadata/ios"


def block(lines, block_id):
    """The text of a block of the record; ValueError when it is missing
    or empty."""
    marker = f"<!-- block id={block_id} "
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
        if text:
            return text
        break
    raise ValueError(f"the record has no {block_id} block")


def appstore(lines, root=ROOT, check=False):
    """Writes or checks the App Store's What's New files; the list of
    problems, empty when all is well."""
    problems = []
    for lang, locale in LOCALES.items():
        text = block(lines, f"appstore_whatsnew_{lang}") + "\n"
        path = os.path.join(root, IOS, locale, "release_notes.txt")
        if check:
            try:
                with open(path, encoding="utf-8") as f:
                    if f.read() != text:
                        problems.append(f"{locale}/release_notes.txt is not the record's appstore_whatsnew_{lang}")
            except OSError:
                problems.append(f"{locale}/release_notes.txt is missing")
        else:
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
    return problems


def named(lines, heading, verb):
    """The frame numbers a console section's step names after `verb`."""
    grab = False
    for line in lines:
        if line.startswith("## "):
            grab = line.strip() == heading
            continue
        if not grab:
            continue
        m = re.search(verb + r" (none|[0-9][0-9, and]*)", line)
        if m:
            words = m.group(1)
            return set() if words == "none" else {int(n) for n in re.findall(r"[0-9]+", words)}
    raise ValueError(f'"{heading}" names no frames after "{verb}"')


def changed(base, folder, root=ROOT):
    """The numbers of the frames under `folder` that differ from `base`."""
    out = subprocess.run(
        ["git", "diff", "--name-only", base, "--", folder],
        cwd=root, capture_output=True, text=True, check=True,
    ).stdout
    numbers = set()
    for path in out.split():
        name = os.path.basename(path)
        m = re.match(r"([0-9]+)-.*\.png$", name)
        if m and ("phoneScreenshots" in path or "iphone-" in path):
            numbers.add(int(m.group(1)))
    return numbers


def frames(lines, base, root=ROOT):
    """The problems of the console steps against the frames that changed."""
    problems = []
    pairs = [
        ("## Play Console, in order", "replace phone screenshots", PLAY),
        ("## App Store Connect, in order", "Replace frames", IOS),
    ]
    for heading, verb, folder in pairs:
        said = named(lines, heading, verb)
        real = changed(base, folder, root)
        if said != real:
            problems.append(
                f'{heading[3:]}: names {sorted(said) or "none"}, but {sorted(real) or "none"} changed since {base}'
            )
    return problems


def main(argv):
    if len(argv) >= 2 and argv[0] == "appstore":
        with open(argv[1], encoding="utf-8") as f:
            problems = appstore(f.readlines(), check="--check" in argv[2:])
    elif len(argv) == 3 and argv[0] == "frames":
        with open(argv[1], encoding="utf-8") as f:
            problems = frames(f.readlines(), argv[2])
    else:
        sys.exit("usage: store_release.py appstore <notes> [--check] | frames <notes> <base tag>")
    for p in problems:
        print(p, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except (OSError, ValueError, subprocess.CalledProcessError) as e:
        sys.exit(str(e))

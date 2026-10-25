"""Renders the desktop app's AppStream metainfo from its template (#45).

    python3 tool/metainfo.py <template> <CHANGELOG.md> <version> <date> > out.xml

The version goes into the screenshot URLs, which point at that version's
GitHub Release. Every CHANGELOG section up to that version becomes a
<release>: the version itself dated <date> (YYYY-MM-DD), each older one
dated by its git tag; an older version without a tag is left out.
"""
import re
import subprocess
import sys
from xml.sax.saxutils import escape

from whats_new import section

HEADING = re.compile(r"^## \[(\d+)\.(\d+)\.(\d+)\]")


def versions(lines):
    """The versions the CHANGELOG has a section for, newest first."""
    found = []
    for line in lines:
        m = HEADING.match(line)
        if m:
            found.append(tuple(int(p) for p in m.groups()))
    return sorted(found, reverse=True)


def tag_date(version):
    """The date of the tag v<version>, or None without one."""
    out = subprocess.run(
        ["git", "log", "-1", "--format=%cs", f"v{version}"],
        capture_output=True,
        text=True,
        check=False,
    )
    if out.returncode != 0:
        return None
    return out.stdout.strip() or None


def release(version, date, lines):
    """One <release> with the version's CHANGELOG entries as its notes."""
    entries = [
        line[2:].strip().replace("`", "")
        for line in section(version, lines)
        if line.startswith("- ")
    ]
    items = "".join(f"\n          <li>{escape(e)}</li>" for e in entries)
    return (
        f'<release version="{version}" date="{date}" type="stable">\n'
        "      <description>\n"
        f"        <ul>{items}\n        </ul>\n"
        "      </description>\n"
        "    </release>"
    )


def render(template, lines, version, date, dated=tag_date):
    current = tuple(int(p) for p in version.split("."))
    releases = []
    for v in versions(lines):
        if v > current:
            continue
        text = ".".join(str(p) for p in v)
        when = date if v == current else dated(text)
        if when:
            releases.append(release(text, when, lines))
    if not releases or not releases[0].startswith(f'<release version="{version}"'):
        raise SystemExit(f"CHANGELOG.md has no section for {version}")
    return template.replace("@VERSION@", version).replace(
        "<!-- @RELEASES@ -->", "\n    ".join(releases)
    )


if __name__ == "__main__":
    template_path, changelog_path, version, date = sys.argv[1:5]
    with open(template_path, encoding="utf-8") as f:
        template = f.read()
    with open(changelog_path, encoding="utf-8") as f:
        lines = f.readlines()
    sys.stdout.write(render(template, lines, version, date))

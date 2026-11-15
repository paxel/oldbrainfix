"""Checks the highest glibc a Linux build of the desktop app needs (#332),
for the release: the tarball the Homebrew formula installs, the .deb and
the AppImage use the system's glibc, so the binary must not need a newer
one than the oldest system the install page names.

    python3 tool/glibc_ceiling.py <binary> <highest allowed, such as 2.35>

Reads the binary's dynamic symbols with objdump -T. Exits 1, with a GitHub
error, when the binary needs a newer GLIBC_ version than allowed, or when
that cannot be read.
"""

import re
import subprocess
import sys

VERSION = re.compile(r"\bGLIBC_([0-9]+(?:\.[0-9]+)*)\b")


def version(text):
    """A version such as 2.35 as a tuple of numbers, so 2.4 sorts below
    2.35 and 2.2.5 below 2.3; None when it is not one."""
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+)*", text):
        return None
    return tuple(int(part) for part in text.split("."))


def highest(symbols):
    """The highest GLIBC_ version objdump -T's output names, as text, or
    None when it names none. GLIBCXX_ and GLIBC_PRIVATE are not glibc
    releases and are left out."""
    found = {match.group(1) for match in VERSION.finditer(symbols)}
    return max(found, key=version, default=None)


def report(binary, needed, allowed):
    """The lines to print; a line that starts with ::error:: fails the
    check. needed is None when the binary names no GLIBC_ version."""
    if needed is None:
        return [f"::error::{binary} names no GLIBC_ version: not a binary linked against glibc"]
    if version(needed) > version(allowed):
        return [
            f"::error::{binary} needs glibc {needed}, newer than {allowed}: it would not "
            f"start on the oldest Linux the install page names. Build it on a system "
            f"with glibc {allowed}"
        ]
    return [f"{binary} needs glibc {needed}, at most {allowed}"]


def failed(lines):
    """Whether the report holds an error."""
    return any(line.startswith("::error::") for line in lines)


def main(argv):
    if len(argv) != 3 or version(argv[2]) is None:
        print("usage: glibc_ceiling.py <binary> <highest allowed glibc, such as 2.35>")
        return 2
    binary, allowed = argv[1], argv[2]
    try:
        symbols = subprocess.run(
            ["objdump", "-T", binary],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"::error::could not read the symbols of {binary}: {error}")
        return 1
    lines = report(binary, highest(symbols), allowed)
    for line in lines:
        print(line)
    return 1 if failed(lines) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

#!/usr/bin/env sh
# Regenerates cargo-sources.json, the crates of desktop/Cargo.lock as
# Flatpak sources, so the Flatpak builds offline (#45). Run it after every
# change to the lock file; CI fails while the committed file is stale.
#
#   desktop/packaging/flatpak/cargo-sources.sh
#
# Uses flatpak-builder-tools' flatpak-cargo-generator.py at a pinned
# commit, checked against its sha256, with pinned Python packages in a
# throwaway virtualenv. Needs python3 (with venv) and curl.
set -eu
commit=41c20aa10819cdb2a4f3ca171758a96d1955c018
sha256=0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
lock=$here/../../Cargo.lock
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir "$work/tool"
curl -fsSL -o "$work/tool/flatpak-cargo-generator.py" \
  "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/$commit/cargo/flatpak-cargo-generator.py"
echo "$sha256  $work/tool/flatpak-cargo-generator.py" | sha256sum -c - >/dev/null
python3 -m venv "$work/venv"
"$work/venv/bin/pip" install --quiet --disable-pip-version-check \
  "aiohttp==3.12.15" "PyYAML==6.0.2" "tomlkit==0.13.3"
"$work/venv/bin/python" -I "$work/tool/flatpak-cargo-generator.py" "$lock" \
  -o "$here/cargo-sources.json"

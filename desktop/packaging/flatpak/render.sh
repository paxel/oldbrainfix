#!/usr/bin/env sh
# Writes the Flatpak's build files into <out dir> (#45): the manifest,
# cargo-sources.json and the metainfo rendered from its template and the
# CHANGELOG. That directory is what the Flathub repository holds, and
# what pack.sh builds the .flatpak file from.
#
#   packaging/flatpak/render.sh <out dir> <version> <date> [<tag> <commit>]
#
# date (YYYY-MM-DD) is the release's in the metainfo. With a tag and its
# commit the app's source is that tag of the GitHub repository, as Flathub
# builds it; without, it is this checkout.
set -eu
out=$1
version=$2
date=$3
tag=${4:-}
commit=${5:-}
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../../.." && pwd)
id=io.github.paxel.oldbrainfix

if [ -n "$tag" ]; then
  [ -n "$commit" ] || { echo "render.sh: a tag needs its commit" >&2; exit 1; }
fi
source="      - type: git
        url: https://github.com/paxel/oldbrainfix.git
        tag: $tag
        commit: $commit"

mkdir -p "$out"
if [ -n "$tag" ]; then
  # The marker's comment and the entry after it, up to the next entry,
  # become the tag.
  awk -v source="$source" '
    /# app-source:/ { print source; skipping = 1; next }
    skipping && /^      - / { if (entered) skipping = 0; else entered = 1 }
    !skipping { print }
  ' "$here/$id.yml" > "$out/$id.yml"
else
  # The checkout, wherever the manifest lands.
  sed "s|^        path: \.\./\.\./\.\.\$|        path: $root|" "$here/$id.yml" > "$out/$id.yml"
fi
cp "$here/cargo-sources.json" "$out/"
python3 "$root/tool/metainfo.py" "$here/$id.metainfo.xml.in" "$root/CHANGELOG.md" \
  "$version" "$date" > "$out/$id.metainfo.xml"

#!/usr/bin/env sh
# Renders the Homebrew cask, the Homebrew formula and the Scoop manifest
# from the templates, with the checksums of the built artifacts.
#
#   packaging/render-channels.sh <version> <dist dir> <out dir>
set -eu
version=$1
dist=$2
out=$3
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sha() { sha256sum "$dist/$1" | cut -d' ' -f1; }
mkdir -p "$out"
sed -e "s/{{VERSION}}/$version/g" \
    -e "s/{{SHA_MACOS_ARM64}}/$(sha "oldbrainfix-$version-macos-arm64.dmg")/" \
    -e "s/{{SHA_MACOS_X86_64}}/$(sha "oldbrainfix-$version-macos-x86_64.dmg")/" \
    "$here/homebrew/oldbrainfix.rb.tmpl" > "$out/oldbrainfix-cask.rb"
sed -e "s/{{VERSION}}/$version/g" \
    -e "s/{{SHA_LINUX_ARM64}}/$(sha "oldbrainfix-$version-linux-arm64.tar.gz")/" \
    -e "s/{{SHA_LINUX_X86_64}}/$(sha "oldbrainfix-$version-linux-x86_64.tar.gz")/" \
    "$here/homebrew/oldbrainfix-linux.rb.tmpl" > "$out/oldbrainfix-formula.rb"
sed -e "s/{{VERSION}}/$version/g" \
    -e "s/{{SHA_WINDOWS_X86_64}}/$(sha "oldbrainfix-$version-windows-x86_64.zip")/" \
    "$here/scoop/oldbrainfix.json.tmpl" > "$out/oldbrainfix.json"
ls -1 "$out"

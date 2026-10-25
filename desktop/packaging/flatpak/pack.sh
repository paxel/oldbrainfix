#!/usr/bin/env sh
# Builds the Flatpak bundle from source with the manifest render.sh
# writes, the one Flathub builds (#45). Needs flatpak, flatpak-builder and
# python3; the Freedesktop runtime, SDK and Rust extension the manifest
# names are installed from Flathub for the user when missing.
#
#   packaging/flatpak/pack.sh <version> <arch> <dist dir> [<repo dir>]
#
# arch is x86_64 or arm64 (the artifact names); Flatpak calls them
# x86_64 and aarch64. The release date in the metainfo is $RELEASE_DATE
# (YYYY-MM-DD), else today. With a repo dir the OSTree repository is kept
# there for flatpak-builder-lint. The bundle installs with
# `flatpak install --user oldbrainfix-<version>-linux-<arch>.flatpak` and
# pulls its runtime from Flathub.
set -eu
version=$1
arch=$2
dist=$3
keep=${4:-}
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
id=io.github.paxel.oldbrainfix
case $arch in
  x86_64) fp_arch=x86_64 ;;
  arm64) fp_arch=aarch64 ;;
  *) fp_arch=$arch ;;
esac
mkdir -p "$dist"
dist=$(CDPATH= cd -- "$dist" && pwd)
stage=$(mktemp -d)

"$here/render.sh" "$stage/files" "$version" "${RELEASE_DATE:-$(date -u +%F)}"

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
# No rofiles-fuse: FUSE is not reliable on CI runners.
(cd "$stage" && flatpak-builder --user --arch="$fp_arch" --install-deps-from=flathub \
  --disable-rofiles-fuse --force-clean --repo=repo build "files/$id.yml")
flatpak build-bundle --arch="$fp_arch" \
  --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  "$stage/repo" "$dist/oldbrainfix-$version-linux-$arch.flatpak" "$id"
if [ -n "$keep" ]; then
  rm -rf "$keep"
  mkdir -p "$(dirname -- "$keep")"
  mv "$stage/repo" "$keep"
fi
rm -rf "$stage"
ls -1 "$dist"

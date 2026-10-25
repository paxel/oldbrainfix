#!/usr/bin/env sh
# Packs the Flatpak bundle from a built binary. Needs flatpak and
# flatpak-builder; the Freedesktop runtime and SDK the manifest names
# are installed from Flathub for the user when missing.
#
#   packaging/flatpak/pack.sh <version> <arch> <binary> <dist dir>
#
# arch is x86_64 or arm64 (the artifact names); Flatpak calls them
# x86_64 and aarch64. The bundle installs with
# `flatpak install --user oldbrainfix-<version>-linux-<arch>.flatpak` and
# pulls its runtime from Flathub.
set -eu
version=$1
arch=$2
binary=$3
dist=$4
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$here/../../..
id=io.github.paxel.oldbrainfix
case $arch in
  x86_64) fp_arch=x86_64 ;;
  arm64) fp_arch=aarch64 ;;
  *) fp_arch=$arch ;;
esac
mkdir -p "$dist"
dist=$(CDPATH= cd -- "$dist" && pwd)
stage=$(mktemp -d)

cp "$here/$id.yml" "$stage/"
cp "$binary" "$stage/oldbrainfix"
# Without the marker: inside the sandbox the app never rewrites it.
grep -v '^X-Oldbrainfix-Managed=' "$here/../linux/$id.desktop" > "$stage/$id.desktop"
cp "$here/../icon/icon_256.png" "$root/LICENSE-APACHE" "$root/LICENSE-MIT" "$stage/"

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
# No rofiles-fuse: FUSE is not reliable on CI runners.
(cd "$stage" && flatpak-builder --user --arch="$fp_arch" --install-deps-from=flathub \
  --disable-rofiles-fuse --force-clean --repo=repo build "$id.yml")
flatpak build-bundle --arch="$fp_arch" \
  --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  "$stage/repo" "$dist/oldbrainfix-$version-linux-$arch.flatpak" "$id"
rm -rf "$stage"
ls -1 "$dist"

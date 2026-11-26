#!/usr/bin/env sh
# Packs the Linux release from a built binary: the tarball the Homebrew
# formula installs, a .deb, an .rpm and an AppImage.
#
#   packaging/linux/pack.sh <version> <arch> <binary> <dist dir>
#
# arch is x86_64 or arm64 (the artifact names). nfpm (the .deb and the
# .rpm) and appimagetool are fetched when missing; appimagetool needs
# file(1).
set -eu
version=$1
arch=$2
binary=$3
dist=$4
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
icons=$(CDPATH= cd -- "$here/../icon" && pwd)
root=$(CDPATH= cd -- "$here/../../.." && pwd)
entry=io.github.paxel.oldbrainfix.desktop
mkdir -p "$dist"
stage=$(mktemp -d)

# The tarball: everything flat, as the formula expects. The app writes
# the AXT file's type (io.github.paxel.oldbrainfix.xml) itself on start.
bundle="$stage/bundle"
mkdir -p "$bundle"
cp "$binary" "$bundle/oldbrainfix"
chmod 0755 "$bundle/oldbrainfix"
cp "$here/$entry" "$here/install-icon.sh" "$bundle/"
cp "$icons/icon_256.png" "$bundle/"
cp "$root/LICENSE-APACHE" "$root/LICENSE-MIT" "$bundle/"
tar -C "$bundle" -czf "$dist/oldbrainfix-$version-linux-$arch.tar.gz" .

# The .deb and the .rpm, from one description: binary, launcher, icon,
# the AXT file's type the entry's MimeType names (obf-1146) and the
# licences. nfpm calls the arches amd64 and arm64 and names them as each
# format does.
case $arch in
  x86_64) pkg_arch=amd64 ;;
  arm64) pkg_arch=arm64 ;;
  *) pkg_arch=$arch ;;
esac
# nfpm, pinned and checked against its sha256 (obf-1152), for the arch
# this runs on; $NFPM names one already there.
nfpm_version=2.49.0
nfpm="${NFPM:-}"
if [ -z "$nfpm" ]; then
  case $(uname -m) in
    x86_64)
      nfpm_arch=x86_64
      nfpm_sha=2afaa5ffb5e3493402e31e7c67a5a1fb42133e3f66fc315ca0051934125561c7
      ;;
    aarch64 | arm64)
      nfpm_arch=arm64
      nfpm_sha=d4eee71839e92c68cbd3b101640bad32d2856fc551acae0635be9118e48c9d9c
      ;;
    *)
      echo "pack.sh: no nfpm for $(uname -m)" >&2
      exit 1
      ;;
  esac
  archive="$stage/nfpm.tar.gz"
  curl -sSfL -o "$archive" \
    "https://github.com/goreleaser/nfpm/releases/download/v$nfpm_version/nfpm_${nfpm_version}_Linux_$nfpm_arch.tar.gz"
  echo "$nfpm_sha  $archive" | sha256sum -c - >/dev/null ||
    { echo "pack.sh: the nfpm archive's sha256 is not the pinned one" >&2; exit 1; }
  mkdir -p "$stage/nfpm"
  tar -C "$stage/nfpm" -xzf "$archive" nfpm
  nfpm="$stage/nfpm/nfpm"
fi
# Depends: what the binary links (readelf -d), plus the libraries winit
# opens at run time. glibc at least the release's build runner's,
# Ubuntu 22.04's 2.35, which tool/glibc_ceiling.py holds the binary to.
# The .rpm names the libraries themselves, so Fedora and openSUSE, which
# name their packages differently, both find them.
cat > "$stage/nfpm.yaml" <<NFPM
name: oldbrainfix
arch: $pkg_arch
platform: linux
version: $version
section: utils
priority: optional
maintainer: paxel <taum@tuta.io>
homepage: https://github.com/paxel/oldbrainfix
license: MIT OR Apache-2.0
description: |-
  Memory aid for forgetful minds
  Old Brain Fix captures a thought in seconds and nags until it is
  sorted or done: Notes, Routines and Reminders, kept on this machine
  and synced with the phone over the home network or a shared folder,
  with no server and no account.
contents:
  - src: $(realpath -- "$binary")
    dst: /usr/bin/oldbrainfix
    file_info:
      mode: 0755
  - src: $here/$entry
    dst: /usr/share/applications/$entry
    file_info:
      mode: 0644
  - src: $here/io.github.paxel.oldbrainfix.xml
    dst: /usr/share/mime/packages/io.github.paxel.oldbrainfix.xml
    file_info:
      mode: 0644
  - src: $icons/icon_256.png
    dst: /usr/share/icons/hicolor/256x256/apps/oldbrainfix.png
    file_info:
      mode: 0644
  - src: $root/LICENSE-APACHE
    dst: /usr/share/doc/oldbrainfix/LICENSE-APACHE
    file_info:
      mode: 0644
  - src: $root/LICENSE-MIT
    dst: /usr/share/doc/oldbrainfix/LICENSE-MIT
    file_info:
      mode: 0644
overrides:
  deb:
    depends:
      - libc6 (>= 2.35)
      - libgcc-s1
      - libstdc++6
      - libxkbcommon0
      - libasound2t64 | libasound2
  rpm:
    depends:
      - libc.so.6(GLIBC_2.35)(64bit)
      - libgcc_s.so.1()(64bit)
      - libstdc++.so.6()(64bit)
      - libxkbcommon.so.0()(64bit)
      - libasound.so.2()(64bit)
NFPM
"$nfpm" package --config "$stage/nfpm.yaml" --packager deb \
  --target "$dist/oldbrainfix-$version-linux-$arch.deb" >/dev/null
"$nfpm" package --config "$stage/nfpm.yaml" --packager rpm \
  --target "$dist/oldbrainfix-$version-linux-$arch.rpm" >/dev/null

# The AppImage.
appdir="$stage/AppDir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/256x256/apps"
cp "$binary" "$appdir/usr/bin/oldbrainfix"
chmod 0755 "$appdir/usr/bin/oldbrainfix"
cp "$here/$entry" "$appdir/$entry"
cp "$here/$entry" "$appdir/usr/share/applications/$entry"
cp "$icons/icon_256.png" "$appdir/oldbrainfix.png"
cp "$icons/icon_256.png" "$appdir/usr/share/icons/hicolor/256x256/apps/oldbrainfix.png"
cat > "$appdir/AppRun" <<'APPRUN'
#!/bin/sh
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$here/usr/bin/oldbrainfix" "$@"
APPRUN
chmod 0755 "$appdir/AppRun"
case $arch in
  x86_64) tool_arch=x86_64 ;;
  arm64) tool_arch=aarch64 ;;
  *) tool_arch=$arch ;;
esac
tool="${APPIMAGETOOL:-$stage/appimagetool}"
if [ ! -x "$tool" ]; then
  curl -sSfL -o "$tool" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$tool_arch.AppImage"
  chmod 0755 "$tool"
fi
ARCH=$tool_arch "$tool" --appimage-extract-and-run "$appdir" \
  "$dist/oldbrainfix-$version-linux-$arch.AppImage" >/dev/null
rm -rf "$stage"
ls -1 "$dist"

#!/usr/bin/env sh
# Packs the Linux release from a built binary: the tarball the Homebrew
# formula installs, a .deb, and an AppImage. The Flatpak is packed by
# packaging/flatpak/pack.sh.
#
#   packaging/linux/pack.sh <version> <arch> <binary> <dist dir>
#
# arch is x86_64 or arm64 (the artifact names); the .deb calls them
# amd64 and arm64. appimagetool is fetched when missing and needs file(1).
set -eu
version=$1
arch=$2
binary=$3
dist=$4
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
icons=$here/../icon
root=$here/../../..
entry=io.github.paxel.oldbrainfix.desktop
mkdir -p "$dist"
stage=$(mktemp -d)

# The tarball: everything flat, as the formula expects.
bundle="$stage/bundle"
mkdir -p "$bundle"
cp "$binary" "$bundle/oldbrainfix"
chmod 0755 "$bundle/oldbrainfix"
cp "$here/$entry" "$here/install-icon.sh" "$bundle/"
cp "$icons/icon_256.png" "$bundle/"
cp "$root/LICENSE-APACHE" "$root/LICENSE-MIT" "$bundle/"
tar -C "$bundle" -czf "$dist/oldbrainfix-$version-linux-$arch.tar.gz" .

# The .deb: binary, launcher, icon.
case $arch in
  x86_64) deb_arch=amd64 ;;
  arm64) deb_arch=arm64 ;;
  *) deb_arch=$arch ;;
esac
deb="$stage/deb"
mkdir -p "$deb/DEBIAN" "$deb/usr/bin" "$deb/usr/share/applications" \
  "$deb/usr/share/icons/hicolor/256x256/apps" "$deb/usr/share/doc/oldbrainfix"
cp "$binary" "$deb/usr/bin/oldbrainfix"
chmod 0755 "$deb/usr/bin/oldbrainfix"
cp "$here/$entry" "$deb/usr/share/applications/$entry"
cp "$icons/icon_256.png" "$deb/usr/share/icons/hicolor/256x256/apps/oldbrainfix.png"
cp "$root/LICENSE-APACHE" "$root/LICENSE-MIT" "$deb/usr/share/doc/oldbrainfix/"
# Depends: what the binary links (readelf -d), plus the libraries winit
# opens at run time.
cat > "$deb/DEBIAN/control" <<CONTROL
Package: oldbrainfix
Version: $version
Section: utils
Priority: optional
Architecture: $deb_arch
Maintainer: paxel <taum@tuta.io>
Depends: libc6, libgcc-s1, libstdc++6, libxkbcommon0, libasound2t64 | libasound2
Homepage: https://github.com/paxel/oldbrainfix
Description: Memory aid for forgetful minds
 Old Brain Fix captures a thought in seconds and nags until it is
 sorted or done: Notes, Routines and Reminders, kept on this machine
 and synced with the phone over the home network or a shared folder,
 with no server and no account.
CONTROL
dpkg-deb --build --root-owner-group "$deb" "$dist/oldbrainfix-$version-linux-$arch.deb" >/dev/null

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

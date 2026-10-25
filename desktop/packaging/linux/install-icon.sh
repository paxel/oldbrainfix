#!/usr/bin/env sh
# Install the Old Brain Fix launcher entry and its icon into the current
# user's XDG directories, so the app appears in the application menu.
# Idempotent; --uninstall takes them out again.
#
# Homebrew's own share directory is not on a desktop session's XDG_DATA_DIRS, so an
# entry installed there alone stays invisible to menus; this copies into ~/.local/share,
# which every session reads. On Wayland it also gives the window its taskbar icon: there
# is no protocol for a client to set its own, so the compositor looks for the entry named
# after the window's app id, io.github.paxel.oldbrainfix.desktop. The app itself writes
# the same entry on every start.
#
# Usage:
#   install-icon.sh                launcher entry and icon
#   install-icon.sh --uninstall    remove them again
set -eu

mode=install
for argument in "$@"; do
  case $argument in
    -h | --help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    --uninstall)
      mode=uninstall
      ;;
    *)
      echo "install-icon: unknown argument $argument" >&2
      exit 2
      ;;
  esac
done

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Works both from a source checkout (desktop/packaging/linux/../icon) and from the
# release tarball, where the icon and the .desktop file sit next to this script.
if [ -f "$here/icon_256.png" ]; then
  icons=$here
else
  icons=$here/../icon
fi
desktop_source="$here/io.github.paxel.oldbrainfix.desktop"

data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
desktop_target="$data_home/applications/io.github.paxel.oldbrainfix.desktop"
png_target="$data_home/icons/hicolor/256x256/apps/oldbrainfix.png"

refresh_caches() {
  # Where the tools exist (harmless if they don't).
  command -v gtk-update-icon-cache >/dev/null 2>&1 &&
    gtk-update-icon-cache -f -t "$data_home/icons/hicolor" >/dev/null 2>&1 || true
  command -v update-desktop-database >/dev/null 2>&1 &&
    update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true
  command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 >/dev/null 2>&1 || true
}

if [ "$mode" = uninstall ]; then
  for target in "$desktop_target" "$png_target"; do
    if [ -e "$target" ]; then
      rm -f "$target"
      echo "  removed $target"
    fi
  done
  refresh_caches
  echo "Old Brain Fix is out of your application menu."
  exit 0
fi

# The 256 pixel PNG is what every desktop draws.
mkdir -p "$(dirname "$desktop_target")" "$(dirname "$png_target")"
cp "$icons/icon_256.png" "$png_target"
echo "  $png_target"
cp "$desktop_source" "$desktop_target"
echo "  $desktop_target"

refresh_caches

echo "Old Brain Fix is in your application menu. A window already open keeps its old"
echo "icon; start Old Brain Fix again to see the new one."

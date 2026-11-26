#!/usr/bin/env sh
# Builds the apt repository the Pages site carries under /apt/ (obf-1153)
# from the .deb files in <deb dir>, signed with the project's key:
#
#   tool/apt-repo.sh <deb dir> <out dir>
#
# The key is $APT_SIGNING_KEY (the ASCII-armoured secret key) and its
# passphrase $APT_SIGNING_PASSPHRASE, as docs/agents/apt-repository.md
# makes them. Writes <out dir>/pool/main/o/oldbrainfix/*.deb,
# dists/stable/main/binary-{amd64,arm64}/Packages(.gz), dists/stable/
# Release, InRelease and Release.gpg, and the public key oldbrainfix.asc
# that users add. Needs apt-ftparchive (apt-utils), dpkg-deb and gpg.
# Without Valid-Until: a repository not rebuilt for a while stays
# usable.
set -eu
debs=$1
out=$2
: "${APT_SIGNING_KEY:?tool/apt-repo.sh: APT_SIGNING_KEY is not set}"
passphrase=${APT_SIGNING_PASSPHRASE:-}
arches="amd64 arm64"

set -- "$debs"/*.deb
[ -e "$1" ] || { echo "tool/apt-repo.sh: no .deb in $debs" >&2; exit 1; }

# Each by Debian's name for it, package_version_arch.deb, by which
# apt-ftparchive tells the arches apart.
pool="$out/pool/main/o/oldbrainfix"
mkdir -p "$pool"
for deb in "$@"; do
  fields=$(dpkg-deb --field "$deb" Package Version Architecture | awk '{ print $2 }' | tr '\n' '_')
  cp "$deb" "$pool/${fields%_}.deb"
done
for arch in $arches; do
  dir="$out/dists/stable/main/binary-$arch"
  mkdir -p "$dir"
  # From the repository's top, so each Filename reads pool/...
  (cd "$out" && apt-ftparchive --arch "$arch" packages pool) > "$dir/Packages"
  gzip -9nkf "$dir/Packages"
done

# Written beside, then moved in: Release does not hash itself.
release=$(mktemp)
(cd "$out/dists/stable" && apt-ftparchive \
  -o APT::FTPArchive::Release::Origin="Old Brain Fix" \
  -o APT::FTPArchive::Release::Label="Old Brain Fix" \
  -o APT::FTPArchive::Release::Suite=stable \
  -o APT::FTPArchive::Release::Codename=stable \
  -o APT::FTPArchive::Release::Architectures="$arches" \
  -o APT::FTPArchive::Release::Components=main \
  -o APT::FTPArchive::Release::Description="Old Brain Fix for the desktop" \
  release .) > "$release"
mv "$release" "$out/dists/stable/Release"

# A keyring of its own, gone afterwards; the passphrase goes in on stdin.
GNUPGHOME=$(mktemp -d)
export GNUPGHOME
trap 'gpgconf --kill gpg-agent >/dev/null 2>&1 || true; rm -rf "$GNUPGHOME"' EXIT
printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --quiet --import
key=$(gpg --batch --with-colons --list-secret-keys | awk -F: '/^fpr:/ { print $10; exit }')
[ -n "$key" ] || { echo "tool/apt-repo.sh: APT_SIGNING_KEY holds no secret key" >&2; exit 1; }
sign() {
  printf '%s' "$passphrase" | gpg --batch --yes --quiet --pinentry-mode loopback \
    --passphrase-fd 0 --local-user "$key" --digest-algo SHA512 "$@"
}
sign --clearsign --output "$out/dists/stable/InRelease" "$out/dists/stable/Release"
sign --armor --detach-sign --output "$out/dists/stable/Release.gpg" "$out/dists/stable/Release"
gpg --batch --armor --export "$key" > "$out/oldbrainfix.asc"

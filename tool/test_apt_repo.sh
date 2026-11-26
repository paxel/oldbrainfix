#!/usr/bin/env sh
# Tests tool/apt-repo.sh (obf-1153): two versions for amd64 and one for
# arm64, signed with a throwaway key, give a repository whose InRelease
# and Release.gpg verify against the key it publishes, whose Packages
# name every version by its path in the pool, and whose Release hashes
# them. Needs apt-ftparchive (apt-utils), dpkg-deb and gpg.
#
#   tool/test_apt_repo.sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
work=$(mktemp -d)
GNUPGHOME="$work/gnupg"
export GNUPGHOME
mkdir -m 0700 "$GNUPGHOME"
trap 'gpgconf --kill gpg-agent >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }

deb() {
  root="$work/build/$1-$2"
  mkdir -p "$root/DEBIAN" "$root/usr/bin"
  printf '#!/bin/sh\n' > "$root/usr/bin/oldbrainfix"
  chmod 0755 "$root/usr/bin/oldbrainfix"
  printf 'Package: oldbrainfix\nVersion: %s\nArchitecture: %s\nMaintainer: test <t@example.org>\nDescription: test\n' \
    "$1" "$2" > "$root/DEBIAN/control"
  dpkg-deb --build --root-owner-group "$root" "$work/debs/oldbrainfix-$1-$2.deb" >/dev/null
}
mkdir -p "$work/debs"
deb 0.1.0 amd64
deb 0.2.0 amd64
deb 0.2.0 arm64

gpg --batch --quiet --passphrase secret --pinentry-mode loopback \
  --quick-gen-key "Test apt key <t@example.org>" ed25519 sign never
key=$(gpg --batch --armor --pinentry-mode loopback --passphrase secret --export-secret-keys)

# Without the key it refuses.
if APT_SIGNING_KEY='' sh "$here/apt-repo.sh" "$work/debs" "$work/none" 2>/dev/null; then
  fail "built without a key"
fi

APT_SIGNING_KEY=$key APT_SIGNING_PASSPHRASE=secret GNUPGHOME='' \
  sh "$here/apt-repo.sh" "$work/debs" "$work/repo"
repo="$work/repo"
dists="$repo/dists/stable"

# Signed by the key it publishes.
gpg --batch --quiet --dearmor --output "$work/key.gpg" "$repo/oldbrainfix.asc"
gpgv --keyring "$work/key.gpg" "$dists/InRelease" 2>/dev/null || fail "InRelease does not verify"
gpgv --keyring "$work/key.gpg" "$dists/Release.gpg" "$dists/Release" 2>/dev/null ||
  fail "Release.gpg does not verify"

# Every version by its path in the pool, each arch apart.
versions() { awk '/^Version:/ { print $2 }' "$dists/main/binary-$1/Packages" | sort | tr '\n' ' '; }
[ "$(versions amd64)" = "0.1.0 0.2.0 " ] || fail "amd64 holds $(versions amd64)"
[ "$(versions arm64)" = "0.2.0 " ] || fail "arm64 holds $(versions arm64)"
grep '^Filename:' "$dists/main/binary-amd64/Packages" | grep -qv '^Filename: pool/main/o/oldbrainfix/' &&
  fail "a Filename outside the pool"
for name in $(awk '/^Filename:/ { print $2 }' "$dists"/main/binary-*/Packages); do
  [ -f "$repo/$name" ] || fail "$name is not in the pool"
done

# Release hashes what it lists, names both arches and never expires.
sum=$(sha256sum "$dists/main/binary-amd64/Packages" | cut -d' ' -f1)
grep -q " $sum .* main/binary-amd64/Packages\$" "$dists/Release" || fail "Release hashes Packages wrongly"
grep -q '^Architectures: amd64 arm64$' "$dists/Release" || fail "Release names other arches"
grep -q '^Suite: stable$' "$dists/Release" || fail "Release has no suite stable"
if grep -q '^Valid-Until:' "$dists/Release"; then fail "Release expires"; fi
echo "tool/test_apt_repo.sh: ok"

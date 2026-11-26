# Apt repository setup

What the user does once so the Pages site carries the signed apt repository
(obf-1153) at `https://paxel.github.io/oldbrainfix/apt/`, from which Debian and
Ubuntu install Old Brain Fix with `apt install oldbrainfix` and get later
versions with their system's updates. Until both secrets exist, the Pages
workflow skips the repository with a notice and the site has no `/apt/`; the
install page then names commands that find nothing.

## How it is built

- `.github/workflows/pages.yml`, on a `v*` tag only, downloads the `.deb` files
  (amd64 and arm64) of the 10 newest published Releases (`APT_VERSIONS`; about
  50 MB a version, and a Pages site may be 1 GB) and runs `tool/apt-repo.sh`,
  which writes `site-dist/apt/`: the pool, `dists/stable/main/binary-{amd64,arm64}/Packages`,
  `Release` signed as `InRelease` and `Release.gpg`, and the public key
  `oldbrainfix.asc`.
- A deploy replaces the whole site, so every run builds the repository anew
  from the Releases; nothing else holds it.
- The tag push runs Pages before the tag's Release exists. The release
  workflow's job `apt` runs Pages again on the tag once the Release holds its
  `.deb` files, and that run adds the new version.
- `tool/test_apt_repo.sh` tests the builder with a throwaway key in CI's job
  "Tools".

## 1. Make the key

On a trusted computer with GnuPG:

- [ ] `gpg --quick-gen-key "Old Brain Fix apt repository <taum@tuta.io>" ed25519 sign never`
      and give it a long passphrase when asked. `never`: the key does not
      expire, because every user holds a copy of the public key in
      `/etc/apt/keyrings/oldbrainfix.asc`, and an expired key stops their
      updates until they fetch it again.
- [ ] `gpg --list-secret-keys --keyid-format long` and note the key's
      fingerprint, the long hex line under `sec`.
- [ ] GnuPG wrote a revocation certificate to
      `~/.gnupg/openpgp-revocs.d/<fingerprint>.rev`. Keep it, with a backup of
      the key (`gpg --armor --export-secret-keys <fingerprint>`), somewhere
      offline: lost, the repository needs a new key and every user a new
      public key.

## 2. Store the secrets

In the repository: Settings → Secrets and variables → Actions → New
repository secret, or with `gh`:

- [ ] `APT_SIGNING_KEY`: the ASCII-armoured secret key,
      `gpg --armor --export-secret-keys <fingerprint> | gh secret set APT_SIGNING_KEY`.
- [ ] `APT_SIGNING_PASSPHRASE`: its passphrase,
      `gh secret set APT_SIGNING_PASSPHRASE` (asks for the value).

Both are read only by the Pages workflow's step "Apt repository from the
Releases' .deb files", on tags; never on a branch or a pull request.

## 3. Check after the next release

- [ ] The Pages run of the tag, started by the release workflow's job `apt`,
      shows no "Apt repository skipped" notice.
- [ ] `https://paxel.github.io/oldbrainfix/apt/dists/stable/InRelease` opens and
      is signed; `https://paxel.github.io/oldbrainfix/apt/oldbrainfix.asc` is
      the public key.
- [ ] On Debian or Ubuntu, the two commands of the install page
      (`site/en/reference/install.html`) install the new version, and
      `apt-cache policy oldbrainfix` lists the earlier ones.

## A new key

Only when the key is lost or exposed: revoke the old one with its revocation
certificate, make a new key as in step 1, replace both secrets and release
again. Every user has to fetch the new public key with the first command of
the install page; say so in the release notes.

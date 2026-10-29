<p align="center">
  <img src="site/assets/icon.png" width="128" alt="Old Brain Fix icon"/>
</p>

<h1 align="center">Old Brain Fix</h1>

<p align="center">
  <a href="https://github.com/paxel/oldbrainfix/actions/workflows/ci.yml"><img src="https://github.com/paxel/oldbrainfix/actions/workflows/ci.yml/badge.svg?branch=summer" alt="CI"/></a>
  <a href="https://github.com/paxel/oldbrainfix/actions/workflows/security.yml"><img src="https://github.com/paxel/oldbrainfix/actions/workflows/security.yml/badge.svg?branch=summer" alt="Security"/></a>
  <a href="https://github.com/paxel/oldbrainfix/releases/latest"><img src="https://img.shields.io/github/v/release/paxel/oldbrainfix" alt="Release"/></a>
  <a href="https://paxel.github.io/fdroid/"><img src="https://img.shields.io/badge/F--Droid-paxel.github.io-1976D2?logo=f-droid" alt="F-Droid repository"/></a>
  <a href="https://paxel.github.io/oldbrainfix/"><img src="https://img.shields.io/badge/docs-paxel.github.io-informational" alt="Documentation"/></a>
  <img src="https://img.shields.io/badge/platforms-Android%20%7C%20Windows%20%7C%20macOS%20%7C%20Linux-informational" alt="Platforms"/>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License"/></a>
</p>

<p align="center">
  Note and reminder app for Android, with a desktop app for Linux, Windows and macOS.<br/>
  No account, no server. Free and open source.
</p>

---

Notes are captured as text, photo or voice and filed into Areas and Topics.
Routines and Reminders ring at the exact minute and repeat until they are
marked Done. Phone and desktop sync over the local network or a shared folder.
All data is stored on the user's devices.

The documentation, in English and German, is at
**[paxel.github.io/oldbrainfix](https://paxel.github.io/oldbrainfix/)**.

Work in progress. The domain language is in [CONTEXT.md](CONTEXT.md), the
decisions in [docs/adr](docs/adr), the plan in the GitHub issues.

## Install

| Platform | Install                                                                                                               |
|----------|-----------------------------------------------------------------------------------------------------------------------|
| Android  | Add the F-Droid repository `https://paxel.github.io/fdroid/repo`, or the APK from the latest release                  |
| Linux    | `brew install paxel/tap/oldbrainfix`, or the `.deb`, `.flatpak` or `.AppImage` from the latest release                |
| macOS    | `brew install --cask paxel/tap/oldbrainfix`, or the `.dmg` from the latest release                                    |
| Windows  | `scoop bucket add paxel https://github.com/paxel/scoop-bucket`, then `scoop install paxel/oldbrainfix`, or the `.zip` |

The macOS app is not signed with an Apple Developer certificate; on the first
start, choose Open Anyway in System Settings, Privacy & Security. Details per
platform are on the [install page](https://paxel.github.io/oldbrainfix/en/reference/install.html).

## License

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at
your option.
Every third-party component the apps ship or download, such as the ocrs text
recognition models (CC BY-SA 4.0) and OpenAI's Whisper speech model (MIT),
is listed with its licence in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md);
the full texts are in [licenses/](licenses) and in each app's open-source
licences.

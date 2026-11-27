# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.9.0]

### Added

- On tablets and iPads from 700 dp wide, the app shows the desktop's layout: a sidebar beside the list, a Note as a dialog. ([obf-1148](https://github.com/paxel/oldbrainfix/issues/464))
- The app runs on the iPad as an iPad app, not at iPhone size. ([obf-1149](https://github.com/paxel/oldbrainfix/issues/465))
- Export as AXT file… hands Notes, a selection, a Topic or an Area to other apps that speak AXT. ([obf-1145](https://github.com/paxel/oldbrainfix/issues/461))
- An AXT file from another app opens in Old Brain Fix, and its ticked records become Notes in the place picked. ([obf-1146](https://github.com/paxel/oldbrainfix/issues/462))
- A later AXT file from the same app updates or removes the Notes it made, unless you changed them since. ([obf-1147](https://github.com/paxel/oldbrainfix/issues/463))
- Fedora and openSUSE get an .rpm of the desktop app, beside the .deb, the AppImage and the tarball. ([obf-1152](https://github.com/paxel/oldbrainfix/issues/470))
- Debian and Ubuntu can install the desktop app from an apt repository and get each new version with their updates. ([obf-1153](https://github.com/paxel/oldbrainfix/issues/471))

### Removed

- The Flatpak is no longer offered; Linux users take the tarball, the .deb, the AppImage or Homebrew. ([obf-1151](https://github.com/paxel/oldbrainfix/issues/469))

### Fixed

- On the desktop Agenda, a Session's picks wrap onto more lines instead of being cut off. ([obf-1160](https://github.com/paxel/oldbrainfix/issues/480))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

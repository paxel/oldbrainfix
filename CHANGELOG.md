# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.1]

### Changed

- Typed text made a Checklist or a Shopping List keeps every line as a Line, its first line too; only a first line that looks like a heading, a Markdown heading or a line ending in ":", becomes the list's name, and a list without one shows "Checklist" or "Shopping List" until a name is typed. ([obf-1002](https://github.com/paxel/oldbrainfix/issues/309))

### Fixed

- On the desktop, Cut, Copy and Paste in a text field's right-click menu and in the Edit menu reach the system clipboard, also in a Note's own window, as Ctrl+X, Ctrl+C and Ctrl+V do. ([obf-1012](https://github.com/paxel/oldbrainfix/issues/319))
- On the desktop, a click on a Tally a Session picked opens only its entry form, on the Agenda as on the Session's card and in its Note. The Tally's Note no longer opens with it, and after Save the Agenda or the Session shows again. ([obf-1013](https://github.com/paxel/oldbrainfix/issues/320))
- On the phone, a tap in the text of the Note editor and of Add to Note puts the cursor where the finger is, no longer at the nearest word boundary. ([obf-1005](https://github.com/paxel/oldbrainfix/issues/312))
- The Linux tarball for Homebrew, the .deb and the AppImage start on older systems: they need glibc 2.35 or newer, such as Ubuntu 22.04 or Debian 12. ([obf-1025](https://github.com/paxel/oldbrainfix/issues/332))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.0]

### Added

- On the desktop, `oldbrainfix --version` and `oldbrainfix -V` print the installed version, such as "Old Brain Fix 0.6.0", without starting the app.

### Changed

- On the phone and the desktop, Sync picks a sync bundle's Device or Everything with radio buttons instead of a dropdown.

### Fixed

- Sending a Note to a person, on the phone and the desktop, no longer picks the first person in the list: nobody is picked until you choose, and an update starts at the person this Device last sent the Note to, so a private Note does not go to the wrong person.
- On the desktop, the app no longer keeps a processor core busy while it waits in the tray or idles: the line under the menu bar moves at a steady pace and stops after two minutes if a sync hangs, nothing is drawn while the window is hidden, and the AI port answers a tool call when it comes instead of checking ten times a second.
- On the desktop, a start of an updated app ends an older version still running in the tray and runs in its place, instead of opening the older version's window until it was quit.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

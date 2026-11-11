# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.0]

### Fixed

- On the desktop, the app no longer keeps a processor core busy while it waits in the tray or idles: the line under the menu bar moves at a steady pace and stops after two minutes if a sync hangs, nothing is drawn while the window is hidden, and the AI port answers a tool call when it comes instead of checking ten times a second.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

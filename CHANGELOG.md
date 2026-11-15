# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.1]

### Added

- Capture offers "Add to Note…", which adds the text, recording, photos or files to a chosen Note at once. ([obf-1001](https://github.com/paxel/oldbrainfix/issues/308))
- Capture offers Text, Checklist and Shopping List above the field, and a list keeps each typed line as a Line. ([obf-1003](https://github.com/paxel/oldbrainfix/issues/310))
- A spoken shopping list goes into a Shopping List as Lines, its recording kept, also without the speech model. ([obf-1004](https://github.com/paxel/oldbrainfix/issues/311))

### Changed

- The Kind badge comes first and names the Kind in words, such as "Plain Note ▾". ([obf-1003](https://github.com/paxel/oldbrainfix/issues/310))
- Text made a list keeps its first line as a Line unless that line is a heading. ([obf-1002](https://github.com/paxel/oldbrainfix/issues/309))
- A Note's added parts continue its text under a dashed tear line instead of sitting in boxes of their own. ([obf-1000](https://github.com/paxel/oldbrainfix/issues/307))
- Once a Note from Unsorted is sorted, the next Unsorted Note opens, and the Agenda when none is left. ([obf-1006](https://github.com/paxel/oldbrainfix/issues/313))
- On the desktop, asking is experimental and off until "AI (experimental)" is ticked; the AI port works either way. ([obf-1027](https://github.com/paxel/oldbrainfix/issues/334))
- On the desktop, AI ▸ AI settings… opens one page with every AI setting. ([obf-1028](https://github.com/paxel/oldbrainfix/issues/335))
- On the phone, asking is experimental and off until "Ask (experimental)" is switched on in the Settings. ([obf-1026](https://github.com/paxel/oldbrainfix/issues/333))

### Fixed

- On the desktop, Cut, Copy and Paste in the right-click and Edit menus reach the clipboard. ([obf-1012](https://github.com/paxel/oldbrainfix/issues/319))
- On the desktop, a Session's Tally opens only its entry form, not its Note as well. ([obf-1013](https://github.com/paxel/oldbrainfix/issues/320))
- On the phone, a tap in the Note editor puts the cursor where the finger is. ([obf-1005](https://github.com/paxel/oldbrainfix/issues/312))
- The Linux .deb, AppImage and Homebrew build start on older systems with glibc 2.35, such as Ubuntu 22.04 or Debian 12. ([obf-1025](https://github.com/paxel/oldbrainfix/issues/332))
- A phone's question no longer goes missing, and Ask on the phone and desktop says what it is doing and why it failed. ([obf-1010](https://github.com/paxel/oldbrainfix/issues/317))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

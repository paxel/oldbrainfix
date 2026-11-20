# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.7.0]

### Changed

- On the phone the search field of the Areas list, an Area and a Topic sits on a band that shares a colour set off from the page with the top bar, with a line under it, so the cards go under it when they scroll or the list is pulled down. ([obf-1041](https://github.com/paxel/oldbrainfix/issues/351))
- One "Remind me…" sets up a Routine or a Reminder wherever a Note offered "Routine…" and "Reminder…" apart, on the phone and the desktop: a card's alarm clock and ⋮, the Note editor's "+", the Note dialog's "+", the Note menu and a card's right-click. Its editor starts as a Reminder, and picking a repeat makes it a Routine, keeping what was set. ([obf-1034](https://github.com/paxel/oldbrainfix/issues/344))
- The quick note, on the phone and in the desktop's quick note dialog and capture popup, shows only the text, the three Nudge buttons and More, which saves the Note and opens it in the full editor or the Note dialog. Back, the iPhone's back arrow and Esc save it as before, and the Save on top is gone. Photo, Voice, Gallery, Paste, Add to a Shopping List and, on the desktop, Record voice and Paste are now in the Note editor and the Note dialog, beside the Kind, Remind me…, File into…, the Frog and the Importance; Settings ▸ Quick note keeps only the Nudge buttons switch. ([obf-1042](https://github.com/paxel/oldbrainfix/issues/352))

### Fixed

- On macOS a click on an Old Brain Fix notification brings the window up on what it is about: an Occurrence's Note dialog with its Due line, Unsorted for a Nudge. ([obf-1039](https://github.com/paxel/oldbrainfix/issues/349))
- On Android a tap on the Catch-up Notification shows the Agenda with the Missed Summary, from whatever page the app showed. ([obf-1039](https://github.com/paxel/oldbrainfix/issues/349))
- A voice recording, a Shopping List spoken in too, no longer stops when the phone locks: the screen stays on while recording, and locked anyway the recording goes on until it is stopped, on Android also from its notification, and is kept and transcribed as before. ([obf-1040](https://github.com/paxel/oldbrainfix/issues/350))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

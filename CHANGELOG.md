# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.3.0]

### Changed

- "Send my Profile" and "Send yours back" send the Profile card as a small file with the picture, such as `Ann.obfcard`, instead of a link, on the phone through the app picked and on the desktop saved where the user says in place of "Copy my Profile link", and a card file tapped in a messenger, the Files app or a download on the phone, or dropped onto the desktop's window, picked under "Import bundle…" or given to `oldbrainfix open`, opens in the app and adds the person by the same rules; card links no longer open the app, and the QR and pasted links work as before.

### Fixed

- In the phone's two-row editor, the row above the keyboard shows the cursor's line from its start with the cursor right after the text before it, and scrolls sideways only when the line is wider than the row, to keep the cursor in view.
- On the phone, the Note editor's table button opens the full-screen view on the table it just put in, its first cell being edited, also in a plain Note holding a box typed since the last save, where the view stayed empty.
- On the phone, the Note editor's section "Routines and Reminders" offers Frog and Session beside Routine and Reminder, opening what the card's menu opens, and on the phone and the desktop All Routines and Reminders has "New Session", which makes a Session whose Note is the text typed in its editor.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

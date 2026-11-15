# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.1]

### Added

- Capture offers "Add to Note…", on the phone and on the desktop, in the quick note and the capture popup: it picks another Note with a filter, the Notes added to last at the top, and adds the text, voice recording, photos or files to it at once as a Section, so nothing waits in Unsorted. Added from Capture to a Note in a Shared Area, the Section is there for everyone in it. On an Unsorted Note's card, Merge into… is now called "Add to Note…" and picks the Note the same way. ([obf-1001](https://github.com/paxel/oldbrainfix/issues/308))
- Capture offers Text, Checklist and Shopping List above the field, on the phone and on the desktop, in the quick note and the capture popup. Checklist or Shopping List saves the Note as that list at once, each typed line a Line unless the first line is a heading, which becomes its name. A recording, photo or file stays on the list, and with nothing typed the Note stays a Plain Note. ([obf-1003](https://github.com/paxel/oldbrainfix/issues/310))
- A voice capture can go into a Shopping List without the speech model too, on the phone and on the desktop. The recording now stays on the list as a Section, so a misheard item can be listened to, instead of the Note being discarded, and the items said become Lines once the recording is transcribed, split at commas, at "and" and "und" and at pauses. Without the speech model they follow once it is downloaded. ([obf-1004](https://github.com/paxel/oldbrainfix/issues/311))

### Changed

- The kind badge of the Note editor and of the desktop's Note dialog comes first and names the kind in words, such as "Plain Note ▾" or "Checklist ▾", instead of showing an icon. ([obf-1003](https://github.com/paxel/oldbrainfix/issues/310))
- Typed text made a Checklist or a Shopping List keeps every line as a Line, its first line too; only a first line that looks like a heading, a Markdown heading or a line ending in ":", becomes the list's name, and a list without one shows "Checklist" or "Shopping List" until a name is typed. ([obf-1002](https://github.com/paxel/oldbrainfix/issues/309))
- A Note reads as one Note: its Sections, each an Amend or a Note merged in, have no box of their own and are parted from the text before them only by a dashed tear line, with their time in a small grey line under them. In the Note editor and the desktop's Note dialog and Note window, a Plain Note's Sections now follow its text in the same box, above the badges; a Checklist, Shopping List, Tally or Session keeps them at the end of the page. ([obf-1000](https://github.com/paxel/oldbrainfix/issues/307))
- Sorting Unsorted goes on by itself, on the phone and on the desktop. Once a Note opened from Unsorted is filed, merged, added to a Note, made a list or a Tally, or given a Routine, Reminder, Frog or Session, the Note editor or the desktop's Note dialog shows the next Unsorted Note, and once none is left the Agenda shows. Sorted from the list itself, the list stays, and the desktop selects its first card. On the desktop, Ctrl+Z brings the sorted Note back into Unsorted and shows it there again. ([obf-1006](https://github.com/paxel/oldbrainfix/issues/313))
- On the desktop, asking is experimental and off until "AI (experimental)" is ticked at the top of the AI menu or of the AI settings, also where a provider was set up before. While it is off there is no Ask panel, no AI ▸ Ask…, no Ctrl+K and no provider card in the AI settings, the phone hears that the computer's AI is off, and the provider and key stay kept; the AI port works either way. ([obf-1027](https://github.com/paxel/oldbrainfix/issues/334))
- On the desktop, AI ▸ AI settings… opens one page with every AI setting, and the Settings keep one row "AI…" that opens it. ([obf-1028](https://github.com/paxel/oldbrainfix/issues/335))
- On the phone, asking is experimental and off until "Ask (experimental)" is switched on in the Settings, offered in Advanced only. While it is off there is no Ask in the Agenda's bar, no Ask page and no Address by hand on the Sync page; an address typed before stays, and sync still tries it. ([obf-1026](https://github.com/paxel/oldbrainfix/issues/333))

### Fixed

- On the desktop, Cut, Copy and Paste in a text field's right-click menu and in the Edit menu reach the system clipboard, also in a Note's own window, as Ctrl+X, Ctrl+C and Ctrl+V do. ([obf-1012](https://github.com/paxel/oldbrainfix/issues/319))
- On the desktop, a click on a Tally a Session picked opens only its entry form, on the Agenda as on the Session's card and in its Note. The Tally's Note no longer opens with it, and after Save the Agenda or the Session shows again. ([obf-1013](https://github.com/paxel/oldbrainfix/issues/320))
- On the phone, a tap in the text of the Note editor and of Add to Note puts the cursor where the finger is, no longer at the nearest word boundary. ([obf-1005](https://github.com/paxel/oldbrainfix/issues/312))
- The Linux tarball for Homebrew, the .deb and the AppImage start on older systems: they need glibc 2.35 or newer, such as Ubuntu 22.04 or Debian 12. ([obf-1025](https://github.com/paxel/oldbrainfix/issues/332))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

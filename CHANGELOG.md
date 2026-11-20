# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.7.0]

### Added

- Heads-ups: a Routine or Reminder can warn ahead of its Occurrence, such as "Dentist in 20 min", with as many as wanted, set in its editor, whole days only for an all-day one, on Android, the iPhone and the desktop. A heads-up is one notification without Re-nag, silent in Quiet Hours; Done, Skip or Snooze before it cancels it, and the Occurrence still rings at its time. In a Shared Area each person sets their own for a shared Routine, kept on their own Devices only. ([obf-1046](https://github.com/paxel/oldbrainfix/issues/356))
- A quiet day: "Quiet today" in the Agenda's menu keeps today quiet on all your own Devices, on the phone and the desktop. The Agenda still shows everything, each to check off on its own, and says "Quiet day until midnight" with End, but nothing rings, Re-nags, warns ahead, escalates or nudges that day. What is left undone at midnight counts as skipped, never as Missed. End brings back what is still to come, not what already passed. ([obf-1044](https://github.com/paxel/oldbrainfix/issues/354))
- A Note shows the Notes that link to it: "Linked from" at its end, after its Sections, lists each Note whose text or Sections hold an app link to it by its first line, and a tap or click opens it, in the detailed card, the Note editor, the Note dialog and a Note's own window, on the phone and the desktop. Notes in the trash, of a Shared Area you are no member of and of a closed locked Area are left out. ([obf-1035](https://github.com/paxel/oldbrainfix/issues/345))

### Changed

- On the phone the search field of the Areas list, an Area and a Topic sits on a band that shares a colour set off from the page with the top bar, with a line under it, so the cards go under it when they scroll or the list is pulled down. ([obf-1041](https://github.com/paxel/oldbrainfix/issues/351))
- One "Remind me…" sets up a Routine or a Reminder wherever a Note offered "Routine…" and "Reminder…" apart, on the phone and the desktop: a card's alarm clock and ⋮, the Note editor's "+", the Note dialog's "+", the Note menu and a card's right-click. Its editor starts as a Reminder, and picking a repeat makes it a Routine, keeping what was set. ([obf-1034](https://github.com/paxel/oldbrainfix/issues/344))
- The quick note, on the phone and in the desktop's quick note dialog and capture popup, shows only the text, the three Nudge buttons and More, which saves the Note and opens it in the full editor or the Note dialog. Back, the iPhone's back arrow and Esc save it as before, and the Save on top is gone. Photo, Voice, Gallery, Paste, Add to a Shopping List and, on the desktop, Record voice and Paste are now in the Note editor and the Note dialog, beside the Kind, Remind me…, File into…, the Frog and the Importance; Settings ▸ Quick note keeps only the Nudge buttons switch. ([obf-1042](https://github.com/paxel/oldbrainfix/issues/352))
- Quiet Hours are quiet: in them a Routine or Reminder shows on time as a notification without sound or vibration, its Re-nags too, on Android, the iPhone and the desktop. Only a Routine with Escalation on rings with sound from its first ring, as before, and an all-day one still waits until they end. ([obf-1045](https://github.com/paxel/oldbrainfix/issues/355))

### Fixed

- On macOS a click on an Old Brain Fix notification brings the window up on what it is about: an Occurrence's Note dialog with its Due line, Unsorted for a Nudge. ([obf-1039](https://github.com/paxel/oldbrainfix/issues/349))
- On Android a tap on the Catch-up Notification shows the Agenda with the Missed Summary, from whatever page the app showed. ([obf-1039](https://github.com/paxel/oldbrainfix/issues/349))
- A voice recording, a Shopping List spoken in too, no longer stops when the phone locks: the screen stays on while recording, and locked anyway the recording goes on until it is stopped, on Android also from its notification, and is kept and transcribed as before. ([obf-1040](https://github.com/paxel/oldbrainfix/issues/350))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.2.0]

### Added
- On the phone and the desktop, the first start asks whether the user is new to Old Brain Fix, adds a Device of their own or restores a Backup, and "Add a Device of mine" shows this Device's pair code or takes the other Device's, then syncs before any setup, bringing the Profile, Areas and everything else, so that the phone skips the Starter Set and only offers the speech model and the alarm setup, while a failed or cancelled Pairing returns to the first screen.
- On the phone, text Capture has a Gallery button next to Photo and Voice that picks one or more pictures from the phone into the Note as its photos, each with its Photo Text.
- On the phone, an audio file shared in from another app, such as a WhatsApp voice message, becomes a voice Note with its Transcript, several shared at once one Note with a recording each, and a file the phone cannot decode is kept in the Note with an announcement that it could not be transcribed, whose tap shows the reason.
- On the desktop, "From file…" in the capture field and files dropped onto the window make pictures and sound files, such as a WhatsApp voice message in Ogg Opus, m4a, mp3 or WAV, into one Note with their photos and voice recordings, each with its Photo Text or Transcript, a sound file the desktop cannot decode kept in the Note with an announcement whose click shows the reason; voice Notes in Ogg Opus or MP3 from the phone also play and are transcribed on the desktop.
- On the phone and the desktop, a person can be blocked on their Profile page or with "Decline and block" in the preview of a Sent Item from them, which deletes everything they wrote but their check-offs from all of the user's Devices, keeps and shows nothing more of theirs, refuses their Sent Items, leaves the Shared Areas they manage and lists them in a folded "Blocked" section of People, from where Unblock fetches their writing again.
- On the phone, Sync shows the phone's own pair code as a QR with the typed code under it for another phone, a tablet or the desktop to join, Share shows an invite to a Shared Area the same way, and the phone hosts automatic sync for its paired Devices while the app is open and during its background sync, so phones and tablets pair and sync with each other without a computer, and the desktop's Sync view takes such a code typed or pasted in to join a phone or tablet.
- On the phone and the desktop, a person can be added from their Profile card: on the phone "Send my Profile" on the Profile page sends a link through any messenger and "Show my QR" shows it as a QR, "Add a person" in People takes a link pasted, a QR scanned with the camera, which marks the person verified at once, or a QR in a picture, and a card link tapped in another app opens in the app; on the desktop "Copy my Profile link" and "Show my QR" on the Profile page do the same and "Add a person from a link" in People takes a pasted link; a person new to the user is offered "Send yours back" on their Profile page.

### Changed
- On the phone, "+" on the Agenda, also while the app is locked, fans out Text, Voice, Photo and Gallery instead of opening text Capture at once, and closes again on a second tap, a tap outside or Back.
- On the phone and the desktop, the unit of a Routine's "every N", its Re-nag and Escalation, and the Device's Re-nag and Nudge repeat in Settings are columns of radio buttons instead of dropdowns, with a value set elsewhere kept as an extra option.
- On the phone and the desktop, the app starts faster with a speech model: its checksum is computed once when it is downloaded or picked, and again only when the file's size or modification time changes.
- On the phone and the desktop, restoring a Backup brings back the playback speed of voice recordings chosen on the Device it was made on.
- On the phone and the desktop, the preview of a Sent Item from a person not among the user's People shows only the sender's name and that they are unknown until "Show", while Accept, Decline and "Decline and block" work before "Show" too, and such an item left undecided waits, also through a restart, until it is decided, behind an "x Sent Items waiting" line on the Agenda.

### Fixed
- On the phone and the desktop, the Note editor's trash button asks once before deleting the Note, with the same dialog as the Note card's Delete, including the choice to keep or delete its Routines and Reminders.
- On the phone and the desktop, a voice Note without words is titled "Voice recording" instead of "Photo" wherever only its words fit, such as the Agenda, the home-screen widget, the Missed Summary and alarms.
- On the phone and the desktop, a new Transcript in which Whisper only repeats the words it was given, the German lead-in or the list of Area and Topic names, shows "No words heard." instead, and such a repeat before or after real speech is left out of the Transcript.
- On the desktop, voice recordings played faster or slower keep the pitch of the voice, as on the phone.
- On the desktop, an Area lists only its Notes without a Topic, as on the phone; a search in the Area still finds the Notes of its Topics.
- On the desktop, the Agenda shows the whole week, as on the phone: after tomorrow come the five following days, each under its weekday and date, an empty day as "Nothing planned".
- On the desktop, the Note editor shows a photo's Photo Text under the photo, as on the phone, not only after the photo is removed.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

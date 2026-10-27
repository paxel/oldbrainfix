# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.2.0]

### Added
- On the phone, text Capture has a Gallery button next to Photo and Voice that picks one or more pictures from the phone into the Note as its photos, each with its Photo Text.
- On the phone, an audio file shared in from another app, such as a WhatsApp voice message, becomes a voice Note with its Transcript, several shared at once one Note with a recording each, and a file the phone cannot decode is kept in the Note with an announcement that it could not be transcribed, whose tap shows the reason.

### Changed
- On the phone, "+" on the Agenda, also while the app is locked, fans out Text, Voice, Photo and Gallery instead of opening text Capture at once, and closes again on a second tap, a tap outside or Back.
- On the phone and the desktop, the unit of a Routine's "every N", its Re-nag and Escalation, and the Device's Re-nag and Nudge repeat in Settings are columns of radio buttons instead of dropdowns, with a value set elsewhere kept as an extra option.
- On the phone and the desktop, the app starts faster with a speech model: its checksum is computed once when it is downloaded or picked, and again only when the file's size or modification time changes.
- On the phone and the desktop, restoring a Backup brings back the playback speed of voice recordings chosen on the Device it was made on.

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

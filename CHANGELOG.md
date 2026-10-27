# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.2.0]

### Changed
- On the phone and the desktop, the unit of a Routine's "every N", its Re-nag and Escalation, and the Device's Re-nag and Nudge repeat in Settings are columns of radio buttons instead of dropdowns, with a value set elsewhere kept as an extra option.
- On the phone and the desktop, the app starts faster with a speech model: its checksum is computed once when it is downloaded or picked, and again only when the file's size or modification time changes.

### Fixed
- On the phone and the desktop, the Note editor's trash button asks once before deleting the Note, with the same dialog as the Note card's Delete, including the choice to keep or delete its Routines and Reminders.
- On the phone and the desktop, a voice Note without words is titled "Voice recording" instead of "Photo" wherever only its words fit, such as the Agenda, the home-screen widget, the Missed Summary and alarms.
- On the phone and the desktop, a new Transcript in which Whisper only repeats the words it was given, the German lead-in or the list of Area and Topic names, shows "No words heard." instead, and such a repeat before or after real speech is left out of the Transcript.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

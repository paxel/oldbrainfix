# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.1.0]

### Added
- Capture a thought as a text Note with one tap on +; leaving by Back or switching apps keeps it.
- Take a photo as a Note, optionally with a few words; kept photos carry no location data.
- Record a voice Note that starts at once and ends with one big stop button; the recording is kept to play back.
- Transcripts of voice Notes, written on the phone by Whisper in English or German, after a one-time speech model download on request or from a file of your own.
- Text in photos and screenshots is recognised on the phone by ocrs, offline, and kept with the photo.
- Share images, text and links from other apps into a new Unsorted Note.
- App shortcuts, Quick Settings tiles and home-screen widgets that open voice, text or photo capture directly, also for hardware buttons.
- An Agenda home-screen widget that lists today's Routines and Reminders to check off, with the Unsorted count.
- English and German.
- Light teal and anthracite dark themes that follow the system or a choice of your own.
- Notes as embossed cards over a background of hand-drawn question marks that drift slowly while scrolling.
- Areas and Topics of your own, two levels deep, with an optional colour and icon per Area.
- A Starter Set of Areas and Topics (Household, Work, Car, Health) to pick from on first start.
- Unsorted: a Note that is not filed, turned into a Routine or Reminder, merged or deleted.
- Deleting an Area or Topic asks whether its Notes go back to Unsorted, into the Area, or into the trash.
- A notes browser with search across Note text, Transcripts and text in photos, swipe navigation through Areas and Topics, and a diary of cards per Area and Topic.
- A Note editor to fix text in place, add dated parts, set Importance and file the Note.
- Importance: important cards stand out, unimportant ones are dimmed and can be hidden.
- A trash that keeps deleted Notes, Routines and Reminders for 30 days.
- Merge a Note into another; it becomes a dated part of that Note.
- Typed text in the editor survives the app being closed or killed.
- Every Note and every added part records when and where it was captured, the calendar event and Bluetooth device at the time, and for shares the app they came from.
- Named Places that show "Home" or "Work" instead of coordinates, matched offline.
- The Agenda as home screen: today's Routines and Reminders, then tomorrow's, dimmed.
- Routines and Reminders made from any Note, each linking back to it.
- Schedules from once to iCalendar repeat rules, with several times per day, fixed across early or late ticks and daylight-saving changes.
- Alarms on the minute with Done and Snooze (10 min, 30 min, 1 h), even from the lock screen and after a restart.
- A step-by-step permission setup and a red Agenda banner while a needed permission is missing.
- Re-nag: an ignored alarm rings again at an interval, per phone or per Routine, until Done, snoozed or Missed.
- A Missed card on the Agenda that lists what was Missed until you tap "Seen".
- One catch-up notification instead of a burst of old alarms after the phone was off.
- Escalation: an ignored Routine can turn into a full-screen alarm with sound.
- A history per Routine with each Done, Missed and snoozed time, its Streak and its Adherence, with Streaks that can be hidden.
- Nudges that keep reminding you of Unsorted Notes, with a nudge time picked at capture.
- An "x Unsorted" banner on the Agenda.
- Quiet Hours that hold Nudges back at night while Routines still ring.
- Deleting a Note with Routines or Reminders asks whether they go to the trash too.
- Add any Note, Routine or Reminder to your calendar app, filled in, without a calendar permission.
- Export all or picked Routines and Reminders as a shareable calendar file with their repeat rules and alarms.
- Optionally keep all Routines and Reminders in a device calendar of your choice, updated as they change and removed when turned off.
- Settings that belong to each phone and are never synced.
- An optional App Lock with a PIN, fingerprint or screen lock that guards your Notes while capturing and alarms stay open.
- Optional Area Locks that keep single Areas and their Notes hidden until unlocked, with their alarms, Agenda rows, widget and calendar entries shown as "Private Routine" or in full, still checkable.
- An About page with the version, documentation, source, licences, a way to report problems or write to the developer, and a donation link.
- A desktop app in the phone's look that captures, edits and sorts Notes, manages Areas, Topics, Routines and Reminders, shows the Agenda like the phone, records and plays voice Notes, keeps pasted pictures as photo Notes, writes Transcripts with Whisper and Photo Texts with ocrs, and finds Notes by their text, Transcripts and Photo Texts, with drag-and-drop filing, keys in place of swipes, notifications with Re-nag from a tray icon that starts at login unless switched off, and a small capture popup that a global hotkey, the tray or the `oldbrainfix capture` command opens, the command also keeping a given text as a Note without opening anything.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

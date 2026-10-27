# Older changes

Changes of versions before the current one, moved here from [CHANGELOG.md](CHANGELOG.md) on each version bump.

## [0.1.1] - 2026-10-27

### Added
- On the phone, the speech model download is offered once after the Starter Set.
- The phone's Routine editor builds a day of the month, the first to fourth or last weekday of the month, and an end (never, on a day or after a number of times) without typing a repeat rule.
- A big red button on the phone's About page.
- On the phone, a swipe left on the Agenda opens the Areas list and a swipe right from the Areas list returns to the Agenda, and every swipe between Agenda, Areas list, Area and Topic moves the pages with the finger, completing past the middle or with a fling and springing back otherwise.
- A voice recording with a Transcript or a photo with a Photo Text can be removed in the Note editor (on the phone also in the card's ⋮ menu), from every Device and the Shared Area, while the text stays on the Note and in search.
- On the phone and the desktop, a label beside every play button shows the playback speed of voice recordings and opens a menu with 0.5, 0.75, 1, 1.5 and 2 times normal speed; the choice applies at once and stays on this Device.
- On the phone and the desktop, the speech model settings offer Whisper's small model (488 MB) beside the base model (148 MB), better with English words in German speech but slower; choosing one downloads it, and only the chosen model is kept.

### Changed
- The phone's Agenda shows the whole week: after tomorrow come the five following days, each under its weekday and date, an empty day as "Nothing planned".
- On the phone, an Area lists only its Notes without a Topic; a search in the Area still finds the Notes of its Topics.
- The phone's Capture page has large Photo and Voice buttons above the reminder choices instead of icons in the top bar.
- On the phone, every single choice is a column of radio buttons, and every switch, button or list sits on its own line below its text.
- A Note card on the phone shows, top to bottom, the Profile of another person who wrote it, the date with the location, its Area and Topic or Unsorted where the screen does not already name them, the Note, and one row of icons for File into, Routine or Reminder, Merge and ⋮, which holds Importance, Send to a person, the calendar, Add to Note and Delete.
- On the phone and the desktop, Whisper is given the Area and Topic names, and for German speech the app's own words and common English words such as Meeting or Update, so Transcripts spell them as they are written.

### Fixed
- With three-button navigation, the end of the phone's pages no longer lies under the navigation bar.
- On the phone, the Unsorted list, the Nudge screen, the Areas list, an Area, a Topic, All Notes and the trash no longer show Notes as they were before a page opened on top of them, such as the Nudge screen from its notification, changed them.

## [0.1.0] - 2026-10-26

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
- Sync the phone with the desktop over the home network by scanning or typing the code the desktop shows; the two stay paired.
- Paired phone and desktop find each other on the home network and sync on their own, at once while the app is open and about every 15 minutes in the background.
- Things changed on two Devices before a sync show up as conflicts on phone and desktop, and the version you pick stays on every Device.
- Phone and desktop also sync through a shared folder such as Syncthing, Nextcloud or a USB stick, on their own and in the background, with everything in it end-to-end encrypted per Area.
- Send what is new as an encrypted sync bundle file through any messenger and import it on the other paired Device.
- Share a single Area with another person as Manager or Member after confirming their Profile, with their picture on the Notes they write there; every other Area stays private.
- Send a Note with its Routines and Reminders as a signed file carrying your Profile to another Old Brain Fix user, encrypted when you know them, who previews it with your name, picture and short code, can compare that code with you in person, and accepts it into Unsorted with its alarms on; later updates or cancellations from the same sender change it or move it to the trash.
- Encrypted Backups of all Areas, Notes, photos, voice recordings, Routines and their history, by hand or weekly into a folder you choose, restored on a new install with your passphrase or the recovery key on a printable Recovery Sheet.
- One passphrase and one Recovery Sheet open the Backups of all your Devices, a restored Backup brings your Profile back, and an opt-in Google backup keeps only the newest encrypted Backup, offered with the passphrase or recovery key on a new phone.
- An optional App Lock with a PIN, fingerprint or screen lock that guards your Notes while capturing and alarms stay open.
- Optional Area Locks that keep single Areas and their Notes hidden until unlocked, with their alarms, Agenda rows, widget and calendar entries shown as "Private Routine" or in full, still checkable.
- An About page with the version, documentation, source, a way to report problems or write to the developer, and a donation link; it and the desktop's About list every third-party component and its licence.
- A documentation site in English and German with a glossary, what's new and a privacy policy, linked from About on the phone and the desktop.
- A desktop app for Linux, Windows and macOS in the phone's look that captures, edits and sorts Notes, manages Areas, Topics, Routines and Reminders, shows the Agenda like the phone, records and plays voice Notes, keeps pasted pictures as photo Notes, writes Transcripts with Whisper and Photo Texts with ocrs, and finds Notes by their text, Transcripts and Photo Texts, with drag-and-drop filing, keys in place of swipes, notifications with Re-nag from a tray icon that starts at login unless switched off, and a small capture popup that a global hotkey, the tray or the `oldbrainfix capture` command opens, the command also keeping a given text as a Note without opening anything; it comes as .deb, AppImage and Flatpak, from Homebrew and Scoop, and sits in the application menu with its own icon.
- A Profile with your name, picture and short code, taken from the phone's name or the computer login without asking and shared by all of your Devices, where pairing two Devices that each have a named Profile asks once which one is you; its page gathers Backups, your Devices, the People you know and the settings, and every other person's Profile shows their short code, a way to compare it, a name and picture of your own for them that only your Devices see, and opens their Notes; the notes browser can show only the Notes of the people you pick, yourself included.
- On the phone, a crash shows what happened with buttons to restart the app and to mail a report to the developer, and an unexpected end is offered as a report at the next start.

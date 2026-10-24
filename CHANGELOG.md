# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.1.0]

### Added
- Tap + on the Agenda to capture a thought as a text Note; its card shows right away with its options, and it stays after the app is closed. All Notes, newest first with their capture time, open from the top of the notes browser.
- English and German.
- A light teal look with a teal accent, anthracite in dark mode; the theme follows the system.
- Notes show as softly embossed cards over a background of hand-drawn question marks that drift slowly as you scroll and stay still when the system's "remove animations" setting is on.
- Areas and Topics: make, rename and delete your own Areas and the Topics inside them (two levels), and give an Area a colour and an icon that mark its Notes' cards.
- The magnifier opens the notes browser: a search field on top finds Notes by their text, including what was added to them and the Notes merged into them, and shows every hit on one page (✕ clears it); with the field empty it shows your Areas. An Area shows its Topics and its Notes as a diary of cards, newest first; a Topic shows its own. In an Area or Topic, order the cards by time or by Importance and hide the unimportant ones. Inside an Area or Topic the search field only searches its cards.
- In the notes browser, swipe right to go back (Topic, Area, Agenda) and swipe left to go forward into the last Area or Topic you opened, which is highlighted.
- Every level of the notes browser has a +: a new Area; in an Area a menu offering a Note filed straight into the Area or a new Topic; inside a Topic a new Note filed straight into it.
- File a Note into an Area or a Topic from its card; a Note that is not filed anywhere shows as Unsorted.
- Deleting an Area or Topic that still holds Notes asks where they go: back to Unsorted, for a Topic into its Area, or into the trash. An Area's Topics are deleted with it, and their Notes go the same way.
- Tap a card to open the Note's editor, showing its text, when it was captured, where it is filed and everything added to it later. Fix the text in place (saved when you leave), file the Note, or tap "Add to Note" to add a dated part with its own capture time.
- Mark a Note unimportant, normal or important: important cards stand out with an accent marker and a stronger emboss, unimportant ones are dimmed.
- The editor's trash button deletes a Note: it goes to the trash for 30 days, where it can be restored to where it was, and after that it is gone for good. The trash opens from the top level of the notes browser. A deleted Note no longer counts as Unsorted.
- Merge a Note into another from its card or its editor: pick the other Note on a page that lists and searches your Notes. The merged Note becomes a dated part of the other one, with its own capture time and everything added to it, and leaves every list and Unsorted.
- Text you type in a Note's editor is kept even if the app is closed or killed: it is saved when the app goes to the background, and an unsaved draft comes back the next time you open the Note.
- Every Note remembers where it was captured, and so does everything added to it later. The Note is saved at once; the place is added when the phone knows it, or left out when location is off or not allowed. After your first Note the app explains why and asks for the location once; capturing itself never asks.
- Named Places: name the spot you are at (Home, Work, Mum's) with a radius from 50 m to 1 km, and rename, resize or delete it later. A Note captured near one shows its name, everywhere else the coordinates; this works offline and your location never leaves the phone. Named Places open from the top of the notes browser for now.
- Cards show the date, time and place a Note was captured; the editor shows the place for the Note and for each part added to it.
- On first start, pick from a Starter Set of Areas and Topics (Household, Work, Car, Health), all ticked; everything stays editable afterwards and shows in your language until you rename it.
- The app opens on the Agenda: today's Routines and Reminders, the all-day ones first and then by time, and after a gap tomorrow's in dimmer colours. Tick each one off as Done, tomorrow's already today; tick again to take it back. Tap a row to change its schedule or delete it; its Note stays.
- Turn any Note into a Routine or a Reminder from its card (a menu at the button) or from its editor; a Note can have several, each links back to its Note, and the Note no longer counts as Unsorted. The alarm button at the top of the Agenda makes a Routine and its Note in one go.
- Schedules: once (a Reminder), every N days, weeks, months or years, or chosen weekdays, with several times of day each checked off on their own, or all day. An advanced mode takes an iCalendar repeat rule (RRULE) such as "first Monday of the month", and the editor shows the next dates. Ticking off early or late never moves the schedule, and times stay put across daylight-saving changes.
- Routines and Reminders ring on the minute as a notification, all-day ones at 09:00, even when the app is closed. The notification offers Done and Snooze; Snooze then offers 10 min, 30 min or 1 h. Both work from the lock screen without opening the app; ticking an Occurrence off in the app takes its notification away. A Done tapped on the notification lands in the Routine's log the next time the app opens (at once while it runs).
- Alarms keep coming after a restart, an app update, and a change of the clock or the time zone, at the same local time. A time skipped when daylight saving starts rings when the clock jumps past it; a time the clock shows twice rings once.
- A step-by-step setup on first start asks for notifications, alarms on the minute and full-screen alarms. Any step can be passed by: without alarms on the minute they come at a less exact time, and while notifications or alarms on the minute are not allowed (or full-screen alarms, once any Routine escalates) a red banner on the Agenda says so; tapping it opens the setup again.
- The Agenda moves on to the new day at midnight while it is open.
- An ignored alarm rings again every 10 minutes until you tick it off or snooze it; after a snooze it rings again at the snooze and keeps going from there. A Routine stops when its next one is due and then counts as Missed; a Reminder stops at midnight. Each Routine and Reminder can have its own interval (5 min to 1 h) in the Re-nag section of its editor, where the phone's default can be changed too for now.
- A Missed card on top of the Agenda lists what was Missed since you last tapped "Seen" and how often, with the latest ones to tick off late. Opening the app never clears it; only "Seen" does.
- Escalation: switch it on for a Routine or Reminder in the Escalation section of its editor, and after the first ring and 3 ignored Re-nags (or 1, 2, 5 or 10, as you pick) it becomes a full-screen alarm with the phone's alarm sound, even over the lock screen. Tick it off as Done or snooze it for 10 min, 30 min or 1 h right there or on its notification. It rings for up to 5 minutes (at most half the Re-nag interval), then goes back to Re-nagging, and each later Re-nag rings again until it is Done, snoozed or Missed. Without the full-screen permission it comes as an urgent notification with the alarm sound instead.
- After the phone was off or dead, old alarms no longer ring in a burst: one notification says how many were Missed meanwhile, and tapping it opens the Agenda.
- Nudges for Unsorted Notes: the capture screen offers "remind me to sort it in" 10 min, 30 min or 1 h at the bottom, each saving the Note; Save or Back means 1 h. Typed text is never lost: leaving by Back saves it, and so does the app going to the background, without making a second Note when you come back. One Nudge comes at the earliest of these times and covers every Unsorted Note, then again every hour while anything is Unsorted, even with the app closed. It is a normal notification saying how many Notes wait, with snoozes of 10 min, 30 min and 1 h; tapping it opens the three oldest Unsorted cards, each opening its Note or sorting it right there, and "+x" for all of them.
- An "x Unsorted" banner on the Agenda, under the Missed card, opens all Unsorted Notes, oldest first.
- Quiet Hours (22:00 to 08:00 unless changed): Nudges wait until they end, Routines and Reminders still ring on time. Quiet Hours and how long an ignored Nudge waits (10 min, 30 min or 1 h) are set per phone in "Nudges on this phone" on the Unsorted list and the Nudge screen for now.
- Deleting a Note that has Routines or Reminders asks whether to delete them too. Yes puts them in the trash with the Note, and restoring the Note brings them back; No keeps them ringing, and their editor shows that the Note is in the trash, or later that it was deleted.
- Deleting a Routine or Reminder puts it in the trash for 30 days, next to the deleted Notes, where it can be restored; it rings again on its usual schedule, and nothing from its time in the trash counts as Missed.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

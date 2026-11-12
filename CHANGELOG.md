# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.0]

### Added

- On the desktop, `oldbrainfix --version` and `oldbrainfix -V` print the installed version, such as "Old Brain Fix 0.6.0", without starting the app.
- On the phone and the desktop, Snooze (10 min, 30 min, 1 h) is also in the Agenda row's ⋮ (on the desktop also its right-click) and beside Done at the top of the Note editor or Note dialog, so an Occurrence can still be snoozed once its notification is gone.
- On the phone and the desktop, Not today in the notification, the Agenda row's ⋮ and the Note editor or Note dialog offers Skip, which leaves an Occurrence neither Done nor Missed and stops its alarm, and Tomorrow, which skips it and adds a one-time Occurrence the day after at the same time, without changing the schedule; on the desktop Ctrl+Z takes either back, and its notification shows Snooze's and Not today's choices in place.
- A skipped Occurrence shows struck through as skipped on the Agenda for the day, never shows in the Missed Summary, and leaves the Adherence, which counts it beside the share, such as "92 % this month · 3 skipped"; a Session's "skipped" now counts these skips instead of its Missed runs.
- On the phone and the desktop, the When line of the Routine editor also reads a one-time Reminder in words relative to now, such as "tomorrow 7 pm", "tonight", "in 2 hours" or "in 3 days" (in German "morgen 19 Uhr", "heute Abend", "in 2 Stunden"), and times such as "7 pm" (in German "19 Uhr"). "Tomorrow" without a time is when this Device's Quiet Hours end, or 09:00 while they are off, and "tonight" is 19:00. On the phone and the desktop a new Reminder also offers the buttons In 1 h, Tonight, Tomorrow and In a week under the When line, each with its time, which set the Reminder with one tap or click.

### Changed

- On the phone and the desktop, Sync picks a sync bundle's Device or Everything with radio buttons instead of a dropdown.
- On the phone and the desktop, deleting what can be restored no longer asks first: a Note moved to the Trash, a Routine or Reminder, a Frog, a Tally entry, a Line, the "From …" label, a Transcript or a Photo Text goes at once. On the phone the announcement at the top undoes it with a tap for 10 seconds; on the desktop Ctrl+Z and Edit ▸ Undo undo the last delete, check-off, filing or Merge. A Checklist and a Shopping List show their deleted Lines under the folded "Removed Lines" in the Note editor and the desktop's Note dialog, each with Restore.
- On the phone and the desktop, a tap or click on an Occurrence on the Agenda or on its notification opens its Note in the editor or the desktop's Note dialog, with Done, Snooze and Not today at the top once it is due, instead of the Routine editor or the app as it was; Edit schedule in the row's ⋮ (on the desktop also its right-click menu) and the Routine's badge open the schedule.
- On the phone and the desktop, File into… starts with a filter over the Areas and Topics, shows the last three Areas or Topics this Device filed into at the top, and offers New Area… at the end and New Topic… under each Area (in Advanced, for who may set up a Topic there), which make the Area or Topic and file the Note into it at once.

### Fixed

- Sending a Note to a person, on the phone and the desktop, no longer picks the first person in the list: nobody is picked until you choose, and an update starts at the person this Device last sent the Note to, so a private Note does not go to the wrong person.
- On the desktop, the app no longer keeps a processor core busy while it waits in the tray or idles: the line under the menu bar moves at a steady pace and stops after two minutes if a sync hangs, nothing is drawn while the window is hidden, and the AI port answers a tool call when it comes instead of checking ten times a second.
- On the desktop, a start of an updated app ends an older version still running in the tray and runs in its place, instead of opening the older version's window until it was quit.
- On a narrow phone, the days and times in the Missed Summary and the share beside each Adherence figure in a Routine's history wrap onto a second line instead of running off the screen.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

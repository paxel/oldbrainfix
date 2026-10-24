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

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

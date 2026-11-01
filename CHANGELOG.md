# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.3.1]

### Changed

- The Note card's ⋮ no longer holds the Importance, which is changed through its badge in the Note editor.

### Fixed

- A Tally that "Another one" swapped into a Session's pick and that then got its entry stays in the pick, done, in its place, even when another Device wrote the day's first pick later, and an Occurrence that lost such a Tally shows it again, so that it is Done once every pick has its entry.
- The Agenda widget draws every row of today, the Missed, Occurrence, Line, Frog, Shopping List and Checklist rows with their boxes included, where it showed "Couldn't add widget." for all but the text rows.
- The text of a Note's Sections, its Amends and the Notes merged into it, can be edited on the phone and the desktop in the Note editor as the Note's own text can, by the Section's author and in a Shared Area by its Managers, and an edited Section keeps its place and its capture time and shows when it was created, when it was last modified and an "edited by" tag with the name of who edited it, the AI port on the desktop edits one as a Pending Change, and the text of a Note and of each of its Sections can be copied with "Copy text" in the ⋮ of its card on the phone and "Copy as text" on the desktop and selected and copied where the editor only shows it.
- On the phone and the desktop, a Checklist and a Shopping List are edited in the Note editor the same way, as one block of text whose Lines each start with an empty box, where a new list starts with one empty Line, Return adds a Line below or moves the rest of a Line into it, Backspace at the start of a Line joins it with the Line above, the Shopping List has no field and no "Add" button at the top and the Checklist no "Add Line" button, and the frame around a group of Lines holds the group's own Line, and the open Lines of a Shopping List keep the order they were typed in, a new Line coming in at the end and a Line that comes back in its place, where the latest showed at the top.
- On the phone and the desktop, each Shopping List has "Show on the Agenda" in the ⋮ of its card and as a badge in the Note editor, as a Checklist has, with Off, one row with the number of its open Lines, the default, or each open Line on a row of its own, chosen by each person for their own Agenda and the Agenda widget and kept on their own Devices, where one setting of each Device in Settings decided for every list; a list without open Lines never shows, and a Device whose setting was "Not at all" or "Each Line that came back today" starts its lists off or with each open Line.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

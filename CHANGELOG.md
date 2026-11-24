# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.8.0]

### Added

- On the phone and the desktop, a search lists where each Note matches, and a find bar highlights every match inside a Note. ([obf-1047](https://github.com/paxel/oldbrainfix/issues/358))
- On the phone a toolbar with Copy, Paste and Search floats above a selection; on the desktop the right-click menu gains Search. ([obf-1055](https://github.com/paxel/oldbrainfix/issues/366))
- On the desktop, a Markdown file opens in a viewer with Obsidian links, and can be imported with the files it links to. ([obf-1056](https://github.com/paxel/oldbrainfix/issues/367))
- On the phone and the desktop, the Note editor has Undo and Redo, step by step, and Revert all changes in its ⋮. ([obf-1051](https://github.com/paxel/oldbrainfix/issues/362))
- On the phone and the desktop, a table's ⋮ copies it, and in the editor moves it up or down or deletes it. ([obf-1050](https://github.com/paxel/oldbrainfix/issues/361))
- On the phone and the desktop, a tap on a table's header sorts it while reading; the editor sorts its rows by a column. ([obf-1052](https://github.com/paxel/oldbrainfix/issues/363))
- On the phone and the desktop, a long press or drag on a table cell selects a rectangle of cells, which copies as a table. ([obf-1054](https://github.com/paxel/oldbrainfix/issues/365))
- On the phone and the desktop, a Heads-up can have a custom length: a number of minutes, hours, days or weeks. ([obf-1087](https://github.com/paxel/oldbrainfix/issues/400))
- On the phone and the desktop, a Show tips switch in the Settings and No more tips on every tip turn the tips off. ([obf-1060](https://github.com/paxel/oldbrainfix/issues/373))
- On the phone and the desktop, a voice recording can end on its own after a pause of 2, 3, 5 or 10 seconds. ([obf-1053](https://github.com/paxel/oldbrainfix/issues/364))
- On the desktop, a Running page lists the background jobs, how far each is, and Abort; an aborted Import keeps its Notes. ([obf-1078](https://github.com/paxel/oldbrainfix/issues/391))
- On the phone, a Running page lists the background jobs with Abort, opened from the moving line or ⋮. ([obf-1078](https://github.com/paxel/oldbrainfix/issues/391))

### Changed

- On the phone and the desktop, a table keeps the text size, wraps long cells and scrolls sideways; a card shows its first 10 rows. ([obf-1048](https://github.com/paxel/oldbrainfix/issues/359))
- On the phone and the desktop, search ignores accents and umlauts, so "muller" finds "Müller" and "strasse" finds "Straße". ([obf-1047](https://github.com/paxel/oldbrainfix/issues/358))
- On the phone, Code sits early in the bar of formats and also makes a code block of several lines or an empty line. ([obf-1097](https://github.com/paxel/oldbrainfix/issues/410))
- On the phone, the Areas list shows move handles only after Rearrange in its ⋮, so scrolling and swiping no longer catch them. ([obf-1089](https://github.com/paxel/oldbrainfix/issues/402))
- On the phone, the quick note's buttons sit right under the text and no longer jump when the keyboard opens. ([obf-1083](https://github.com/paxel/oldbrainfix/issues/396))
- On the phone and the desktop, the Settings section for Simple and Advanced is called Complexity. ([obf-1086](https://github.com/paxel/oldbrainfix/issues/399))
- On the phone, every section of the Settings is a card with its title on top, Look, Units and Complexity included. ([obf-1085](https://github.com/paxel/oldbrainfix/issues/398))
- On the phone and the desktop, the quick note's More is a full-width "Open in the editor" button under the Nudge buttons. ([obf-1080](https://github.com/paxel/oldbrainfix/issues/393))
- On the desktop, every Note list starts with one header band of two rows that grows with Text size. ([obf-1064](https://github.com/paxel/oldbrainfix/issues/377))
- On the desktop, the list's search field is as tall as a button, its text a card's size, with the magnifier and ✕ inside. ([obf-1063](https://github.com/paxel/oldbrainfix/issues/376))
- On the desktop, View and New Note are framed buttons with icon and word, like Show Notes by…. ([obf-1067](https://github.com/paxel/oldbrainfix/issues/380))
- On the phone and the desktop, a Note without text is named by its Transcript or Photo Text on cards, Agenda, calendar and notifications. ([obf-1049](https://github.com/paxel/oldbrainfix/issues/360))
- On the desktop, the Area and Note menus leave the menu bar; their actions sit in the ⋮ of the Area, Topic and Note. ([obf-1065](https://github.com/paxel/oldbrainfix/issues/378))
- On the desktop, Quiet today is a button in the Agenda's top row that reads End quiet day while the day is quiet. ([obf-1066](https://github.com/paxel/oldbrainfix/issues/379))
- On the phone, the quick note shows "Transcribing…" after a recording and puts the Transcript into its text when done. ([obf-1082](https://github.com/paxel/oldbrainfix/issues/395))
- On the phone and the desktop, the Amend box shows only after "Add a part", and a Note being made has none. ([obf-1072](https://github.com/paxel/oldbrainfix/issues/385))
- On the phone and the desktop, a Checklist, Shopping List or Tally is one card with Add a Line; a description waits for its button. ([obf-1071](https://github.com/paxel/oldbrainfix/issues/384))
- On the desktop, sync goes on while the AI port is open; the AI's changes stay on this computer until you accept them. ([obf-1076](https://github.com/paxel/oldbrainfix/issues/389))
- On the desktop, the AI port's line offers Review with the number of changes, or Close the port while nothing has changed. ([obf-1075](https://github.com/paxel/oldbrainfix/issues/388))
- On the phone and the desktop, a Backup brings back the Device's settings and offers the speech model chosen before. ([obf-1098](https://github.com/paxel/oldbrainfix/issues/411))
- On the phone and the desktop, the Note's buttons stand in four named groups: This Note, Add, Share and Tidy. ([obf-1073](https://github.com/paxel/oldbrainfix/issues/386))
- On the phone and the desktop, removing a recording or photo but keeping its words puts them into the Note's text, to edit or delete. ([obf-1084](https://github.com/paxel/oldbrainfix/issues/397))
- On the phone and the desktop, the most used tags sit as chips under the search, ending in All tags… for the whole tree. ([obf-1074](https://github.com/paxel/oldbrainfix/issues/387))
- On the phone and the desktop, buttons, chips and header bands follow one design: outlined buttons are tonal, deleting is red, chips are pills. ([obf-1081](https://github.com/paxel/oldbrainfix/issues/394))

### Fixed

- On the phone and the desktop, no search bar cuts off what scrolls under it; each sits on a band with a line under it. ([obf-1088](https://github.com/paxel/oldbrainfix/issues/401))
- On the desktop, New Note stays when a filter is on; on both apps a Note the filter hides is announced with Show. ([obf-1070](https://github.com/paxel/oldbrainfix/issues/383))
- On the desktop, Review the AI Task is never greyed out; with nothing to review it says "No changes yet". ([obf-1077](https://github.com/paxel/oldbrainfix/issues/390))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

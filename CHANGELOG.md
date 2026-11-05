# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.5.0]

### Added

- An iPhone version of Old Brain Fix, for iOS 26 and newer, which goes to testers through TestFlight once the Apple account is connected; it is not in the App Store yet. The user guide and the privacy policy say where the iPhone differs from Android.
- On the desktop, a right-click in a text field opens a menu at the pointer with Cut, Copy, Paste and Select all, Cut and Copy greyed while nothing is selected. With words selected in a formatted text, and in the quick note and Add to Note while their format bar shows, it also offers the format bar's Bold, Italic, Code, Link and Code block for them. A right-click on the selection keeps it selected.
- On the desktop, every text the Note dialog only shows can be selected and copied: another person's text, a Note's in the Trash, Sections, Transcripts, Photo Texts and the History. Dragging the mouse selects, also over formatted lines, and Ctrl+C copies; a right-click opens Copy and Select all. Links, boxes and tags in them still work with a click.
- On the desktop, a selection being dragged scrolls: while the mouse button is held, the wheel scrolls and the selection follows, and dragging past the top or bottom edge scrolls by itself, faster the further out the pointer is. This works in every text field and in the texts the Note dialog only shows.
- In a formatted text, a Markdown task list item such as `- [ ] milk` or `- [x] bread` becomes a box, ☐ or ✅, on the phone and the desktop: as the space after `]` is typed, when it is pasted and when a Markdown file is imported. It can then be ticked like any box. In code, a code block, a Code text or a plain text it stays as written. Copy as text, Share and Save as Markdown write such a box back as `- [ ]` or `- [x]`, a failed one as `- [x]` with its words struck through, so other Markdown apps show a checkbox.
- A Note with a Frog shows it as a Frog badge in the Note editor, on the phone and the desktop: Open, then Eaten today or Eaten yesterday. A tap offers Eat now while it is open, and Remove the Frog, which asks once and moves it to the Trash. The "+" no longer offers a Frog while the Note has an open one.

### Changed

- On the phone and the desktop, a table in a formatted text, a pasted one too, is edited right in the Note editor, cell by cell, instead of as its `|` lines or full screen. A tap or click on a cell edits it in place; Tab, Shift+Tab, Enter and the arrow keys move between the cells and into and out of the table. "+" adds a row or a column, and a row's number or a column's letter opens a menu to insert, move and delete it and to align a column left, centred or right. On the desktop the columns' widths can be dragged; they are not stored. Every change is written back as the table's Markdown lines, and "Show as lines" shows them for editing as text until "Show as table".
- The help Checklist "Quick start" says where and how for every step: each Line names the place on the phone and what to tap, such as "Tap the round + at the bottom right of the Agenda, then Text, type a thought and tap 1 h." Its description, under its name, gives the desktop's way for each step: the menus, keys, drag and right-click. A Quick start made by 0.4.0 keeps its words.

### Fixed

- On the phone, the swipe between the Agenda and the Areas list reacts at once to a quick short flick and no longer stutters: the two pages lie side by side and the Areas list is ready before the swipe starts. Back and the folder go between them as before. Tapping the version under About Old Brain Fix seven times shows how long the last frames took, at the bottom left, for reporting from the phone; seven more taps hide it.
- The desktop draws arrows, ticks and other symbols that showed as empty squares, such as the → in Notes, with the fonts Noto Sans Symbols 2 and Noto Sans Math.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

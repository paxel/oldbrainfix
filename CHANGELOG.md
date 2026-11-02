# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.4.0]

### Changed

- On the phone and the desktop, a place captured within a Named Place shows only the Named Place's name, such as "Home", on the cards, in the Captured badge and its dialog and with each Section, where it also showed the coordinates; a place without a Named Place still shows its coordinates.
- On the phone and the desktop, the Capture Context of a Note and of each Section shows as one line under its text, smaller and in a lighter colour, so the text reads first: its time, place, calendar events, Bluetooth devices and the app it was shared from, joined by " · ", such as "12:40 · Home · Dentist", the Note's on its full card and each Section's on the full card and in the Note editor, where it showed above the text on full cards, with a line of its own for each detail. A Section captured the same day as the part before it shows only its time, and in the Note editor a Section's line opens its Captured details, each with Remove.
- On the phone and the desktop, a Checklist, a Shopping List or a Tally starts with its name, the first line of its text, as a heading, and its description, the rest of the text, small under it and folded to one line until a tap unfolds it; its Lines or its Tally follow right after, then the badges, and the Amends at the end. In the Note editor the name and the description are two fields, saved back as one text; no Note is rewritten for it.
- On the phone and the desktop, the Note editor's kind badge shows on every Note, "Plain Note" too, and changes the kind: a Plain Note becomes a Checklist, a Shopping List or a Tally, a Checklist becomes a Shopping List and a Shopping List a Checklist with their Lines kept, and a list becomes a Plain Note again. The editor's own "Make it a Checklist" and "Make it a Shopping List" buttons are gone.
- On the phone and the desktop, every card in a Note list has a compact view and a detailed view, where a compact card showed the first line and the media and a tap only unfolded it. The compact view shows a Plain Note's first line and the two lines after it, a Checklist's name and "3 of 7 open", a Shopping List's name and "4 open", a Tally's name and each field's total today against its target, and for a Note that runs a Session its next run and "done 4 / skipped 1 this week"; it has no Capture Context and none of the buttons at the bottom, its ⋮ sits top right and offers File into… and Merge into… on the phone. The list's switch shows every card compact or detailed; a tap on a card, on the desktop its view button beside ⋮, switches only that card, and the Device keeps it so until the list's switch changes. A Shopping List's detailed view folds its struck Lines under the open ones until a tap shows them; that fold and a card's unfolded description stay as left on the Device. The Agenda's just-captured card starts detailed.
- On the desktop the AI port reads the name and description of a Note of a kind, and can turn a Checklist into a Shopping List or back as a change that waits for the review.
- On the phone and the desktop, an all-day Routine or Reminder rings from 15:00, where it rang from 09:00 and then Re-nagged all day; before that time it only shows on the Agenda. The time is set for each Device in Settings under Alarms, "All-day items remind from". When the Quiet Hours hold that time, its first ring waits until they end. A calendar file from Export or Share gives an all-day event its alarm at the same time, and the AI port's read_settings lists it.

### Fixed

- On the phone and the desktop, a Line just typed into a Checklist or a Shopping List in the Note editor shows its ⋮ as soon as it holds text, where it showed only after Return and Backspace. An entry of its ⋮ stores the Line first, then acts; an empty new Line has no ⋮.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

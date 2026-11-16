# Changelog

All notable changes to Old Brain Fix are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: [SemVer](https://semver.org/).

## [0.6.1]

### Added

- Capture offers "Add to Note…", which adds the text, recording, photos or files to a chosen Note at once. ([obf-1001](https://github.com/paxel/oldbrainfix/issues/308))
- Capture offers Text, Checklist and Shopping List above the field, and a list keeps each typed line as a Line. ([obf-1003](https://github.com/paxel/oldbrainfix/issues/310))
- A spoken shopping list goes into a Shopping List as Lines, its recording kept, also without the speech model. ([obf-1004](https://github.com/paxel/oldbrainfix/issues/311))
- An Ask answer can also be copied, added to an existing Note and shared, on the desktop saved as Markdown. ([obf-1009](https://github.com/paxel/oldbrainfix/issues/316))
- On the desktop, Ask asks an OpenAI-compatible model not to think unless "Let the model think" is ticked, and the AI settings name models tried. ([obf-1011](https://github.com/paxel/oldbrainfix/issues/318))
- Copying a Checklist or Shopping List puts its Lines on the clipboard as a Markdown task list, which pastes back as boxes. ([obf-1016](https://github.com/paxel/oldbrainfix/issues/323))
- A table opened full screen offers Copy, which pastes as a table into formatted text and as Markdown elsewhere. ([obf-1017](https://github.com/paxel/oldbrainfix/issues/324))
- Text size makes the text of Notes, Lines, the Agenda and Ask's answers larger in steps; on the desktop Ctrl+Plus and Ctrl+Minus change it. ([obf-1019](https://github.com/paxel/oldbrainfix/issues/326))
- On the desktop, a Note's own window has a Reading view, also F9, showing only its content in a calm column. ([obf-1015](https://github.com/paxel/oldbrainfix/issues/322))

### Changed

- The Kind badge comes first and names the Kind in words, such as "Plain Note ▾". ([obf-1003](https://github.com/paxel/oldbrainfix/issues/310))
- Text made a list keeps its first line as a Line unless that line is a heading. ([obf-1002](https://github.com/paxel/oldbrainfix/issues/309))
- A Note's added parts continue its text under a dashed tear line instead of sitting in boxes of their own. ([obf-1000](https://github.com/paxel/oldbrainfix/issues/307))
- Once a Note from Unsorted is sorted, the next Unsorted Note opens, and the Agenda when none is left. ([obf-1006](https://github.com/paxel/oldbrainfix/issues/313))
- On the desktop, asking is experimental and off until "AI (experimental)" is ticked; the AI port works either way. ([obf-1027](https://github.com/paxel/oldbrainfix/issues/334))
- On the desktop, AI ▸ AI settings… opens one page with every AI setting. ([obf-1028](https://github.com/paxel/oldbrainfix/issues/335))
- On the phone, asking is experimental and off until "Ask (experimental)" is switched on in the Settings. ([obf-1026](https://github.com/paxel/oldbrainfix/issues/333))
- Under each Ask answer, "Also read" lists the other Notes the model read, and a small local model now searches and reads before answering. ([obf-1007](https://github.com/paxel/oldbrainfix/issues/314))
- Ask shows its questions as a thread, the oldest first, each with "Edit" to ask it again; on the desktop Up fetches the last. ([obf-1008](https://github.com/paxel/oldbrainfix/issues/315))
- On the desktop, the Note menu says "Move to trash" instead of "Delete", as the phone and the card's right-click do. ([obf-1022](https://github.com/paxel/oldbrainfix/issues/329))
- A card starts with its text: the place sits beside the buttons without a divider, and an Area's or Topic's list leaves it out. ([obf-1023](https://github.com/paxel/oldbrainfix/issues/330))
- On the phone, the search bar of the Areas list, an Area and a Topic scrolls away with the list and returns once scrolled up. ([obf-1029](https://github.com/paxel/oldbrainfix/issues/336))
- Formatted text reads calmer, with more room between lines and paragraphs, lighter headings, and code on a light tint instead of a grey box. ([obf-1020](https://github.com/paxel/oldbrainfix/issues/327))
- About Old Brain Fix has left the Settings: on the phone it is behind the Profile picture, on the desktop under Help. ([obf-1018](https://github.com/paxel/oldbrainfix/issues/325))
- A Shopping List's struck Lines show the one coming back soonest first, each with when it comes back beside it, such as "in 3 days". ([obf-1021](https://github.com/paxel/oldbrainfix/issues/328))

### Fixed

- On the desktop, Cut, Copy and Paste in the right-click and Edit menus reach the clipboard. ([obf-1012](https://github.com/paxel/oldbrainfix/issues/319))
- On the desktop, a Session's Tally opens only its entry form, not its Note as well. ([obf-1013](https://github.com/paxel/oldbrainfix/issues/320))
- On the phone, a tap in the Note editor puts the cursor where the finger is. ([obf-1005](https://github.com/paxel/oldbrainfix/issues/312))
- The Linux .deb, AppImage and Homebrew build start on older systems with glibc 2.35, such as Ubuntu 22.04 or Debian 12. ([obf-1025](https://github.com/paxel/oldbrainfix/issues/332))
- A tapped box ticks and its Line strikes through at once, the save following; a failed save shows it as before with a message. ([obf-1031](https://github.com/paxel/oldbrainfix/issues/338))
- A phone's question no longer goes missing, and Ask on the phone and desktop says what it is doing and why it failed. ([obf-1010](https://github.com/paxel/oldbrainfix/issues/317))
- On the phone, the app stays quick while a recording is transcribed; a Transcript may take about twice as long. ([obf-1030](https://github.com/paxel/oldbrainfix/issues/337))
- A Reminder's notification opens its Note at once, ticking, Capture and lists are quicker, and the phone stays responsive while syncing with the computer. ([obf-1032](https://github.com/paxel/oldbrainfix/issues/339))

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).

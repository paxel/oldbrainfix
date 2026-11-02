# Old Brain Fix

A local-first, privacy-first memory aid for anyone who forgets things. Capturing a thought takes one tap; the app then keeps nagging until every Note is sorted, and turns Notes into Routines and Reminders that fire until they are done. Phone and desktop are equal; no central server.

## Language

### Capturing

**Note**:
Anything the user captured: text, a voice recording, a photo, or something shared in from another app, together with its Capture Context. The diary entry everything else hangs off. Every Note has one Kind.
_Avoid_: entry (that is a Tally's), memo, item, card (a card is only how a Note is drawn; a Profile card is a person's)

**Kind**:
What a Note is, and with it its page: a Plain Note, a Checklist, a Shopping List, a Tally or a Session. A Plain Note is its text; every other kind shows its content first, its text only as its Name and Description above it. The kind changes on the kind badge of the Note editor: a Plain Note becomes a Checklist, a Shopping List, a Tally or a Session, a list becomes the other list, keeping its Lines, or a Plain Note again, as a Session does; a Tally stays a Tally. What belonged to the old kind stays in the log and shows no more. A Frog is no kind but a mark any Note can carry. German: Art.
_Avoid_: type, template, mode

**Plain Note**:
A Note of the Kind that is its text alone, with no Lines and no Tally fields. German: einfache Notiz.
_Avoid_: text note, normal note

**Name**:
The first line of the text of a Note of any Kind but a Plain Note, shown as the heading of its page and card. In the editor it is a one-line field; the text is not rewritten for it. German: Name.
_Avoid_: title, heading, label

**Description**:
The rest of the text of a Note of any Kind but a Plain Note, after its Name: shown small right under the Name, folded to one line until a tap unfolds it. In the editor it is its own field; saved, Name and Description are one text again. German: Beschreibung.
_Avoid_: body, notes, details

**Compact view**:
How a Note's card shows it in a list for a glance, as its Kind defines it: a Plain Note its first line and the two lines after it, a Checklist its Name and how many of its Lines are open ("3 of 7 open"), a Shopping List its Name and how many are open ("4 open"), a Tally its Name and each field's total today against its target ("Steps 7000 / 10000 steps"), in a Shared Area of the user's own entries, a Session its Name, the next run of any of its Routines and this week's runs of all of them done and skipped. It has no Capture Context and none of the buttons at the bottom of the card; its ⋮ sits top right and offers File into… and Merge into…. The list's switch shows every card compact or detailed; a tap on a card (on the desktop its view button) switches that card, and this Device keeps it so until the list's switch changes again. German: Kompaktansicht.
_Avoid_: folded card, short view, summary

**Detailed view**:
How a Note's card shows it whole in a list, as its Kind defines it: a Plain Note its whole text, its Amends, photos and recordings, a Checklist all its Lines with their boxes, tickable, in their groups, a Shopping List its open Lines, tickable, the struck ones folded under them, a Tally "Add entry", its charts and its entries, the older folded under the latest five, a Session its pool, today's picks of each of its Routines, recorded there as on the Agenda, and how often it was done and skipped in each of the last four weeks; each part with its Capture Context as one line under its text, the buttons at the bottom of the card. The default of every list. German: Detailansicht.
_Avoid_: full card, expanded view, long view

**Capture**:
The act of creating a Note through one of the quick entry points (the "+" button, a widget, a tile, a shortcut (which the phone's own key or gesture settings or a button-mapper app can also start; the app reads no key itself), the desktop hotkey). Capturing never asks for sorting or unlocking.
_Avoid_: quick add, record

**Quick note**:
The text capture screen, and on the desktop the quick note dialog of the window (File ▸ New Note, Ctrl+N, "+") and the capture popup: the text, Photo, Voice, Gallery and Paste (on the desktop Record voice, Paste and From file…), the Nudge buttons and "To full editor", which saves the Note Unsorted and opens it in the Note editor, on the desktop the Note dialog. What sorts the Note (File into…, Routine, Reminder, Frog, Session or Tally, Add to a Shopping List, the Importance) and the format bar show only where the Device's settings switch them on; the Nudge buttons can be switched off for a single Save. German: Schnellnotiz.
_Avoid_: quick add

**Capture Context**:
What was true when a Note was captured and helps make sense of it days later: time, Named Place or coordinates, source app, the calendar event running, the connected Bluetooth device. Each of them but the time can be removed on its own; the time stays.
_Avoid_: metadata (in the UI), tags (a Tag is a #word of the text)

**Named Place**:
A spot the user gave a name ("Home", "Work", "Mum's"). A Note captured near it carries that name in its Capture Context.
_Avoid_: location, geofence, favourite

**Transcript**:
Text recognised from a Note's voice recording, kept next to the recording. Only the machine writes it; the user can copy it into the Note's text, replacing or appending, or remove it while the recording stays, and transcribe the recording again. The recording can be removed and the Transcript stays.

**Photo Text**:
Text recognised in a Note's photo or screenshot, kept next to the photo. Only the machine writes it; the user can copy it into the Note's text, replacing or appending, or remove it while the photo stays, and read the text again. The photo can be removed and the Photo Text stays.
_Avoid_: OCR (in the UI), caption

**Section**:
A dated part of a Note, added by Amend or Merge, carrying its own Capture Context. Its text is fixed in place as the Note's is, by the same people; it keeps its date and its place, and shows when it was last modified and who edited it.
_Avoid_: comment, update, revision

**Amend**:
Adding a new Section to a Note. Fixing a Note's text in place is an edit, not an Amend.
_Avoid_: append, reply

**Merge**:
Folding one Note into another: the merged Note becomes a Section of the other and leaves every list.
_Avoid_: combine, join, link

**Importance**:
How much a Note matters: unimportant, normal (default) or important.
_Avoid_: priority, star

**Paste**:
Capturing what the clipboard holds: on the phone "Paste" on the text capture screen and in the fan of "+", which reads the clipboard only when tapped and fills the capture with its text and with its picture as the Note's photo, both into one Note; on the desktop the capture's Paste button and Ctrl+V in the empty capture window. German: Einfügen.
_Avoid_: import, clipboard capture

**Import**:
Making Notes from files another app or a folder holds: on the phone under Settings, on the desktop in the File menu, the files picked with the system's picker. Each Markdown or text file of a folder becomes one Note, formatted from Markdown, with the file's name as its first line unless its text begins with it, captured when the file was last changed; they all land in a new Area named after the source, beside an Area of that name rather than in it, and none of them is Unsorted or Nudged. Other files are skipped and counted. A Google Takeout export of Google Keep, zipped or unpacked, is imported the same way into its own new Area: each note becomes a Note captured when it was made in Keep, its title as its first line, its labels as #tags on a last line, a list as a Checklist with its Lines checked as in Keep, off the Agenda until switched on, its pictures as its photos; notes in Keep's trash are skipped and counted, archived ones imported. German: Import.
_Avoid_: Pack (a Pack brings copies of Old Brain Fix Notes), Paste, bundle (a bundle is a sync transport), upload

### Text

**Formatted text**:
A Note's or Section's text kept as Markdown of the app's own subset: bold, italic, strike-through, two heading levels, bulleted and numbered lists one level in, quotes, links, inline code, code blocks and Tables. A text stays plain until it is first formatted, from the Format bar, by typing Markdown or by a Paste that uses a format, and stays formatted from then on. Transcripts and Photo Texts stay plain. German: Formatierter Text.
_Avoid_: rich text, styled text

**Format bar**:
The row of buttons that formats the selection or the word at the cursor, or the lines the selection touches, and puts in a ☐ or a Table: on the phone on the keyboard in the Note editor and in Add to Note, on the desktop above the text field of the Note editor. German: Formatleiste.
_Avoid_: toolbar, formatting toolbar

**Table**:
Rows and columns in a Formatted text, written as a GitHub Markdown table: a header row, the first column as headings when it is marked so, and cells of one line of formatting each. It is opened full screen to edit its cells and to put in, move and delete rows and columns. German: Tabelle.
_Avoid_: spreadsheet, grid

**Box**:
A ☐ (open), ✅ (done) or ❌ (failed) anywhere in a Note's or Section's text, in the middle of a word and in a Table's cell too, but not in inline code or a code block; it becomes a Box, with an id of its own kept invisibly in the text, when the text is saved, and the glyph it was written with is the state it starts in. A tap or click on one offers the other two states. Each change of its state is kept beside the text, the latest one winning, so the Boxes of a Section can be changed too while its text stays as it is, and in a Shared Area anyone with a role there changes any Box, while adding, moving and deleting Boxes stays an edit of the text. Copies, shares and Sent Items show each Box in its state now. Transcripts and Photo Texts hold no Boxes. German: Statuskästchen.
_Avoid_: status mark (its earlier name), checkbox (a Line of a Checklist has the checkbox)

**Tag**:
A word with "#" in front in a text, such as "#Garten": letters, digits, "_" and "-", at least one of them a letter. It is drawn highlighted, and a tap or click on it opens the search for it. Nothing is stored for a Tag and there is no list of Tags; it is no Topic and no part of the Capture Context. German: #Wort.
_Avoid_: label, keyword, hashtag

### Sorting

**Area**:
The top level the user files Notes under, e.g. Household, Work, Kids, Doctor. Has an optional colour and icon. German: Bereich.
_Avoid_: domain, category, folder

**Topic**:
The second and last level, inside an Area, e.g. Household/Kitchen, Kids/School. German: Thema.
_Avoid_: category, subcategory, tag (a Tag is a #word of the text)

**Browser**:
Where the user finds and opens filed Notes. On the phone the page that "Search and browse" (the folder in the Agenda's top bar) opens: the Areas list with a search over every Note, All Notes, Frogs and Trash in its bar, and from it each Area and Topic. On the desktop the sidebar and the list beside it. German: Browser.
_Avoid_: search screen, library

**Unsorted**:
The state of a Note nobody has processed yet. A Note leaves Unsorted when it gets an Area or Topic, spawns a Routine, Reminder or Frog, is made a Tally, a Checklist or a Shopping List, is merged, or is deleted. It becomes Unsorted again when its Area or Topic is deleted and the user sends its Notes back to Unsorted. German: Unsortiert.
_Avoid_: uncategorized, inbox

**Nudge**:
The reminder that Unsorted Notes are waiting. Its time is chosen at Capture (10 min, 30 min, 1 h), one Nudge covers all Unsorted Notes, and Nudges keep coming until none are left.
_Avoid_: reminder (that is a different thing), nag (that is Re-nag)

**Quiet Hours**:
A daily window in which Nudges and the Changes Nudge wait. Routines and Reminders are not affected, but for the first ring of an all-day Occurrence, which waits until the window ends (#201).

**Starter Set**:
The Areas and Topics offered on first start; the user picks which ones to keep.

### Routines and Reminders

**Routine**:
Something to be done on a repeating schedule (pills, homework, cleaning), always spawned from a Note and linked back to it. A Routine of a Session runs it. German: Routine.
_Avoid_: chore, task, habit, to-do

**Reminder**:
A Routine that happens only once. With "Move a Note to the Trash when its one-time Reminder is done" on, a setting of the person kept on all of their own Devices, Done on a Reminder that is a Plain Note's only Routine or Reminder moves the Note with it to the trash; unchecking does not bring it back, the trash does. German: Erinnerung.
_Avoid_: alarm, appointment, event

**Frog**:
A Note's job with no time, done once, offered on the Agenda to eat one a day. German: Frosch.
_Avoid_: chore, task, to-do

**Occurrence**:
One due moment of a Routine, e.g. Tuesday 08:00. Each Occurrence is checked off on its own.
_Avoid_: instance, slot

**Done**:
An Occurrence or Frog the user checked off. Checking off early or late never moves the schedule.
_Avoid_: completed, ticked (in the UI)

**Re-nag**:
Firing an ignored Occurrence again at a fixed interval until it is Done, snoozed, or the next Occurrence starts.
_Avoid_: repeat, reminder

**Escalation**:
An opt-in step where an Occurrence ignored for several Re-nags turns into a full-screen alarm with sound. The full-screen alarm is phone-only; on the desktop Escalation makes the notification urgent, with the system's alarm sound.

**Missed**:
An Occurrence that was never Done: a Routine's when its next Occurrence starts, a Reminder's at midnight.
_Avoid_: overdue, skipped

**Missed Summary**:
The card on the Agenda listing what was Missed and how often since the user last confirmed it.

**Catch-up Notification**:
The single notification after the device was off or dead, saying how much was Missed while it was down.

**Streak**:
The run of consecutive Occurrences of a Routine that were Done.

**Adherence**:
The share of a Routine's Occurrences that were Done over a period, e.g. "92 % this month".

**Agenda**:
The home screen: today's Occurrences, then two Frogs to eat one of, then the rest of the week dimmed (tomorrow and the five days after), with the Missed Summary and the Unsorted count on top.
_Avoid_: dashboard, today view, home

**Calendar page**:
The phone's page for its calendars: "Read from your calendar", which shows the appointments of the calendars picked on the Agenda and notes the running appointment at Capture, and "Write to your calendar", which writes the Routines and Reminders into a calendar. Phone-only: the desktop has no Calendar page and shows no appointments. German: Kalenderseite.

### Tallies

**Tally**:
A Note that records how much of something was done, such as push-ups, pages read or minutes of video: its text is its Name and Description, and its content its fields, each a number with a unit, an optional target and a rule for a day with several entries (their sum or the largest), and its entries, listed as rows, newest first, and shown as one chart per field by day, week, month or year. A field may instead multiply another of its fields (#200), as Rounds multiply Push-ups: the other field then counts the product in each entry, and the multiplier counts no value of its own. German: Strichliste.
_Avoid_: tracker, log, workout, exercise, habit

**Entry**:
One record of a Tally: a value for some of its fields, a date, one line of text and its author. German: Eintrag.
_Avoid_: Section (a Section is a dated part of a Note's text), record, set

**Session**:
A Note that picks a few Tallies from its pool for each Occurrence of its Routines, as many as its count says for that weekday, by its rule: Random, Rolling (through the pool in its order, continuing where it stopped) or Least selected (the fewest entries first). The Note holds the pool, the count and the rule; each of its Routines, such as Monday and Wednesday at 7:00 and Saturday at 10:00, only says when it runs, and Rolling goes on from one Routine's run to the next. The picks are the same on all of the user's own Devices and for everyone in a Shared Area, and "Another one" swaps one of them. An Occurrence of it is Done once every pick has an entry from it, by anyone, or when checked off. German: Einheit.
_Avoid_: workout, training, playlist, set

### Checklists and Shopping Lists

**Checklist**:
A Note whose Lines are checked off, such as a packing list: its text is its Name and Description above the Lines, every Line has its checkbox, and a Line can have a group of Lines under it, two levels at most. Checking every Line of a group checks its parent, checking the parent checks its whole group, and unchecking one Line of the group unchecks the parent. A Note is at most one of a Tally, a Checklist and a Shopping List; it turns back into a Plain Note with every Line a line of its text, or into a Shopping List of its Lines, the checked ones struck. A Routine or Reminder spawned from a Checklist starts each Occurrence with every Line unchecked, on all of the user's own Devices and for everyone in a Shared Area alike; the Occurrence is Done once every Line is checked for it, by anyone, or when checked off. Today's Agenda shows each Checklist without a Routine or Reminder that has open Lines as its name with every open Line under it, or as one row with the count, such as "Before the trip: 3 of 7 open", or not at all, as the user chose under "Show on the Agenda" for it: a choice of their own, kept on their own Devices as an Override is, so in a Shared Area each person decides for their own Agenda. German: Checkliste.
_Avoid_: to-do list, task list, todo

**Shopping List**:
A Note whose Lines come back a while after they are checked, such as the weekly shopping: its text is its Name and Description above the Lines, and its Lines have no groups. The open Lines keep the order they were typed in, a new Line added at the end. Each Line has its reappear time, Once or one to four weeks, one week for a new Line. Checking a Line strikes it through and moves it to the end, showing the day it comes back; at the start of that day, the same weekday as many weeks on by the Device's wall clock, it is open again, in its place. A Line Once is gone when checked. Adding a name the list already has, in any case and with spaces around it, adds nothing: a struck or gone Line of that name comes back at once, in its place, and an open one stays. It is made from a Note and turns back into a Plain Note as a Checklist does, the struck Lines marked "✓ ", or into a Checklist of its Lines, the struck ones checked. A Routine or Reminder spawned from it is Done only when checked off. Today's Agenda shows each Shopping List with open Lines the way the user chose with "Show on the Agenda" for it: one row with the number of its open Lines, such as "Groceries: 4 open", the default; each open Line on its own row, checkable there; or not at all. A list without open Lines never shows. The choice is the user's own, kept on their own Devices as an Override is, so in a Shared Area each person decides for their own Agenda; a list nobody chose for starts as the Device's earlier setting for all Shopping Lists showed them. A Capture can go into a Shopping List instead of becoming a Note: each of its lines, typed or in a recording's Transcript, becomes a Line of the list, and no Note is kept. German: Einkaufsliste.
_Avoid_: grocery list, perpetual list, to-buy list

**Line**:
One line of a Checklist or a Shopping List: a text, whether it is checked, and its place, at the top or, on a Checklist, in the group of the Line above; on a Shopping List also its reappear time. Each check and uncheck of it is an entry of its own; the Note's text is never edited by it. German: Zeile.
_Avoid_: item, task, to-do, entry (that is a Tally's)

### People and devices

**Device**:
One installation of the app, on a phone or a desktop. Settings belong to a Device and are never synced.

**Pairing**:
Introducing two Devices, or a person, to each other in person or by code, which hands over the keys they need.

**Shared Area**:
An Area that other people sync with. Everything outside it stays private. It is shared in person by Pairing, or from afar with a person in People through an offer file the person accepts or declines; changes then travel by a shared folder or by bundles.

**Changes Nudge**:
The notice that people synced by bundles lack changes, such as "Changes for Bob and Carol". It comes 30 minutes after the first change no bundle for them carried yet, one for everyone waiting, and repeats as the Nudge does until a bundle went to each of them. Opening it sends each person their bundle.
_Avoid_: reminder (that is a different thing), sync reminder

**Manager**:
A person who may do everything in a Shared Area.

**Member**:
A person in a Shared Area who may add and amend Notes, check off Occurrences and Frogs, add and check Lines of any Checklist or Shopping List, and change the state of any Box, but not change Routines, Reminders, Frogs, other people's Notes or the Lines of their Checklists and Shopping Lists.
_Avoid_: guest, viewer

**Sent Item**:
A single Note with its Routines or Reminders, sent as a file to another Old Brain Fix user outside any Shared Area. It is sent with "Send to a person". The receiver previews and accepts it; the sender may later send an update or a cancellation for it.
_Avoid_: invite, share (Share to an app hands a Note to another app), bundle (a bundle is a sync transport)

**Share to an app**:
Handing a copy of a Note to another app, such as a messenger or mail: its text and its Sections, each under its date, with HTML beside the Markdown of a Formatted text, and its photos and recordings as files. On the phone it gives them to the share sheet; on the desktop "Copy as text" and "Save as Markdown…" put the same on the clipboard or into a file. Nothing comes back from the other app and nothing of it syncs. German: Mit einer App teilen.
_Avoid_: Sent Item and "Send to a person" (they send a Note to another Old Brain Fix user), Shared Area, export

**Profile**:
A person as the app shows them: a name and a picture, under an id of their own. Each person has one Profile for all of their Devices. German: Profil.
_Avoid_: account, user, avatar (the picture is only part of it)

**Override**:
A name or picture the user gives another person's Profile for themselves alone, such as "Mum". It stays on the user's own Devices.
_Avoid_: nickname, contact name

**People**:
Every Profile the user's Devices know, listed on the Profile page.
_Avoid_: contacts, friends

**Profile card**:
A person's own Profile, sent as a small file with the picture through any messenger, or shown as a QR without it. Opening it adds the person to People, not verified unless the QR was scanned in person; it never shares an Area. German: Profilkarte.
_Avoid_: business card, contact card, invite

**Block**:
What the user does to a person whose writing they never want to see again: on all of the user's own Devices, everything that person wrote is deleted and nothing more of it is stored, shown or passed on, in any Shared Area and from any Device that relays it, and no Sent Item of theirs is shown; only their check-offs stay, without their name or picture. The person is not told. Blocking a Manager of a Shared Area means leaving that Area. Unblocking fetches their writing again.
_Avoid_: ban, mute, ignore

**Conflict**:
Two versions of the same user-edited field written on different Devices without either having seen the other. The newest shows meanwhile; the user picks which one stays.
_Avoid_: clash, merge conflict

**Pending Changes**:
Changes the user sees as if they were kept, which are kept only once the user accepts them; declining leaves nothing behind. What the user edits in the meantime wins on Accept. Used for an AI Task, an Area offered from afar and a stranger's Sent Item until "Show".
_Avoid_: draft, staging, preview (a preview shows without keeping anything)

**AI Task**:
One job the user hands to an AI model of their choice through the desktop's AI port, such as fixing typos or removing everything about a topic. Everything it changes is a Pending Change, reviewed together when the user says it is done and accepted or rejected as a whole. One at a time; sync waits meanwhile.
_Avoid_: agent, assistant, chat, AI mode

### Privacy

**App Lock**:
An optional lock in front of browsing the app. Capture never needs it.

**Area Lock**:
An optional lock on a single Area that stays closed even while the app is open.

**Masked**:
How an Occurrence from a locked Area appears in alarms and on the Agenda when its Area chooses so: "Private Routine, 08:00", still checkable.

**Backup**:
An encrypted file holding everything on a Device, opened with a passphrase.

**Recovery Sheet**:
A printable page with the recovery key for the Backups of all of a person's Devices, for a drawer or a password manager. One per Profile.

# Old Brain Fix

A local-first, privacy-first memory aid for anyone who forgets things. Capturing a thought takes one tap; the app then keeps nagging until every Note is sorted, and turns Notes into Routines and Reminders that fire until they are done. Phone and desktop are equal; no central server.

## Language

### Capturing

**Note**:
Anything the user captured: text, a voice recording, a photo, or something shared in from another app, together with its Capture Context. The diary entry everything else hangs off.
_Avoid_: entry (that is a Tally's), memo, item, card (a card is only how a Note is drawn; a Profile card is a person's)

**Capture**:
The act of creating a Note through one of the quick entry points (the "+" button, a widget, a tile, a shortcut, a hardware button, the desktop hotkey). Capturing never asks for sorting or unlocking.
_Avoid_: quick add, record

**Capture Context**:
What was true when a Note was captured and helps make sense of it days later: time, Named Place or coordinates, source app, the calendar event running, the connected Bluetooth device.
_Avoid_: metadata (in the UI), tags

**Named Place**:
A spot the user gave a name ("Home", "Work", "Mum's"). A Note captured near it carries that name in its Capture Context.
_Avoid_: location, geofence, favourite

**Transcript**:
Text recognised from a Note's voice recording, kept next to the recording. Only the machine writes it; the user can copy it into the Note's text, replacing or appending, or remove it while the recording stays, and transcribe the recording again. The recording can be removed and the Transcript stays.

**Photo Text**:
Text recognised in a Note's photo or screenshot, kept next to the photo. Only the machine writes it; the user can copy it into the Note's text, replacing or appending, or remove it while the photo stays, and read the text again. The photo can be removed and the Photo Text stays.
_Avoid_: OCR (in the UI), caption

**Section**:
A dated part of a Note, added by Amend or Merge, carrying its own Capture Context.
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

### Sorting

**Area**:
The top level the user files Notes under, e.g. Household, Work, Kids, Doctor. Has an optional colour and icon. German: Bereich.
_Avoid_: domain, category, folder

**Topic**:
The second and last level, inside an Area, e.g. Household/Kitchen, Kids/School. German: Thema.
_Avoid_: category, subcategory, tag

**Unsorted**:
The state of a Note nobody has processed yet. A Note leaves Unsorted when it gets an Area or Topic, spawns a Routine, Reminder or Frog, is made a Tally, is merged, or is deleted. It becomes Unsorted again when its Area or Topic is deleted and the user sends its Notes back to Unsorted. German: Unsortiert.
_Avoid_: uncategorized, inbox

**Nudge**:
The reminder that Unsorted Notes are waiting. Its time is chosen at Capture (10 min, 30 min, 1 h), one Nudge covers all Unsorted Notes, and Nudges keep coming until none are left.
_Avoid_: reminder (that is a different thing), nag (that is Re-nag)

**Quiet Hours**:
A daily window in which Nudges and the Changes Nudge wait. Routines and Reminders are not affected.

**Starter Set**:
The Areas and Topics offered on first start; the user picks which ones to keep.

### Routines and Reminders

**Routine**:
Something to be done on a repeating schedule (pills, homework, cleaning), always spawned from a Note and linked back to it. German: Routine.
_Avoid_: chore, task, habit, to-do

**Reminder**:
A Routine that happens only once. German: Erinnerung.
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
An opt-in step where an Occurrence ignored for several Re-nags turns into a full-screen alarm with sound.

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

### Tallies

**Tally**:
A Note that records how much of something was done, such as push-ups, pages read or minutes of video: its fields, each a number with a unit, an optional target and a rule for a day with several entries (their sum or the largest), and its entries, shown as a table and as one chart per field by day, week, month or year. German: Strichliste.
_Avoid_: tracker, log, workout, exercise, habit

**Entry**:
One record of a Tally: a value for some of its fields, a date, one line of text and its author. German: Eintrag.
_Avoid_: Section (a Section is a dated part of a Note's text), record, set

**Session**:
A Routine that picks a few Tallies from its pool for each Occurrence, as many as its count says for that weekday, by its rule: Random, Rolling (through the pool in its order, continuing where it stopped) or Least selected (the fewest entries first). The picks are the same on all of the user's own Devices and for everyone in a Shared Area, and "Another one" swaps one of them. An Occurrence of it is Done once every pick has an entry from it, by anyone, or when checked off. German: Einheit.
_Avoid_: workout, training, playlist, set

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
A person in a Shared Area who may add and amend Notes and check off Occurrences and Frogs, but not change Routines, Reminders, Frogs or other people's Notes.
_Avoid_: guest, viewer

**Sent Item**:
A single Note with its Routines or Reminders, sent as a file to another Old Brain Fix user outside any Shared Area. The receiver previews and accepts it; the sender may later send an update or a cancellation for it.
_Avoid_: invite, share, bundle (a bundle is a sync transport)

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
A person's own Profile, without the picture, sent as a link through any messenger or shown as a QR. Opening it adds the person to People, not verified unless the QR was scanned in person; it never shares an Area. German: Profilkarte.
_Avoid_: business card, contact card, invite

**Block**:
What the user does to a person whose writing they never want to see again: on all of the user's own Devices, everything that person wrote is deleted and nothing more of it is stored, shown or passed on, in any Shared Area and from any Device that relays it, and no Sent Item of theirs is shown; only their check-offs stay, without their name or picture. The person is not told. Blocking a Manager of a Shared Area means leaving that Area. Unblocking fetches their writing again.
_Avoid_: ban, mute, ignore

**Conflict**:
Two versions of the same user-edited field written on different Devices without either having seen the other. The newest shows meanwhile; the user picks which one stays.
_Avoid_: clash, merge conflict

**Pending Changes**:
Changes the user sees as if they were kept, which are kept only once the user accepts them; declining leaves nothing behind. What the user edits in the meantime wins on Accept. Used for an AI Task, an Area offered from afar, a stranger's Sent Item until "Show", and a bundle with a lot in it.
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

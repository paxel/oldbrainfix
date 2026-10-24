# Old Brain Fix

A local-first, privacy-first memory aid for anyone who forgets things. Capturing a thought takes one tap; the app then keeps nagging until every Note is sorted, and turns Notes into Routines and Reminders that fire until they are done. Phone and desktop are equal; no central server.

## Language

### Capturing

**Note**:
Anything the user captured: text, a voice recording, a photo, or something shared in from another app, together with its Capture Context. The diary entry everything else hangs off.
_Avoid_: entry, memo, item, card (a card is only how a Note is drawn)

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
Text recognised from a Note's voice recording, kept next to the recording, never replacing it.

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
The state of a Note nobody has processed yet. A Note leaves Unsorted when it gets an Area or Topic, spawns a Routine or Reminder, is merged, or is deleted. It becomes Unsorted again when its Area or Topic is deleted and the user sends its Notes back to Unsorted. German: Unsortiert.
_Avoid_: uncategorized, inbox

**Nudge**:
The reminder that Unsorted Notes are waiting. Its time is chosen at Capture (10 min, 30 min, 1 h), one Nudge covers all Unsorted Notes, and Nudges keep coming until none are left.
_Avoid_: reminder (that is a different thing), nag (that is Re-nag)

**Quiet Hours**:
A daily window in which Nudges wait. Routines and Reminders are not affected.

**Starter Set**:
The Areas and Topics offered on first start; the user picks which ones to keep.

### Routines and Reminders

**Routine**:
Something to be done on a repeating schedule (pills, homework, cleaning), always spawned from a Note and linked back to it. German: Routine.
_Avoid_: chore, task, habit, to-do

**Reminder**:
A Routine that happens only once. German: Erinnerung.
_Avoid_: alarm, appointment, event

**Occurrence**:
One due moment of a Routine, e.g. Tuesday 08:00. Each Occurrence is checked off on its own.
_Avoid_: instance, slot

**Done**:
An Occurrence the user checked off. Checking off early or late never moves the schedule.
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
The home screen: today's Occurrences, then a dimmed tomorrow, with the Missed Summary and the Unsorted count on top.
_Avoid_: dashboard, today view, home

### People and devices

**Device**:
One installation of the app, on a phone or a desktop. Settings belong to a Device and are never synced.

**Pairing**:
Introducing two Devices, or a person, to each other in person or by code, which hands over the keys they need.

**Shared Area**:
An Area that other people sync with. Everything outside it stays private.

**Manager**:
A person who may do everything in a Shared Area.

**Member**:
A person in a Shared Area who may add and amend Notes and check off Occurrences, but not change Routines, Reminders or other people's Notes.
_Avoid_: guest, viewer

**Sent Item**:
A single Note with its Routines or Reminders, sent as a file to another Old Brain Fix user outside any Shared Area. The receiver previews and accepts it; the sender may later send an update or a cancellation for it.
_Avoid_: invite, share, bundle (a bundle is a sync transport)

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
A printable page with the recovery key for Backups, for a drawer or a password manager.

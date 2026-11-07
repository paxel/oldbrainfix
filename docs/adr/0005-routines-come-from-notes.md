# Every Routine and Reminder comes from a Note; a Reminder is a one-time Routine

Routines and Reminders are never standalone: each one is spawned from a Note and links back to it, and creating one directly creates its Note behind the scenes. A Note can spawn several. This keeps the answer to "why did I set this?" one tap away, and deleting a Routine leaves the diary intact. Reminders are not a separate concept: a Reminder is a Routine whose schedule happens once, so there is one scheduler, one check-off and one Missed tracking.

## Consequences

Deleting a Note that spawned Routines trashes them with it, after one question naming them, and restoring the Note brings them back (amended by #262; see below). A Routine left without its Note shows it as trashed or deleted.

## Frogs (ADR 0027)

Amended by ADR 0027 (#101): a Note also spawns Frogs, jobs with no time done once, and spawning one takes it out of Unsorted as a Routine does. A Note has one open Frog at a time.

## Tallies (ADR 0028)

Amended by ADR 0028 (#103): a Note made a Tally, by its first field, leaves Unsorted as a Note that spawned something does. A Tally spawns nothing: its fields lie on the Note.

## Sessions (ADR 0029)

Amended by ADR 0029 (#104): a Session is a kind of Routine, spawned from a Note like any, that picks Tallies from a pool for each Occurrence.

## A Note is its kind (ADR 0038)

Amended by ADR 0038 (#188): a Note has one kind, Plain Note, Checklist, Shopping List or Tally, and for every kind but the Plain Note the kind's content is the page: the text shows as its name, heading it, and its description, folded under it. Nothing is rewritten for it.

## Deleting a Note takes its Routines along (#262)

Amended by #262: every deletion of a Note, from the editor, the card's ⋮, the desktop's dialog, an Area or Topic deleted with its Notes into the trash, and the AI port, moves its Routines and Reminders into the trash with it; restoring the Note brings them back. "Delete only the Note" is gone: a Routine without its Note has no use, so there is nothing to keep it for. Routines left behind before, their Note in the trash or deleted, stay as they are; All Routines and Reminders marks them "The Note is in the trash" or "The Note was deleted", and nothing deletes them on the person's behalf. Nothing in the format changes: a Note trashed by an older version without its Routines still reads as before.

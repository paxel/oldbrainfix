# Every Routine and Reminder comes from a Note; a Reminder is a one-time Routine

Routines and Reminders are never standalone: each one is spawned from a Note and links back to it, and creating one directly creates its Note behind the scenes. A Note can spawn several. This keeps the answer to "why did I set this?" one tap away, and deleting a Routine leaves the diary intact. Reminders are not a separate concept: a Reminder is a Routine whose schedule happens once, so there is one scheduler, one check-off and one Missed tracking.

## Consequences

Deleting a Note that spawned Routines asks whether to trash them too; if not, they stay and their back-link shows the Note as trashed or deleted.

## Frogs (ADR 0027)

Amended by ADR 0027 (#101): a Note also spawns Frogs, jobs with no time done once, and spawning one takes it out of Unsorted as a Routine does. A Note has one open Frog at a time.

## Tallies (ADR 0028)

Amended by ADR 0028 (#103): a Note made a Tally, by its first field, leaves Unsorted as a Note that spawned something does. A Tally spawns nothing: its fields lie on the Note.

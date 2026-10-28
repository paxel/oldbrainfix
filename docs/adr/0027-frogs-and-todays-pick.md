# Frogs: a third thing a Note spawns, and today's pick between one's own Devices

A Frog (#101) is a Note's job with no time, done once, after the "Eat the frog" method: the Agenda offers two a day, and eating one is enough. It is the third thing a Note spawns, next to Routines and Reminders (ADR 0005). Both cores read and write it the same way (`frogs.dart`, `frogs.rs`).

## The Frog

A Frog is the entity `frog:<id>` with `$type` = `frog`, `note` (the Note it was spawned from, ADR 0005), `done` (when it was checked off, an instant; null when it was unchecked again) and `trashed` (when it was moved to the trash; null when restored). It has no schedule, so it never rings, is never Missed and never reaches the alarm plan: something that has to ring is a Reminder. Spawning one writes the Note's `spawned` as a Routine does, so the Note leaves Unsorted and stays out of it. A Note has one open Frog at a time; spawning another while one is open returns that one and writes nothing. A todo list Note is one Frog as a whole, never one per line.

A Frog shows under its Note, through a Merge to the Note it went into, and leaves every list while it or its Note is in the trash; a restored Note brings it back. The open Frogs are listed oldest first, by the date of their first `$type`; those Done most recently first.

## Shared like its Note

A Frog lies in the Area of its Note: it is sealed under that Area's key, goes with the Note when the Note is filed into a Shared Area and with an offer from afar, and is judged like a Routine (ADR 0013). A Manager spawns, trashes and restores Frogs; anyone with a role in the Area writes `done`, and nothing else of the Frog, so every Member can check it off and uncheck it. Done is shared: a Frog one person eats is eaten for everyone. A blocked person's check-off of a Frog is kept like their check-offs of Occurrences (ADR 0021), in its exact shape: `done` on `frog:<id>`, null or an instant.

## Today's pick

The Agenda's "Eat a frog" section offers two open Frogs a day. Which two is kept in the log so the user's own Devices show the same: the entity `frogs:pick`, outside every Area, field `pick`, value the day and the Frogs' ids joined by commas (`2026-10-28,frog:a,frog:b`). Lying in no Area, it is sealed under the personal key and goes only to the user's own Devices, never to another person's, a Shared Area's segments or a Sent Item. Only picks one's own Devices wrote count (this one, one paired as its own, or one whose key is bound to the own Profile), as for an Override (ADR 0017): a pick another person's modified app slips into the log changes nothing. Each person in a Shared Area thus keeps their own rotation over the same Frogs.

The rotation orders the open Frogs by the last pick that offered them, those never offered first, then oldest first. Every pick ever written counts, in latest-wins order, so a Frog swapped out by "Another one" counts as offered. Today's pick is written when the Agenda shows it (`pickToday`): the first of the day takes the first two of the rotation; a Frog of it that left the lists, or was Done on another day, is replaced in its place by the next in the rotation, and a pick of one is topped up when another Frog opens. While a Frog of today's pick was Done today (on the wall clock of the Device reading it), the pick stays as it is and the section shows only that one, checked; unchecked, the others show again. "Another one" writes a new pick with the Frog swapped for the next in the rotation not already offered. The section shows while a Frog is open, or while one eaten today is shown. Two own Devices that pick apart before they meet write the same pick from the same log; after a sync, the newest pick wins.

The day is the store's clock on the wall clock the app passes in, never a system clock.

## Considered Options

- A Frog as a Routine without a schedule (rejected: every Routine has one; the alarm plan, Missed and the History would all need to leave it out).
- Today's pick as a setting of each Device (rejected: phone and desktop would offer different Frogs).
- Today's pick computed from the Frogs alone, written nowhere (rejected: "offered the longest ago" needs a record of what was offered, and "Another one" a record of the swap).
- Today's pick in the Shared Area of its Frogs (rejected: the people of an Area would share one rotation, and a pick spans Areas).
- The day and the two ids in fields of their own (rejected: two Devices picking apart could leave one Device's day with the other's Frogs).
- Members spawning Frogs (rejected for now: a Frog is a job for the Area, as a Routine is; Members check them off).

## Consequences

- Frogs do not travel in Sent Items yet; a Note sent with its Routines leaves its Frog behind.
- An AI Task cannot spawn or change Frogs: the AI port refuses fields of `frog:` entities (ADR 0026).
- A Frog in the trash is never purged; it stays out of every list.

## Changes to earlier decisions

- ADR 0005: a Note spawns Frogs as well as Routines and Reminders; spawning one takes it out of Unsorted.
- ADR 0013: a Frog is judged by its Note like a Routine; a Member writes only its `done`.
- ADR 0021: a blocked person's check-off of a Frog is kept.

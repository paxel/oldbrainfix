# Sessions: a Routine that picks Tallies, its picks on the Routine

A Session (#104) takes the planning off the user: from a pool of Tallies (ADR 0028) it picks a few for each Occurrence and offers them on the Agenda, such as "Sport: 3 of push-ups, sit-ups, half bridge, wall sit". It is a kind of Routine (ADR 0005, 0006): spawned from a Note, with the full schedule, alarms, Occurrences, Done, Missed, Streak and Adherence. Both cores read and write it the same way (`sessions.dart`, `sessions.rs`).

## The Session lies on its Routine

A Session is a Routine whose entity has a pool: `session:pool`, the Notes of its Tallies joined by commas, in the order Rolling goes through them. `session:count` is how many Tallies an Occurrence picks, 1 to 99; `session:count:<weekday>` (1 for Monday to 7 for Sunday) overrides it for that weekday, null when it has none; `session:rule` is `random`, `rolling` or `least`. Each is read only in the shape the apps write it: a count as one or two digits without a leading zero, the weekdays 1 to 7 by their exact keys, an unknown rule as Rolling, an unreadable count as 1. These are fields of the Routine, so a Manager changes them as any of its fields (ADR 0013); a Member's writes to them count for nothing.

The pool takes Tallies in the lists. While the Session lies in a Shared Area it takes only Tallies of that Area, which everyone in it holds; elsewhere any Tally of the user's. The Tallies a Session picks from are those of its pool that are Tallies in the lists now, a merged one as the Note it went into, each once, and in a Shared Area only those of that Area. Filing a Session into a Shared Area thus leaves the Tallies of other Areas out of its picks without changing its pool.

## The picks lie on the Routine too

The pick of an Occurrence is the field `pick@<occurrence key>` on the Routine, keyed by the wall-clock Occurrence as `done@` is, its value the Notes of the picked Tallies joined by commas. Lying on the Routine, the pick is sealed under the Session's Area key, goes wherever the Routine goes (filing, offers from afar, Backups) and is the same for every Device that holds the Routine: the user's own Devices and everyone in a Shared Area. Anyone with a role in the Area writes picks, as anyone checks off; in a Shared Area the roles decide (ADR 0013), and anywhere else only picks the user's own Devices wrote count, as for today's Frog pick (ADR 0027), so a pick another person's modified app slips into the log changes nothing.

A pick is written when the Agenda shows the Occurrence's day (`pickToday`), for that day's Occurrences only; called again, it writes nothing. A pick held stays as it is, but a Tally of it that left the pool without an entry for the Occurrence is replaced in its place, and the pick is brought to the day's count, trimmed from the end but never by a Tally with an entry. Without a pick held, the first of the rule's order are taken:

- **Random** orders the pool by the SHA-256 of the Session's id, the Occurrence's key and the Tally's Note, each joined by a line break, in hex. No random state is kept: every Device that holds the same pool draws the same.
- **Rolling** orders by when each Tally was picked last, over the picks of the Occurrences before this one in schedule order and the place in each pick; those never picked come first, in pool order. Only the current pick of an Occurrence counts, so a day nobody looked advances nothing, continuing where it stopped.
- **Least selected** orders by the number of entries of each Tally not removed, everyone's, then as Rolling.

"Another one" writes a new pick with the swapped Tally replaced in its place by the first of the rule's order this Occurrence has not offered yet, every value its pick field ever held counting as offered; when all were offered, the first not picked. As the swapped Tally leaves the current pick, it was never picked for Rolling and has no entry for Least selected: it keeps its place in the rule and comes first again. Two Devices that pick apart before they meet, such as from different entries, go by latest wins on the one field after a sync, the same on every Device.

## Done from the entries of the picks

An entry recorded for a pick (`record`) is a Tally entry as any (ADR 0028), lying in its Tally's Area, plus the field `entry@<occurrence key>@<entry id>` on the Routine, its value the Tally's Note. It is written in the same batch as the entry, by anyone with a role, and judged and trusted as a pick is. An Occurrence is Done once every Tally of its current pick has such an entry not removed: `Routine.done` holds it then, with the moment the last of them got its first entry, so the Agenda, the alarm plan, Missed, the Streak, Adherence and the History take it as any check-off. A check-off (`done@`) makes it Done as for any Routine and keeps its own moment. Removing an entry undoes a Done that came from it; unchecking does not, while every pick has its entry. An entry made on the Tally itself is no pick's.

The marks of a pick lie on the Routine rather than on the entry, so a private Session's Occurrences never show in the Area of a Tally it picks.

## Considered Options

- Picks computed from the log on every read, written nowhere (rejected: Rolling and Least selected change with every entry and pick, so a pick would change under the user's hands, and "Another one" needs a record).
- Picks outside every Area, as today's Frog pick (rejected: everyone in a Shared Area must get the same picks).
- A random number drawn and kept per Occurrence (rejected: a hash needs nothing kept and every Device computes the same).
- Done written as a check-off when the last pick gets an entry (rejected: two people recording the last two picks offline would each see one missing, and nothing would ever write it).
- Entries counting by their date in the Occurrence's day (rejected: it needs the wall clock of the Device reading, and an Occurrence twice a day would split days).
- A mark on the entry naming the Occurrence (rejected: a private Session's mark would lie in the Shared Area of its Tally).
- Picks for the days after today written in advance (rejected: Least selected would decide before the entries of today).

## Consequences

- Sessions travel in Sent Items as plain Routines: their schedule goes, the pool and picks stay behind.
- The AI port writes none of the Session's fields (ADR 0026): `session:`, `pick@` and `entry@` are not among a Routine's fields it takes.
- A blocked person's picks and marks are not check-offs: they are refused and deleted like the rest of their writing (ADR 0021). Their entries go too, so a Done that came from them is undone.
- Tomorrow and the days after show no picks yet.

## Changes to earlier decisions

- ADR 0005: a Session is a kind of Routine, spawned from a Note.
- ADR 0013: a Member writes a Session's `pick@` and `entry@` fields, as `done@`; the rest of it is the Manager's.
- ADR 0021: a blocked person's picks and marks are refused.

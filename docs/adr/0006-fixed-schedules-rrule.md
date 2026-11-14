# Fixed schedules expressed as RRULE, unlike cat(a)log's chores

Routine schedules are iCalendar RRULEs, edited through a simple editor with an advanced mode, and they include several times per day (medication). Schedules are fixed: checking off early or late never moves the next Occurrence. This deliberately differs from cat(a)log, whose "every N days" chores count from the day they were last done and allow only one time per day. RRULE matches the calendar export one to one, and fixed schedules make Missed and Adherence well defined.

## The calendar file

Export (every Routine and Reminder) and Share (one Routine as its whole series with its end, or one Reminder) write the same iCalendar file, and the Dart and the Rust core write it byte for byte the same (fixture `ics.json`). Times are floating, without a TZID: an 08:00 Routine is 08:00 wherever the calendar is read, as the app's alarm is, and no VTIMEZONE is needed. Each time of day is one event with the Routine's RRULE; its UID is the Routine's id and that time, so importing a newer file replaces events instead of doubling them. COUNT over several times becomes the last Occurrence as UNTIL. DTSTAMP is the store's clock when the file is made, never the system time, so the same Routines at the same moment give the same file.

## Skip and Not today (#269)

An Occurrence the user won't do today is answered without changing the schedule. Skip marks it skipped: it is neither Done nor Missed, its alarm and Re-nag stop, it never shows in the Missed Summary, and it leaves the Adherence, which reports the skips beside it ("92 % this month · 3 skipped"); a skip neither breaks a Streak nor counts in it. Tomorrow skips it the same way and adds a one-time Occurrence of the same Routine the day after at the same time, an all-day one all day. The RRULE never changes.

Both write one field per Occurrence on the Routine, `skip@<Occurrence key>`, as Done writes `done@<key>`: its value is the moment it was skipped, as Dart's `toIso8601String` writes it, followed by a space and `tomorrow` for Tomorrow; null takes it back (unskip), and the Occurrence Tomorrow added goes with it. A Skip and a Tomorrow of the same Occurrence on two Devices go by latest wins. The added Occurrence is never written: it follows from the key, the next day at the same time, and only while the key is an Occurrence of the schedule or one added before it, so Tomorrow can move a moved Occurrence on again, a new schedule that no longer has the skipped Occurrence drops what was added for it, and no writer can plant an Occurrence anywhere else. When the schedule has that Occurrence already, nothing is added. An added Occurrence is one of the Routine's like any other: it rings, Re-nags until the Occurrence after it starts, is checked off, is Missed, and counts as the next Occurrence of the one before it. Done wins over a skip of the same Occurrence; an Occurrence already Done is not skipped. A value whose moment is not one the calendar has, in that exact shape, skips nothing; a word after the moment other than `tomorrow` is a later version's and reads as a plain skip. The field is no schedule field, so a skip never bounds what was asked for (`scheduledAt`).

Who may skip goes as who may check off: a Member's skip counts (ADR 0013), a blocked person's is kept in its exact shape as a check-off is (ADR 0021), and the AI port reads skips but never drafts one (ADR 0026). The calendar file stays the schedule: skips and added Occurrences are not exported.

## Considered Options

- cat(a)log's repeat set extended by several times per day (rejected: no "first Monday", and a lossy calendar export).
- "Every N days" counted from the last check-off (rejected: Adherence becomes meaningless, and medication must not drift).
- Tomorrow as a change of the RRULE or an exception date in it (rejected: the schedule would move, the calendar file with it, and a Member could not do it).
- Tomorrow writing the Occurrence it adds (rejected: anyone who may skip could place an alarm at any moment; derived from the key it can only be the next day at the same time).
- A skip counting as Done or as Missed in the Adherence (rejected: a skip is a decision, not a lapse; counting it beside the share keeps the user honest without spoiling it).

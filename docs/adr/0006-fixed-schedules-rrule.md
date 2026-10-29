# Fixed schedules expressed as RRULE, unlike Catlog's chores

Routine schedules are iCalendar RRULEs, edited through a simple editor with an advanced mode, and they include several times per day (medication). Schedules are fixed: checking off early or late never moves the next Occurrence. This deliberately differs from Catlog, whose "every N days" chores count from the day they were last done and allow only one time per day. RRULE matches the calendar export one to one, and fixed schedules make Missed and Adherence well defined.

## The calendar file

Export (every Routine and Reminder) and Share (one Routine as its whole series with its end, or one Reminder) write the same iCalendar file, and the Dart and the Rust core write it byte for byte the same (fixture `ics.json`). Times are floating, without a TZID: an 08:00 Routine is 08:00 wherever the calendar is read, as the app's alarm is, and no VTIMEZONE is needed. Each time of day is one event with the Routine's RRULE; its UID is the Routine's id and that time, so importing a newer file replaces events instead of doubling them. COUNT over several times becomes the last Occurrence as UNTIL. DTSTAMP is the store's clock when the file is made, never the system time, so the same Routines at the same moment give the same file.

## Considered Options

- Catlog's repeat set extended by several times per day (rejected: no "first Monday", and a lossy calendar export).
- "Every N days" counted from the last check-off (rejected: Adherence becomes meaningless, and medication must not drift).

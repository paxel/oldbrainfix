# Fixed schedules expressed as RRULE, unlike Catlog's chores

Routine schedules are iCalendar RRULEs, edited through a simple editor with an advanced mode, and they include several times per day (medication). Schedules are fixed: checking off early or late never moves the next Occurrence. This deliberately differs from Catlog, whose "every N days" chores count from the day they were last done and allow only one time per day. RRULE matches the calendar export one to one, and fixed schedules make Missed and Adherence well defined.

## Considered Options

- Catlog's repeat set extended by several times per day (rejected: no "first Monday", and a lossy calendar export).
- "Every N days" counted from the last check-off (rejected: Adherence becomes meaningless, and medication must not drift).

# A quiet day is a setting of the person, and the cores plan nothing on it

On a sick day the user wants to see what is due but not be pushed to it: the pills still shown, the sport ignored (obf-1044, #354). The user chose (C2a to C2e) a **Quiet day**: "Quiet today" in the Agenda's menu; while it holds, the Agenda shows "Quiet day until midnight" with End. Today's Occurrences stay on the Agenda and can be checked off one by one, but nothing notifies that day: no ring, Re-nag, Heads-up (ADR 0052), Escalation, Nudge or Changes Nudge. What is left undone at midnight counts as skipped, never as Missed. It is the person's, the same on all of their own Devices, and ends at midnight on each. Both cores read, plan and count it the same way, checked by the cross-core fixture `desktop/fixtures/quiet-day/`.

## Kept in the log as a setting of the person, one field per Day

A quiet day lies on the entity `settings:person` (ADR 0040), so it goes to the user's own Devices only and only what they wrote counts, the newest entry of each field winning. Each Day has a field of its own, `quietDay@<day>`, the Day as the wall clock of the Device that made it quiet showed it (`2026-01-06`). Its value is `on` while the day lasts to midnight, and the moment End was tapped once it ended, an instant in UTC as a Skip's is written. A key that is no Day as written, or any other value, reads as not quiet, so nothing a later version or a modified app writes keeps anything quiet. One field per Day, rather than one field for "today", keeps every earlier quiet day: the next one cannot overwrite it and turn its leftovers back into Missed. End writes the moment rather than null, since what passed before End stays quiet while what comes after it rings. "Quiet today" again after End writes `on` again: the whole day is quiet once more. It carries no version.

The quiet stretch of a Day is from its start to midnight after it, on the wall clock of the Device reading it, or to End when that came first. Each Device therefore ends it at its own midnight without writing anything.

## The cores plan nothing on it

The quiet days are a parameter of the pure plans (`alarmPlan`, `headsUpPlan`, `nudgePlan`, `changesNudgePlan`, `missedSummary`, `openOccurrences`, `routineHistory` and their Rust twins), empty by default, and the store's seams pass the person's:

- An Occurrence whose start lies in a quiet stretch has no alarm, also not by a snooze, and no Heads-up; no Heads-up fires in a quiet stretch, whatever it warns of.
- An Occurrence that began before the stretch the plan's now lies in, such as last Monday's still Re-nagging, waits it out: its alarm's first firing moves to the first Re-nag at or after the stretch's end, with its wall-clock moment, so its Escalation counts from there as after a snooze.
- The Nudge and the Changes Nudge, while now lies in a stretch, take no own time and no snooze end before the stretch's end; Quiet Hours then defer them as ever.
- The Occurrences starting in a quiet stretch are among those whose notifications may close (`recentlyDone`).

The platforms (Android's `Scheduler.kt`, the iPhone's `AlarmsChannel.swift`, the desktop's `alerts.rs`) follow the plan as they always did: they arm exactly what they are handed and close what is settled, so they need no code of their own. A Device that learns of a quiet day by sync plans again after the entries arrive, as after any change, and what it had armed for the day is gone. Nothing needs to plan again at midnight: the plan made during the day already holds tomorrow's alarms, and the waiting ones start at midnight.

End brings back what is still to come: an Occurrence that starts after End rings, one that started before it stays quiet, and what began before the day Re-nags again from End.

## Left undone, it is skipped

An Occurrence that starts in a quiet stretch and is not Done is never Missed: the Missed Summary leaves it out, and the Note editor's Due line offers it only while it is due. From the midnight after its day the history counts it as skipped, with that midnight as when it was skipped, so the Adherence counts it beside the skips and the Streak passes over it. The skip is read, not written: a `skip@` field is the Routine's and would go to everyone in a Shared Area, while the quiet day is one person's. On the Agenda the day's rows stay as they are, open and checkable, not struck as skipped. Done on a quiet day stays Done.

## The AI port

`read_settings` reports `quietDayUntil`: when today's quiet day ends, midnight or End, or null. No tool writes it, as no tool writes a setting (ADR 0026); making a day quiet is the user's own decision about being reminded.

## Considered Options

- Writing a Skip for each Occurrence left undone at midnight (rejected: it needs a Device awake at midnight, writes as many times as there are Devices, and goes to everyone in a Shared Area).
- One field holding today's quiet day (rejected: the next quiet day would overwrite it, and its leftovers would become Missed again).
- End writing null (rejected: what passed before End could no longer be told from what is still due).
- Platform code that silences the day (rejected: three platforms would each need it; the plans already decide everything that rings).
- Dropping what began before the day for good (rejected: a weekly Routine ignored before a quiet day would never Re-nag again).

## Consequences

- Two of one's own Devices in different time zones read the same Day by their own clocks, so the stretch starts and ends at each one's own midnight.
- A Member's quiet day keeps their own Devices quiet; the other people of a Shared Area are rung as ever.
- An Occurrence that starts on a quiet day after End and stays undone is Missed, as any other.

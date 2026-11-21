# Heads-ups are the person's own, kept on their own Devices, and planned beside the alarms

A Routine or Reminder can warn ahead of its Occurrence, such as "Dentist in 20 min" at 09:40 for 10:00 (obf-1046, #356). The user chose (Q8, Q9, Q10, Q19 revised, Q20, Q25) a **Heads-up**: one notification before the Occurrence, with no Re-nag and nothing to check off, while the Occurrence still rings and Re-nags as before; any number per schedule, each an offset, on Reminders and Routines alike; in Quiet Hours without sound; in a Shared Area each person's own; for an all-day Occurrence whole days only. Both cores keep and plan them the same way, checked by the cross-core fixture `desktop/fixtures/routines/alarms.json`.

## Kept as an Override is

The Heads-ups of a Routine are the entity `headsups:<routine id>` in the log, outside every Area, as an Override (ADR 0017), the person's settings (ADR 0040), "Show on the Agenda" (ADR 0030) and Hidden from AI (ADR 0049) are. It is sealed under the personal key and goes to the user's own Devices only, by sync, the shared folder, bundles and Backups, never to another person, a Shared Area's segments or a Sent Item. Only what the user's own Devices wrote counts, read from the log alone, the newest such entry winning, so Heads-ups another person's modified app slips into the log ring nowhere. Setting them asks for no role: a Member sets their own for a shared Routine without the right to change it, and nobody else gets them.

The same entity holds them for a Routine of the user's own, so there is one way to read them. They are not a field of the Routine: the Routine's fields go to everyone in its Area, and one person's "1 day before" would ring for all.

The field `headsUps` holds the offsets before the Occurrence's start in whole minutes, as decimal numbers joined by `,`, ascending, without duplicates, each from 1 to 43200 (30 days, as far as the alarm plan looks ahead); null for none. A part that is no such number is left out on reading. The same list again writes nothing. It carries no version (ADR 0034): a later one that needs another shape takes another field. An offset list maps one to one onto the `reminders` of an AXT Appointment (#340), which are offsets from its `start`.

## Planned beside the alarms

`headsUpPlan` / `heads_up_plan` is a plan of its own next to the alarm plan (ADR 0004), so the alarms, their limits and their fixture stay as they were. For each Occurrence the alarm plan looks at that is neither Done nor skipped nor snoozed, each offset gives one Heads-up:

- A timed Occurrence: its wall-clock moment less the offset, counted back on the calendar, so "1 day before" 10:00 is 10:00 the day before, across a daylight-saving switch too, and placed as the alarms are (`instantOf`).
- An all-day Occurrence: only offsets of whole days; on that day at the time its all-day ring would ring there, this Device's "All-day items remind from" (#201), later when Quiet Hours hold that time, to their end.

A Heads-up is planned only while it lies ahead and before its Occurrence starts. Done, Skip or a snooze of the Occurrence before it takes it out of the plan, so it is not rung. In Quiet Hours it shows without sound or vibration (`QuietHours.holds`), whether or not the Routine escalates: Escalation is for an Occurrence ignored, and a Heads-up is never ignored into anything. It is a plain notification on Android, the iPhone and the desktop, without full screen and without Done, Snooze or Not today, and shares its Occurrence's notification, so the ring replaces it; on the iPhone, where a delivered notification cannot be replaced, it is in the Occurrence's group and goes with it, and takes at most 16 of the 64 notifications iOS holds ahead, so it never crowds out a ring.

## The AI port

The tools read a Routine's Heads-ups (`headsUpMinutes`) and set them up (`set_heads_ups`), as they set up "Show on the Agenda": a choice of the user's own about how they are reminded, not something they did. This amends ADR 0026: an AI Task may write `headsUps` on `headsups:<routine id>`, of a Routine the task can read, without `may_manage`. The draft waits for review like any other.

## Considered Options

- A field on the Routine (rejected: everyone in a Shared Area would get one person's Heads-ups, and a Member could not set any).
- Heads-ups as alarms of a new kind in the alarm plan (rejected: every platform rule keyed on the Routine and Occurrence, a snooze's Re-nag, the Catch-up count, the full-screen ring, would have to tell them apart, and the alarms' fixture would change throughout).
- Offsets as instants before the start (rejected: "1 day before" would move by an hour across a daylight-saving switch).
- For an all-day Occurrence 00:00 of the day, or the end of Quiet Hours (rejected for now: a Heads-up would come at a time its own Occurrence never rings at; it rings when the all-day ring would ring on that day).

## Consequences

- A Heads-up rings only on the Devices of the person who set it; a new Device of theirs gets them with the rest of their own entries.
- Changing a Routine from timed to all-day keeps its minute offsets in the log, unplanned, until the editor saves the schedule's whole days only.
- A Routine moved to the trash keeps its Heads-ups; it plans none while trashed.

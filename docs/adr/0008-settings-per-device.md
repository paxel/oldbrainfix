# Settings belong to a Device and are never synced

Notes, Areas, Routines and their history sync; settings do not. Each Device decides on its own whether it alerts, its Re-nag interval, Quiet Hours, theme, locks and the sound its Occurrences ring with (the app's own sound unless one is chosen: a system sound on the phone, a sound file on the desktop; #102). Someone who lives at the desk can turn phone alerts off and keep desktop alerts on, and someone who lost the phone can keep the desktop nagging, and vice versa.

## Consequences

Two Devices with alerts on both fire the same Occurrence until a sync tells one of them it is Done (see ADR 0003 for the delay).

A Backup carries none of them but the playback speed of voice recordings, which a restored Device takes over (ADR 0016, #86).

## Shopping Lists on the Agenda (ADR 0030)

Amended by #123: how a Device's Agenda shows Shopping Lists today is a setting of its own, one row per list on a day when Lines came back until another way is chosen.

Amended by #186: each person chooses how each Shopping List shows on their Agenda, kept on their own Devices as an Override is (ADR 0030); the Device's setting is no longer offered and is read only for the default of a list nobody chose for.

## Units (ADR 0028)

Amended by ADR 0028 (#103): whether a Device shows the known units of Tallies in metric or imperial is a setting of its own, following the region the system reports until one is chosen.

## From when all-day items remind (#201)

Amended by #201: from when on an all-day Occurrence rings is a setting of each Device ("All-day items remind from", 15:00 unless changed), replacing the fixed 09:00 of both cores; before it the Occurrence only shows on the Agenda. A calendar file a Device exports gives an all-day event its alarm at that Device's time.

## Settings of the person (ADR 0040)

Amended by #211: "Move a Note to the Trash when its one-time Reminder is done" belongs to the person, not the Device. It lies in the log as an Override does and is the same on all of their own Devices (ADR 0040).

Amended by #197: Simple or Advanced is the second setting of the person, kept the same way and the same on all of their own Devices (ADR 0041).

## The last Shopping List (#276)

Amended by #276: the Shopping List a capture on a Device went into last is a setting of that Device (`lastShoppingList`, the list's Note id), put at the top of the lists Add to a Shopping List offers. An empty value reads as none, and it may name a list gone since, which the pick does not list. Capturing writes it; the user never sets it, so the AI port has no tool for it.

## The window's geometry (#278)

Amended by #278: on the desktop the window's size and position, whether it is maximized, the sidebar's width and the Note dialog's size are settings of that Device (`windowSize`, `windowPosition`, `windowMaximized`, `sidebarWidth`, `noteDialogSize`), written as the user leaves them and read at the next start. A position the system does not tell, as on Wayland, is not written, and one on no monitor any more opens the window centred. The user never sets them by name, so the AI port has no tool for them.

# Settings belong to a Device and are never synced

Notes, Areas, Routines and their history sync; settings do not. Each Device decides on its own whether it alerts, its Re-nag interval, Quiet Hours, theme, locks and the sound its Occurrences ring with (the app's own sound unless one is chosen: a system sound on the phone, a sound file on the desktop; #102). Someone who lives at the desk can turn phone alerts off and keep desktop alerts on, and someone who lost the phone can keep the desktop nagging, and vice versa.

## Consequences

Two Devices with alerts on both fire the same Occurrence until a sync tells one of them it is Done (see ADR 0003 for the delay).

A Backup carries none of them but the playback speed of voice recordings, which a restored Device takes over (ADR 0016, #86).

## Shopping Lists on the Agenda (ADR 0030)

Amended by #123: how a Device's Agenda shows Shopping Lists today is a setting of its own, one row per list on a day when Lines came back until another way is chosen.

## Units (ADR 0028)

Amended by ADR 0028 (#103): whether a Device shows the known units of Tallies in metric or imperial is a setting of its own, following the region the system reports until one is chosen.

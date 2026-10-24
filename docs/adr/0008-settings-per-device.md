# Settings belong to a Device and are never synced

Notes, Areas, Routines and their history sync; settings do not. Each Device decides on its own whether it alerts, its Re-nag interval, Quiet Hours, theme and locks. Someone who lives at the desk can turn phone alerts off and keep desktop alerts on, and someone who lost the phone can keep the desktop nagging, and vice versa.

## Consequences

Two Devices with alerts on both fire the same Occurrence until a sync tells one of them it is Done (see ADR 0003 for the delay).

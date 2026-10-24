# Exact alarms through our own Kotlin layer, user-granted permissions

Routines and Reminders fire on the minute through Chaos Alert's own Kotlin alarm layer, not through `flutter_local_notifications` with inexact timing as in Catlog: a nudge that comes 15 minutes late is a missed nudge for forgetful people. Because the app ships on Google Play, which reserves `USE_EXACT_ALARM` and `USE_FULL_SCREEN_INTENT` for alarm-clock and calendar apps, we request `SCHEDULE_EXACT_ALARM` and the full-screen permission as special access the user grants in a step-by-step setup. If the user declines, alarms fall back to inexact timing and plain notifications, and a red banner stays until it is fixed.

## Considered Options

- Inexact alarms as in Catlog (rejected: late alarms).
- `USE_EXACT_ALARM` (rejected: Play may reject the app).
- Separate builds with `USE_EXACT_ALARM` for F-Droid and GitHub (rejected: two behaviours to test and explain).

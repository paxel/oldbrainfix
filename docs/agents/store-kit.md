# Store kit

Project file for the `store-kit` skill: the ids, paths and terms it reads on
every release.

## Stores

- Google Play: `io.github.paxel.oldbrainfix`. The tag pipeline uploads the
  bundle to the internal track; promotion is done in the Play Console.
- App Store: none. Every App Store block, frame table and step stays empty.
- F-Droid: the tag pipeline copies `fastlane/metadata/android/` into the
  repo's F-Droid metadata, screenshots included.

## Locales

`en-US` and `de-DE`, from `lib/l10n/app_en.arb` and `lib/l10n/app_de.arb`.

## Paths

- Listing texts: `fastlane/metadata/android/<locale>/` (`title.txt`,
  `short_description.txt`, `full_description.txt`).
- Release notes record: `docs/release-notes-<version>.md`. The tag pipeline
  takes What's new for Google Play and F-Droid from its `play_notes_en` and
  `play_notes_de` blocks, word for word, and refuses a tag without them.
- Privacy policy: https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
  and https://paxel.github.io/oldbrainfix/de/reference/privacy-policy.html.

## Screenshots

Generator: `test/screenshots/screenshots_test.dart`, skipped in normal runs.

```sh
flutter test test/screenshots --run-skipped --concurrency=1
```

It writes phone frames, 1080 × 1920, per locale to
`fastlane/metadata/android/<locale>/images/phoneScreenshots/`, where Play
and F-Droid both take them. `docs/screenshots/play` links to
`fastlane/metadata/android`, so the release notes file names a frame as
`play/<locale>/images/phoneScreenshots/<file>`.

| # | File             | What is on it                                              |
|---|------------------|------------------------------------------------------------|
| 1 | `1-agenda.png`   | Agenda with today's Routines and the Unsorted banner       |
| 2 | `2-capture.png`  | Capture after the plus, with voice and photo buttons       |
| 3 | `3-diary.png`    | An Area's diary of Note cards                              |
| 4 | `4-editor.png`   | Note editor with dated parts and Importance                |
| 5 | `5-routine.png`  | Routine editor with its schedule                           |
| 6 | `6-alarm.png`    | Alarm ring screen with Done and Snooze                     |
| 7 | `7-history.png`  | Routine history with Streak and Adherence                  |
| 8 | `8-dark.png`     | Agenda in the dark theme                                   |

No tablet frames. Feature graphic (1024 × 500) and hi-res icon (512 × 512)
are uploaded in the Play Console by hand.

## Pages

- Play Kit: https://claude.ai/artifact/Sey69kjoaAhnUVxvHrst77
- App Store Kit: not used.

## Glossary

| English        | arb key             |
|----------------|---------------------|
| Note           | `noteTitle`         |
| Areas          | `areasTitle`        |
| Topic          | `topicChoice`       |
| Routine        | `routine`           |
| Reminder       | `reminder`          |
| Unsorted       | `unsorted`          |
| Streak         | `streakHeading`     |
| Adherence      | `adherenceHeading`  |
| Importance     | `importance`        |
| Snooze         | `snooze`            |
| Missed         | `missedTitle`       |
| Trash          | `trashTitle`        |
| Backups        | `backupsTitle`      |
| Profile        | `profileTitle`      |
| Nudges         | `settingsNudges`    |
| Quiet Hours    | `quietHours`        |
| Re-nag         | `renagHeading`      |
| Recovery Sheet | `backupSheetTitle`  |
| App Lock       | `appLock`           |
| Escalation     | `escalationHeading` |

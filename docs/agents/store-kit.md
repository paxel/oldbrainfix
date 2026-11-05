# Store kit

Project file for the `store-kit` skill: the ids, paths and terms it reads on
every release.

## Stores

- Google Play: `io.github.paxel.oldbrainfix`. The tag pipeline uploads the
  bundle to the internal track; promotion is done in the Play Console.
- App Store: the app record for bundle id `io.github.paxel.oldbrainfix`
  (the same id as the Android package), Apple id `<to be filled in by the
  user>`, SKU `oldbrainfix`, iOS 26 and newer. The tag
  pipeline uploads each build to TestFlight once the Apple secrets exist
  (`docs/agents/ios-release.md`). Until the first App Store release
  (#245) the app reaches testers through TestFlight only; the listing
  texts are written then.
- F-Droid: the tag pipeline copies `fastlane/metadata/android/` into the
  repo's F-Droid metadata, screenshots included.

## Locales

`en-US` and `de-DE`, from `lib/l10n/app_en.arb` and `lib/l10n/app_de.arb`,
in Google Play and in App Store Connect alike.

## Paths

- Listing texts: `fastlane/metadata/android/<locale>/` (`title.txt`,
  `short_description.txt`, `full_description.txt`).
- Release notes record: `docs/release-notes-<version>.md`. The tag pipeline
  takes What's new for Google Play and F-Droid from its `play_notes_en` and
  `play_notes_de` blocks, word for word, and refuses a tag without them.
  The same blocks are the in-app "What's new in X.Y.Z" Note (#180, ADR
  0042), which both apps bundle from `assets/help/whats_new.json`: once the
  record is written, run `python3 tool/whats_new.py --bundle` and commit the
  file with it. The tag pipeline refuses a tag whose version the bundle
  lacks, and CI one whose bundle is not current. Write the notes as a Note
  reads them too: an update's user sees them in the app, not only in Play.
- Privacy policy: https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
  and https://paxel.github.io/oldbrainfix/de/reference/privacy-policy.html.

## Screenshots

Generator: `test/screenshots/screenshots_test.dart`, skipped in normal runs.

```sh
flutter test test/screenshots --run-skipped --concurrency=1 --plain-name 'store screenshots'
```

It writes phone frames, 1080 × 1920, per locale to
`fastlane/metadata/android/<locale>/images/phoneScreenshots/`, where Play
and F-Droid both take them. `docs/screenshots/play` links to
`fastlane/metadata/android`, so the release notes file names a frame as
`play/<locale>/images/phoneScreenshots/<file>`.

| # | File            | What is on it                                                                            |
|---|-----------------|------------------------------------------------------------------------------------------|
| 1 | `1-agenda.png`  | Agenda: today's Routines, two Frogs, the Unsorted banner                                 |
| 2 | `2-capture.png` | The quick note with Photo, Voice, Gallery, Paste, the Nudge buttons and "To full editor" |
| 3 | `3-diary.png`   | An Area's diary of Note cards in full                                                    |
| 4 | `4-editor.png`  | Note editor with its badges and dated parts                                              |
| 5 | `5-fan.png`     | The "+" fanned out: Text, Voice, Photo, Gallery, Paste                                   |
| 6 | `6-alarm.png`   | Alarm ring screen with Done and Snooze                                                   |
| 7 | `7-history.png` | Routine history with Streak and Adherence                                                |
| 8 | `8-dark.png`    | Agenda in the dark theme, scrolled to the rest of the week                               |

No tablet frames. Feature graphic (1024 × 500) and hi-res icon (512 × 512)
are uploaded in the Play Console by hand.

### App Store

The same generator draws the App Store's frames (#244), skipped in normal
runs too:

```sh
flutter test test/screenshots --run-skipped --concurrency=1 --plain-name 'iphone screenshots'
```

It writes eight frames per locale, 1320 × 2868, to
`fastlane/metadata/ios/<locale>/screenshots/iphone-6.9/`, where they are
uploaded in App Store Connect by hand. They show the app as on iOS: the
iOS platform (the Cupertino back arrow on pushed pages), the iPhone 16
Pro Max's safe area (440 × 956 points at 3), and opaque PNGs without an
alpha channel, which App Store Connect refuses. Roboto stands in for
Apple's system font, which cannot be loaded outside Apple's systems. The
release job's `appstore-screenshots` renders the same frames and attaches
them to the run as the artifact `appstore-screenshots`; they come from
the Flutter test, not the iOS simulator.

Sizes, from Apple's
[screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
(read 2026-11-05):

| Display (Apple's name)                         | Portrait sizes                        | Here                      |
|------------------------------------------------|---------------------------------------|---------------------------|
| iPhone with Dynamic Island (large), 6.9"       | 1320 × 2868, 1290 × 2796, 1260 × 2736 | 1320 × 2868, `iphone-6.9` |
| iPhone with Face ID (large), 6.5"              | 1284 × 2778, 1242 × 2688              | scaled by Apple           |
| iPhone with Dynamic Island (medium), 6.3"/6.1" | 1206 × 2622, 1179 × 2556              | scaled by Apple           |
| Smaller iPhones                                | down to 640 × 1136                    | scaled by Apple           |

- Apple requires at least one screenshot for the iPhone with Dynamic
  Island (medium); "you only need to provide screenshots for the highest
  required resolution" and App Store Connect scales them down. A missing
  medium size takes the Face ID (large) screenshots, and a missing
  Face ID (large) size the Dynamic Island (large) ones, so the one set at
  1320 × 2868 covers every iPhone.
- Up to 10 screenshots per size, PNG or JPEG, no alpha channel.
- iPad: an app that supports iPadOS also needs at least one iPad 13"
  screenshot. The Xcode project still says `TARGETED_DEVICE_FAMILY =
  "1,2"` (iPhone and iPad, Flutter's default) while #236 plans the
  iPhone; before #245 submits, either the project goes to `1` or iPad
  frames are added here.
- iPhone Duo (1398 × 2034 outer, 2007 × 2853 inner) is required from
  April 2027 for apps built with the iOS 27.1 SDK or later; no frames for
  it yet.

| # | File             | What is on it                                                                                |
|---|------------------|----------------------------------------------------------------------------------------------|
| 1 | `1-agenda.png`   | as Play's 1                                                                                  |
| 2 | `2-capture.png`  | as Play's 2                                                                                  |
| 3 | `3-diary.png`    | as Play's 3, with the Cupertino back arrow                                                   |
| 4 | `4-editor.png`   | as Play's 4, with the Cupertino back arrow                                                   |
| 5 | `5-fan.png`      | as Play's 5                                                                                  |
| 6 | `6-routines.png` | All Routines and Reminders; the iPhone has no alarm ring screen, AlarmKit's own screen rings |
| 7 | `7-history.png`  | as Play's 7, with the Cupertino back arrow                                                   |
| 8 | `8-dark.png`     | as Play's 8                                                                                  |

The app icon comes from the build (`ios/Runner/Assets.xcassets`); App
Store Connect needs no separate one.


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

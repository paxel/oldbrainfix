# Release notes 0.1.1

## Summary

Fixes and changes from the first round of testing 0.1.0 on phones. Phone only; the desktop app is unchanged apart from opening the new repeat rules in its text field. The listing stays as in 0.1.0. The store screenshots predate the new Agenda week and Capture buttons and are not regenerated in this release.

<!-- store-kit
live_play: 0.1.0 (internal)
live_appstore: none
head: 0.1.1
sections: 0.1.1
frames: unchanged
-->

## Play: release notes

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Internal testing release" -->
```
New
• The Agenda shows the whole week, each day under its weekday and date.
• Large Photo and Voice buttons when capturing.
• Routines on a day of the month, the first or last weekday of the month, and with an end.
• The speech model is offered once after setup.
• An Area shows only its Notes without a Topic.
• Choices are one option per line.
Fixed
• Page ends no longer hide under the navigation bar.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Neu
• Die Agenda zeigt die ganze Woche, jeden Tag unter Wochentag und Datum.
• Große Knöpfe für Foto und Sprache beim Erfassen.
• Routinen an einem Tag des Monats, am ersten oder letzten Wochentag des Monats und mit Ende.
• Das Sprachmodell wird einmal nach der Einrichtung angeboten.
• Ein Bereich zeigt nur seine Notizen ohne Thema.
• Auswahlen stehen eine Option pro Zeile.
Behoben
• Seitenenden liegen nicht mehr unter der Navigationsleiste.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="Play internal testing" -->
```
What to test
• With three-button navigation: scroll to the end of the Starter Set, Settings and an Area; nothing hides under the bar.
• The Agenda: seven days, an empty day says "Nothing planned".
• A Routine "every month on the last Friday" and one "after 5 times"; check the Agenda and the alarms.
• Capture: Photo and Voice buttons stay visible with the keyboard open.
• Open an Area that has Topics: only its own Notes show; searching there still finds the Topics' Notes.
```

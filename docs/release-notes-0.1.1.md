# Release notes 0.1.1

## Summary

Fixes and changes from the first round of testing 0.1.0 on phones: the Agenda week, new Note cards, swipes that follow the finger, removing a recording or photo while its text stays, playback speed, and better transcription of English words in German speech with an optional larger model. Removing recordings and photos, playback speed and the speech model changes are on the desktop too. The listing stays as in 0.1.0. The store screenshots predate the new Agenda week, Capture buttons and Note cards and are regenerated after the next Capture changes (#98).

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
• The Agenda shows the whole week.
• Note cards with an icon row and a ⋮ menu.
• Remove a recording or photo and keep its text.
• Playback speed from 0.5× to 2×.
• English words in German speech are transcribed better; a larger speech model is optional.
• Routines on a day of the month, with an end.
• Large Photo and Voice buttons when capturing.
• Swipe from the Agenda to the Areas.
Fixed
• Page ends no longer hide under the navigation bar.
• Lists no longer show old Notes after sorting.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Neu
• Die Agenda zeigt die ganze Woche.
• Notizkarten mit Symbolzeile und ⋮-Menü.
• Aufnahme oder Foto entfernen, der Text bleibt.
• Wiedergabe von 0,5× bis 2×.
• Englische Wörter im Deutschen besser erkannt; größeres Sprachmodell wählbar.
• Routinen an einem Tag des Monats, mit Ende.
• Große Foto- und Sprachknöpfe beim Erfassen.
• Wischen von der Agenda zu den Bereichen.
Behoben
• Seitenenden nicht mehr unter der Navigationsleiste.
• Keine alten Notizen in Listen nach dem Einsortieren.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="Play internal testing" -->
```
What to test
• With three-button navigation: scroll to the end of the Starter Set, Settings and an Area; nothing hides under the bar.
• The Agenda: seven days, an empty day says "Nothing planned". Swipe left opens the Areas list, swipe right returns; the pages follow the finger and spring back when let go early.
• A Routine "every month on the last Friday" and one "after 5 times"; check the Agenda and the alarms.
• Capture: Photo and Voice buttons stay visible with the keyboard open.
• Open an Area that has Topics: only its own Notes show; searching there still finds the Topics' Notes.
• Note cards: the icon row and ⋮ (Importance, Send, calendar, Add to Note, Delete); Area and Topic show only where the screen does not name them.
• Open Unsorted, then the Nudge notification, sort both Notes there, go back: Unsorted is empty.
• A voice Note with a Transcript: ⋮ → remove the recording; the Transcript stays and search finds it. The same with a photo and its Photo Text.
• Play a recording at 0.5× and 2×; the speed stays for the next recording.
• Speak German with English words (Meeting, Update, Unsorted, your Area names) and check the Transcript. In Settings, switch to the small model (488 MB download) and compare.
```

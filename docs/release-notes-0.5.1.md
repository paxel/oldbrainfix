# Release notes 0.5.1

Paste source for the Google Play Console and App Store Connect: the release's What's new (the tag pipeline reads the `play_notes_en` and `play_notes_de` blocks, and the apps bundle them as the in-app "What's new in 0.5.1" Note), tester notes and both store listings. The changes themselves are in [CHANGELOG.md](../CHANGELOG.md).

## Summary

Version 0.5.1 after 0.5.0, which is on Google Play's internal track, on F-Droid and in the GitHub Release, and in App Store review as the App Store's first version. It covers the 0.5.1 section of the changelog: the iPhone app's icon, which was still Flutter's logo, is the Old Brain Fix icon. Nothing changes on Android or the desktop. The listing and the screenshots are unchanged.

<!-- store-kit
live_play: 0.5.0 (internal)
live_appstore: 0.5.0 (in review)
head: 0.5.1
sections: 0.5.1
frames: reused
-->

## Play: release notes

The same text is the in-app What's new Note on every platform, so it names the one change there is.

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Internal testing release" -->
```
Fixed
• The iPhone shows the app's own icon.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Behoben
• Das iPhone zeigt das eigene Symbol der App.
```

## App Store: What's New

Pasted when 0.5.1 is the App Store's second version; if it replaces 0.5.0 in review instead, there is no What's New (see the steps below).

<!-- block id=appstore_whatsnew_en limit=4000 lang=en title="What's New, English" where="Version information" -->
```
The app shows its own icon, the question and exclamation mark over a box, on the home screen and in the App Store.
```

<!-- block id=appstore_whatsnew_de limit=4000 lang=de title="Neue Funktionen, Deutsch" where="Version information" -->
```
Die App zeigt ihr eigenes Symbol, Frage- und Ausrufezeichen über einem Kästchen, auf dem Startbildschirm und im App Store.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="TestFlight test information · Play internal track" -->
```
What to test
• iPhone: install the build and look at the icon on the home screen, in the App Library, in Spotlight, in Settings and on a notification: the question and exclamation mark over a box on a dark ground, no blue Flutter logo.
• iPhone: long-press the icon; the shortcuts still open.
Known
• Nothing else changed since the last build; Android and the desktop are the same as before.
```

## Play: listing

The live text, verbatim. Changed only through an accepted edit below.

<!-- block id=play_title limit=30 lang=en title="Title" where="Main store listing" -->
```
Old Brain Fix
```

<!-- block id=play_short_en limit=80 lang=en title="Short description, en-US" where="Main store listing" -->
```
Notes, reminders and routines with exact alarms. No account, no server.
```

<!-- block id=play_short_de limit=80 lang=de title="Kurzbeschreibung, de-DE" where="Main store listing" -->
```
Notizen, Erinnerungen und Routinen mit exakten Alarmen. Ohne Konto, ohne Server.
```

<!-- block id=play_full_en limit=4000 lang=en title="Full description, en-US" where="Main store listing · b i u br count" -->
```
Old Brain Fix is a note and reminder app for Android. A desktop app for Linux, Windows and macOS uses the same data. There is no account and no server; all data is stored on the user's own devices.

<b>Capture</b>
• Text, photo and voice Notes, opened from the + button, home-screen widgets, Quick Settings tiles, app shortcuts, or a hardware button where the phone allows assigning one.
• Content shared from other apps is saved as a Note.
• Each Note records the time of capture. Optional: the location (shown as a Named Place such as "Home" when one matches), the current calendar event and the connected Bluetooth device.
• Voice Notes are transcribed on the device with Whisper (English and German). The speech model (148 MB) is downloaded once, on request.
• Text in photos and screenshots is recognised on the device with RustO, German letters and the euro sign included.
• Transcripts and recognised text are included in search.

<b>Organising</b>
• New Notes are kept in Unsorted. A notification (Nudge) reports Unsorted Notes at a set interval; Quiet Hours suppress it at night.
• Notes are filed into Areas and Topics (two levels), each with a colour and an icon. A starter set of Areas is offered at first start.
• Notes can be edited, amended and merged. Deleted items stay in the trash for 30 days.
• A Note can become a Checklist, a Shopping List whose items come back after a set number of days, weeks, months or years, a Tally with entries and charts, or a Session that picks Tallies. Its text can be formatted with headings, lists, tables, code blocks, boxes to tick and links that open other Notes and places in the app.

<b>Routines and Reminders</b>
• Any Note can have Reminders (one time) and Routines (repeating by a recurrence rule, several times a day if needed).
• Alarms ring at the exact minute, also on the lock screen, and are restored after a restart. This requires the "Alarms & reminders" permission.
• An alarm that is neither marked Done nor snoozed rings again at a set interval (Re-nag, default 10 minutes). Optionally, after a set number of repeats it is shown as a full-screen alarm with sound.
• The Agenda lists today's Routines and Reminders, missed ones and the number of Unsorted Notes. It is also available as a home-screen widget.
• Each Routine has a history with streak and adherence.
• Routines and Reminders can be written to a device calendar or exported as an ICS file.

<b>Sync and sharing</b>
• Phone and desktop sync directly over the local network (TLS, paired by QR code or typed code), through a shared folder such as Syncthing or Nextcloud (end-to-end encrypted), or through an encrypted file sent with any app.
• A single Area can be shared with another person. Other Areas are not shared.
• Your Profile unites all of your Devices. Your name, a unique code and an optional picture show on the Notes you share with others.
• A Note can be sent to another person as a file, together with its Routines and Reminders. The receiver can accept or decline it.
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.

<b>Privacy and security</b>
• No account, no server, no analytics, no advertising.
• The app uses the network only for sync and for the speech model download.
• Optional App Lock (PIN, fingerprint) and Area Locks. Capture and alarms keep working while locked.
• Encrypted Backups (Argon2id, XChaCha20-Poly1305) into a chosen folder, optionally weekly, with a printable Recovery Sheet. Optionally, Android's backup to Google stores the encrypted Backup file and nothing else.

<b>Other</b>
• English and German. Light and dark theme.
• Free and open source, licensed under Apache-2.0 or MIT.
```

<!-- block id=play_full_de limit=4000 lang=de title="Vollständige Beschreibung, de-DE" where="Main store listing · b i u br count" -->
```
Old Brain Fix ist eine Notiz- und Erinnerungs-App für Android. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.

<b>Erfassen</b>
• Text-, Foto- und Sprachnotizen, geöffnet über den +-Knopf, Widgets, Schnelleinstellungen, App-Verknüpfungen oder eine Hardwaretaste, sofern das Telefon die Belegung erlaubt.
• Inhalte, die aus anderen Apps geteilt werden, werden als Notiz gespeichert.
• Jede Notiz speichert den Zeitpunkt der Erfassung. Optional: den Ort (als benannter Ort wie „Zuhause“ angezeigt, wenn einer passt), den laufenden Kalendertermin und das verbundene Bluetooth-Gerät.
• Sprachnotizen werden auf dem Gerät mit Whisper transkribiert (Deutsch und Englisch). Das Sprachmodell (148 MB) wird einmalig auf Anforderung heruntergeladen.
• Text in Fotos und Screenshots wird auf dem Gerät mit RustO erkannt, Umlaute, ß und das Euro-Zeichen eingeschlossen.
• Transkripte und erkannter Text werden bei der Suche berücksichtigt.

<b>Ordnen</b>
• Neue Notizen liegen in „Unsortiert“. Eine Benachrichtigung (Stupser) meldet unsortierte Notizen in einem einstellbaren Abstand; Ruhezeiten unterdrücken sie nachts.
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol. Beim ersten Start wird eine Auswahl an Bereichen angeboten.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen, Tabellen und Statuskästchen formatieren.

<b>Routinen und Erinnerungen</b>
• Jede Notiz kann Erinnerungen (einmalig) und Routinen (wiederkehrend nach einer Wiederholungsregel, bei Bedarf mehrmals täglich) haben.
• Alarme klingeln minutengenau, auch auf dem Sperrbildschirm, und werden nach einem Neustart wiederhergestellt. Dafür ist die Berechtigung „Wecker und Erinnerungen“ nötig.
• Ein Alarm, der weder als erledigt markiert noch verschoben wird, klingelt in einem festen Abstand erneut (Erneut erinnern, Standard 10 Minuten). Optional erscheint er nach einer einstellbaren Anzahl von Wiederholungen als Vollbild-Alarm mit Ton.
• Die Agenda zeigt die Routinen und Erinnerungen des Tages, verpasste Termine und die Zahl der unsortierten Notizen. Sie ist auch als Widget verfügbar.
• Jede Routine hat einen Verlauf mit Serie und Einhaltung.
• Routinen und Erinnerungen können in einen Gerätekalender geschrieben oder als ICS-Datei exportiert werden.

<b>Abgleich und Teilen</b>
• Telefon und Desktop gleichen sich direkt im lokalen Netz ab (TLS, gekoppelt per QR-Code oder eingetippten Code), über einen geteilten Ordner wie Syncthing oder Nextcloud (Ende-zu-Ende-verschlüsselt) oder über eine verschlüsselte Datei, die mit einer beliebigen App verschickt wird.
• Ein einzelner Bereich kann mit einer anderen Person geteilt werden. Andere Bereiche werden nicht geteilt.
• Dein Profil vereint alle deine Geräte. Deinen Namen, einen eindeutigen Code und ein optionales Bild sieht jeder an den Notizen, die du mit anderen teilst.
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden. Die empfangende Person kann sie annehmen oder ablehnen.

<b>Datenschutz und Sicherheit</b>
• Kein Konto, kein Server, keine Analyse, keine Werbung.
• Die App nutzt das Netzwerk nur für den Abgleich und den Download des Sprachmodells.
• Optionale App-Sperre (PIN, Fingerabdruck) und Bereichssperren. Erfassen und Alarme funktionieren auch im gesperrten Zustand.
• Verschlüsselte Sicherungen (Argon2id, XChaCha20-Poly1305) in einen gewählten Ordner, optional wöchentlich, mit einem druckbaren Wiederherstellungsblatt. Optional speichert die Android-Sicherung bei Google nur die verschlüsselte Sicherungsdatei.

<b>Sonstiges</b>
• Deutsch und Englisch. Helles und dunkles Design.
• Kostenlos und quelloffen, lizenziert unter Apache-2.0 oder MIT.
```

<!-- block id=play_privacy_url limit=255 lang=en title="Privacy policy URL" where="Policy and programs · App content · Privacy policy" -->
```
https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
```

## App Store: listing

The text in `fastlane/metadata/ios/`, verbatim, which `fastlane deliver` uploads on the submission; nothing is live yet. Changed only through an accepted edit below.

<!-- block id=appstore_name limit=30 lang=en title="Name" where="App information" -->
```
Old Brain Fix
```

<!-- block id=appstore_subtitle_en limit=30 lang=en title="Subtitle, English" where="App information" -->
```
Notes, reminders and routines
```

<!-- block id=appstore_subtitle_de limit=30 lang=de title="Untertitel, Deutsch" where="App information" -->
```
Notizen und Erinnerungen
```

<!-- block id=appstore_promo_en limit=170 lang=en title="Promotional text, English" where="Changeable without a build" -->
```
Text, photo and voice Notes, with Routines and Reminders that come back until they are Done. No account and no server: the data stays on your own Devices.
```

<!-- block id=appstore_promo_de limit=170 lang=de title="Werbetext, Deutsch" where="Changeable without a build" -->
```
Text-, Foto- und Sprachnotizen mit Routinen und Erinnerungen, die wiederkommen, bis sie erledigt sind. Ohne Konto, ohne Server: Die Daten bleiben auf deinen Geräten.
```

<!-- block id=appstore_keywords_en limit=100 lang=en title="Keywords, English" where="Comma separated, no spaces" -->
```
notes,reminder,routine,alarm,habit,checklist,shopping list,voice note,transcribe,offline,sync,todo
```

<!-- block id=appstore_keywords_de limit=100 lang=de title="Schlüsselwörter, Deutsch" where="Comma separated, no spaces" -->
```
notizen,erinnerung,routine,alarm,wecker,gewohnheit,checkliste,einkaufsliste,sprachnotiz,offline,todo
```

<!-- block id=appstore_desc_en limit=4000 lang=en title="Description, English" where="Plain text" -->
```
Old Brain Fix is a note and reminder app for the iPhone. A desktop app for Linux, Windows and macOS uses the same data. There is no account and no server; all data is stored on the user's own devices.

Capture
• Text, photo and voice Notes, opened from the + button, the Capture widget on the home screen and the lock screen, or the Control Center controls for voice, text and photo.
• Text, links and pictures shared from other apps are kept and open in Capture the next time the app opens.
• Each Note records the time of capture. Optional: the location (shown as a Named Place such as "Home" when one matches), the current calendar event and the car or headphones the sound plays through.
• Voice Notes are transcribed on the device with Whisper (English and German). The speech model (148 MB) is downloaded once, on request.
• Text in photos and screenshots is recognised on the device with RustO.
• Transcripts and recognised text are included in search.

Organising
• New Notes are kept in Unsorted. A notification (Nudge) reports Unsorted Notes at a set interval; Quiet Hours suppress it at night.
• Notes are filed into Areas and Topics (two levels), each with a colour and an icon. A starter set of Areas is offered at first start.
• Notes can be edited, amended and merged. Deleted items stay in the trash for 30 days.
• A Note can become a Checklist, a Shopping List whose items come back, a Tally or a Session. Its text can be formatted with headings, lists, tables, boxes to tick and links that open other Notes and places in the app.

Routines and Reminders
• Any Note can have Reminders (one time) and Routines (repeating by a recurrence rule, several times a day if needed).
• They come as time-sensitive notifications with Done and Snooze.
• A notification that is neither Done nor snoozed comes again at a set interval (Re-nag, default 10 minutes). Optionally, after a set number of Re-nags it rings as an iPhone alarm, through silent mode and Focus.
• The Agenda lists today's Routines and Reminders, missed ones and the number of Unsorted Notes. It is also a home-screen widget, whose boxes tick without opening the app.
• Each Routine has a history with streak and adherence.
• The Agenda can show the appointments of the calendars you pick. Routines and Reminders can be written to a calendar or exported as an ICS file.

Sync and sharing
• Your Devices sync directly over the local network while the app is open (TLS, paired by QR code or typed code), through a shared folder in iCloud Drive or Möbius Sync (end-to-end encrypted), or through an encrypted file sent with any app.
• A single Area can be shared with another person. Other Areas are not shared.
• A Note can be sent to another person as a file, with its Routines and Reminders, to accept or decline.
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.

Privacy and security
• No account, no server, no analytics, no advertising.
• The app uses the network only for sync and for the speech model download.
• Optional App Lock (PIN, Face ID or Touch ID) and Area Locks. Capture and alarms keep working while locked.
• Encrypted Backups (Argon2id, XChaCha20-Poly1305) into a chosen folder, optionally weekly, with a printable Recovery Sheet. The iPhone's own backup leaves the app's data out.

Other
• English and German. Light and dark theme. iOS 26 or newer.
• Free and open source, licensed under Apache-2.0 or MIT.
```

<!-- block id=appstore_desc_de limit=4000 lang=de title="Beschreibung, Deutsch" where="Plain text" -->
```
Old Brain Fix ist eine Notiz- und Erinnerungs-App für das iPhone. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.

Erfassen
• Text-, Foto- und Sprachnotizen, geöffnet über den +-Knopf, das Widget „Erfassen“ auf dem Startbildschirm und dem Sperrbildschirm oder die Steuerelemente im Kontrollzentrum für Sprache, Text und Foto.
• Text, Links und Bilder, die aus anderen Apps geteilt werden, werden aufbewahrt und öffnen sich beim nächsten Öffnen der App im Erfassen.
• Jede Notiz speichert den Zeitpunkt der Erfassung. Optional: den Ort (als benannter Ort wie „Zuhause“ angezeigt, wenn einer passt), den laufenden Kalendertermin und das Auto oder die Kopfhörer, über die der Ton läuft.
• Sprachnotizen werden auf dem Gerät mit Whisper transkribiert (Deutsch und Englisch). Das Sprachmodell (148 MB) wird einmalig auf Anforderung heruntergeladen.
• Text in Fotos und Screenshots wird auf dem Gerät mit RustO erkannt.
• Transkripte und erkannter Text werden bei der Suche berücksichtigt.

Ordnen
• Neue Notizen liegen in „Unsortiert“. Eine Benachrichtigung (Stupser) meldet unsortierte Notizen in einem einstellbaren Abstand; die Ruhezeit unterdrückt sie nachts.
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol. Beim ersten Start wird eine Auswahl an Bereichen angeboten.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen und Tabellen formatieren.

Routinen und Erinnerungen
• Jede Notiz kann Erinnerungen (einmalig) und Routinen (wiederkehrend nach einer Wiederholungsregel, bei Bedarf mehrmals täglich) haben.
• Sie kommen als zeitkritische Benachrichtigungen mit Erledigt und Später.
• Eine Benachrichtigung, die weder erledigt noch verschoben wird, kommt in einem festen Abstand wieder (Erneut erinnern, Standard 10 Minuten). Optional klingelt sie nach einer einstellbaren Zahl von Wiederholungen als iPhone-Alarm, auch im Stummmodus und im Fokus.
• Die Agenda zeigt die Routinen und Erinnerungen des Tages, verpasste Termine und die Zahl der unsortierten Notizen. Sie ist auch ein Widget auf dem Startbildschirm, dessen Kästchen sich abhaken lassen, ohne die App zu öffnen.
• Jede Routine hat einen Verlauf mit Serie und Einhaltung.
• Die Agenda kann die Termine gewählter Kalender zeigen. Routinen und Erinnerungen können in einen Kalender geschrieben oder als ICS-Datei exportiert werden.

Abgleich und Teilen
• Deine Geräte gleichen sich direkt im lokalen Netz ab, solange die App offen ist (TLS, gekoppelt per QR-Code oder eingetippten Code), über einen geteilten Ordner in iCloud Drive oder Möbius Sync (Ende-zu-Ende-verschlüsselt) oder über eine verschlüsselte Datei, die mit einer beliebigen App verschickt wird.
• Ein einzelner Bereich lässt sich mit einer anderen Person teilen; andere Bereiche nicht.
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.
• Notizen, Themen oder ganze Bereiche lassen sich als Paket mit Kopien an andere weitergeben, die sie als ihre eigenen behalten.

Datenschutz und Sicherheit
• Kein Konto, kein Server, keine Analyse, keine Werbung.
• Die App nutzt das Netzwerk nur für den Abgleich und den Download des Sprachmodells.
• Optionale App-Sperre (PIN, Face ID oder Touch ID) und Bereichssperren. Erfassen und Alarme funktionieren auch im gesperrten Zustand.
• Verschlüsselte Sicherungen (Argon2id, XChaCha20-Poly1305) in einen gewählten Ordner, optional wöchentlich, mit einem druckbaren Wiederherstellungsblatt. Die eigene Datensicherung des iPhones lässt die Daten der App aus.

Sonstiges
• Deutsch und Englisch. Helles und dunkles Design. Ab iOS 26.
• Kostenlos und quelloffen, lizenziert unter Apache-2.0 oder MIT.
```

<!-- block id=appstore_privacy_url_en limit=255 lang=en title="Privacy Policy URL, English" where="App Privacy · per localization" -->
```
https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
```

<!-- block id=appstore_privacy_url_de limit=255 lang=de title="Datenschutz-URL, Deutsch" where="App Privacy · German localization" -->
```
https://paxel.github.io/oldbrainfix/de/reference/privacy-policy.html
```


## Edits

None. The listing is the live text, unchanged.

## Screenshots: Play

Phone 1080 × 1920 per locale; unchanged since 0.5.0, so the ones in the console stay. Feature graphic 1024 × 500, unchanged.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `play/en-US/images/phoneScreenshots/1-agenda.png · play/de-DE/images/phoneScreenshots/1-agenda.png` | 1080 × 1920 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder in the bar |
| 2 | `play/en-US/images/phoneScreenshots/2-capture.png · play/de-DE/images/phoneScreenshots/2-capture.png` | 1080 × 1920 | The quick note with Photo, Voice, Gallery, Paste, the alarm clock, the Nudge buttons and "To full editor" |
| 3 | `play/en-US/images/phoneScreenshots/3-diary.png · play/de-DE/images/phoneScreenshots/3-diary.png` | 1080 × 1920 | An Area's diary of Note cards in full |
| 4 | `play/en-US/images/phoneScreenshots/4-editor.png · play/de-DE/images/phoneScreenshots/4-editor.png` | 1080 × 1920 | Note editor with its badges and dated parts |
| 5 | `play/en-US/images/phoneScreenshots/5-fan.png · play/de-DE/images/phoneScreenshots/5-fan.png` | 1080 × 1920 | The Agenda's "+" fanned out: Text, Voice, Photo, Gallery, Paste |
| 6 | `play/en-US/images/phoneScreenshots/6-alarm.png · play/de-DE/images/phoneScreenshots/6-alarm.png` | 1080 × 1920 | Alarm ring screen with Done and Snooze |
| 7 | `play/en-US/images/phoneScreenshots/7-history.png · play/de-DE/images/phoneScreenshots/7-history.png` | 1080 × 1920 | Routine history with Streak and Adherence |
| 8 | `play/en-US/images/phoneScreenshots/8-dark.png · play/de-DE/images/phoneScreenshots/8-dark.png` | 1080 × 1920 | Agenda in the dark theme, scrolled to the rest of the week |

## Screenshots: App Store

iPhone 6.9″ 1320 × 2868 per locale, the one size App Store Connect scales to every iPhone; no iPad (iPhone only). Unchanged since 0.5.0; the ones uploaded for it stay.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/1-agenda.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/1-agenda.png` | 1320 × 2868 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder in the bar |
| 2 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/2-capture.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/2-capture.png` | 1320 × 2868 | The quick note with Photo, Voice, Gallery, Paste, the alarm clock, the Nudge buttons and "To full editor" |
| 3 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/3-diary.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/3-diary.png` | 1320 × 2868 | An Area's diary of Note cards in full, with the Cupertino back arrow |
| 4 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/4-editor.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/4-editor.png` | 1320 × 2868 | Note editor with its badges and dated parts, with the Cupertino back arrow |
| 5 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/5-fan.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/5-fan.png` | 1320 × 2868 | The Agenda's "+" fanned out: Text, Voice, Photo, Gallery, Paste |
| 6 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/6-routines.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/6-routines.png` | 1320 × 2868 | All Routines and Reminders |
| 7 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/7-history.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/7-history.png` | 1320 × 2868 | Routine history with Streak and Adherence, with the Cupertino back arrow |
| 8 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/8-dark.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/8-dark.png` | 1320 × 2868 | Agenda in the dark theme, scrolled to the rest of the week |


## Play Console, in order

1. Testing, Internal testing: the tag pipeline uploads the AAB; open the new release.
2. Release notes: en-US and de-DE from above, inside the language tags the console shows.
3. Next, then review, then Start rollout to Internal testing.

## App Store Connect, in order

1. The tag pipeline uploads the build to TestFlight; try it, and the tester notes above go into Test Information.
2. While 0.5.0 is still in review: either let its review finish and submit 0.5.1 as the second version (step 3), or take 0.5.0 out of review (Remove from Review), rename the version to 0.5.1, pick the 0.5.1 build and submit again; then there is no What's New, as on any first version.
3. As the second version: Add a version 0.5.1, pick the build, paste the What's New above in English and German, and submit with Actions, Release, Run workflow: the tag, publish on, submit_app_store on.
4. After approval, release the version by hand.

# Release notes 0.1.0

## Summary

First release; nothing is live on Google Play yet. This covers the 0.1.0 changelog section, phone features only. The eight phone frames per language are new, rendered by the screenshot generator. Play only: no App Store.

- Play Kit — https://claude.ai/artifact/Sey69kjoaAhnUVxvHrst77

<!-- store-kit
live_play: none
live_appstore: none
head: 0.1.0
sections: 0.1.0
frames: regenerated
-->

## Play: release notes

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Production release" -->
```
Added
• Text, photo and voice Notes from the +, widgets, Quick Settings tiles and app shortcuts.
• Areas and Topics to file Notes into; new Notes wait in Unsorted.
• Routines and Reminders with exact alarms, Re-nag and a history per Routine.
• Sync with the desktop app over the home network, a shared folder or a file.
• Encrypted Backups, App Lock and Area Locks. No account, no server.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Production release" -->
```
Neu
• Text-, Foto- und Sprachnotizen über +, Widgets, Schnelleinstellungen und App-Verknüpfungen.
• Bereiche und Themen zum Einsortieren; neue Notizen warten in Unsortiert.
• Routinen und Erinnerungen mit exakten Alarmen, Erneut erinnern und einem Verlauf je Routine.
• Abgleich mit der Desktop-App im Heimnetz, über einen geteilten Ordner oder eine Datei.
• Verschlüsselte Sicherungen, App-Sperre und Bereichssperren. Kein Konto, kein Server.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="Play internal and closed testing" -->
```
What to test
• Set a Routine a few minutes ahead, lock the phone and leave it: the alarm rings on the minute, and rings again (Re-nag) until Done or snoozed.
• Restart the phone with a Routine due: the alarm still rings.
• Put the Agenda widget on the home screen and check off a Routine there.
• Pair with the desktop app by QR code, change a Note on both, sync.
• Make a Backup, reinstall, restore it with the passphrase.

Known
• Text recognition in photos reads unaccented Latin letters only.
• Transcripts need a one-time 148 MB speech model download.
```

## Play: listing

The text in `fastlane/metadata/android/`, verbatim (nothing is live yet). Changed only through an accepted edit below.

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
• Text in photos and screenshots is recognised on the device with ocrs. Recognition covers unaccented Latin letters only.
• Transcripts and recognised text are included in search.

<b>Organising</b>
• New Notes are kept in Unsorted. A notification (Nudge) reports Unsorted Notes at a set interval; Quiet Hours suppress it at night.
• Notes are filed into Areas and Topics (two levels), each with a colour and an icon. A starter set of Areas is offered at first start.
• Notes can be edited, amended and merged. Deleted items stay in the trash for 30 days.

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
• Text in Fotos und Screenshots wird auf dem Gerät mit ocrs erkannt. Die Erkennung umfasst nur lateinische Buchstaben ohne Umlaute und ß.
• Transkripte und erkannter Text werden bei der Suche berücksichtigt.

<b>Ordnen</b>
• Neue Notizen liegen in „Unsortiert“. Eine Benachrichtigung (Stupser) meldet unsortierte Notizen in einem einstellbaren Abstand; Ruhezeiten unterdrücken sie nachts.
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol. Beim ersten Start wird eine Auswahl an Bereichen angeboten.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.

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

## Edits

One per changed listing line. `status` is `pending` until the user answers, then `accepted` (and applied to the block above) or `rejected` (block left as live).

<!-- edit block=play_full_de status=accepted -->
Glossary: the English text names Re-nag; the app calls it "Erneut erinnern" (`renagHeading`), the German text does not name it.
```
• Ein Alarm, der weder als erledigt markiert noch verschoben wird, klingelt in einem festen Abstand erneut (Standard 10 Minuten). Optional erscheint er nach einer einstellbaren Anzahl von Wiederholungen als Vollbild-Alarm mit Ton.
```
```
• Ein Alarm, der weder als erledigt markiert noch verschoben wird, klingelt in einem festen Abstand erneut (Erneut erinnern, Standard 10 Minuten). Optional erscheint er nach einer einstellbaren Anzahl von Wiederholungen als Vollbild-Alarm mit Ton.
```

<!-- edit block=play_full_en status=accepted -->
Changelog 0.1.0, Added: "A Profile with your name, picture and short code, … shared by all of your Devices …; the notes browser can show only the Notes of the people you pick"
```
• A single Area can be shared with another person. Other Areas are not shared.
```
```
• A single Area can be shared with another person. Other Areas are not shared.
• Your Profile unites all of your Devices. Your name, a unique code and an optional picture show on the Notes you share with others.
```

<!-- edit block=play_full_de status=accepted -->
Changelog 0.1.0, Added: the same Profile entry, German listing.
```
• Ein einzelner Bereich kann mit einer anderen Person geteilt werden. Andere Bereiche werden nicht geteilt.
```
```
• Ein einzelner Bereich kann mit einer anderen Person geteilt werden. Andere Bereiche werden nicht geteilt.
• Dein Profil vereint alle deine Geräte. Deinen Namen, einen eindeutigen Code und ein optionales Bild sieht jeder an den Notizen, die du mit anderen teilst.
```

## Screenshots: Play

Phone 1080 × 1920, 8 per language; the first three carry the listing. No tablet frames. Hi-res icon `play-developer/app-icon-512.png` (512 × 512) and feature graphic `play-developer/feature-graphic.png` (1024 × 500) are uploaded by hand.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `play/en-US/images/phoneScreenshots/1-agenda.png · play/de-DE/images/phoneScreenshots/1-agenda.png` | 1080 × 1920 | Agenda: today's Routines and Reminders, tomorrow dimmed, Unsorted banner |
| 2 | `play/en-US/images/phoneScreenshots/2-capture.png · play/de-DE/images/phoneScreenshots/2-capture.png` | 1080 × 1920 | Capture after the plus, with voice and photo buttons |
| 3 | `play/en-US/images/phoneScreenshots/3-diary.png · play/de-DE/images/phoneScreenshots/3-diary.png` | 1080 × 1920 | The Household Area's diary of Note cards |
| 4 | `play/en-US/images/phoneScreenshots/4-editor.png · play/de-DE/images/phoneScreenshots/4-editor.png` | 1080 × 1920 | Note editor: Importance, place, two dated parts |
| 5 | `play/en-US/images/phoneScreenshots/5-routine.png · play/de-DE/images/phoneScreenshots/5-routine.png` | 1080 × 1920 | Routine editor: every day at 08:00 and 20:00 |
| 6 | `play/en-US/images/phoneScreenshots/6-alarm.png · play/de-DE/images/phoneScreenshots/6-alarm.png` | 1080 × 1920 | Alarm ring screen with Done and Snooze |
| 7 | `play/en-US/images/phoneScreenshots/7-history.png · play/de-DE/images/phoneScreenshots/7-history.png` | 1080 × 1920 | Routine history: Streak and Adherence |
| 8 | `play/en-US/images/phoneScreenshots/8-dark.png · play/de-DE/images/phoneScreenshots/8-dark.png` | 1080 × 1920 | Agenda in the dark theme |

## Play Console, in order

1. Create the app in the Play Console (default language en-US, app, free) if it does not exist yet.
2. Store listing: title, short and full description per language from above, the hi-res icon, the feature graphic, and the eight phone screenshots per language.
3. App content: privacy policy URL from above, then data safety, ads, target audience, the full-screen intent and foreground service declarations as in the handover page.
4. Testing, Internal testing, Create new release: upload the AAB of the tag pipeline by hand once; later tags upload to the internal track on their own.
5. Release notes en-US and de-DE from above, inside the language tags the console shows. Start rollout to Internal testing.
6. Closed testing with at least 12 testers for 14 days, then apply for production access.

# Release notes 0.4.0

Paste source for the Google Play Console: the release's What's new (the tag pipeline reads the `play_notes_en` and `play_notes_de` blocks, and the apps bundle them as the in-app "What's new in 0.4.0" Note), tester notes and store listing. The changes themselves are in [CHANGELOG.md](../CHANGELOG.md).

## Summary

Version 0.4.0 after 0.3.1, which is on Google Play's internal track and on F-Droid. It covers the 0.4.0 section of the changelog: Notes by kind, Packs, History, Code text, Simple or Advanced, Fat fingers, the grouped Agenda and widget, help Notes, feedback, and sync and the Agenda in the background. The store screenshots were regenerated. The listing takes three accepted edits: reappear times and Session in the kinds line (both languages), and Packs under sharing (English only: the German description is at Play's 4000-character limit).

<!-- store-kit
live_play: 0.3.1 (internal)
live_appstore: none
head: 0.4.0
sections: 0.4.0
frames: regenerated
-->

## Play: release notes

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Internal testing release" -->
```
Added
• Notes have kinds: Checklist, Shopping List, Tally, Session.
• Packs hand copies of Notes to other people.
• A Note's History; Code text and code blocks.
• Simple or Advanced; Fat fingers.
• Done right from a Note and its cards.
• Help Notes and Send feedback.
Changed
• The Agenda groups its rows and can hide what is checked.
• The widget shows the Agenda's days.
• Sync and the Agenda load in the background.
Fixed
• A quick swipe opens the Areas at once.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Neu
• Notizen haben Arten: Checkliste, Einkaufsliste, Strichliste, Einheit.
• Pakete geben Kopien von Notizen weiter.
• Verlauf einer Notiz; Code-Text, Codeblöcke.
• Einfach oder Erweitert; Dicke Finger.
• Erledigt direkt in Notiz und Karten.
• Hilfe-Notizen und Feedback senden.
Geändert
• Die Agenda gruppiert Reihen, blendet Abgehaktes auf Wunsch aus.
• Das Widget zeigt die Tage der Agenda.
• Abgleich und Agenda im Hintergrund.
Behoben
• Schnelles Wischen öffnet sofort die Bereiche.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="Play internal testing" -->
```
What to test
• Change a Note's kind with its kind badge; switch a card between compact and detailed with a tap.
• Export as Pack… from an Area, send the file to a second phone, open it there: new Areas named "(Pack)".
• Settings ▸ Look ▸ Fat fingers, and Simple or Advanced: what disappears in Simple, what stays.
• Start the app after a day apart from your other Devices: the moving line shows while it syncs, the screen stays usable.
• The Agenda widget: today, then the coming days; only today's rows and Missed tick.
Known
• Sync and Agenda load were measured on a desktop CPU, not yet on a phone.
• The widget's drawing was checked in tests only, not on every launcher.
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
• A Note can become a Checklist, a Shopping List whose items come back after a set number of days, weeks, months or years, a Tally with entries and charts, or a Session that picks Tallies. Its text can be formatted with headings, lists, tables, code blocks and boxes to tick.

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

## Edits

One per changed listing line. `status` is `pending` until the user answers,
then `accepted` (and applied to the block above) or `rejected` (block left as
live).

<!-- edit block=play_full_en status=accepted -->
Changelog 0.4.0, Changed: a Shopping List Line's reappear time is set from a badge in days, weeks, months or years (#195); Notes have kinds, Session included (#188, #191).
```
• A Note can become a Checklist, a Shopping List whose items come back after a set number of weeks, or a Tally with entries and charts. Its text can be formatted with headings, lists, tables and boxes to tick.
```
```
• A Note can become a Checklist, a Shopping List whose items come back after a set number of days, weeks, months or years, a Tally with entries and charts, or a Session that picks Tallies. Its text can be formatted with headings, lists, tables, code blocks and boxes to tick.
```

<!-- edit block=play_full_de status=accepted -->
Changelog 0.4.0, the same change in German.
```
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen oder Strichliste mit Diagrammen werden. Ihr Text lässt sich mit Überschriften, Listen, Tabellen und Statuskästchen formatieren.
```
```
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen, Tabellen und Statuskästchen formatieren.
```

<!-- edit block=play_full_en status=accepted -->
Changelog 0.4.0, Added: Packs (#164).
```
• A Note can be sent to another person as a file, together with its Routines and Reminders. The receiver can accept or decline it.
```
```
• A Note can be sent to another person as a file, together with its Routines and Reminders. The receiver can accept or decline it.
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.
```

<!-- edit block=play_full_de status=rejected -->
Changelog 0.4.0, the same in German.
```
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden. Die empfangende Person kann sie annehmen oder ablehnen.
```
```
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden. Die empfangende Person kann sie annehmen oder ablehnen.
• Notizen, Themen oder ganze Bereiche lassen sich als Paket mit Kopien weitergeben, die die andere Person als ihre eigenen behält.
```

## Screenshots: Play

Phone 1080 × 1920 per locale; regenerated for this version (Agenda, capture, editor, fan and dark changed). Feature graphic 1024 × 500, unchanged.

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

## Play Console, in order

1. Store listing: apply the accepted edits; replace the phone screenshots with the regenerated frames in both languages.
2. Testing, Internal testing: the tag pipeline uploads the AAB; open the new release.
3. Release notes: en-US and de-DE from above, inside the language tags the console shows.
4. Next, then review, then Start rollout to Internal testing.

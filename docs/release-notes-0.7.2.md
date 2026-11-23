# Release notes 0.7.2

Paste source for the Google Play Console and App Store Connect: the release's What's new (the tag pipeline reads the `play_notes_en` and `play_notes_de` blocks, and the apps bundle them as the in-app "What's new in 0.7.2" Note), tester notes and both store listings. The changes themselves are in [CHANGELOG.md](../CHANGELOG.md).

## Summary

Version 0.7.2. Live: 0.6.1 in both stores, with 0.7.1 described in its own record and on its way through the consoles; every text here describes only the `## [0.7.2]` section of the changelog, against 0.7.1. 0.7.2 is speed and crash fixes: the app answers faster with many Notes, search results follow once typing stops, Imports and moving an Area to the trash run beside the app with a count, and two crashes are gone. The desktop-only changes (the alarms planned on a connection of their own, the window shown before the Agenda is read) stay out of every phone text. The iPhone build with Xcode 27.2 has no changelog line and no text. Frames reused: 0.7.2 changes no screen on the frames; the Area row's count and the Import's count show only while one runs. The listing blocks are the files in `fastlane/metadata/`, word for word; 0.7.2 brings no listing edit.

- [Play Kit](https://claude.ai/artifact/Ds5iewxVwTLoPwAWFbpWiJ)
- [App Store Kit](https://claude.ai/artifact/PvZUte3cRrwyoHg2feLj1T)

<!-- store-kit
live_play: 0.6.1 (0.7.1 in its own record)
live_appstore: 0.6.1 (0.7.1 in its own record)
head: 0.7.2
sections: 0.7.2
frames: reused
-->

## Play: release notes

The same text is the in-app What's new Note on every platform.

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Internal testing release" -->
```
Changed
• The app answers faster when you keep many Notes.
• Search results follow once you stop typing.
• Text read from photos and recordings no longer slows the app.
• The trash and Google Keep imports are faster in a shared Area.
• The app opens faster.
Fixed
• An Import runs beside the app and counts its Notes.
• Moving an Area to the trash no longer freezes the app.
• More on the quick note no longer crashes while a tip shows.
• Restarting after a crash no longer crashes again.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Geändert
• Schneller mit vielen Notizen.
• Suchergebnisse folgen, sobald du aufhörst zu tippen.
• Text aus Fotos und Aufnahmen bremst die App nicht mehr.
• Papierkorb und Keep-Import im geteilten Bereich schneller.
• Die App öffnet schneller.
Behoben
• Ein Import läuft neben der App und zählt seine Notizen.
• Ein Bereich im Papierkorb friert die App nicht mehr ein.
• „Mehr“ in der Schnellnotiz stürzt bei einem Tipp nicht mehr ab.
• Ein Neustart nach einem Absturz stürzt nicht erneut ab.
```

## App Store: What's New

<!-- block id=appstore_whatsnew_en limit=4000 lang=en title="What's New, English" where="Version information" -->
```
The app answers faster when you keep many Notes: lists and Notes open sooner, and the app itself opens faster. Typing in a search field no longer waits; the results follow shortly after you stop typing. Reading the text of photos and voice recordings no longer slows the app down, and moving an Area to the trash and importing from Google Keep are faster in a shared Area.

An Import no longer freezes the app: it runs beside it and counts its Notes, and you can leave its page while it goes on. Moving an Area to the trash no longer freezes the app either; the Area's row counts the Notes moved. More on the quick note opens the Note editor even while a tip shows, instead of crashing, and restarting the app after a crash no longer makes it crash again shortly after.
```

<!-- block id=appstore_whatsnew_de limit=4000 lang=de title="Neue Funktionen, Deutsch" where="Version information" -->
```
Die App reagiert schneller, auch wenn du viele Notizen hast: Listen und Notizen öffnen sich früher, und die App selbst startet schneller. Das Tippen in einem Suchfeld wartet nicht mehr; die Ergebnisse folgen kurz nachdem du aufhörst zu tippen. Das Lesen des Texts von Fotos und Sprachaufnahmen bremst die App nicht mehr, und einen Bereich in den Papierkorb zu legen oder aus Google Keep zu importieren geht in einem geteilten Bereich schneller.

Ein Import friert die App nicht mehr ein: Er läuft neben ihr, zählt seine Notizen, und du kannst seine Seite verlassen, während er weiterläuft. Auch einen Bereich in den Papierkorb zu legen friert die App nicht mehr ein; die Zeile des Bereichs zählt die verschobenen Notizen. „Mehr“ in der Schnellnotiz öffnet den Notiz-Editor auch, während ein Tipp zu sehen ist, statt abzustürzen, und ein Neustart der App nach einem Absturz lässt sie nicht kurz darauf erneut abstürzen.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="TestFlight test information · Play internal track" -->
```
What to test
• With many Notes: open All Notes, an Area and a Note; type in a search field and see the results follow once typing stops.
• Import a folder or a Google Keep export: the app stays usable, the Import counts "N of M Notes", and you can leave the page; the top of the screen says when it is done.
• Move an Area with many Notes to the trash: its row reads "Being deleted: N of M Notes moved" and the app stays usable.
• While a tip shows on the quick note, tap More: the Note editor opens.
• Import photos or voice recordings and keep using the app while their text is read.
Known
• An Import or a move to the trash that is cut off (the app closed meanwhile) keeps what it had done: the Notes imported so far stay in their new Area, the Notes moved stay in the trash. No need to report it.
```

## Play: listing

The live text, the same as in `fastlane/metadata/android/`, verbatim. Changed only through an accepted edit below.

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
• Print to Old Brain Fix from any app; the page comes in as a PDF.
• Each Note records the time of capture. Optional: the location (shown as a Named Place such as "Home" when one matches), the current calendar event and the connected Bluetooth device.
• Voice Notes are transcribed on the device with Whisper (English and German). The speech model (148 MB) is downloaded once, on request.
• Text in photos and screenshots is recognised on the device with RustO, German letters and the euro sign included.
• Transcripts and recognised text are included in search.
• Any file can be attached to a Note; a PDF opens full screen, and its text is searched.

<b>Organising</b>
• New Notes are kept in Unsorted. A notification (Nudge) reports Unsorted Notes at a set interval; Quiet Hours suppress it at night.
• Notes are filed into Areas and Topics (two levels), each with a colour and an icon; tags nest, such as #life/health. A starter set of Areas is offered at first start.
• Notes can be edited, amended and merged. Deleted items stay in the trash for 30 days.
• A Note can become a Checklist, a Shopping List whose items come back after a set number of days, weeks, months or years, a Tally with entries and charts, or a Session that picks Tallies. Its text can be formatted with headings, lists, tables, code blocks, boxes to tick and links that open other Notes and places in the app.

<b>Routines and Reminders</b>
• Any Note can have Reminders (one time) and Routines (repeating by a recurrence rule, several times a day if needed). Heads-ups can warn ahead of time.
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
• The app uses the network only for sync, for questions to your own desktop and for the speech model download.
• Optional App Lock (PIN, fingerprint) and Area Locks. Capture and alarms keep working while locked.
• Encrypted Backups (Argon2id, XChaCha20-Poly1305) into a chosen folder, optionally weekly, with a printable Recovery Sheet. Optionally, Android's backup to Google stores the encrypted Backup file and nothing else.

<b>Other</b>
• English and German. Light and dark theme. Text in three sizes.
• Free and open source, licensed under Apache-2.0 or MIT.
```

<!-- block id=play_full_de limit=4000 lang=de title="Vollständige Beschreibung, de-DE" where="Main store listing · b i u br count" -->
```
Old Brain Fix ist eine Notiz- und Erinnerungs-App für Android. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.

<b>Erfassen</b>
• Text-, Foto- und Sprachnotizen, geöffnet über den +-Knopf, Widgets, Schnelleinstellungen, App-Verknüpfungen oder eine Hardwaretaste, wo das Telefon es erlaubt.
• Aus anderen Apps Geteiltes wird als Notiz gespeichert.
• Aus jeder App an Old Brain Fix drucken; die Seite kommt als PDF an.
• Jede Notiz speichert den Zeitpunkt der Erfassung. Optional: den Ort (als benannter Ort wie „Zuhause“, wenn einer passt), den laufenden Kalendertermin und das verbundene Bluetooth-Gerät.
• Sprachnotizen werden auf dem Gerät mit Whisper transkribiert (Deutsch und Englisch). Das Sprachmodell (148 MB) wird einmalig auf Wunsch geladen.
• Text in Fotos und Screenshots wird auf dem Gerät mit RustO erkannt, auch Umlaute, ß und €.
• Die Suche findet auch Transkripte und erkannten Text.
• Jede Datei lässt sich anhängen; ein PDF öffnet im Vollbild, sein Text wird durchsucht.

<b>Ordnen</b>
• Neue Notizen liegen in „Unsortiert“. Eine Benachrichtigung (Stupser) meldet sie in einstellbarem Abstand; Ruhezeiten unterdrücken sie nachts.
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol; #Wörter wie #leben/gesundheit lassen sich verschachteln. Beim ersten Start gibt es eine Auswahl an Bereichen.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen, Tabellen und Statuskästchen formatieren.

<b>Routinen und Erinnerungen</b>
• Jede Notiz kann Erinnerungen (einmalig) und Routinen (wiederkehrend nach einer Wiederholungsregel, bei Bedarf mehrmals täglich) haben. Auf Wunsch mit Vorwarnungen.
• Alarme klingeln minutengenau, auch auf dem Sperrbildschirm, und werden nach einem Neustart wiederhergestellt. Dafür ist die Berechtigung „Wecker und Erinnerungen“ nötig.
• Ein Alarm, der weder erledigt noch verschoben wird, klingelt in festem Abstand erneut (Erneut erinnern, Standard 10 Minuten), optional nach einer einstellbaren Zahl von Wiederholungen als Vollbild-Alarm mit Ton.
• Die Agenda zeigt die Routinen und Erinnerungen des Tages, verpasste Termine und die Zahl der unsortierten Notizen. Sie gibt es auch als Widget.
• Jede Routine hat einen Verlauf mit Serie und Einhaltung.
• Routinen und Erinnerungen können in einen Gerätekalender geschrieben oder als ICS-Datei exportiert werden.

<b>Abgleich und Teilen</b>
• Telefon und Desktop gleichen sich direkt im lokalen Netz ab (TLS, gekoppelt per QR-Code oder eingetippten Code), über einen geteilten Ordner wie Syncthing oder Nextcloud (Ende-zu-Ende-verschlüsselt) oder über eine verschlüsselte Datei, die eine beliebige App verschickt.
• Ein einzelner Bereich lässt sich mit einer anderen Person teilen, andere nicht.
• Dein Profil vereint alle deine Geräte. Name, eindeutiger Code und optionales Bild stehen an den Notizen, die du mit anderen teilst.
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.

<b>Datenschutz und Sicherheit</b>
• Kein Konto, kein Server, keine Analyse, keine Werbung.
• Die App nutzt das Netzwerk nur für den Abgleich, für Fragen an den eigenen Desktop und den Download des Sprachmodells.
• Optionale App-Sperre (PIN, Fingerabdruck) und Bereichssperren. Erfassen und Alarme gehen auch gesperrt.
• Verschlüsselte Sicherungen (Argon2id, XChaCha20-Poly1305) in einen gewählten Ordner, optional wöchentlich, mit einem druckbaren Wiederherstellungsblatt. Die Android-Sicherung bei Google speichert optional nur die verschlüsselte Datei.

<b>Sonstiges</b>
• Deutsch und Englisch. Helles und dunkles Design. Text in drei Größen.
• Kostenlos und quelloffen (Apache-2.0 oder MIT).
```

<!-- block id=play_privacy_url limit=255 lang=en title="Privacy policy URL" where="Policy and programs · App content · Privacy policy" -->
```
https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
```

## App Store: listing

The text in `fastlane/metadata/ios/`, verbatim; the App Store still shows 0.5.1's description, without the lines 0.6.0 and 0.6.1 added to these files. Changed only through an accepted edit below.

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
• Any file can be attached to a Note; a PDF opens full screen, and its text is searched.

Organising
• New Notes are kept in Unsorted. A notification (Nudge) reports Unsorted Notes at a set interval; Quiet Hours suppress it at night.
• Notes are filed into Areas and Topics (two levels), each with a colour and an icon; tags nest, such as #life/health. A starter set of Areas is offered at first start.
• Notes can be edited, amended and merged. Deleted items stay in the trash for 30 days.
• A Note can become a Checklist, a Shopping List whose items come back, a Tally or a Session. Its text can be formatted with headings, lists, tables, boxes to tick and links that open other Notes and places in the app.

Routines and Reminders
• Any Note can have Reminders (one time) and Routines (repeating by a recurrence rule, several times a day if needed). Heads-ups can warn ahead of time.
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
• The app uses the network only for sync, for questions to your own desktop and for the speech model download.
• Optional App Lock (PIN, Face ID or Touch ID) and Area Locks. Capture and alarms keep working while locked.
• Encrypted Backups (Argon2id, XChaCha20-Poly1305) into a chosen folder, optionally weekly, with a printable Recovery Sheet. The iPhone's own backup leaves the app's data out.

Other
• English and German. Light and dark theme. Text in three sizes. iOS 26 or newer.
• Free and open source, licensed under Apache-2.0 or MIT.
```

<!-- block id=appstore_desc_de limit=4000 lang=de title="Beschreibung, Deutsch" where="Plain text" -->
```
Old Brain Fix ist eine Notiz- und Erinnerungs-App für das iPhone. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.

Erfassen
• Text-, Foto- und Sprachnotizen, geöffnet über den +-Knopf, das Widget „Erfassen“ auf Start- und Sperrbildschirm oder die Steuerelemente im Kontrollzentrum für Sprache, Text und Foto.
• Aus anderen Apps geteilte Texte, Links und Bilder öffnen sich beim nächsten Start der App im Erfassen.
• Jede Notiz speichert den Zeitpunkt der Erfassung. Optional: den Ort (als benannter Ort wie „Zuhause“, wenn einer passt), den laufenden Kalendertermin und das Auto oder die Kopfhörer, über die der Ton läuft.
• Sprachnotizen werden auf dem Gerät mit Whisper transkribiert (Deutsch und Englisch). Das Sprachmodell (148 MB) wird einmalig auf Wunsch geladen.
• Text in Fotos und Screenshots wird auf dem Gerät mit RustO erkannt.
• Die Suche findet auch Transkripte und erkannten Text.
• Jede Datei lässt sich anhängen; ein PDF öffnet im Vollbild, sein Text wird durchsucht.

Ordnen
• Neue Notizen liegen in „Unsortiert“. Eine Benachrichtigung (Stupser) meldet sie in einstellbarem Abstand; die Ruhezeit unterdrückt sie nachts.
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol; #Wörter wie #leben/gesundheit lassen sich verschachteln. Beim ersten Start gibt es eine Auswahl an Bereichen.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen und Tabellen formatieren.

Routinen und Erinnerungen
• Jede Notiz kann Erinnerungen (einmalig) und Routinen (wiederkehrend nach einer Wiederholungsregel, bei Bedarf mehrmals täglich) haben. Auf Wunsch mit Vorwarnungen.
• Sie kommen als zeitkritische Benachrichtigungen mit Erledigt und Später.
• Eine Benachrichtigung, die weder erledigt noch verschoben wird, kommt in festem Abstand wieder (Erneut erinnern, Standard 10 Minuten), optional klingelt sie nach einer einstellbaren Zahl von Wiederholungen als iPhone-Alarm, auch im Stummmodus und im Fokus.
• Die Agenda zeigt die Routinen und Erinnerungen des Tages, verpasste Termine und die Zahl der unsortierten Notizen. Sie ist auch ein Widget, dessen Kästchen sich ohne die App abhaken lassen.
• Jede Routine hat einen Verlauf mit Serie und Einhaltung.
• Die Agenda kann die Termine gewählter Kalender zeigen. Routinen und Erinnerungen können in einen Kalender geschrieben oder als ICS-Datei exportiert werden.

Abgleich und Teilen
• Deine Geräte gleichen sich direkt im lokalen Netz ab, solange die App offen ist (TLS, gekoppelt per QR-Code oder eingetippten Code), über einen geteilten Ordner in iCloud Drive oder Möbius Sync (Ende-zu-Ende-verschlüsselt) oder über eine verschlüsselte Datei, die eine beliebige App verschickt.
• Ein einzelner Bereich lässt sich mit einer anderen Person teilen, andere nicht.
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.
• Notizen, Themen oder ganze Bereiche lassen sich als Paket mit Kopien an andere weitergeben, die sie als ihre eigenen behalten.

Datenschutz und Sicherheit
• Kein Konto, kein Server, keine Analyse, keine Werbung.
• Die App nutzt das Netzwerk nur für den Abgleich, für Fragen an den eigenen Desktop und den Download des Sprachmodells.
• Optionale App-Sperre (PIN, Face ID oder Touch ID) und Bereichssperren. Erfassen und Alarme gehen auch gesperrt.
• Verschlüsselte Sicherungen (Argon2id, XChaCha20-Poly1305) in einen gewählten Ordner, optional wöchentlich, mit einem druckbaren Wiederherstellungsblatt. Die iPhone-Datensicherung lässt die Daten der App aus.

Sonstiges
• Deutsch und Englisch. Helles und dunkles Design. Text in drei Größen. Ab iOS 26.
• Kostenlos und quelloffen (Apache-2.0 oder MIT).
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

None: 0.7.2 makes no line of either listing untrue and brings no capability the listing would name.

## Screenshots: Play

Phone 1080 × 1920 per locale, reused: regenerated for 0.7.0 and current at HEAD, as neither 0.7.1 nor 0.7.2 changes a screen on them. Against the 0.6.1 set frames 1, 2, 3, 4, 5 and 8 differ and replace it, if 0.7.1 has not done so already; 6 and 7 are unchanged and stay. Feature graphic 1024 × 500, unchanged.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `play/en-US/images/phoneScreenshots/1-agenda.png · play/de-DE/images/phoneScreenshots/1-agenda.png` | 1080 × 1920 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder and the Quiet today bell in the bar |
| 2 | `play/en-US/images/phoneScreenshots/2-capture.png · play/de-DE/images/phoneScreenshots/2-capture.png` | 1080 × 1920 | The quick note: its text at the top, "Remind me to sort it in" 10 min, 30 min, 1 h, and More |
| 3 | `play/en-US/images/phoneScreenshots/3-diary.png · play/de-DE/images/phoneScreenshots/3-diary.png` | 1080 × 1920 | An Area's diary of Note cards in full, the search field on its own band, added parts under tear lines, Select in the bar and the pencil on each card |
| 4 | `play/en-US/images/phoneScreenshots/4-editor.png · play/de-DE/images/phoneScreenshots/4-editor.png` | 1080 × 1920 | Note editor with the Kind badge first ("Plain Note ▾"), Hidden from AI among the badges, the camera in the bar, and added parts under tear lines |
| 5 | `play/en-US/images/phoneScreenshots/5-fan.png · play/de-DE/images/phoneScreenshots/5-fan.png` | 1080 × 1920 | The Agenda's "+" fanned out: Text, Voice, Photo, Gallery, Paste, File |
| 6 | `play/en-US/images/phoneScreenshots/6-alarm.png · play/de-DE/images/phoneScreenshots/6-alarm.png` | 1080 × 1920 | Alarm ring screen with Done and Snooze |
| 7 | `play/en-US/images/phoneScreenshots/7-history.png · play/de-DE/images/phoneScreenshots/7-history.png` | 1080 × 1920 | Routine history with Streak and Adherence |
| 8 | `play/en-US/images/phoneScreenshots/8-dark.png · play/de-DE/images/phoneScreenshots/8-dark.png` | 1080 × 1920 | Agenda in the dark theme, scrolled to the rest of the week, the Quiet today bell in the bar |

## Screenshots: App Store

iPhone 6.9″ 1320 × 2868 per locale, the one size App Store Connect scales to every iPhone; no iPad (iPhone only). Reused: regenerated for 0.7.0 and current at HEAD, as neither 0.7.1 nor 0.7.2 changes a screen on them. Against the set uploaded for 0.6.1 frames 1, 2, 3, 4, 5 and 8 differ and replace it, if 0.7.1 has not done so already; 6 and 7 are unchanged and stay.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/1-agenda.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/1-agenda.png` | 1320 × 2868 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder and the Quiet today bell in the bar |
| 2 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/2-capture.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/2-capture.png` | 1320 × 2868 | The quick note: the back arrow, its text at the top, "Remind me to sort it in" 10 min, 30 min, 1 h, and More |
| 3 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/3-diary.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/3-diary.png` | 1320 × 2868 | An Area's diary of Note cards in full, the search field on its own band, added parts under tear lines, Select in the bar and the pencil on each card, with the Cupertino back arrow |
| 4 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/4-editor.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/4-editor.png` | 1320 × 2868 | Note editor with the Kind badge first ("Plain Note ▾"), Hidden from AI among the badges, the camera and the microphone in the bar, and added parts under tear lines, with the Cupertino back arrow |
| 5 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/5-fan.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/5-fan.png` | 1320 × 2868 | The Agenda's "+" fanned out: Text, Voice, Photo, Gallery, Paste, File |
| 6 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/6-routines.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/6-routines.png` | 1320 × 2868 | All Routines and Reminders, with the Cupertino back arrow |
| 7 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/7-history.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/7-history.png` | 1320 × 2868 | Routine history with Streak and Adherence, with the Cupertino back arrow |
| 8 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/8-dark.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/8-dark.png` | 1320 × 2868 | Agenda in the dark theme, scrolled to the rest of the week, the Quiet today bell in the bar |

## Play Console, in order

1. Store listing: replace the phone screenshots with frames 1 to 8 above, and apply the accepted edits, if any.
2. Testing, Internal testing: the tag pipeline uploads the AAB; open the new release.
3. Release notes: en-US and de-DE from above, inside the language tags the console shows.
4. Next, then review, then Start rollout to Internal testing.

## App Store Connect, in order

1. The tag pipeline uploads the build to TestFlight; try it, and the tester notes above go into Test Information.
2. Write the What's New above into `fastlane/metadata/ios/en-US/release_notes.txt` and `fastlane/metadata/ios/de-DE/release_notes.txt`, and the accepted listing edits into the description files.
3. Replace frames 1, 2, 3, 4, 5 and 8 by hand in App Store Connect, if 0.7.1 has not done so; `deliver` skips the screenshots.
4. Submit with Actions, Release, Run workflow: the tag, publish on, submit_app_store on. `fastlane deliver` uploads the listing and What's New from `fastlane/metadata/ios/`, and submits the build that run uploads.
5. After approval, release the version by hand.

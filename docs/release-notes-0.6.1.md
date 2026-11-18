# Release notes 0.6.1

Paste source for the Google Play Console and App Store Connect: the release's What's new (the tag pipeline reads the `play_notes_en` and `play_notes_de` blocks, and the apps bundle them as the in-app "What's new in 0.6.1" Note), tester notes and both store listings. The changes themselves are in [CHANGELOG.md](../CHANGELOG.md) and [OLDER_CHANGES.md](../OLDER_CHANGES.md).

## Summary

Version 0.6.1. Live: 0.5.1 on the App Store; on Google Play only internal testing, where 0.6.0 went but was unusable, so it counts as never released and 0.5.0 is the last one used; 0.6.0 on F-Droid and in the GitHub Release. Every text here, the Play notes and in-app Note, the App Store's What's New and the tester notes, describes the 0.6.0 and 0.6.1 sections of the changelog as one change against 0.5.x: where 0.6.1 reworks a 0.6.0 feature, the text gives its final behaviour (Ask on the phone, which 0.6.0 brought, is experimental and off until switched on). The desktop-only changes stay out of every phone text, and Print to Old Brain Fix, which only Android has, stays out of the App Store texts; the Play notes' 500 characters leave it out too, as the listing names it. Frames regenerated: only frame 3 changed, in a band of rows by the search field (Play rows 250–269, just above it; iPhone rows 662–677, at its lower edge); its cards are pixel-identical.

The listing blocks are the files in `fastlane/metadata/`, which carry 0.6.0's accepted edits; those never reached a console, and the App Store still shows 0.5.1's description. On the App Store, the submitting dispatch's `fastlane deliver` uploads the whole listing from `fastlane/metadata/ios/`, so 0.6.0's edits arrive with it, and What's New from `release_notes.txt`, which stays empty until the `appstore_whatsnew_*` blocks are written into it; it skips the screenshots, which are uploaded by hand.

<!-- store-kit
live_play: 0.5.0 (internal; 0.6.0 internal only, unusable)
live_appstore: 0.5.1
head: 0.6.1
sections: 0.6.0, 0.6.1
frames: regenerated
-->

## Play: release notes

The same text is the in-app What's new Note on every platform.

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Internal testing release" -->
```
Added
• Capture makes a Checklist or Shopping List, or adds to a Note.
• A spoken shopping list becomes Lines of a Shopping List.
• Attach any file to a Note; a PDF opens full screen.
• Snooze and Not today in more places.
• Select several Notes and act on them at once.
• Text size makes text larger.
• Ask your desktop about your Notes; switch it on in Settings.
Changed
• Deleting no longer asks; Undo brings it back.
• A tap on an Occurrence opens its Note.
Fixed
• A tapped box ticks at once.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Neu
• Erfassen macht Checklisten und Einkaufslisten und ergänzt Notizen.
• Eine gesprochene Einkaufsliste wird zu Zeilen.
• Jede Datei anhängen; PDFs im Vollbild.
• Später und Nicht heute an mehr Stellen.
• Mehrere Notizen auf einmal bearbeiten.
• Textgröße macht Text größer.
• Desktop nach Notizen fragen; in den Einstellungen einschalten.
Geändert
• Löschen fragt nicht; Rückgängig holt es zurück.
• Ein Tipp auf ein Vorkommen öffnet seine Notiz.
Behoben
• Kästchen sind sofort abgehakt.
```

## App Store: What's New

<!-- block id=appstore_whatsnew_en limit=4000 lang=en title="What's New, English" where="Version information" -->
```
Capture offers Text, Checklist and Shopping List above the field, so typed lines become a list at once. A spoken shopping list goes into a Shopping List as Lines, also without the speech model. Add to Note… adds what you captured to a Note you pick.

Any file can be attached to a Note, from Capture, the Note editor or another app. A PDF opens full screen, and the text of a PDF or an Office file is found by the search.

Snooze and Not today are in the notification, in the Agenda row's menu and at the top of the Note. Not today offers Skip and Tomorrow. A tap on an Occurrence opens its Note. A one-time Reminder takes words such as "tomorrow 7 pm", and buttons set In 1 h, Tonight, Tomorrow or In a week with one tap.

Every Note list has Select, to file, merge or move several Notes to the Trash at once. Deleting no longer asks first; Undo at the top brings it back. Once a Note from Unsorted is sorted, the next one opens.

Text size in the Settings makes text larger. Cards start with their text, formatted text reads calmer, and a Note's added parts continue its text under a dashed tear line. A Shopping List's struck Lines show when each comes back. Tags nest, such as #life/health, the Areas keep the order you drag them into, and Settings has a search field. About Old Brain Fix is behind the Profile picture. Open a Backup… brings Notes back from a Backup file as copies.

Ask is experimental and off until Ask (experimental) is switched on in the Settings. It asks a paired desktop of yours, with AI (experimental) switched on there and an AI provider set up, about your Notes; the answer links the Notes it came from and can be kept as a Note, copied, added to a Note or shared. A Note or an Area can be kept Hidden from AI.

Sending a Note to a person no longer picks the first person in the list. A tap in the Note editor puts the cursor where the finger is, a tapped box ticks at once, and the app stays responsive while it syncs or transcribes a recording. The app's SQLite is updated for a security fix.
```

<!-- block id=appstore_whatsnew_de limit=4000 lang=de title="Neue Funktionen, Deutsch" where="Version information" -->
```
Die Schnellnotiz bietet Text, Checkliste und Einkaufsliste über dem Feld, so werden getippte Zeilen gleich eine Liste. Eine gesprochene Einkaufsliste kommt als Zeilen in eine Einkaufsliste, auch ohne Sprachmodell. „Zu einer Notiz hinzufügen …“ fügt das Erfasste einer gewählten Notiz hinzu.

Jede Datei lässt sich an eine Notiz anhängen, aus dem Erfassen, dem Notiz-Editor oder einer anderen App. Ein PDF öffnet im Vollbild, und der Text eines PDFs oder einer Office-Datei wird von der Suche gefunden.

Später und Nicht heute gibt es in der Benachrichtigung, im Menü der Agenda-Zeile und oben in der Notiz. Nicht heute bietet Auslassen und Morgen. Ein Tipp auf ein Vorkommen öffnet seine Notiz. Eine einmalige Erinnerung versteht Wörter wie „morgen 19 Uhr“, und Knöpfe setzen In 1 Std., Heute Abend, Morgen oder In einer Woche mit einem Tipp.

Jede Notizliste hat Auswählen, um mehrere Notizen auf einmal abzulegen, zusammenzuführen oder in den Papierkorb zu legen. Löschen fragt nicht mehr vorher; Rückgängig oben holt es zurück. Ist eine Notiz aus „Unsortiert“ einsortiert, öffnet sich die nächste.

„Textgröße“ in den Einstellungen macht Text größer. Karten beginnen mit ihrem Text, formatierter Text liest sich ruhiger, und die Abschnitte einer Notiz setzen ihren Text unter einer gestrichelten Abrisslinie fort. Die durchgestrichenen Zeilen einer Einkaufsliste zeigen, wann jede wiederkommt. #Wörter lassen sich verschachteln, etwa #leben/gesundheit, die Bereiche behalten die Reihenfolge, in die du sie ziehst, und die Einstellungen haben ein Suchfeld. „Über Old Brain Fix“ steht hinter dem Profilbild. Sicherung öffnen … holt Notizen aus einer Sicherungsdatei als Kopien zurück.

Fragen ist experimentell und aus, bis „Fragen (experimentell)“ in den Einstellungen eingeschaltet ist. Es fragt einen gekoppelten Desktop von dir, auf dem KI (experimentell) eingeschaltet und ein KI-Anbieter eingerichtet ist, nach deinen Notizen; die Antwort verlinkt die Notizen, aus denen sie stammt, und lässt sich als Notiz behalten, kopieren, einer Notiz hinzufügen oder teilen. Eine Notiz oder ein Bereich lässt sich Vor KI verborgen halten.

Das Senden einer Notiz an eine Person wählt nicht mehr die erste Person der Liste. Ein Tippen im Notiz-Editor setzt den Cursor dorthin, wo der Finger ist, ein angetipptes Kästchen ist sofort abgehakt, und die App reagiert auch beim Abgleich und während sie eine Aufnahme transkribiert. Das SQLite der App ist für eine Sicherheitskorrektur aktualisiert.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="TestFlight test information · Play internal track" -->
```
What to test
• Capture: pick Checklist or Shopping List above the field, type three lines, leave, and find them as Lines. Record "milk, bread and eggs", choose Add to a Shopping List, and see the items arrive as Lines once the recording is transcribed (by the speech model on this or another Device).
• Capture a photo or a file and use Add to Note… to add it to a Note there is. Attach a PDF and an Office file to a Note; open the PDF, swipe, zoom, switch to its text. Android: turn on Print to Old Brain Fix in Settings and print a web page to it.
• Answer a Routine with Snooze, Not today ▸ Skip and Not today ▸ Tomorrow, from the notification and from the Agenda row's ⋮; tap a notification and see its Note open at once.
• Settings ▸ Text size: Larger and Largest on the Agenda, a Note, a Checklist and an Ask answer.
• Settings ▸ Ask (experimental) on, with a desktop paired that has AI (experimental) switched on and an AI provider set up: ask, open a linked Note and one under Also read, Edit a question, use Copy text and Add to Note… on an answer.
Known
• Asking needs a desktop of yours that is reachable, has AI (experimental) switched on and a provider set up.
• A Transcript takes about twice as long as before; the app stays usable meanwhile.
• Android may open Capture for a print only the next time the app opens.
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
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol; #Wörter wie #leben/gesundheit lassen sich verschachteln. Beim ersten Start wird eine Auswahl an Bereichen angeboten.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen, Tabellen und Statuskästchen formatieren.

<b>Routinen und Erinnerungen</b>
• Jede Notiz kann Erinnerungen (einmalig) und Routinen (wiederkehrend nach einer Wiederholungsregel, bei Bedarf mehrmals täglich) haben.
• Alarme klingeln minutengenau, auch auf dem Sperrbildschirm, und werden nach einem Neustart wiederhergestellt. Dafür ist die Berechtigung „Wecker und Erinnerungen“ nötig.
• Ein Alarm, der weder erledigt noch verschoben wird, klingelt in festem Abstand erneut (Erneut erinnern, Standard 10 Minuten), optional nach einer einstellbaren Zahl von Wiederholungen als Vollbild-Alarm mit Ton.
• Die Agenda zeigt die Routinen und Erinnerungen des Tages, verpasste Termine und die Zahl der unsortierten Notizen. Sie gibt es auch als Widget.
• Jede Routine hat einen Verlauf mit Serie und Einhaltung.
• Routinen und Erinnerungen können in einen Gerätekalender geschrieben oder als ICS-Datei exportiert werden.

<b>Abgleich und Teilen</b>
• Telefon und Desktop gleichen sich direkt im lokalen Netz ab (TLS, gekoppelt per QR-Code oder eingetippten Code), über einen geteilten Ordner wie Syncthing oder Nextcloud (Ende-zu-Ende-verschlüsselt) oder über eine verschlüsselte Datei, die eine beliebige App verschickt.
• Ein einzelner Bereich lässt sich mit einer anderen Person teilen; andere Bereiche nicht.
• Dein Profil vereint alle deine Geräte. Name, eindeutiger Code und optionales Bild stehen an den Notizen, die du mit anderen teilst.
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.

<b>Datenschutz und Sicherheit</b>
• Kein Konto, kein Server, keine Analyse, keine Werbung.
• Die App nutzt das Netzwerk nur für den Abgleich, für Fragen an den eigenen Desktop und den Download des Sprachmodells.
• Optionale App-Sperre (PIN, Fingerabdruck) und Bereichssperren. Erfassen und Alarme gehen auch gesperrt.
• Verschlüsselte Sicherungen (Argon2id, XChaCha20-Poly1305) in einen gewählten Ordner, optional wöchentlich, mit einem druckbaren Wiederherstellungsblatt. Die Android-Sicherung bei Google speichert optional nur die verschlüsselte Datei.

<b>Sonstiges</b>
• Deutsch und Englisch. Helles und dunkles Design. Text in drei Größen.
• Kostenlos und quelloffen, lizenziert unter Apache-2.0 oder MIT.
```

<!-- block id=play_privacy_url limit=255 lang=en title="Privacy policy URL" where="Policy and programs · App content · Privacy policy" -->
```
https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
```

## App Store: listing

The text in `fastlane/metadata/ios/`, verbatim; the App Store still shows 0.5.1's description, without 0.6.0's accepted edits. Changed only through an accepted edit below.

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
• Notizen werden in Bereiche und Themen (zwei Ebenen) einsortiert, jeweils mit Farbe und Symbol; #Wörter wie #leben/gesundheit lassen sich verschachteln. Beim ersten Start wird eine Auswahl an Bereichen angeboten.
• Notizen lassen sich bearbeiten, ergänzen und zusammenführen. Gelöschtes bleibt 30 Tage im Papierkorb.
• Eine Notiz kann Checkliste, Einkaufsliste mit wiederkehrenden Zeilen, Strichliste oder Einheit werden. Ihr Text lässt sich mit Überschriften, Listen und Tabellen formatieren.

Routinen und Erinnerungen
• Jede Notiz kann Erinnerungen (einmalig) und Routinen (wiederkehrend nach einer Wiederholungsregel, bei Bedarf mehrmals täglich) haben.
• Sie kommen als zeitkritische Benachrichtigungen mit Erledigt und Später.
• Eine Benachrichtigung, die weder erledigt noch verschoben wird, kommt in festem Abstand wieder (Erneut erinnern, Standard 10 Minuten), optional klingelt sie nach einer einstellbaren Zahl von Wiederholungen als iPhone-Alarm, auch im Stummmodus und im Fokus.
• Die Agenda zeigt die Routinen und Erinnerungen des Tages, verpasste Termine und die Zahl der unsortierten Notizen. Sie ist auch ein Widget, dessen Kästchen sich ohne die App abhaken lassen.
• Jede Routine hat einen Verlauf mit Serie und Einhaltung.
• Die Agenda kann die Termine gewählter Kalender zeigen. Routinen und Erinnerungen können in einen Kalender geschrieben oder als ICS-Datei exportiert werden.

Abgleich und Teilen
• Deine Geräte gleichen sich direkt im lokalen Netz ab, solange die App offen ist (TLS, gekoppelt per QR-Code oder eingetippten Code), über einen geteilten Ordner in iCloud Drive oder Möbius Sync (Ende-zu-Ende-verschlüsselt) oder über eine verschlüsselte Datei, die eine beliebige App verschickt.
• Ein einzelner Bereich lässt sich mit einer anderen Person teilen; andere Bereiche nicht.
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.
• Notizen, Themen oder ganze Bereiche lassen sich als Paket mit Kopien an andere weitergeben, die sie als ihre eigenen behalten.

Datenschutz und Sicherheit
• Kein Konto, kein Server, keine Analyse, keine Werbung.
• Die App nutzt das Netzwerk nur für den Abgleich, für Fragen an den eigenen Desktop und den Download des Sprachmodells.
• Optionale App-Sperre (PIN, Face ID oder Touch ID) und Bereichssperren. Erfassen und Alarme gehen auch gesperrt.
• Verschlüsselte Sicherungen (Argon2id, XChaCha20-Poly1305) in einen gewählten Ordner, optional wöchentlich, mit einem druckbaren Wiederherstellungsblatt. Die iPhone-Datensicherung lässt die Daten der App aus.

Sonstiges
• Deutsch und Englisch. Helles und dunkles Design. Text in drei Größen. Ab iOS 26.
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

One per changed listing line; all accepted by the user and applied to their blocks and the fastlane files. The two German shortenings make room for the Text size line: without them `play_full_de` would have 4019 characters and `appstore_desc_de` 4012.

<!-- edit block=play_full_en status=accepted -->
Changelog 0.6.1, Added: "Text size makes the text of Notes, Lines, the Agenda and Ask's answers larger in steps …" (3936 → 3957 characters)
```
• English and German. Light and dark theme.
```
```
• English and German. Light and dark theme. Text in three sizes.
```

<!-- edit block=play_full_de status=accepted -->
Changelog 0.6.1, Added: "Text size makes the text of Notes, Lines, the Agenda and Ask's answers larger in steps …" (with the two shortenings below: 3998 → 3999 characters)
```
• Deutsch und Englisch. Helles und dunkles Design.
```
```
• Deutsch und Englisch. Helles und dunkles Design. Text in drei Größen.
```

<!-- edit block=play_full_de status=accepted -->
Shortening to make room for the Text size line, same content.
```
• Transkripte und erkannter Text werden bei der Suche berücksichtigt.
```
```
• Die Suche findet auch Transkripte und erkannten Text.
```

<!-- edit block=play_full_de status=accepted -->
Shortening to make room for the Text size line, same content.
```
Sie ist auch als Widget verfügbar.
```
```
Sie gibt es auch als Widget.
```

<!-- edit block=appstore_desc_en status=accepted -->
Changelog 0.6.1, Added: "Text size makes the text of Notes, Lines, the Agenda and Ask's answers larger in steps …" (3612 → 3633 characters)
```
• English and German. Light and dark theme. iOS 26 or newer.
```
```
• English and German. Light and dark theme. Text in three sizes. iOS 26 or newer.
```

<!-- edit block=appstore_desc_de status=accepted -->
Changelog 0.6.1, Added: "Text size makes the text of Notes, Lines, the Agenda and Ask's answers larger in steps …" (with the shortening below: 3991 → 3998 characters)
```
• Deutsch und Englisch. Helles und dunkles Design. Ab iOS 26.
```
```
• Deutsch und Englisch. Helles und dunkles Design. Text in drei Größen. Ab iOS 26.
```

<!-- edit block=appstore_desc_de status=accepted -->
Shortening to make room for the Text size line, same content.
```
• Transkripte und erkannter Text werden bei der Suche berücksichtigt.
```
```
• Die Suche findet auch Transkripte und erkannten Text.
```

## Screenshots: Play

Phone 1080 × 1920 per locale, regenerated at HEAD. Against the set the Play listing carries since 0.5.x, frames 1, 2, 3, 4, 5 and 8 differ and replace it; 6 and 7 are unchanged and stay. Against the 0.6.0 set F-Droid has, 2, 3 and 4 differ, and this run changed only frame 3, in rows 250–269 just above the search field; its cards are pixel-identical. Feature graphic 1024 × 500, unchanged.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `play/en-US/images/phoneScreenshots/1-agenda.png · play/de-DE/images/phoneScreenshots/1-agenda.png` | 1080 × 1920 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder in the bar |
| 2 | `play/en-US/images/phoneScreenshots/2-capture.png · play/de-DE/images/phoneScreenshots/2-capture.png` | 1080 × 1920 | The quick note with Text, Checklist and Shopping List above the field, Photo, Voice, Gallery, Paste, File, "To full editor", Add to Note…, the alarm clock and the Nudge buttons |
| 3 | `play/en-US/images/phoneScreenshots/3-diary.png · play/de-DE/images/phoneScreenshots/3-diary.png` | 1080 × 1920 | An Area's diary of Note cards in full, each starting with its text, added parts under tear lines, Select in the bar and the pencil on each card |
| 4 | `play/en-US/images/phoneScreenshots/4-editor.png · play/de-DE/images/phoneScreenshots/4-editor.png` | 1080 × 1920 | Note editor with the Kind badge first ("Plain Note ▾"), Hidden from AI among the badges, Attach a file in the bar, and added parts under tear lines |
| 5 | `play/en-US/images/phoneScreenshots/5-fan.png · play/de-DE/images/phoneScreenshots/5-fan.png` | 1080 × 1920 | The Agenda's "+" fanned out: Text, Voice, Photo, Gallery, Paste, File |
| 6 | `play/en-US/images/phoneScreenshots/6-alarm.png · play/de-DE/images/phoneScreenshots/6-alarm.png` | 1080 × 1920 | Alarm ring screen with Done and Snooze |
| 7 | `play/en-US/images/phoneScreenshots/7-history.png · play/de-DE/images/phoneScreenshots/7-history.png` | 1080 × 1920 | Routine history with Streak and Adherence |
| 8 | `play/en-US/images/phoneScreenshots/8-dark.png · play/de-DE/images/phoneScreenshots/8-dark.png` | 1080 × 1920 | Agenda in the dark theme, scrolled to the rest of the week |

## Screenshots: App Store

iPhone 6.9″ 1320 × 2868 per locale, the one size App Store Connect scales to every iPhone; no iPad (iPhone only). Regenerated at HEAD. Against the set uploaded for 0.5.1, frames 1, 2, 3, 4, 5 and 8 differ and replace it; 6 and 7 are unchanged and stay.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/1-agenda.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/1-agenda.png` | 1320 × 2868 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder in the bar |
| 2 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/2-capture.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/2-capture.png` | 1320 × 2868 | The quick note with Text, Checklist and Shopping List above the field, Photo, Voice, Gallery, Paste, File, "To full editor", Add to Note…, the alarm clock and the Nudge buttons |
| 3 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/3-diary.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/3-diary.png` | 1320 × 2868 | An Area's diary of Note cards in full, each starting with its text, added parts under tear lines, Select in the bar and the pencil on each card, with the Cupertino back arrow |
| 4 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/4-editor.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/4-editor.png` | 1320 × 2868 | Note editor with the Kind badge first ("Plain Note ▾"), Hidden from AI among the badges, Attach a file in the bar, and added parts under tear lines, with the Cupertino back arrow |
| 5 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/5-fan.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/5-fan.png` | 1320 × 2868 | The Agenda's "+" fanned out: Text, Voice, Photo, Gallery, Paste, File |
| 6 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/6-routines.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/6-routines.png` | 1320 × 2868 | All Routines and Reminders, with the Cupertino back arrow |
| 7 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/7-history.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/7-history.png` | 1320 × 2868 | Routine history with Streak and Adherence, with the Cupertino back arrow |
| 8 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/8-dark.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/8-dark.png` | 1320 × 2868 | Agenda in the dark theme, scrolled to the rest of the week |

## Play Console, in order

1. Store listing: replace the phone screenshots with frames 1 to 8 above, and apply the accepted edits, if any.
2. Testing, Internal testing: the tag pipeline uploads the AAB; open the new release.
3. Release notes: en-US and de-DE from above, inside the language tags the console shows.
4. Next, then review, then Start rollout to Internal testing.

## App Store Connect, in order

1. The tag pipeline uploads the build to TestFlight; try it, and the tester notes above go into Test Information.
2. Write the What's New above into `fastlane/metadata/ios/en-US/release_notes.txt` and `fastlane/metadata/ios/de-DE/release_notes.txt`, and the accepted listing edits into the description files.
3. Replace frames 1, 2, 3, 4, 5 and 8 by hand in App Store Connect; `deliver` skips the screenshots.
4. Submit with Actions, Release, Run workflow: the tag, publish on, submit_app_store on. `fastlane deliver` uploads the listing and What's New from `fastlane/metadata/ios/`, the whole description replacing 0.5.1's, and submits the build that run uploads.
5. After approval, release the version by hand.

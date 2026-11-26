# Release notes 0.9.0

Paste source for the Google Play Console and App Store Connect: the release's What's new (the tag pipeline reads the `play_notes_en` and `play_notes_de` blocks, and the apps bundle them as the in-app "What's new in 0.9.0" Note), tester notes and both store listings. The changes themselves are in [CHANGELOG.md](../CHANGELOG.md).

## Summary

Version 0.9.0. Live: 0.8.0 on Google Play, 0.7.2 on the App Store (0.8.0 not submitted there yet), read with `live.py` on 2026-11-25. Every text here describes the `## [0.9.0]` section of the changelog against 0.8.0: AXT files to and from other apps (export, open and take, later updates and deletions), the desktop's layout on tablets and iPads from 700 dp wide, and the iPad as a device of its own. The Play and App Store texts name all three; the desktop's own AXT dialog and menus stay out of the phone texts. Listing: the App Store description's first line names the iPad (decided in the grilling, Q3), and one line per description names AXT files, accepted on 2026-11-26. Frames: Play's 10" tablet frames and the App Store's iPad 13" frames are new (1 to 8 each); the iPhone's frame 4 now shows Export as AXT file… in the Share group; the phone frames are unchanged.

- [Play Kit](https://claude.ai/artifact/Ds5iewxVwTLoPwAWFbpWiJ)
- [App Store Kit](https://claude.ai/artifact/PvZUte3cRrwyoHg2feLj1T)

## Play: release notes

The same text is the in-app What's new Note on every platform.

<!-- block id=play_notes_en limit=500 lang=en title="Release notes, en-US" where="Internal testing release" -->
```
Added
• AXT files: Notes and reminders to and from other apps.
• Tablets and iPads show a sidebar beside the list.
• The iPad is a device of its own.
```

<!-- block id=play_notes_de limit=500 lang=de title="Versionshinweise, de-DE" where="Internal testing release" -->
```
Neu
• AXT-Dateien: Notizen und Erinnerungen von und zu anderen Apps.
• Tablets und iPads zeigen eine Seitenleiste neben der Liste.
• Das iPad ist ein eigenes Gerät.
```

## App Store: What's New

<!-- block id=appstore_whatsnew_en limit=4000 lang=en title="What's New, English" where="Version information" -->
```
Old Brain Fix now speaks AXT, an open format small apps use to hand records to each other. "Export as AXT file…", beside Export as Pack…, gives a Note, a selection, a Topic or an Area to another app; a Note with a Reminder or Routine goes as a task. An AXT file opened or shared to Old Brain Fix shows its records, all ticked, and "Take it" files them into the place picked, Unsorted at first; an appointment becomes a Reminder with its Heads-ups. A later file from the same app updates or removes what it brought, unless you changed it in the meantime.

On the iPad, Old Brain Fix is now an iPad app, and on a screen from 700 points wide it shows a sidebar with the Agenda, Unsorted, All Notes, Frogs and your Areas beside the list, and a Note opens as a dialog over it.
```

<!-- block id=appstore_whatsnew_de limit=4000 lang=de title="Neue Funktionen, Deutsch" where="Version information" -->
```
Old Brain Fix spricht jetzt AXT, ein offenes Format, mit dem kleine Apps einander Einträge geben. „Als AXT-Datei exportieren…“, neben „Als Paket exportieren…“, gibt eine Notiz, eine Auswahl, ein Thema oder einen Bereich an eine andere App; eine Notiz mit Erinnerung oder Routine geht als Aufgabe. Eine geöffnete oder geteilte AXT-Datei zeigt ihre Einträge, alle angehakt, und „Übernehmen“ legt sie am gewählten Ort ab, zuerst in Unsortiert; ein Termin wird eine Erinnerung mit ihren Vorwarnungen. Eine spätere Datei derselben App ändert oder entfernt, was sie gebracht hat, außer du hast es seither geändert.

Auf dem iPad ist Old Brain Fix jetzt eine iPad-App, und ab 700 Punkten Breite zeigt es neben der Liste eine Seitenleiste mit Agenda, Unsortiert, Alle Notizen, Frösche und deinen Bereichen; eine Notiz öffnet sich als Dialog darüber.
```

## Tester notes

<!-- block id=tester_notes limit=4000 lang=en title="What to test" where="TestFlight test information · Play internal track" -->
```
What to test
• On a Note's card, ⋮ ▸ Share ▸ Export as AXT file…: share the file to yourself and open it again; its page lists the Note, ticked, from your own Device. Take it into an Area.
• Export a Note with a Routine, change the Routine on the sending phone, export again, open the second file: the page offers "Updates …"; take it. Edit the taken Note first and the page says "Kept your change".
• Export an Area and a selection of cards as AXT files.
• On an Android tablet or an iPad: the sidebar beside the list from 700 dp wide, a Note as a dialog over it; turn the device and the place shown stays.
• On an iPad: install from TestFlight; it runs as an iPad app, not at iPhone size.
Known
• Each Device is its own AXT sender: a file from your phone cannot update a Note your desktop's file brought.
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
• Notes, Reminders and Routines go to other apps, and come from them, as AXT files, an open format for small apps.

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
Old Brain Fix is a note and reminder app for the iPhone and the iPad. A desktop app for Linux, Windows and macOS uses the same data. There is no account and no server; all data is stored on the user's own devices.

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
• Notes, Reminders and Routines go to other apps, and come from them, as AXT files, an open format for small apps.

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
Old Brain Fix ist eine Notiz- und Erinnerungs-App für iPhone und iPad. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.

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

<!-- edit block=appstore_desc_en status=accepted -->
Changelog 0.9.0, Added: "The app runs on the iPad as an iPad app, not at iPhone size." (decided in the grilling, Q3)
```
Old Brain Fix is a note and reminder app for the iPhone. A desktop app for Linux, Windows and macOS uses the same data. There is no account and no server; all data is stored on the user's own devices.
```
```
Old Brain Fix is a note and reminder app for the iPhone and the iPad. A desktop app for Linux, Windows and macOS uses the same data. There is no account and no server; all data is stored on the user's own devices.
```

<!-- edit block=appstore_desc_de status=accepted -->
Changelog 0.9.0, Added: "The app runs on the iPad as an iPad app, not at iPhone size." (decided in the grilling, Q3)
```
Old Brain Fix ist eine Notiz- und Erinnerungs-App für das iPhone. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.
```
```
Old Brain Fix ist eine Notiz- und Erinnerungs-App für iPhone und iPad. Eine Desktop-App für Linux, Windows und macOS nutzt dieselben Daten. Es gibt kein Konto und keinen Server; alle Daten liegen auf den eigenen Geräten.
```

<!-- edit block=play_full_en status=accepted -->
Changelog 0.9.0, Added: "Export as AXT file… hands Notes, a selection, a Topic or an Area to other apps that speak AXT."
```
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.
```
```
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.
• Notes, Reminders and Routines go to other apps, and come from them, as AXT files, an open format for small apps.
```

<!-- edit block=play_full_de status=pending -->
Changelog 0.9.0, Added: "Export as AXT file… hands Notes, a selection, a Topic or an Area to other apps that speak AXT."
```
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.
```
```
• Eine Notiz kann samt Routinen und Erinnerungen als Datei an eine andere Person geschickt werden, die sie annimmt oder ablehnt.
• Notizen, Erinnerungen und Routinen gehen als AXT-Dateien, einem offenen Format für kleine Apps, an andere Apps und kommen von ihnen.
```

<!-- edit block=appstore_desc_en status=accepted -->
Changelog 0.9.0, Added: "Export as AXT file… hands Notes, a selection, a Topic or an Area to other apps that speak AXT."
```
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.
```
```
• Notes, Topics or whole Areas can be handed to others as a Pack of copies, which they keep as their own.
• Notes, Reminders and Routines go to other apps, and come from them, as AXT files, an open format for small apps.
```

<!-- edit block=appstore_desc_de status=pending -->
Changelog 0.9.0, Added: "Export as AXT file… hands Notes, a selection, a Topic or an Area to other apps that speak AXT."
```
• Notizen, Themen oder ganze Bereiche lassen sich als Paket mit Kopien an andere weitergeben, die sie als ihre eigenen behalten.
```
```
• Notizen, Themen oder ganze Bereiche lassen sich als Paket mit Kopien an andere weitergeben, die sie als ihre eigenen behalten.
• Notizen, Erinnerungen und Routinen gehen als AXT-Dateien, einem offenen Format für kleine Apps, an andere Apps und kommen von ihnen.
```

## Screenshots: Play

Phone 1080 × 1920 per locale, unchanged since 0.8.0. 10-inch tablet 1440 × 2560 per locale, new: the big-screen layout. Feature graphic 1024 × 500, unchanged.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `play/en-US/images/phoneScreenshots/1-agenda.png · play/de-DE/images/phoneScreenshots/1-agenda.png` | 1080 × 1920 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder, the Quiet today bell and ⋮ in the bar |
| 2 | `play/en-US/images/phoneScreenshots/2-capture.png · play/de-DE/images/phoneScreenshots/2-capture.png` | 1080 × 1920 | The quick note: its text at the top, "Remind me to sort it in" 10 min, 30 min, 1 h, and Open in the editor |
| 3 | `play/en-US/images/phoneScreenshots/3-diary.png · play/de-DE/images/phoneScreenshots/3-diary.png` | 1080 × 1920 | An Area's diary of Note cards in full, the search field on its own band, added parts under tear lines, Select in the bar and the pencil on each card |
| 4 | `play/en-US/images/phoneScreenshots/4-editor.png · play/de-DE/images/phoneScreenshots/4-editor.png` | 1080 × 1920 | Note editor: Undo, Redo and Find in the bar, dated parts under tear lines, the groups This Note, Add and Share |
| 5 | `play/en-US/images/phoneScreenshots/5-fan.png · play/de-DE/images/phoneScreenshots/5-fan.png` | 1080 × 1920 | The Agenda's "+" fanned out: Editor, File, Paste, Gallery, Photo, Voice, Text |
| 6 | `play/en-US/images/phoneScreenshots/6-alarm.png · play/de-DE/images/phoneScreenshots/6-alarm.png` | 1080 × 1920 | Alarm ring screen with Done and Snooze |
| 7 | `play/en-US/images/phoneScreenshots/7-history.png · play/de-DE/images/phoneScreenshots/7-history.png` | 1080 × 1920 | Routine history with Streak and Adherence |
| 8 | `play/en-US/images/phoneScreenshots/8-dark.png · play/de-DE/images/phoneScreenshots/8-dark.png` | 1080 × 1920 | Agenda in the dark theme, scrolled to the rest of the week, the Quiet today bell and ⋮ in the bar |

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `play/en-US/images/tenInchScreenshots/1-agenda.png · play/de-DE/images/tenInchScreenshots/1-agenda.png` | 1440 × 2560 | The sidebar (Agenda, Unsorted, All Notes, Frogs, the Areas, Tags, Trash) beside the Agenda |
| 2 | `play/en-US/images/tenInchScreenshots/2-capture.png · play/de-DE/images/tenInchScreenshots/2-capture.png` | 1440 × 2560 | The quick note over the sidebar layout |
| 3 | `play/en-US/images/tenInchScreenshots/3-diary.png · play/de-DE/images/tenInchScreenshots/3-diary.png` | 1440 × 2560 | An Area's diary of Note cards beside the sidebar |
| 4 | `play/en-US/images/tenInchScreenshots/4-editor.png · play/de-DE/images/tenInchScreenshots/4-editor.png` | 1440 × 2560 | The Note editor as a dialog over the list: This Note, Add, Share with Export as AXT file…, Tidy |
| 5 | `play/en-US/images/tenInchScreenshots/5-fan.png · play/de-DE/images/tenInchScreenshots/5-fan.png` | 1440 × 2560 | The Agenda's "+" fanned out beside the sidebar |
| 6 | `play/en-US/images/tenInchScreenshots/6-alarm.png · play/de-DE/images/tenInchScreenshots/6-alarm.png` | 1440 × 2560 | Alarm ring screen with Done and Snooze |
| 7 | `play/en-US/images/tenInchScreenshots/7-history.png · play/de-DE/images/tenInchScreenshots/7-history.png` | 1440 × 2560 | Routine history beside the sidebar |
| 8 | `play/en-US/images/tenInchScreenshots/8-dark.png · play/de-DE/images/tenInchScreenshots/8-dark.png` | 1440 × 2560 | The sidebar layout in the dark theme |

## Screenshots: App Store

iPhone 6.9″ 1320 × 2868 per locale; frame 4 is regenerated (Export as AXT file… in the Share group), the rest are unchanged since 0.8.0. iPad 13″ 2064 × 2752 per locale, new: the big-screen layout.

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/1-agenda.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/1-agenda.png` | 1320 × 2868 | Agenda: today's Routines, two Frogs, the Unsorted banner, the folder, the Quiet today bell and ⋮ in the bar |
| 2 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/2-capture.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/2-capture.png` | 1320 × 2868 | The quick note: its text at the top, "Remind me to sort it in" 10 min, 30 min, 1 h, and Open in the editor |
| 3 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/3-diary.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/3-diary.png` | 1320 × 2868 | An Area's diary of Note cards in full, the search field on its own band, added parts under tear lines, Select in the bar and the pencil on each card |
| 4 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/4-editor.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/4-editor.png` | 1320 × 2868 | Note editor: Undo, Redo and Find in the bar, dated parts under tear lines, the groups This Note, Add and Share (Export as Pack… and Export as AXT file…) |
| 5 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/5-fan.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/5-fan.png` | 1320 × 2868 | The Agenda's "+" fanned out: Editor, File, Paste, Gallery, Photo, Voice, Text |
| 6 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/6-routines.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/6-routines.png` | 1320 × 2868 | All Routines and Reminders, with the Cupertino back arrow |
| 7 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/7-history.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/7-history.png` | 1320 × 2868 | Routine history with Streak and Adherence |
| 8 | `../../fastlane/metadata/ios/en-US/screenshots/iphone-6.9/8-dark.png · ../../fastlane/metadata/ios/de-DE/screenshots/iphone-6.9/8-dark.png` | 1320 × 2868 | Agenda in the dark theme, scrolled to the rest of the week, the Quiet today bell and ⋮ in the bar |

| # | Files | Size | What is on it |
|---|---|---|---|
| 1 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/1-agenda.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/1-agenda.png` | 2064 × 2752 | The sidebar (Agenda, Unsorted, All Notes, Frogs, the Areas, Tags, Trash) beside the Agenda |
| 2 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/2-capture.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/2-capture.png` | 2064 × 2752 | The quick note over the sidebar layout |
| 3 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/3-diary.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/3-diary.png` | 2064 × 2752 | An Area's diary of Note cards beside the sidebar |
| 4 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/4-editor.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/4-editor.png` | 2064 × 2752 | The Note editor as a dialog over the list: This Note, Add, Share with Export as AXT file…, Tidy |
| 5 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/5-fan.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/5-fan.png` | 2064 × 2752 | The Agenda's "+" fanned out beside the sidebar |
| 6 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/6-routines.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/6-routines.png` | 2064 × 2752 | All Routines and Reminders beside the sidebar |
| 7 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/7-history.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/7-history.png` | 2064 × 2752 | Routine history beside the sidebar |
| 8 | `../../fastlane/metadata/ios/en-US/screenshots/ipad-13/8-dark.png · ../../fastlane/metadata/ios/de-DE/screenshots/ipad-13/8-dark.png` | 2064 × 2752 | The sidebar layout in the dark theme |

## Play Console, in order

1. Store listing: replace phone screenshots none; the phone frames stay.
2. Store listing: replace tablet screenshots 1, 2, 3, 4, 5, 6, 7 and 8 with the 10-inch frames above (Phone and tablet screenshots, 10-inch tablet).
3. Store listing: apply the accepted listing edits to the full description, en-US and de-DE.
4. Testing, Internal testing: the tag pipeline uploads the AAB; open the new release.
5. Release notes: en-US and de-DE from above, inside the language tags the console shows.
6. Next, then review, then Start rollout to Internal testing, then promote.

## App Store Connect, in order

1. The tag pipeline uploads the build to TestFlight; try it on an iPhone and an iPad, and the tester notes above go into Test Information.
2. The What's New above is in `fastlane/metadata/ios/en-US/release_notes.txt` and `fastlane/metadata/ios/de-DE/release_notes.txt` (written by `tool/store_release.py appstore`), and the accepted listing edits are in the description files.
3. Replace frames 4 by hand in App Store Connect (iPhone 6.9″); the other iPhone frames stay.
4. Replace iPad frames 1, 2, 3, 4, 5, 6, 7 and 8 by hand in App Store Connect (iPad 13″).
5. Submit with Actions, Release, Run workflow: the tag, publish on, submit_app_store on. `fastlane deliver` uploads the listing and What's New from `fastlane/metadata/ios/`, and submits the build that run uploads.
6. After approval, release the version by hand.

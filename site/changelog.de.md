# Änderungen auf Deutsch

Die deutschen Änderungsnotizen zu Old Brain Fix, Abschnitt für Abschnitt wie `CHANGELOG.md` und `OLDER_CHANGES.md` im Wurzelverzeichnis, in einer Datei: die aktuelle Version oben ohne Datum, die veröffentlichten darunter mit dem Datum ihres Tags. Die Seite "Was ist neu" der Dokumentation wird daraus gebaut; eine englische Version ohne deutschen Abschnitt lässt den Bau fehlschlagen.

## [0.2.0]

### Neu
- Auf dem Telefon hat die Text-Erfassung neben „Foto“ und „Sprache“ einen Knopf „Galerie“, der ein oder mehrere Bilder vom Telefon als Fotos in die Notiz übernimmt, jedes mit seinem erkannten Text.
- Auf dem Telefon wird eine aus einer anderen App geteilte Audiodatei, etwa eine WhatsApp-Sprachnachricht, zu einer Sprachnotiz mit Transkript, mehrere auf einmal geteilte zu einer Notiz mit je einer Aufnahme, und eine Datei, die das Telefon nicht lesen kann, bleibt in der Notiz, mit einer Meldung, dass sie nicht transkribiert werden konnte, die beim Tippen den Grund zeigt.
- Auf dem Desktop machen „Aus Datei…“ im Erfassungsfeld und auf das Fenster gezogene Dateien aus Bildern und Tondateien, etwa einer WhatsApp-Sprachnachricht in Ogg Opus, m4a, mp3 oder WAV, eine Notiz mit ihren Fotos und Sprachaufnahmen, jede mit ihrem erkannten Text oder Transkript, wobei eine Tondatei, die der Desktop nicht lesen kann, mit einer Meldung in der Notiz bleibt, die beim Klicken den Grund zeigt; Sprachnotizen vom Telefon in Ogg Opus oder MP3 werden auch auf dem Desktop abgespielt und transkribiert.
- Auf dem Telefon und dem Desktop lässt sich eine Person auf ihrer Profilseite oder mit „Ablehnen und blockieren“ in der Vorschau einer von ihr gesendeten Notiz blockieren; das löscht alles, was sie geschrieben hat, außer ihren Erledigt-Haken von allen eigenen Geräten, behält und zeigt nichts mehr von ihr, weist ihre gesendeten Notizen ab, verlässt die geteilten Bereiche, die sie verwaltet, und führt sie unter „Personen“ in einem eingeklappten Abschnitt „Blockiert“, wo „Blockierung aufheben“ ihre Inhalte wieder holt.
- Auf dem Telefon zeigt „Synchronisieren“ den eigenen Kopplungscode des Telefons als QR-Code mit dem Code zum Eintippen darunter, damit ein anderes Telefon, ein Tablet oder der Desktop beitreten kann, „Teilen“ zeigt eine Einladung in einen geteilten Bereich ebenso, und das Telefon ist für die automatische Synchronisierung seiner gekoppelten Geräte erreichbar, solange die App offen ist und während seiner Synchronisierung im Hintergrund, sodass Telefone und Tablets sich ohne Computer miteinander koppeln und synchronisieren, und in der Ansicht „Synchronisieren“ des Desktops lässt sich ein solcher Code eintippen oder einfügen, um einem Telefon oder Tablet beizutreten.

### Geändert
- Auf dem Telefon fächert „+“ in der Agenda, auch solange die App gesperrt ist, „Text“, „Sprache“, „Foto“ und „Galerie“ auf, statt sofort die Text-Erfassung zu öffnen, und schließt sich wieder bei einem zweiten Tippen, einem Tippen daneben oder mit „Zurück“.
- Auf dem Telefon und dem Desktop sind die Einheit von „Alle N“ einer Routine, ihr erneutes Erinnern und ihre Eskalation sowie in den Einstellungen das erneute Erinnern und das erneute Stupsen des Geräts Spalten von Optionsfeldern statt Auswahllisten, und ein anderswo gesetzter Wert bleibt als zusätzliche Option erhalten.
- Auf dem Telefon und dem Desktop startet die App mit einem Sprachmodell schneller: Seine Prüfsumme wird einmal beim Herunterladen oder Auswählen berechnet und erst wieder, wenn sich Größe oder Änderungszeit der Datei ändern.
- Auf dem Telefon und dem Desktop bringt das Wiederherstellen einer Sicherung die Wiedergabegeschwindigkeit von Sprachaufnahmen zurück, die auf dem Gerät gewählt war, auf dem die Sicherung erstellt wurde.
- Auf dem Telefon und dem Desktop zeigt die Vorschau einer gesendeten Notiz von einer Person, die nicht unter den eigenen Personen ist, bis „Zeigen“ nur den Namen und dass die Person unbekannt ist, wobei „Annehmen“, „Ablehnen“ und „Ablehnen und blockieren“ auch vorher gehen, und eine solche unentschiedene Notiz wartet, auch über einen Neustart, bis sie entschieden ist, hinter einer Zeile „x gesendete Notizen warten“ in der Agenda.

### Behoben
- Auf dem Telefon und dem Desktop fragt der Papierkorb-Knopf im Notiz-Editor einmal nach, bevor die Notiz gelöscht wird, mit demselben Dialog wie „Löschen“ auf der Notizkarte, einschließlich der Wahl, ihre Routinen und Erinnerungen zu behalten oder mitzulöschen.
- Auf dem Telefon und dem Desktop heißt eine Sprachnotiz ohne Worte überall dort, wo nur ihre Worte Platz haben, etwa in der Agenda, im Startbildschirm-Widget, auf der Verpasst-Karte und in Alarmen, „Sprachaufnahme“ statt „Foto“.
- Auf dem Telefon und dem Desktop zeigt ein neues Transkript, in dem Whisper nur die ihm vorgegebenen Wörter wiederholt, den deutschen Einleitungssatz oder die Liste der Namen von Bereichen und Themen, stattdessen „Keine Worte erkannt.“, und eine solche Wiederholung vor oder nach echter Sprache wird aus dem Transkript weggelassen.
- Auf dem Desktop behalten schneller oder langsamer abgespielte Sprachaufnahmen die Tonhöhe der Stimme, wie auf dem Telefon.
- Auf dem Desktop zeigt ein Bereich nur seine Notizen ohne Thema, wie auf dem Telefon; eine Suche im Bereich findet weiter die Notizen seiner Themen.
- Auf dem Desktop zeigt die Agenda die ganze Woche, wie auf dem Telefon: Nach morgen folgen die fünf nächsten Tage, jeder unter seinem Wochentag und Datum, ein leerer Tag mit „Nichts geplant“.
- Auf dem Desktop zeigt der Notiz-Editor den erkannten Text eines Fotos unter dem Foto, wie auf dem Telefon, und nicht erst, nachdem das Foto entfernt wurde.

## [0.1.1] - 2026-10-27

### Neu
- Auf dem Telefon wird der Download des Sprachmodells einmal nach der Startauswahl angeboten.
- Der Routinen-Editor auf dem Telefon baut ohne Tippen einer Wiederholungsregel einen Tag des Monats, den ersten bis vierten oder letzten Wochentag des Monats und ein Ende (nie, an einem Tag oder nach einer Anzahl von Malen).
- Ein großer roter Knopf auf der Seite „Über Old Brain Fix“ des Telefons.
- Auf dem Telefon öffnet ein Wischen nach links auf der Agenda die Liste der Bereiche und ein Wischen nach rechts aus der Liste der Bereiche führt zur Agenda zurück, und bei jedem Wischen zwischen der Agenda, der Liste der Bereiche, einem Bereich und einem Thema folgen die Seiten dem Finger, wobei der Wechsel ab der Mitte oder mit einem schnellen Wischen abgeschlossen wird und sonst zurückfedert.
- Eine Sprachaufnahme mit Transkript oder ein Foto mit erkanntem Text lässt sich im Notiz-Editor (auf dem Telefon auch im ⋮-Menü der Karte) entfernen, von jedem Gerät und aus dem geteilten Bereich, während der Text in der Notiz und in der Suche bleibt.
- Auf dem Telefon und dem Computer zeigt eine Anzeige neben jedem Abspielknopf die Wiedergabegeschwindigkeit von Sprachaufnahmen und öffnet ein Menü mit 0,5-, 0,75-, 1-, 1,5- und 2-facher Geschwindigkeit; die Wahl gilt sofort und bleibt auf diesem Gerät.
- Auf dem Telefon und dem Computer bieten die Einstellungen des Sprachmodells neben dem Basismodell (148 MB) das kleine Modell von Whisper (488 MB) an, das englische Wörter in deutscher Sprache besser erkennt, aber langsamer ist; die Wahl eines Modells lädt es herunter, und nur das gewählte Modell wird behalten.

### Geändert
- Die Agenda auf dem Telefon zeigt die ganze Woche: nach morgen die fünf folgenden Tage, jeder unter Wochentag und Datum, ein leerer Tag als „Nichts geplant“.
- Auf dem Telefon zeigt ein Bereich nur seine Notizen ohne Thema; eine Suche im Bereich findet weiter die Notizen seiner Themen.
- Die Erfassen-Seite auf dem Telefon hat große Knöpfe für Foto und Sprache über der Erinnerungsauswahl statt Symbolen in der oberen Leiste.
- Auf dem Telefon ist jede einzelne Auswahl eine Spalte von Optionsfeldern, und jeder Schalter, Knopf und jede Liste steht in einer eigenen Zeile unter ihrem Text.
- Eine Notizkarte auf dem Telefon zeigt von oben nach unten das Profil einer anderen Person, die sie geschrieben hat, das Datum mit dem Ort, Bereich und Thema oder Unsortiert dort, wo die Seite sie nicht schon nennt, die Notiz und eine Reihe von Symbolen für Ablegen, Routine oder Erinnerung, Zusammenführen und ⋮ mit Wichtigkeit, An eine Person senden, dem Kalender, Zur Notiz hinzufügen und Löschen.
- Auf dem Telefon und dem Computer bekommt Whisper die Namen der Bereiche und Themen und bei deutscher Sprache dazu die Wörter der App und gängige englische Wörter wie Meeting oder Update, damit Transkripte sie so schreiben, wie sie geschrieben werden.

### Behoben
- Bei Navigation mit drei Knöpfen liegt das Ende der Seiten auf dem Telefon nicht mehr unter der Navigationsleiste.
- Auf dem Telefon zeigen die Liste Unsortiert, die Seite des Stupsers, die Liste der Bereiche, ein Bereich, ein Thema, Alle Notizen und der Papierkorb Notizen nicht mehr so, wie sie waren, bevor eine darüber geöffnete Seite sie geändert hat, etwa die Seite des Stupsers aus seiner Benachrichtigung.

## [0.1.0] - 2026-10-26

### Neu
- Einen Gedanken mit einem Tipp auf + als Textnotiz erfassen; wer mit Zurück geht oder die App wechselt, behält sie.
- Ein Foto als Notiz aufnehmen, auf Wunsch mit ein paar Worten; behaltene Fotos tragen keine Ortsdaten.
- Eine Sprachnotiz aufnehmen, die sofort beginnt und mit einem großen Stopp-Knopf endet; die Aufnahme bleibt zum Abspielen erhalten.
- Transkripte von Sprachnotizen, auf dem Handy von Whisper auf Englisch oder Deutsch geschrieben, nach einem einmaligen Download des Sprachmodells auf Wunsch oder aus einer eigenen Datei.
- Text in Fotos und Bildschirmfotos wird auf dem Handy von ocrs erkannt, offline, und mit dem Foto aufbewahrt.
- Bilder, Text und Links aus anderen Apps in eine neue unsortierte Notiz teilen.
- App-Verknüpfungen, Schnelleinstellungs-Kacheln und Startbildschirm-Widgets, die Sprach-, Text- oder Fotoerfassung direkt öffnen, auch für Hardwaretasten.
- Ein Agenda-Widget für den Startbildschirm mit den Routinen und Erinnerungen von heute zum Abhaken und der Zahl der unsortierten Notizen.
- Englisch und Deutsch.
- Ein helles Petrol- und ein dunkles Anthrazit-Farbschema, die dem System folgen oder deiner eigenen Wahl.
- Notizen als geprägte Karten über einem Hintergrund aus handgezeichneten Fragezeichen, die beim Scrollen langsam treiben.
- Eigene Bereiche und Themen, zwei Ebenen tief, mit optionaler Farbe und optionalem Symbol je Bereich.
- Eine Startauswahl an Bereichen und Themen (Haushalt, Arbeit, Auto, Gesundheit) zum Aussuchen beim ersten Start.
- Unsortiert: eine Notiz, die nicht abgelegt, nicht zu einer Routine oder Erinnerung gemacht, nicht zusammengeführt und nicht gelöscht ist.
- Beim Löschen eines Bereichs oder Themas wird gefragt, ob seine Notizen zurück nach Unsortiert, in den Bereich oder in den Papierkorb gehen.
- Ein Notizbrowser mit Suche über Notiztext, Transkripte und Text in Fotos, Wischnavigation durch Bereiche und Themen und einem Tagebuch aus Karten je Bereich und Thema.
- Ein Notizeditor, um Text an Ort und Stelle zu verbessern, datierte Teile hinzuzufügen, die Wichtigkeit zu setzen und die Notiz abzulegen.
- Wichtigkeit: wichtige Karten stechen hervor, unwichtige sind gedimmt und lassen sich ausblenden.
- Ein Papierkorb, der gelöschte Notizen, Routinen und Erinnerungen 30 Tage lang aufbewahrt.
- Eine Notiz in eine andere zusammenführen; sie wird ein datierter Teil dieser Notiz.
- Getippter Text im Editor übersteht es, wenn die App geschlossen oder beendet wird.
- Jede Notiz und jeder hinzugefügte Teil hält fest, wann und wo erfasst wurde, den Kalendertermin und das Bluetooth-Gerät in dem Moment und bei Geteiltem die App, aus der es kam.
- Benannte Orte, die "Zuhause" oder "Arbeit" statt Koordinaten zeigen, offline zugeordnet.
- Die Agenda als Startbildschirm: die Routinen und Erinnerungen von heute, dann gedimmt die von morgen.
- Routinen und Erinnerungen aus jeder Notiz, jede mit einem Verweis zurück auf sie.
- Pläne von einmalig bis zu iCalendar-Wiederholungsregeln, mit mehreren Zeiten am Tag, fest bei frühem oder spätem Abhaken und bei der Zeitumstellung.
- Alarme auf die Minute mit Erledigt und Später (10 Min., 30 Min., 1 Std.), auch vom Sperrbildschirm und nach einem Neustart.
- Eine Schritt-für-Schritt-Einrichtung der Berechtigungen und ein rotes Banner in der Agenda, solange eine nötige Berechtigung fehlt.
- Erneut erinnern: ein unbeachteter Alarm klingelt in einem Abstand wieder, je Handy oder je Routine, bis er erledigt, verschoben oder verpasst ist.
- Eine Verpasst-Karte in der Agenda, die auflistet, was verpasst wurde, bis du auf "Gesehen" tippst.
- Eine Nachhol-Benachrichtigung statt einer Flut alter Alarme, nachdem das Handy aus war.
- Eskalation: eine unbeachtete Routine kann zu einem Vollbild-Alarm mit Ton werden.
- Ein Verlauf je Routine mit jeder erledigten, verpassten und verschobenen Zeit, ihrer Serie und ihrer Einhaltung, mit Serien, die sich ausblenden lassen.
- Stupser, die dich weiter an unsortierte Notizen erinnern, mit einer Stupszeit, die beim Erfassen gewählt wird.
- Ein Banner "x unsortiert" in der Agenda.
- Ruhezeiten, die Stupser nachts zurückhalten, während Routinen weiter klingeln.
- Beim Löschen einer Notiz mit Routinen oder Erinnerungen wird gefragt, ob diese auch in den Papierkorb gehen.
- Jede Notiz, Routine oder Erinnerung ausgefüllt in deine Kalender-App übernehmen, ohne Kalenderberechtigung.
- Alle oder ausgewählte Routinen und Erinnerungen als teilbare Kalenderdatei exportieren, mit ihren Wiederholungsregeln und Alarmen.
- Auf Wunsch alle Routinen und Erinnerungen in einem Gerätekalender deiner Wahl führen, aktualisiert, wenn sie sich ändern, und entfernt, wenn es ausgeschaltet wird.
- Einstellungen, die zu jedem Handy gehören und nie abgeglichen werden.
- Das Handy mit dem Computer über das Heimnetz abgleichen, indem du den Code scannst oder eintippst, den der Computer zeigt; die beiden bleiben gekoppelt.
- Gekoppeltes Handy und gekoppelter Computer finden sich im Heimnetz und gleichen von selbst ab, sofort, solange die App offen ist, und etwa alle 15 Minuten im Hintergrund.
- Was auf zwei Geräten vor einem Abgleich geändert wurde, erscheint auf Handy und Computer als Konflikt, und die Fassung, die du wählst, bleibt auf jedem Gerät.
- Handy und Computer gleichen auch über einen geteilten Ordner ab, etwa Syncthing, Nextcloud oder einen USB-Stick, von selbst und im Hintergrund, mit allem darin Ende-zu-Ende-verschlüsselt je Bereich.
- Das Neue als verschlüsselte Abgleich-Paketdatei über einen beliebigen Messenger schicken und auf dem anderen gekoppelten Gerät importieren.
- Einen einzelnen Bereich mit einem anderen Menschen als Verwalter oder Mitglied teilen, nachdem du sein Profil bestätigt hast, mit seinem Bild auf den Notizen, die er dort schreibt; jeder andere Bereich bleibt privat.
- Eine Notiz mit ihren Routinen und Erinnerungen als signierte Datei mit deinem Profil an einen anderen Menschen mit Old Brain Fix schicken, verschlüsselt, wenn du ihn kennst, der sie mit deinem Namen, Bild und Kurzcode ansieht, den Code persönlich mit dir vergleichen kann und sie mit eingeschalteten Alarmen nach Unsortiert übernimmt; spätere Aktualisierungen oder Absagen vom selben Absender ändern sie oder legen sie in den Papierkorb.
- Verschlüsselte Sicherungen aller Bereiche, Notizen, Fotos, Sprachaufnahmen, Routinen und ihres Verlaufs, von Hand oder wöchentlich in einen Ordner deiner Wahl, auf einer neuen Installation mit deiner Passphrase oder dem Wiederherstellungsschlüssel von einem druckbaren Wiederherstellungsblatt wiederhergestellt.
- Eine Passphrase und ein Wiederherstellungsblatt öffnen die Sicherungen all deiner Geräte, eine wiederhergestellte Sicherung bringt dein Profil zurück, und eine Google-Sicherung, die du einschaltest, behält nur die neueste verschlüsselte Sicherung, angeboten mit der Passphrase oder dem Wiederherstellungsschlüssel auf einem neuen Handy.
- Eine optionale App-Sperre mit PIN, Fingerabdruck oder Bildschirmsperre, die deine Notizen schützt, während Erfassen und Alarme offen bleiben.
- Optionale Bereichssperren, die einzelne Bereiche und ihre Notizen verbergen, bis sie entsperrt werden, mit ihren Alarmen, Agenda-Zeilen, Widget- und Kalendereinträgen als "Private Routine" oder vollständig gezeigt, trotzdem abhakbar.
- Eine Seite "Über" mit der Version, der Dokumentation, dem Quellcode, einem Weg, Probleme zu melden oder dem Entwickler zu schreiben, und einem Spendenlink; sie und "Über" am Computer listen jede Fremdkomponente mit ihrer Lizenz.
- Eine Dokumentation auf Englisch und Deutsch mit Glossar, "Was ist neu" und Datenschutzerklärung, verlinkt von "Über" auf dem Handy und am Computer.
- Eine Computer-App für Linux, Windows und macOS im Aussehen des Handys, die Notizen erfasst, bearbeitet und sortiert, Bereiche, Themen, Routinen und Erinnerungen verwaltet, die Agenda wie das Handy zeigt, Sprachnotizen aufnimmt und abspielt, eingefügte Bilder als Fotonotizen behält, Transkripte mit Whisper und Fototexte mit ocrs schreibt und Notizen über ihren Text, ihre Transkripte und Fototexte findet, mit Ablegen per Ziehen und Loslassen, Tasten statt Wischen, Benachrichtigungen mit Erneut erinnern von einem Tray-Symbol, das bei der Anmeldung startet, wenn es nicht ausgeschaltet ist, und einem kleinen Erfassungsfenster, das ein globales Tastenkürzel, das Tray oder der Befehl `oldbrainfix capture` öffnet, wobei der Befehl einen mitgegebenen Text auch als Notiz behält, ohne etwas zu öffnen; sie kommt als .deb, AppImage und Flatpak, über Homebrew und Scoop, und steht mit eigenem Symbol im Anwendungsmenü.
- Ein Profil mit deinem Namen, Bild und Kurzcode, ohne Nachfrage aus dem Namen des Handys oder der Anmeldung am Computer übernommen und auf allen deinen Geräten dasselbe, wobei das Koppeln zweier Geräte mit je einem benannten Profil einmal fragt, welches du bist; seine Seite bündelt Sicherungen, deine Geräte, die Personen, die du kennst, und die Einstellungen, und das Profil jeder anderen Person zeigt ihren Kurzcode, eine Möglichkeit, ihn zu vergleichen, einen eigenen Namen und ein eigenes Bild für sie, die nur deine Geräte sehen, und öffnet ihre Notizen; der Notizbrowser kann nur die Notizen der Personen zeigen, die du wählst, dich eingeschlossen.
- Auf dem Telefon zeigt ein Absturz, was passiert ist, mit Knöpfen zum Neustart der App und zum Mailen eines Berichts an den Entwickler; ein unerwartetes Ende wird beim nächsten Start als Bericht angeboten.

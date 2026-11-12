// The phone store screenshots (docs/agents/store-kit.md): eight bare app
// screens per locale, 1080 × 1920, into the fastlane metadata that Play
// and F-Droid read. Skipped in normal runs; written with
//
//   flutter test test/screenshots --run-skipped --concurrency=1 \
//     --plain-name 'store screenshots'
//
// The App Store's iPhone frames (#244), 1320 × 2868, drawn as on iOS into
// fastlane/metadata/ios/<locale>/screenshots/iphone-6.9/, likewise with
// --plain-name 'iphone screenshots'.
//
// The user guide's phone frames (#134) come from the same seeded data:
// each screen in English and German, light and dark, 648 × 1152, under
// site/assets/frames/phone/<language>/<theme>/, a guide box's frame in
// the folder of its page as <page>/<box>.png (#171). They are committed; a run
// compares each frame with the committed one and fails when they differ,
// and UPDATE_FRAMES=1 writes them instead (tool/guide-frames.sh):
//
//   flutter test test/screenshots --run-skipped --concurrency=1 \
//     --plain-name 'guide frames'
import 'dart:async';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/all_routines_screen.dart';
import 'package:oldbrainfix/src/appointments.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/calendar_screen.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/backup_screen.dart';
import 'package:oldbrainfix/src/feedback_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/first_start_screen.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/frogs_screen.dart';
import 'package:oldbrainfix/src/help_notes.dart';
import 'package:oldbrainfix/src/lock_settings.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/missed_summary_card.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/note_history_screen.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/pack_screen.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix/src/profile_card.dart';
import 'package:oldbrainfix/src/ring_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/routine_history_screen.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/setup_screen.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix/src/speech_model_screen.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../backup_test.dart' show DirFolders, quickKey;
import '../bundle_test.dart' show FakeBundles;
import '../fake_alarms.dart';
import '../fake_appointments.dart';
import '../fake_photos.dart';
import '../fake_speech.dart';
import '../fake_voice.dart';
import '../open_capture.dart';
import '../reveal.dart';

/// The phone's screen in logical pixels and the density of a common
/// phone: 432 × 768, which 2.5 scales to exactly 1080 × 1920.
const _screen = Size(432, 768);
const _dpr = 2.5;

/// The App Store's iPhone frame (#244): the iPhone 16 Pro Max's screen,
/// 440 × 956 points at 3, exactly 1320 × 2868, the size App Store
/// Connect scales down for every smaller iPhone (docs/agents/store-kit.md),
/// with its safe area: the status bar and Dynamic Island on top, the
/// home indicator below.
const _iphoneScreen = Size(440, 956);
const _iphoneDpr = 3.0;
const _iphoneSafeArea = FakeViewPadding(top: 62 * 3, bottom: 34 * 3);

/// The guide's frames: the same 432 × 768 screen at 1.5, 648 × 1152,
/// enough for a page and light in the repository.
const _guideDpr = 1.5;

/// How far a guide frame may drift from the committed one before it
/// counts as changed: a pixel differs when a channel is more than
/// [_channelSlack] off, and a frame fails with more than [_pixelSlack]
/// differing pixels. The slack absorbs the edges another engine version
/// rasterises a shade differently; a changed word or button exceeds it.
const _channelSlack = 24;
const _pixelSlack = 200;

/// Tuesday, 27 October 2026, 09:40 on the test machine's clock: the same
/// Agenda, Streak and Adherence on every run.
final _now = DateTime(2026, 10, 27, 9, 40);

/// The texts of the store in one language.
typedef _Texts = ({
  String snack,
  String walk,
  String plants,
  String timesheet,
  String callAnna,
  String bins,
  String carService,
  List<String> unsorted,
  String boiler,
  List<String> boilerParts,
  String spareKey,
  String smokeAlarm,
  String paint,
  String hooverBags,
  String captureText,
  String home,
  List<String> frogs,
  String packing,
  String groceries,
  String reading,
  String pagesRead,
  String pages,
  String exercises,
  String plank,
  String squats,
  String evening,
  String formatted,
  String meters,
  String script,
  String checks,
  String spoken,
  String receipt,
  String tag,
  String search,
  String sent,
  String boilerHere,
  String boilerThere,
  String packArea,
  String packTopic,
  String packTally,
  String packList,
  String packStretch,
});

const _en = (
  snack: 'Snack: an apple and some nuts',
  walk: 'Walk round the block, 20 minutes',
  plants: 'Water the plants on the balcony',
  timesheet: 'Send the timesheet to Sarah',
  callAnna: 'Call Anna about Saturday',
  bins: 'Put the paper bin out',
  carService: 'Car service at Weber\'s garage, bring the service booklet',
  unsorted: [
    'Batteries for the smoke alarm, 9 V',
    'Parked on level 3, row F',
    'Birthday present for Tom: that board game',
  ],
  boiler: 'Boiler knocks in the mornings',
  boilerParts: [
    'Called the plumber: earliest slot Thursday, 8 to 10',
    'Plumber was here: pressure too low, refilled to 1.5 bar. '
        'Check again in a week.',
  ],
  spareKey: 'Spare key is with the Müllers, 2nd floor',
  smokeAlarm: 'Smoke alarm in the hallway beeps every few minutes',
  paint: 'Bedroom paint: "Sea Mist", matt, 2.5 l',
  hooverBags: 'Hoover bags are type G',
  captureText: 'Ask Anna if she still needs the ladder',
  home: 'Home',
  frogs: [
    'Sort the receipts for the tax return',
    'Clear out the shelf in the cellar',
    'Cancel the old gym membership',
  ],
  packing:
      'Packing for the weekend at the lake\n'
      '✓ Toothbrush\n'
      'Phone charger\n'
      'Hiking gear\n'
      '  ✓ Boots\n'
      '  Rain jacket',
  groceries: 'Weekly shopping\nMilk\nBread\nCoffee\nApples\nWashing-up liquid',
  reading: 'Reading before bed',
  pagesRead: 'Pages read',
  pages: 'pages',
  exercises: 'Morning exercises',
  plank: 'Plank',
  squats: 'Squats',
  evening: 'Evening routine',
  formatted:
      '## Boiler service\n'
      'Booked for **Thursday, 8 to 10**. Have the *service booklet* ready.\n'
      '- Pressure checked: 1.5 bar\n'
      '- Spare valve in stock\n'
      '- Bleed the radiators\n'
      'Manual: [the maker\'s page](https://example.org/boiler)\n'
      'Error `F28` means no ignition. #boiler',
  meters:
      'Meter readings 2026\n'
      '\n'
      '<!-- first column headings -->\n'
      '| Meter | Jan | Feb | Mar | Apr | May | Jun |\n'
      '| --- | --: | --: | --: | --: | --: | --: |\n'
      '| Electricity kWh | 4210 | 4402 | 4588 | 4795 | 4960 | 5121 |\n'
      '| Water m³ | 312 | 318 | 324 | 330 | 337 | 343 |\n'
      '| Gas m³ | 8840 | 9012 | 9150 | 9241 | 9290 | 9318 |',
  script:
      '#!/bin/sh\n'
      '# Copy the photos to the USB disk every Sunday\n'
      'rsync -a --delete ~/Pictures/ /media/usb/photos/\n'
      'echo "Photos copied: \$(date +%F)"',
  checks:
      'Before the holiday\n'
      '✅ Turn the heating down\n'
      '✅ Give the spare key to the Müllers\n'
      '❌ Cancel the newspaper\n'
      '✅ Water the plants on Sunday',
  spoken:
      'Call the plumber about the boiler, Thursday morning between eight '
      'and ten.',
  receipt: 'Bakery receipt for the party',
  tag: '#boiler',
  search: 'boiler',
  sent: 'Lena\'s birthday on Saturday: bring the cake at 3',
  boilerHere: 'Boiler knocks in the mornings, plumber on Thursday',
  boilerThere: 'Boiler knocks in the mornings and evenings',
  packArea: 'Fitness',
  packTopic: 'Running',
  packTally: 'Laps',
  packList: 'Race day\nShoes\nStart number\nWater',
  packStretch: 'Stretch after running',
);

const _de = (
  snack: 'Snack: Apfel und ein paar Nüsse',
  walk: 'Runde um den Block, 20 Minuten',
  plants: 'Pflanzen auf dem Balkon gießen',
  timesheet: 'Stundenzettel an Sarah schicken',
  callAnna: 'Anna wegen Samstag anrufen',
  bins: 'Papiertonne rausstellen',
  carService: 'Inspektion im Autohaus Weber, Serviceheft mitnehmen',
  unsorted: [
    'Batterien für den Rauchmelder, 9 V',
    'Geparkt auf Ebene 3, Reihe F',
    'Geburtstagsgeschenk für Tom: das Brettspiel',
  ],
  boiler: 'Therme klopft morgens',
  boilerParts: [
    'Installateur angerufen: frühester Termin Donnerstag, 8 bis 10 Uhr',
    'Installateur war da: Druck zu niedrig, auf 1,5 bar aufgefüllt. '
        'In einer Woche nochmal prüfen.',
  ],
  spareKey: 'Ersatzschlüssel liegt bei Müllers, 2. Stock',
  smokeAlarm: 'Rauchmelder im Flur piept alle paar Minuten',
  paint: 'Farbe fürs Schlafzimmer: „Seenebel“, matt, 2,5 l',
  hooverBags: 'Staubsaugerbeutel sind Typ G',
  captureText: 'Anna fragen, ob sie die Leiter noch braucht',
  home: 'Zuhause',
  frogs: [
    'Belege für die Steuererklärung sortieren',
    'Das Regal im Keller ausmisten',
    'Den alten Fitnessstudio-Vertrag kündigen',
  ],
  packing:
      'Packen fürs Wochenende am See\n'
      '✓ Zahnbürste\n'
      'Ladekabel fürs Handy\n'
      'Wandersachen\n'
      '  ✓ Wanderschuhe\n'
      '  Regenjacke',
  groceries: 'Wocheneinkauf\nMilch\nBrot\nKaffee\nÄpfel\nSpülmittel',
  reading: 'Lesen vor dem Schlafen',
  pagesRead: 'Gelesene Seiten',
  pages: 'Seiten',
  exercises: 'Morgenübungen',
  plank: 'Unterarmstütz',
  squats: 'Kniebeugen',
  evening: 'Abendrunde',
  formatted:
      '## Wartung der Therme\n'
      'Termin am **Donnerstag, 8 bis 10 Uhr**. Das *Serviceheft* bereitlegen.\n'
      '- Druck geprüft: 1,5 bar\n'
      '- Ersatzventil vorrätig\n'
      '- Heizkörper entlüften\n'
      'Anleitung: [Seite des Herstellers](https://example.org/therme)\n'
      'Fehler `F28` heißt: keine Zündung. #Therme',
  meters:
      'Zählerstände 2026\n'
      '\n'
      '<!-- first column headings -->\n'
      '| Zähler | Jan | Feb | Mär | Apr | Mai | Jun |\n'
      '| --- | --: | --: | --: | --: | --: | --: |\n'
      '| Strom kWh | 4210 | 4402 | 4588 | 4795 | 4960 | 5121 |\n'
      '| Wasser m³ | 312 | 318 | 324 | 330 | 337 | 343 |\n'
      '| Gas m³ | 8840 | 9012 | 9150 | 9241 | 9290 | 9318 |',
  script:
      '#!/bin/sh\n'
      '# Die Fotos jeden Sonntag auf die USB-Platte kopieren\n'
      'rsync -a --delete ~/Bilder/ /media/usb/fotos/\n'
      'echo "Fotos kopiert: \$(date +%F)"',
  checks:
      'Vor dem Urlaub\n'
      '✅ Heizung herunterdrehen\n'
      '✅ Ersatzschlüssel zu Müllers bringen\n'
      '❌ Zeitung abbestellen\n'
      '✅ Pflanzen am Sonntag gießen',
  spoken:
      'Den Installateur wegen der Therme anrufen, Donnerstag früh zwischen '
      'acht und zehn.',
  receipt: 'Beleg vom Bäcker für die Feier',
  tag: '#Therme',
  search: 'Therme',
  sent: 'Lenas Geburtstag am Samstag: den Kuchen um 3 mitbringen',
  boilerHere: 'Therme klopft morgens, Installateur am Donnerstag',
  boilerThere: 'Therme klopft morgens und abends',
  packArea: 'Fitness',
  packTopic: 'Laufen',
  packTally: 'Runden',
  packList: 'Lauftag\nSchuhe\nStartnummer\nWasser',
  packStretch: 'Nach dem Laufen dehnen',
);

/// A store seeded with a believable household, work and car life in
/// [texts], as of [_now].
class _Seeded {
  final Store store;
  final Routine snack;

  /// The Sent Item Lena sent, with [sends].
  final Uint8List? sent;

  /// The Pack Lena made with her Profile, with [packs] (#164).
  final Uint8List? pack;

  _Seeded._(this.store, this.snack, [this.sent, this.pack]);

  /// With [lists], the Household also holds a Checklist, a Shopping List
  /// and a Tally with its entries, for the guide's frames of them.
  /// With [formatted], it holds a formatted Note and one with a table.
  /// With [people], the user's Profile is Sam's and Lena is in People;
  /// with [shared] too, Work is shared with her from afar. With [missed],
  /// what the history Missed stays unseen, in the Missed Summary; with
  /// [dueReminder], the boiler's Note has a Reminder due at 09:30.
  /// With [sends], Lena sent a Note with a Reminder, not encrypted
  /// ([sent]). With [packs], Lena made a Pack of her Fitness with her
  /// Profile ([pack]). With [conflict], the Laptop is paired and it and this
  /// Device changed the boiler's text before they synced.
  factory _Seeded(
    _Texts x, {
    bool lists = false,
    bool formatted = false,
    bool people = false,
    bool shared = false,
    bool missed = false,
    bool dueReminder = false,
    bool sends = false,
    bool packs = false,
    bool conflict = false,
  }) {
    // Each Note and Done at its own moment while seeding, then [_now].
    var clock = _now;
    // With People, the own key from a fixed seed: the same short code
    // on the Profile page on every run.
    final withPeople = people || shared || sends || packs;
    final store =
        Store.inMemory(
            clock: () => clock,
            newKey: withPeople
                ? () => SigningKey.fromSeed(
                    Uint8List.fromList([for (var i = 0; i < 32; i++) 44]),
                  )
                : SigningKey.generate,
          )
          ..starterSetOffered = true
          ..alarmSetupDone = true;
    final snack = _seed(store, x, (at) => clock = at, dueReminder: dueReminder);
    if (lists) _seedLists(store, x, (at) => clock = at);
    if (formatted) _seedFormatted(store, x, (at) => clock = at);
    Uint8List? sent;
    Uint8List? pack;
    if (withPeople) {
      (sent, pack) = _seedPeople(
        store,
        x,
        shared: shared,
        sends: sends,
        packs: packs,
      );
    }
    clock = _now;
    if (conflict) _seedConflict(store, x);
    // What the history Missed was seen already: no Missed Summary.
    if (!missed) Alarms(store).markSeen();
    // The screens show Tallies, Sessions, Frogs, Topics, the format bar
    // and roles: Advanced, chosen (#197).
    store.advanced = true;
    return _Seeded._(store, snack, sent, pack);
  }

  static Routine _seed(
    Store store,
    _Texts x,
    void Function(DateTime) setClock, {
    bool dueReminder = false,
  }) {
    final places = Places(store);
    final notes = Notes(store);
    final routines = Routines(store);
    places.plantStarterSet({'household', 'kitchen', 'work', 'car', 'service'});
    Area area(String slug) =>
        places.areas().firstWhere((a) => a.starter == slug);
    final household = area('household');
    final work = area('work');
    final car = area('car');
    Topic topic(Area of, String slug) =>
        places.topics(of.id).firstWhere((t) => t.starter == slug);
    final kitchen = topic(household, 'kitchen');
    final service = topic(car, 'service');
    places
      ..setColour(household.id, '2E7D32')
      ..setIcon(household.id, 'home')
      ..setColour(work.id, '1565C0')
      ..setIcon(work.id, 'work')
      ..setColour(car.id, 'C62828')
      ..setIcon(car.id, 'car');
    final home = NamedPlaces(store)
        .add(x.home, Coordinates(52.5200, 13.4050), radius: 100);

    /// Captures [text] at [at], into [place], at home when [atHome].
    Note capture(
      DateTime at,
      String text, {
      String? place,
      bool atHome = false,
    }) {
      setClock(at);
      final note = notes.capture(text, place: place);
      if (atHome) notes.locate(note.id, home.centre);
      return note;
    }

    Day day(int d) => Day(2026, 10, d);
    ClockTime at(int h, [int m = 0]) => ClockTime(h, m);
    final daily = Recurrence.parse('FREQ=DAILY');

    // The snack, twice a day since 1 October; two afternoon snacks
    // forgotten.
    final snackNote = capture(
      DateTime(2026, 9, 30, 19, 12),
      x.snack,
      place: kitchen.id,
    );
    final snack = routines.spawn(
      snackNote.id,
      Schedule(start: day(1), times: [at(9), at(15)], rule: daily),
    );
    for (var d = 1; d <= 27; d++) {
      for (final hour in [9, 15]) {
        if (d == 27 && hour == 15) continue;
        if ((d == 9 && hour == 15) || (d == 22 && hour == 15)) continue;
        routines.markDone(
          snack.id,
          Occurrence(day(d), at(hour)),
          at: DateTime(2026, 10, d, hour, 4 + d % 9),
        );
      }
    }

    // A walk every morning, done today.
    final walkNote = capture(
      DateTime(2026, 10, 18, 21, 30),
      x.walk,
      place: household.id,
    );
    final walk = routines.spawn(
      walkNote.id,
      Schedule(start: day(19), times: [at(7, 30)], rule: daily),
    );
    for (var d = 19; d <= 27; d++) {
      routines.markDone(
        walk.id,
        Occurrence(day(d), at(7, 30)),
        at: DateTime(2026, 10, d, 7, 52),
      );
    }

    // The plants every Monday and Thursday, all day.
    final plantsNote = capture(
      DateTime(2026, 10, 2, 18, 5),
      x.plants,
      place: household.id,
      atHome: true,
    );
    routines.spawn(
      plantsNote.id,
      Schedule(
        start: day(6),
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO,TH'),
      ),
    );

    // The timesheet every Wednesday at 11.
    final timesheetNote = capture(
      DateTime(2026, 10, 5, 16, 40),
      x.timesheet,
      place: work.id,
    );
    routines.spawn(
      timesheetNote.id,
      Schedule(
        start: day(7),
        times: [at(11)],
        rule: Recurrence.parse('FREQ=WEEKLY'),
      ),
    );

    // Reminders: the bin today, the car service and Anna tomorrow.
    final annaNote = capture(DateTime(2026, 10, 26, 20, 15), x.callAnna);
    routines.spawn(annaNote.id, Schedule(start: day(28), times: [at(12, 30)]));
    final binsNote = capture(
      DateTime(2026, 10, 5, 7, 50),
      x.bins,
      place: household.id,
      atHome: true,
    );
    routines.spawn(binsNote.id, Schedule(start: day(27), times: [at(18)]));
    final carServiceNote = capture(
      DateTime(2026, 10, 20, 10, 3),
      x.carService,
      place: service.id,
    );
    routines.spawn(
      carServiceNote.id,
      Schedule(start: day(28), times: [at(10, 15)]),
    );

    // The Household's diary.
    final hoover = capture(
      DateTime(2026, 10, 12, 11, 20),
      x.hooverBags,
      place: household.id,
      atHome: true,
    );
    notes.setImportance(hoover.id, Importance.unimportant);
    capture(DateTime(2026, 10, 17, 15, 45), x.paint, place: household.id);
    final key = capture(
      DateTime(2026, 10, 19, 18, 30),
      x.spareKey,
      place: household.id,
      atHome: true,
    );
    notes.setImportance(key.id, Importance.important);
    capture(
      DateTime(2026, 10, 22, 22, 10),
      x.smokeAlarm,
      place: household.id,
      atHome: true,
    );
    final boiler = capture(
      DateTime(2026, 10, 23, 7, 15),
      x.boiler,
      place: household.id,
      atHome: true,
    );
    notes.setImportance(boiler.id, Importance.important);
    if (dueReminder) {
      routines.spawn(boiler.id, Schedule(start: day(27), times: [at(9, 30)]));
    }
    setClock(DateTime(2026, 10, 23, 12, 40));
    notes.amend(boiler.id, x.boilerParts[0]);
    setClock(DateTime(2026, 10, 26, 10, 25));
    final visit = notes.amend(boiler.id, x.boilerParts[1]);
    notes.locate(visit.id, home.centre);

    // Frogs put off for a while: two of them offered today.
    final frogs = Frogs(store);
    for (final (i, (text, place)) in [
      (x.frogs[0], work.id),
      (x.frogs[1], household.id),
      (x.frogs[2], household.id),
    ].indexed) {
      frogs.spawn(
        capture(DateTime(2026, 10, 8 + 3 * i, 20, 10), text, place: place).id,
      );
    }

    // Captured on the way, not sorted yet.
    capture(DateTime(2026, 10, 26, 17, 58), x.unsorted[0]);
    capture(DateTime(2026, 10, 26, 19, 4), x.unsorted[1]);
    capture(DateTime(2026, 10, 27, 8, 31), x.unsorted[2]);
    return snack;
  }

  /// A Tally of the pages read in the evenings, a packing Checklist and
  /// the weekly Shopping List, all in the Household, after [_seed].
  static void _seedLists(
    Store store,
    _Texts x,
    void Function(DateTime) setClock,
  ) {
    final places = Places(store);
    final notes = Notes(store);
    final household = places.areas().firstWhere(
      (a) => a.starter == 'household',
    );

    // Pages read most evenings since 13 October, 30 a day the target.
    setClock(DateTime(2026, 10, 13, 22, 5));
    final reading = notes.capture(x.reading, place: household.id);
    final tallies = Tallies(store);
    final pages = tallies.addField(
      reading.id,
      name: x.pagesRead,
      unit: x.pages,
      target: 30,
    );
    const read = [24, 31, 0, 18, 40, 35, 12, 0, 30, 27, 22, 45, 33, 16];
    for (final (i, count) in read.indexed) {
      if (count == 0) continue;
      final at = DateTime(2026, 10, 13 + i, 22, 15);
      setClock(at);
      tallies.add(reading.id, values: {pages: count.toDouble()});
    }

    // The packing list, two Lines checked already.
    setClock(DateTime(2026, 10, 25, 19, 40));
    final packing = notes.capture(x.packing, place: household.id);
    Checklists(store).make(packing.id);

    // The weekly shopping, two Lines bought on Saturday.
    setClock(DateTime(2026, 10, 24, 9, 30));
    final groceries = notes.capture(x.groceries, place: household.id);
    final shopping = ShoppingLists(store)..make(groceries.id);
    setClock(DateTime(2026, 10, 24, 11, 5));
    final lines = shopping.byNote(groceries.id, deviceWallClock)!.lines;
    for (final line in [lines[1], lines[3]]) {
      shopping.check(line.id, deviceWallClock);
    }
  }
}

/// The Household's texts, its newest Notes: the table of meter readings
/// and the boiler's service, both formatted; the backup script as Code
/// (#213); the holiday's boxes, written checked or failed, the first
/// failed and checked again, so that it has a History; the receipt's
/// photo with its Photo Text; and a voice note with its Transcript, the newest.
void _seedFormatted(Store store, _Texts x, void Function(DateTime) setClock) {
  final notes = Notes(store);
  final household = Places(store)
      .areas()
      .firstWhere((a) => a.starter == 'household');
  setClock(DateTime(2026, 10, 27, 8, 50));
  notes.capture(x.meters, format: TextFormats.markdown, place: household.id);
  setClock(DateTime(2026, 10, 27, 9, 10));
  notes.capture(x.formatted, format: TextFormats.markdown, place: household.id);
  setClock(DateTime(2026, 10, 27, 9, 40));
  notes.capture(x.script, format: TextFormats.code, place: household.id);
  setClock(DateTime(2026, 10, 27, 10, 5));
  final checks = notes.capture(x.checks, place: household.id);
  final boxes = Boxes(store);
  final shown = boxes.of(checks.id);
  setClock(DateTime(2026, 10, 27, 18, 20));
  boxes.tick(checks.id, shown[0].id, BoxState.failed);
  setClock(DateTime(2026, 10, 27, 18, 25));
  boxes.tick(checks.id, shown[0].id, BoxState.done);
  setClock(DateTime(2026, 10, 27, 19, 30));
  final receipt = notes.capture(
    x.receipt,
    place: household.id,
    photos: [File(_receiptPhoto).readAsBytesSync()],
  );
  notes.setPhotoText(receipt.id, receipt.photos.single, _receiptText);
  setClock(DateTime(2026, 10, 27, 20, 15));
  final voice = notes.capture(
    '',
    place: household.id,
    recordings: [
      Uint8List.fromList([1, 2, 3]),
    ],
  );
  notes.setTranscript(voice.id, voice.recordings.single, x.spoken);
}

/// The photo of a bakery's receipt the OCR tests read.
const _receiptPhoto = 'android/ocr/testdata/german-receipt.jpg';

/// The words read from [_receiptPhoto], line by line.
const _receiptText =
    'Bäckerei Müller GmbH\n'
    'Hauptstraße 7, 80331 München\n'
    'Brötchen 6x 2,40 €\n'
    'Käsekuchen 3,90 €\n'
    'Süßes Teilchen 1,85 €\n'
    'Summe EUR 8,15\n'
    'MwSt 7% 0,53\n'
    'Vielen Dank für Ihren Einkauf';

/// The user's own Profile as Sam's, and Lena in People from her Profile
/// card, sent through a messenger, so not verified. With [shared], Work
/// is shared with her from afar as a Member, its changes travelling by
/// bundles. With [sends], she sends a Note with a Reminder, not
/// encrypted: its file. With [packs], she makes a Pack of her Fitness,
/// a Tally, a Checklist and a Note with a Routine,
/// with her Profile (#164): its file.
(Uint8List?, Uint8List?) _seedPeople(
  Store store,
  _Texts x, {
  required bool shared,
  bool sends = false,
  bool packs = false,
}) {
  store
    ..applyProfileDefaults(name: 'Sam')
    ..renameOwnProfile('Sam');
  // Her key from a fixed seed: the same short code on every run.
  final lena =
      Store.inMemory(
          clock: () => _now,
          newKey: () => SigningKey.fromSeed(
            Uint8List.fromList([for (var i = 0; i < 32; i++) 45]),
          ),
        )
        ..applyProfileDefaults(name: 'Lena')
        ..renameOwnProfile('Lena');
  final id = lena.ownProfile!.id;
  final card = lena.profileCard()!;
  Uint8List? sent;
  if (sends) {
    final party = Notes(lena).capture(x.sent);
    Routines(lena).spawn(
      party.id,
      Schedule(start: Day(2026, 10, 31), times: [ClockTime(14, 30)]),
    );
    sent = sendNote(lena, party.id);
  }
  Uint8List? pack;
  if (packs) {
    final fitness = Places(lena).addArea(x.packArea);
    final running = Places(lena).addTopic(fitness.id, x.packTopic);
    final notes = Notes(lena);
    final laps = notes.capture(x.packTally, place: running.id);
    Tallies(lena).addField(laps.id, name: x.packTally, target: 10);
    final list = notes.capture(x.packList, place: running.id);
    Checklists(lena).make(list.id);
    final stretch = notes.capture(x.packStretch, place: fitness.id);
    Routines(lena).spawn(
      stretch.id,
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(19, 0)],
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO,WE,FR'),
      ),
    );
    final whole = lena.packOf(fitness.id);
    pack = lena.writePack(whole, whole.allKeys(), addProfile: true);
  }
  lena.close();
  store.openProfileCard(card);
  if (!shared) return (sent, pack);
  final work = Places(store).areas().firstWhere((a) => a.starter == 'work');
  shareAreaWith(
    store,
    work.id,
    id,
    role: AreaRole.member,
    transport: Transport.bundle,
  );
  return (sent, pack);
}

/// The Laptop, a Device of the user's own, paired, and a Conflict: it and
/// this phone changed the boiler's text before they synced.
void _seedConflict(Store store, _Texts x) {
  final laptop = Store.inMemory(
    clock: () => _now.subtract(const Duration(minutes: 25)),
    newKey: () => SigningKey.fromSeed(
      Uint8List.fromList([for (var i = 0; i < 32; i++) 46]),
    ),
  );
  laptop.applyEntries(store.entriesSince(const {}), keys: [store.ownKeyRecord]);
  final boiler = Notes(store).all().firstWhere((n) => n.text == x.boiler);
  Notes(store).edit(boiler.id, x.boilerHere);
  Notes(laptop).edit(boiler.id, x.boilerThere);
  store
    ..applyEntries(laptop.entriesSince(const {}), keys: [laptop.ownKeyRecord])
    ..rememberDevice(
      PairedDevice(
        id: laptop.deviceId,
        name: 'Laptop',
        fingerprint: '',
        host: null,
        port: null,
        lastSync: _now,
        shared: false,
      ),
    );
  laptop.close();
}

/// The Flutter SDK's root, from the test runner it started.
String _flutterRoot() {
  final runner = Platform.resolvedExecutable;
  final cut = runner.indexOf(
    '${Platform.pathSeparator}bin'
    '${Platform.pathSeparator}cache${Platform.pathSeparator}',
  );
  if (cut < 0) {
    return Platform.environment['FLUTTER_ROOT'] ??
        (throw StateError('no Flutter SDK root next to $runner'));
  }
  return runner.substring(0, cut);
}

/// Loads Roboto and the Material icons from the SDK, so the frames show
/// text instead of the test font's boxes.
Future<void> _loadFonts() async {
  final dir = Directory(
    [
      _flutterRoot(),
      'bin',
      'cache',
      'artifacts',
      'material_fonts',
    ].join(Platform.pathSeparator),
  );
  Future<ByteData> bytes(File file) async =>
      ByteData.sublistView(await file.readAsBytes());
  final roboto = FontLoader('Roboto');
  final files = dir.listSync().whereType<File>();
  for (final file in files) {
    final name = file.uri.pathSegments.last;
    if (name.startsWith('Roboto-') && name.endsWith('.ttf')) {
      roboto.addFont(bytes(file));
    }
  }
  await roboto.load();
  final icons = FontLoader('MaterialIcons')
    ..addFont(bytes(File('${dir.path}/MaterialIcons-Regular.otf')));
  await icons.load();
  // Code in a formatted text, as a phone's monospace font would show it.
  final mono = FontLoader('monospace')
    ..addFont(
      bytes(
        File(
          [
            _flutterRoot(),
            'bin',
            'cache',
            'dart-sdk',
            'bin',
            'resources',
            'devtools',
            'assets',
            'fonts',
            'Roboto_Mono',
            'RobotoMono-Regular.ttf',
          ].join(Platform.pathSeparator),
        ),
      ),
    );
  await mono.load();
  // The Frog's emoji, as a phone's own emoji font would show it.
  final emoji = FontLoader('Noto Color Emoji')
    ..addFont(
      bytes(
        File(
          [
            _flutterRoot(),
            'engine',
            'src',
            'flutter',
            'txt',
            'third_party',
            'fonts',
            'NotoColorEmoji.ttf',
          ].join(Platform.pathSeparator),
        ),
      ),
    );
  await emoji.load();
}

/// Roboto under the names Flutter's iOS typography asks for, so the
/// iPhone frames show text: the system font of iOS is not there to load
/// outside Apple's systems.
Future<void> _loadCupertinoFonts() async {
  final dir = [
    _flutterRoot(),
    'bin',
    'cache',
    'artifacts',
    'material_fonts',
  ].join(Platform.pathSeparator);
  for (final family in ['CupertinoSystemText', 'CupertinoSystemDisplay']) {
    final loader = FontLoader(family);
    for (final file in Directory(dir).listSync().whereType<File>()) {
      final name = file.uri.pathSegments.last;
      if (name.startsWith('Roboto-') && name.endsWith('.ttf')) {
        loader.addFont(
          file.readAsBytes().then((bytes) => ByteData.sublistView(bytes)),
        );
      }
    }
    await loader.load();
  }
}

/// [rgba], [width] × [height], as a PNG without an alpha channel (colour
/// type 2), as App Store Connect wants its screenshots. Every pixel has to
/// be opaque already: a see-through one fails instead of changing colour.
Uint8List _opaquePng(int width, int height, Uint8List rgba) {
  final rows = BytesBuilder(copy: false);
  for (var y = 0; y < height; y++) {
    final row = Uint8List(1 + width * 3);
    for (var x = 0; x < width; x++) {
      final i = (y * width + x) * 4;
      if (rgba[i + 3] != 255) fail('pixel $x, $y is not opaque');
      row.setRange(1 + x * 3, 4 + x * 3, rgba, i);
    }
    rows.add(row);
  }
  final png = BytesBuilder(copy: false)
    ..add(const [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  void chunk(String type, List<int> data) {
    final body = Uint8List.fromList([...type.codeUnits, ...data]);
    png
      ..add(_bigEndian(data.length))
      ..add(body)
      ..add(_bigEndian(_crc32(body)));
  }

  chunk('IHDR', [
    ..._bigEndian(width),
    ..._bigEndian(height),
    8, // bits per channel
    2, // RGB
    0,
    0,
    0,
  ]);
  chunk('IDAT', ZLibCodec(level: 6).encode(rows.takeBytes()));
  chunk('IEND', const []);
  return png.takeBytes();
}

Uint8List _bigEndian(int value) =>
    Uint8List(4)..buffer.asByteData().setUint32(0, value);

/// The CRC-32 a PNG chunk ends with (ISO 3309, as zlib's).
int _crc32(List<int> bytes) {
  var crc = 0xffffffff;
  for (final byte in bytes) {
    crc ^= byte;
    for (var k = 0; k < 8; k++) {
      crc = crc & 1 == 1 ? (crc >> 1) ^ 0xedb88320 : crc >> 1;
    }
  }
  return crc ^ 0xffffffff;
}

/// Shows the app on [store] in [language] at [dpr], inside [boundary];
/// with [bundles], a Backup can be picked on the first start.
Future<AppLocalizations> _start(
  WidgetTester tester,
  GlobalKey boundary,
  String language,
  Store store,
  double dpr, {
  FakeAlarms? alarms,
  BundlePlatform bundles = const NoBundles(),
  FolderPlatform folders = const NoFolders(),
  AppointmentSource appointments = const NoAppointments(),
  SpeechPlatform speech = const NoSpeech(),
  Size screen = _screen,
  FakeViewPadding padding = FakeViewPadding.zero,
}) async {
  tester.view
    ..physicalSize = screen * dpr
    ..devicePixelRatio = dpr
    ..padding = padding
    ..viewPadding = padding;
  tester.platformDispatcher.localesTestValue = [Locale(language)];
  addTearDown(tester.view.reset);
  addTearDown(tester.platformDispatcher.clearLocalesTestValue);
  addTearDown(store.close);
  await tester.pumpWidget(
    RepaintBoundary(
      key: boundary,
      child: OldBrainFixApp(
        store: store,
        alarms: alarms ?? FakeAlarms(),
        voice: FakeVoice(),
        photos: FakePhotos(),
        bundles: bundles,
        folders: folders,
        appointments: appointments,
        speech: speech,
      ),
    ),
  );
  // The brain in the Agenda's title, decoded before the frame.
  final context = tester.element(find.byType(Navigator));
  await tester.runAsync(
    () => precacheImage(const AssetImage('assets/brain.png'), context),
  );
  await tester.pumpAndSettle();
  return AppLocalizations.of(context);
}

Future<void> _tap(WidgetTester tester, Finder finder) async {
  await tester.tap(finder);
  await tester.pumpAndSettle();
}

/// What [boundary] shows now at [dpr]: as PNG and as raw RGBA.
Future<({int width, int height, Uint8List png, Uint8List rgba})> _capture(
  WidgetTester tester,
  GlobalKey boundary,
  double dpr,
) async {
  await tester.pumpAndSettle();
  final render =
      boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final shot = await tester.runAsync(() async {
    final image = await render.toImage(pixelRatio: dpr);
    try {
      final png = await image.toByteData(format: ui.ImageByteFormat.png);
      final rgba = await image.toByteData();
      return (
        width: image.width,
        height: image.height,
        png: png!.buffer.asUint8List(),
        rgba: rgba!.buffer.asUint8List(),
      );
    } finally {
      image.dispose();
    }
  });
  return shot!;
}

/// The raw RGBA of the PNG [bytes], with its size.
Future<({int width, int height, Uint8List rgba})> _decode(
  WidgetTester tester,
  Uint8List bytes,
) async {
  final decoded = await tester.runAsync(() async {
    final codec = await ui.instantiateImageCodec(bytes);
    final image = (await codec.getNextFrame()).image;
    try {
      final rgba = await image.toByteData();
      return (
        width: image.width,
        height: image.height,
        rgba: rgba!.buffer.asUint8List(),
      );
    } finally {
      image.dispose();
      codec.dispose();
    }
  });
  return decoded!;
}

/// Compares what [boundary] shows with the committed guide frame at
/// [path], or writes it there with UPDATE_FRAMES=1. A changed frame fails
/// and is written to build/guide-frames/ for a look.
Future<void> _matchFrame(
  WidgetTester tester,
  GlobalKey boundary,
  String path, {
  double dpr = _guideDpr,
}) async {
  final shot = await _capture(tester, boundary, dpr);
  final file = File(path);
  if (Platform.environment['UPDATE_FRAMES'] == '1') {
    file.parent.createSync(recursive: true);
    file.writeAsBytesSync(shot.png);
    return;
  }
  String? problem;
  if (!file.existsSync()) {
    problem = 'no committed frame';
  } else {
    final old = await _decode(tester, file.readAsBytesSync());
    if ((old.width, old.height) != (shot.width, shot.height)) {
      problem =
          'size ${shot.width} × ${shot.height}, '
          'committed ${old.width} × ${old.height}';
    } else {
      var differing = 0;
      for (var i = 0; i < shot.rgba.length; i += 4) {
        for (var c = 0; c < 4; c++) {
          if ((shot.rgba[i + c] - old.rgba[i + c]).abs() > _channelSlack) {
            differing++;
            break;
          }
        }
      }
      if (differing > _pixelSlack) problem = '$differing pixels differ';
    }
  }
  if (problem != null) {
    final fresh = File('build/guide-frames/${file.path}')
      ..parent.createSync(recursive: true)
      ..writeAsBytesSync(shot.png);
    fail(
      '$path no longer matches its screen ($problem); the new frame is '
      '${fresh.path}. Regenerate with tool/guide-frames.sh.',
    );
  }
}

/// Runs [body] as test [name] with real shadows and counted ids: the
/// embossed cards are flat otherwise, and the own Profile's placeholder
/// colour, made from its id, is the same on every run. Restored before
/// the test ends, which checks it. With [platform], the app runs as on
/// that platform.
void _shot(
  String name,
  Future<void> Function(WidgetTester) body, {
  TargetPlatform? platform,
}) {
  testWidgets(name, (tester) async {
    debugDefaultTargetPlatformOverride = platform;
    debugDisableShadows = false;
    EditableText.debugDeterministicCursor = true;
    // Bare screens: no debug banner in the corner.
    WidgetsApp.debugAllowBannerOverride = false;
    final random = Store.idMaker;
    var ids = 0;
    Store.idMaker = () => 'shot${++ids}';
    try {
      await body(tester);
    } finally {
      debugDefaultTargetPlatformOverride = null;
      debugDisableShadows = true;
      EditableText.debugDeterministicCursor = false;
      WidgetsApp.debugAllowBannerOverride = true;
      Store.idMaker = random;
    }
  });
}

/// The store frames of one store, eight per locale: Play's and
/// F-Droid's 1080 × 1920 frames, or with [iphone] the App Store's
/// 1320 × 2868 ones (#244), drawn as on iOS: the iOS platform, so pushed
/// pages carry the Cupertino back arrow and the edge back swipe, its
/// safe area, Roboto standing in for the system font, and opaque PNGs,
/// as App Store Connect takes no alpha channel. The iPhone has no alarm
/// ring screen, as AlarmKit's own screen replaces it, so its sixth frame
/// is All Routines and Reminders.
void _storeScreenshots({required bool iphone}) {
  group(
    iphone ? 'iphone screenshots' : 'store screenshots',
    skip: 'writes PNGs; run with --run-skipped',
    () {
      setUpAll(() async {
        await _loadFonts();
        if (iphone) await _loadCupertinoFonts();
      });
      final screen = iphone ? _iphoneScreen : _screen;
      final dpr = iphone ? _iphoneDpr : _dpr;
      final size = (
        (screen.width * dpr).round(),
        (screen.height * dpr).round(),
      );

      for (final (locale, language, texts) in [
        ('en-US', 'en', _en),
        ('de-DE', 'de', _de),
      ]) {
        final out = Directory(
          iphone
              ? 'fastlane/metadata/ios/$locale/screenshots/iphone-6.9'
              : 'fastlane/metadata/android/$locale/images/phoneScreenshots',
        );
        final boundary = GlobalKey();

        Future<AppLocalizations> start(
          WidgetTester tester,
          _Seeded seeded, {
          FakeAlarms? alarms,
          AppointmentSource appointments = const NoAppointments(),
        }) => _start(
          tester,
          boundary,
          language,
          seeded.store,
          dpr,
          alarms: alarms,
          appointments: appointments,
          screen: screen,
          padding: iphone ? _iphoneSafeArea : FakeViewPadding.zero,
        );

        Future<void> tap(WidgetTester tester, Finder finder) =>
            _tap(tester, finder);

        /// Writes what the app shows now as [name].
        Future<void> shoot(WidgetTester tester, String name) async {
          final shot = await _capture(tester, boundary, dpr);
          expect((shot.width, shot.height), size);
          out.createSync(recursive: true);
          File('${out.path}/$name').writeAsBytesSync(
            iphone ? _opaquePng(shot.width, shot.height, shot.rgba) : shot.png,
          );
        }

        void frame(String name, Future<void> Function(WidgetTester) body) {
          _shot('$locale $name', platform: iphone ? TargetPlatform.iOS : null, (
            tester,
          ) async {
            await body(tester);
            await shoot(tester, name);
          });
        }

        /// On the iPhone, a pushed page goes back with iOS's arrow.
        void backArrow() {
          if (!iphone) return;
          expect(find.byIcon(Icons.arrow_back_ios_new_rounded), findsWidgets);
          expect(find.byIcon(Icons.arrow_back), findsNothing);
        }

        frame('1-agenda.png', (tester) async {
          await start(tester, _Seeded(texts));
          expect(find.byType(AgendaScreen), findsOneWidget);
        });

        frame('2-capture.png', (tester) async {
          final t = await start(tester, _Seeded(texts));
          await openTextCapture(tester);
          await tester.enterText(find.byType(TextField), texts.captureText);
          await tester.pumpAndSettle();
          expect(find.text(t.voice), findsOneWidget);
          expect(find.text(t.photo), findsOneWidget);
        });

        frame('3-diary.png', (tester) async {
          final t = await start(tester, _Seeded(texts));
          await tap(tester, find.byTooltip(t.browseTooltip));
          await tap(tester, find.text(t.starterHousehold));
          backArrow();
        });

        frame('4-editor.png', (tester) async {
          final t = await start(tester, _Seeded(texts));
          await tap(tester, find.byTooltip(t.browseTooltip));
          await tap(tester, find.text(t.starterHousehold));
          await tester.longPress(find.text(texts.boiler));
          await tester.pumpAndSettle();
          expect(find.byType(NoteEditorScreen), findsOneWidget);
          backArrow();
        });

        if (iphone) {
          frame('6-routines.png', (tester) async {
            await start(tester, _Seeded(texts));
            await tap(tester, find.byKey(const ValueKey('allRoutines')));
            expect(find.byType(AllRoutinesScreen), findsOneWidget);
            backArrow();
          });
        } else {
          frame('6-alarm.png', (tester) async {
            final seeded = _Seeded(texts);
            final alarms = FakeAlarms()
              ..ringingNow = Ringing(
                routineId: seeded.snack.id,
                occurrenceKey: Occurrence(
                  Day(2026, 10, 27),
                  ClockTime(15, 0),
                ).key,
                title: texts.snack,
                body: '15:00',
              );
            await start(tester, seeded, alarms: alarms);
            expect(find.byType(RingScreen), findsOneWidget);
          });
        }

        frame('7-history.png', (tester) async {
          await start(tester, _Seeded(texts));
          // From the row's menu, as a tap opens the Note (#270).
          await tester.longPress(find.text(texts.snack).first);
          await tester.pumpAndSettle();
          await tap(tester, find.byKey(const ValueKey('row-history')));
          expect(find.byType(RoutineHistoryScreen), findsOneWidget);
          backArrow();
        });

        frame('5-fan.png', (tester) async {
          await start(tester, _Seeded(texts));
          await tap(tester, find.byIcon(Icons.add));
          for (final way in ['text', 'voice', 'photo', 'gallery']) {
            expect(fanEntry(way), findsOneWidget);
          }
        });

        frame('8-dark.png', (tester) async {
          final seeded = _Seeded(texts);
          seeded.store.themeChoice = ThemeChoice.dark;
          final t = await start(tester, seeded);
          // Scrolled to the rest of the week, dimmed.
          Scrollable.ensureVisible(tester.element(find.text(t.agendaTomorrow)));
          await tester.pumpAndSettle();
          expect(find.byType(AgendaScreen), findsOneWidget);
        });
      }
    },
  );
}

void main() {
  _storeScreenshots(iphone: false);
  _storeScreenshots(iphone: true);

  group(
    'guide frames',
    skip: 'compares the frames; run with --run-skipped',
    () {
      setUpAll(_loadFonts);

      // The pictures of "What the icons mean" (#180), one per group and
      // language, light, written into the app's assets.
      for (final language in ['en', 'de']) {
        for (final group in helpIconGroups) {
          _shot('$language help ${group.slug}', (tester) async {
            final boundary = GlobalKey();
            const dpr = 2.0;
            tester.view
              ..physicalSize = HelpIconsPicture.sizeOf(group) * dpr
              ..devicePixelRatio = dpr;
            addTearDown(tester.view.reset);
            await tester.pumpWidget(
              MaterialApp(
                debugShowCheckedModeBanner: false,
                locale: Locale(language),
                localizationsDelegates: AppLocalizations.localizationsDelegates,
                supportedLocales: AppLocalizations.supportedLocales,
                theme: lookTheme(Brightness.light),
                home: RepaintBoundary(
                  key: boundary,
                  child: HelpIconsPicture(group: group),
                ),
              ),
            );
            await _matchFrame(
              tester,
              boundary,
              group.asset(language),
              dpr: dpr,
            );
          });
        }
      }

      for (final (language, texts) in [('en', _en), ('de', _de)]) {
        for (final (theme, choice) in [
          ('light', ThemeChoice.light),
          ('dark', ThemeChoice.dark),
        ]) {
          final boundary = GlobalKey();

          /// The seeded store in this frame's theme.
          _Seeded seed() => _Seeded(texts)..store.themeChoice = choice;

          Future<AppLocalizations> start(WidgetTester tester, _Seeded seeded) =>
              _start(tester, boundary, language, seeded.store, _guideDpr);

          /// The first start of a Device that holds nothing yet, in this
          /// frame's theme, a Backup file pickable.
          Future<AppLocalizations> firstStart(WidgetTester tester) => _start(
            tester,
            boundary,
            language,
            Store.inMemory(clock: () => _now)..themeChoice = choice,
            _guideDpr,
            bundles: FakeBundles(Directory('build')),
          );

          /// The guide frame [name]: [body] brings the app to its screen.
          void frame(String name, Future<void> Function(WidgetTester) body) {
            _shot('$language $theme $name', (tester) async {
              await body(tester);
              await _matchFrame(
                tester,
                boundary,
                'site/assets/frames/phone/$language/$theme/$name.png',
              );
            });
          }

          /// The frame of the box [box] on the guide page [page] (#171),
          /// as the site's box names it: <page>/<box>.png. A page's
          /// ticket only adds its scenes here.
          void boxFrame(
            String page,
            String box,
            Future<void> Function(WidgetTester) body,
          ) => frame('$page/$box', body);

          frame('agenda', (tester) async {
            await start(tester, seed());
            expect(find.byType(AgendaScreen), findsOneWidget);
          });

          // Getting started: the first start's choices, the Import rows
          // and the alarm setup.
          boxFrame('getting-started', 'start-new', (tester) async {
            await firstStart(tester);
            await _tap(tester, find.byKey(const ValueKey('first-new')));
            expect(find.byType(StarterSetScreen), findsOneWidget);
          });

          boxFrame('getting-started', 'add-device', (tester) async {
            await firstStart(tester);
            await _tap(tester, find.byKey(const ValueKey('first-device')));
            expect(find.byType(SyncScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('show-code')), findsOneWidget);
          });

          boxFrame('getting-started', 'restore', (tester) async {
            await firstStart(tester);
            expect(find.byType(FirstStartScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('restore-offer')), findsOneWidget);
          });

          /// The phone's Settings scrolled to its Import rows, a folder
          /// pickable.
          Future<void> importRows(WidgetTester tester) async {
            final dir = Directory('build/guide-import')
              ..createSync(recursive: true);
            final t = await _start(
              tester,
              boundary,
              language,
              seed().store,
              _guideDpr,
              folders: DirFolders(dir.path),
            );
            await _tap(tester, find.byTooltip(t.profileTitle));
            final settings = find.byKey(const ValueKey('deviceSettings'));
            await tester.ensureVisible(settings);
            await _tap(tester, settings);
            final keep = find.byKey(const ValueKey('importKeep'));
            await tester.scrollUntilVisible(
              keep,
              300,
              scrollable: find
                  .descendant(
                    of: find.byType(SettingsScreen),
                    matching: find.byType(Scrollable),
                  )
                  .first,
            );
            await tester.pumpAndSettle();
          }

          boxFrame('getting-started', 'import-folder', (tester) async {
            await importRows(tester);
            expect(find.byKey(const ValueKey('importFolder')), findsOneWidget);
          });

          boxFrame('getting-started', 'import-keep', (tester) async {
            await importRows(tester);
            await _tap(tester, find.byKey(const ValueKey('importKeep')));
            expect(find.byKey(const ValueKey('importKeepZip')), findsOneWidget);
          });

          boxFrame('getting-started', 'set-up', (tester) async {
            final t = await firstStart(tester);
            await _tap(tester, find.byKey(const ValueKey('first-new')));
            await _tap(tester, find.text(t.starterSetDone));
            expect(find.byType(SetupScreen), findsOneWidget);
          });

          // Capture and sort: the fan of "+", a thought typed as a quick
          // entry opens it, the quick note filed into the Household, the
          // Unsorted Notes, the Note editor and its History.
          boxFrame('capture-and-sort', 'capture', (tester) async {
            await start(tester, seed());
            await _tap(tester, find.byIcon(Icons.add));
            for (final way in ['text', 'voice', 'photo', 'gallery', 'paste']) {
              expect(fanEntry(way), findsOneWidget);
            }
          });

          boxFrame('capture-and-sort', 'capture-outside', (tester) async {
            await start(tester, seed());
            await openTextCapture(tester);
            await tester.enterText(find.byType(TextField), texts.captureText);
            await tester.pumpAndSettle();
            expect(find.byType(CaptureScreen), findsOneWidget);
            expect(find.byType(NudgeButtons), findsOneWidget);
          });

          boxFrame('capture-and-sort', 'sort-while-capturing', (tester) async {
            final seeded = seed();
            // Settings ▸ Quick note: File into…, the alarm clock and the
            // Importance switched on (#142).
            for (final control in [
              QuickNoteControl.fileInto,
              QuickNoteControl.schedule,
              QuickNoteControl.importance,
            ]) {
              seeded.store.setQuickNoteShows(control, true);
            }
            // Filed into the Kitchen, then Work, before: on top (#272).
            final places = Places(seeded.store);
            final household = places.areas().firstWhere(
              (a) => a.starter == 'household',
            );
            seeded.store
              ..filedInto(
                places
                    .topics(household.id)
                    .firstWhere((t) => t.starter == 'kitchen')
                    .id,
              )
              ..filedInto(
                places.areas().firstWhere((a) => a.starter == 'work').id,
              );
            final t = await start(tester, seeded);
            await openTextCapture(tester);
            await tester.enterText(find.byType(TextField), texts.captureText);
            await _tap(
              tester,
              find.descendant(
                of: find.byKey(const ValueKey('note-actions')),
                matching: find.byTooltip(t.fileInto),
              ),
            );
            // File into… over the quick note: "→" has no glyph in the
            // frames' fonts, so the frame stops before the pick.
            expect(find.byType(FileIntoScreen), findsOneWidget);
          });

          boxFrame('capture-and-sort', 'sort-later', (tester) async {
            await start(tester, seed());
            await _tap(tester, find.byKey(const ValueKey('unsortedBanner')));
            expect(find.byType(UnsortedScreen), findsOneWidget);
            // Two of them ticked for File into… in the bar (#273).
            await _tap(tester, find.byKey(const ValueKey('selectNotes')));
            final boxes = find.byWidgetPredicate(
              (w) =>
                  w is Checkbox &&
                  w.key is ValueKey<String> &&
                  (w.key! as ValueKey<String>).value.startsWith('select-'),
            );
            await _tap(tester, boxes.at(0));
            await _tap(tester, boxes.at(1));
          });

          /// The boiler's Note, with its two Amends, in the Note editor.
          Future<void> boilerEditor(WidgetTester tester) async {
            final t = await start(tester, seed());
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
            await tester.longPress(find.text(texts.boiler));
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
          }

          boxFrame('capture-and-sort', 'edit', (tester) async {
            await boilerEditor(tester);
            // The row of badges under the text (#160).
            expect(find.byKey(const ValueKey('note-badges')), findsOneWidget);
          });

          boxFrame('capture-and-sort', 'history', (tester) async {
            await boilerEditor(tester);
            await _tap(tester, find.byKey(const ValueKey('note-history')));
            expect(find.byType(NoteHistoryScreen), findsOneWidget);
          });

          // Routines and the Agenda (#173): a new Reminder typed from the
          // Agenda's bar with its chips, the snack's schedule on weekdays,
          // its Re-nag and Escalation, a Reminder due now in its Note's
          // editor, the Missed Summary, the snack's History, the Agenda
          // with its groups, All Routines and Reminders and the Calendar
          // page.
          boxFrame('routines-and-agenda', 'set-up', (tester) async {
            final t = await start(tester, seed());
            await _tap(tester, find.byTooltip(t.newRoutineTooltip));
            await tester.enterText(
              find.byType(TextField).first,
              texts.captureText,
            );
            await tester.pumpAndSettle();
            // Once, with the new Reminder's chips (#271).
            await _tap(tester, find.text(t.repeatOnce));
            expect(find.byKey(const ValueKey('quickReminders')), findsOne);
          });

          /// The snack's Routine in its editor, through its row's Edit
          /// schedule (#270).
          Future<AppLocalizations> snackEditor(WidgetTester tester) async {
            final t = await start(tester, seed());
            await tester.longPress(find.text(texts.snack).first);
            await tester.pumpAndSettle();
            await _tap(tester, find.byKey(const ValueKey('row-schedule')));
            expect(find.byType(RoutineEditorScreen), findsOneWidget);
            return t;
          }

          boxFrame('routines-and-agenda', 'schedule', (tester) async {
            final t = await snackEditor(tester);
            await _tap(tester, find.text(t.repeatWeekdays));
            // The weekdays under the kinds, the start and the end below.
            await tester.drag(
              find.byType(Scrollable).first,
              const Offset(0, -170),
            );
            await tester.pumpAndSettle();
          });

          boxFrame('routines-and-agenda', 're-nag', (tester) async {
            await snackEditor(tester);
            for (final key in ['renag', 'escalation', 'escalationOn']) {
              final control = find.byKey(ValueKey(key));
              await reveal(tester, control);
              await _tap(tester, control);
            }
            await reveal(tester, find.byKey(const ValueKey('escalationAfter')));
          });

          boxFrame('routines-and-agenda', 'check-off', (tester) async {
            final t = await start(
              tester,
              _Seeded(texts, dueReminder: true)..store.themeChoice = choice,
            );
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
            await tester.longPress(find.text(texts.boiler));
            await tester.pumpAndSettle();
            expect(find.byKey(const ValueKey('open-done')), findsOneWidget);
            expect(
              find.byKey(const ValueKey('open-done-trash')),
              findsOneWidget,
            );
          });

          boxFrame('routines-and-agenda', 'missed', (tester) async {
            await start(
              tester,
              _Seeded(texts, missed: true)..store.themeChoice = choice,
            );
            expect(find.byType(MissedSummaryCard), findsOneWidget);
          });

          boxFrame('routines-and-agenda', 'history', (tester) async {
            final t = await snackEditor(tester);
            await _tap(tester, find.byTooltip(t.historyTitle));
            expect(find.byType(RoutineHistoryScreen), findsOneWidget);
          });

          boxFrame('routines-and-agenda', 'agenda', (tester) async {
            await start(
              tester,
              _Seeded(texts, lists: true)..store.themeChoice = choice,
            );
            expect(find.byType(AgendaScreen), findsOneWidget);
          });

          boxFrame('routines-and-agenda', 'all-routines', (tester) async {
            await start(tester, seed());
            await _tap(tester, find.byKey(const ValueKey('allRoutines')));
            expect(find.byType(AllRoutinesScreen), findsOneWidget);
          });

          boxFrame('routines-and-agenda', 'calendar', (tester) async {
            await _start(
              tester,
              boundary,
              language,
              seed().store,
              _guideDpr,
              appointments: FakeAppointments(),
            );
            await _tap(tester, find.byKey(const ValueKey('calendarPage')));
            expect(find.byType(CalendarScreen), findsOneWidget);
          });

          // Lists, Tallies, Sessions and Frogs (#174): the fan of the
          // Household's "+", its cards compact, a plain Note's kinds, the
          // packing Checklist in the editor, the Shopping List's Agenda
          // choices and its card, a new Tally, a Tally's card, a new
          // Session and the Frog view.
          /// The Household with its Checklist, Shopping List and Tally.
          Future<void> household(WidgetTester tester) async {
            final t = await start(
              tester,
              _Seeded(texts, lists: true)..store.themeChoice = choice,
            );
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
          }

          /// The Household's "+" opened on what it makes (#161).
          Future<void> addFan(WidgetTester tester) async {
            await household(tester);
            final t = lookupAppLocalizations(Locale(language));
            await _tap(tester, find.byTooltip(t.addHere));
          }

          boxFrame('lists-tallies-sessions-frogs', 'make', (tester) async {
            await addFan(tester);
            expect(find.byKey(const ValueKey('addFan-tally')), findsOneWidget);
          });

          boxFrame('lists-tallies-sessions-frogs', 'tally', (tester) async {
            await addFan(tester);
            await _tap(tester, find.byKey(const ValueKey('addFan-tally')));
            Finder key(String k) => find.byKey(ValueKey(k));
            await tester.enterText(key('tally-form-name'), texts.exercises);
            await tester.enterText(key('tally-form-field-name-0'), texts.plank);
            await _tap(tester, key('tally-form-unit-0'));
            await tester.enterText(key('tally-unit-text'), 'min');
            await _tap(tester, key('badge-close'));
            await _tap(tester, key('tally-form-target-0'));
            await tester.enterText(key('tally-field-target'), '1');
            await _tap(tester, key('badge-close'));
            await _tap(tester, key('tally-form-add-field'));
            await tester.enterText(
              key('tally-form-field-name-1'),
              texts.squats,
            );
            FocusManager.instance.primaryFocus?.unfocus();
            await tester.pumpAndSettle();
            expect(find.text(texts.squats), findsOneWidget);
          });

          boxFrame('lists-tallies-sessions-frogs', 'session', (tester) async {
            await addFan(tester);
            await _tap(tester, find.byKey(const ValueKey('addFan-session')));
            await tester.enterText(
              find.byKey(const ValueKey('session-form-name')),
              texts.evening,
            );
            await _tap(tester, find.text(texts.reading));
            FocusManager.instance.primaryFocus?.unfocus();
            await tester.pumpAndSettle();
            expect(
              find.byKey(const ValueKey('session-new-tally')),
              findsOneWidget,
            );
          });

          /// The Household's list of Notes, scrolling down.
          final notesList = find.byWidgetPredicate(
            (w) => w is Scrollable && w.axisDirection == AxisDirection.down,
          );

          boxFrame('lists-tallies-sessions-frogs', 'shopping-list', (
            tester,
          ) async {
            await household(tester);
            // The lower part of the Checklist above the Shopping List,
            // its struck Lines at the end.
            final groceries = find.text(texts.groceries.split('\n').first);
            await tester.scrollUntilVisible(
              groceries,
              300,
              scrollable: notesList.first,
            );
            Scrollable.ensureVisible(tester.element(groceries), alignment: 0.3);
            await tester.pumpAndSettle();
            expect(groceries, findsOneWidget);
          });

          boxFrame('lists-tallies-sessions-frogs', 'checklist', (tester) async {
            await household(tester);
            // The packing Checklist in the editor (#184): one block of
            // text with boxes, the group in a frame with its own Line.
            final packing = find.text(texts.packing.split('\n').first);
            await tester.scrollUntilVisible(
              packing,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
            await tester.longPress(packing);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            final lines = find.byKey(const ValueKey('checklist'));
            Scrollable.ensureVisible(tester.element(lines), alignment: 0.2);
            await tester.pumpAndSettle();
            expect(lines, findsOneWidget);
          });

          boxFrame('lists-tallies-sessions-frogs', 'tally-entry', (
            tester,
          ) async {
            await household(tester);
            final reading = find.text(texts.reading);
            await tester.scrollUntilVisible(
              reading,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
            // The detailed card (#190): "Add entry", the length of a bar and
            // the chart of the pages read, with the target as a line.
            expect(
              find.text('${texts.pagesRead} (${texts.pages})'),
              findsOneWidget,
            );
            Scrollable.ensureVisible(tester.element(reading));
            await tester.pumpAndSettle();
          });

          boxFrame('lists-tallies-sessions-frogs', 'views', (tester) async {
            await household(tester);
            // Every card of the Household in its compact view (#189).
            await _tap(tester, find.byKey(const ValueKey('compactNotes')));
            expect(find.byType(NoteCard), findsWidgets);
          });

          boxFrame('lists-tallies-sessions-frogs', 'kind', (tester) async {
            await boilerEditor(tester);
            // The kind badge's choices for a plain Note (#188).
            final kind = find.byKey(const ValueKey('badge-kind'));
            await reveal(tester, kind);
            await _tap(tester, kind);
            expect(
              find.byKey(const ValueKey('kind-checklist')),
              findsOneWidget,
            );
          });

          boxFrame('lists-tallies-sessions-frogs', 'list-on-agenda', (
            tester,
          ) async {
            await household(tester);
            // The weekly Shopping List's "Show on the Agenda" (#186).
            final groceries = find.text(texts.groceries.split('\n').first);
            await tester.scrollUntilVisible(
              groceries,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
            await tester.longPress(groceries);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            final agenda = find.byKey(const ValueKey('badge-agenda'));
            await reveal(tester, agenda);
            await _tap(tester, agenda);
          });

          boxFrame('lists-tallies-sessions-frogs', 'frog', (tester) async {
            final t = await start(tester, seed());
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.byTooltip(t.frogsTitle));
            expect(find.byType(FrogsScreen), findsOneWidget);
          });

          // Formatted text, Transcripts and Photo Texts (#175): the format
          // bar, a script as Code, a table full screen, a box's menu, a
          // #tag's and a word's search, the speech model, a receipt's
          // Photo Text and a Transcript's buttons in the editor.
          const texts175 = 'formatted-text-transcripts-photo-texts';

          /// The Household with its texts, the newest on top, [speech]
          /// running Whisper with the base model.
          Future<AppLocalizations> formattedNotes(
            WidgetTester tester, {
            SpeechPlatform speech = const NoSpeech(),
          }) async {
            final t = await _start(
              tester,
              boundary,
              language,
              (_Seeded(
                texts,
                formatted: true,
              )..store.themeChoice = choice).store,
              _guideDpr,
              speech: speech,
            );
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
            return t;
          }

          /// The card's text of [text], as drawn on the list.
          Finder cardText(String text) =>
              find.byWidgetPredicate((w) => w is MarkedText && w.text == text);

          /// Scrolls the Household's list to [finder].
          Future<void> scrollTo(WidgetTester tester, Finder finder) async {
            await tester.scrollUntilVisible(
              finder,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
          }

          boxFrame(texts175, 'format', (tester) async {
            await formattedNotes(tester);
            final boiler = cardText(texts.formatted);
            await scrollTo(tester, boiler);
            await tester.longPress(boiler.first);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            // The keyboard up, drawn as nothing: the format bar and the
            // cursor's line show at the bottom, the cursor in the bold.
            final field = find.byType(TextField).first;
            await tester.showKeyboard(field);
            tester.view.viewInsets = const FakeViewPadding(bottom: 1);
            addTearDown(tester.view.resetViewInsets);
            final controller = tester.widget<TextField>(field).controller!;
            final bold = controller.text.indexOf('**') + 6;
            controller.selection = TextSelection.collapsed(offset: bold);
            await tester.pumpAndSettle();
            expect(find.byKey(const ValueKey('format-bar')), findsOneWidget);
            expect(find.byKey(const ValueKey('two-row-bar')), findsOneWidget);
          });

          boxFrame(texts175, 'code', (tester) async {
            await formattedNotes(tester);
            // The backup script's Text format badge open (#213).
            final script = find.textContaining(texts.script.split('\n')[1]);
            await scrollTo(tester, script);
            await tester.longPress(script.first);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            final format = find.byKey(const ValueKey('badge-format'));
            await reveal(tester, format);
            await _tap(tester, format);
          });

          boxFrame(texts175, 'table', (tester) async {
            await formattedNotes(tester);
            final meters = cardText(texts.meters);
            await scrollTo(tester, meters);
            await tester.longPress(meters.first);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            // The keyboard up, drawn as nothing: a cell of the table is
            // edited in its grid in the text (#257), the bar of its words'
            // formats above the keyboard.
            await tester.showKeyboard(find.byType(TextField).first);
            tester.view.viewInsets = const FakeViewPadding(bottom: 1);
            addTearDown(tester.view.resetViewInsets);
            await tester.pumpAndSettle();
            final cell = find.byKey(const ValueKey('grid-cell-1-1'));
            await _tap(tester, cell);
            expect(
              find.descendant(of: cell, matching: find.byType(TextField)),
              findsOneWidget,
            );
            expect(find.byKey(const ValueKey('format-bar')), findsOneWidget);
          });

          boxFrame(texts175, 'box', (tester) async {
            await formattedNotes(tester);
            // The History of the holiday's first box, failed and checked
            // again; its menu offers ☐, which no font here draws.
            final box = find.byWidgetPredicate((w) {
              final key = w.key;
              // A box's button is keyed by its id, four letters or
              // digits; its menu's entries by their state.
              return key is ValueKey<String> &&
                  RegExp(r'^box-[0-9a-z]{4}$').hasMatch(key.value) &&
                  !const {'box-open', 'box-done'}.contains(key.value);
            });
            final heading = texts.checks.split('\n').first;
            final checks = find.byWidgetPredicate(
              (w) => w is MarkedText && w.text.startsWith(heading),
            );
            await scrollTo(tester, checks);
            await _tap(tester, box.first);
            await _tap(tester, find.byKey(const ValueKey('box-history')));
            expect(
              find.byKey(const ValueKey('box-history-dialog')),
              findsOneWidget,
            );
          });

          /// The search of all Notes for [query], as typed.
          Future<void> search(WidgetTester tester, String query) async {
            final t = await _start(
              tester,
              boundary,
              language,
              (_Seeded(
                texts,
                formatted: true,
              )..store.themeChoice = choice).store,
              _guideDpr,
            );
            await _tap(tester, find.byTooltip(t.browseTooltip));
            final field = find.widgetWithText(TextField, t.searchAllHint);
            await tester.enterText(field, query);
            await tester.pumpAndSettle();
          }

          boxFrame(texts175, 'tag', (tester) async {
            await search(tester, texts.tag);
          });

          boxFrame(texts175, 'search', (tester) async {
            await search(tester, texts.search);
          });

          /// The Speech model page, opened from Settings.
          Future<void> speechModel(WidgetTester tester) async {
            final t = await _start(
              tester,
              boundary,
              language,
              seed().store,
              _guideDpr,
              speech: FakeSpeech(installed: WhisperModel.base.sha256),
            );
            await _tap(tester, find.byTooltip(t.profileTitle));
            final settings = find.byKey(const ValueKey('deviceSettings'));
            await tester.ensureVisible(settings);
            await _tap(tester, settings);
            final model = find.text(t.speechModelTitle);
            await tester.scrollUntilVisible(
              model,
              300,
              scrollable: find
                  .descendant(
                    of: find.byType(SettingsScreen),
                    matching: find.byType(Scrollable),
                  )
                  .first,
            );
            await tester.pumpAndSettle();
            await _tap(tester, model);
            expect(find.byType(SpeechModelScreen), findsOneWidget);
          }

          boxFrame(texts175, 'transcript', speechModel);

          boxFrame(texts175, 'photo-text', (tester) async {
            await formattedNotes(tester);
            // The receipt's card, its photo decoded, its Photo Text under
            // it.
            final receipt = cardText(texts.receipt);
            await scrollTo(tester, receipt);
            await tester.runAsync(
              () => Future<void>.delayed(const Duration(milliseconds: 500)),
            );
            await tester.pumpAndSettle();
            await tester.ensureVisible(receipt.first);
            await tester.pumpAndSettle();
          });

          boxFrame(texts175, 'generated', (tester) async {
            await formattedNotes(
              tester,
              speech: FakeSpeech(installed: WhisperModel.base.sha256),
            );
            // The voice note's editor, its Transcript's buttons in view.
            final spoken = find.textContaining(texts.spoken);
            await scrollTo(tester, spoken);
            await tester.longPress(spoken.first);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            final t = AppLocalizations.of(
              tester.element(find.byType(NoteEditorScreen)),
            );
            await reveal(tester, find.text(t.transcribeAgain));
          });

          // Sharing and sync (#176): the Profile page, Add a person,
          // Lena's Profile, sharing the Household in person and from
          // afar, Work's People, a card's ⋮ and Send this Note, Lena's
          // Sent Item, Sync with its shared folder and bundles, the
          // Changes Nudge's button, a Conflict and the paired Laptop.
          const sharing176 = 'sharing-and-sync';

          /// The app on [seeded] in this frame's theme, a bundle and a
          /// folder pickable; [waiting] opens as a file handed over.
          Future<AppLocalizations> sharingApp(
            WidgetTester tester,
            _Seeded seeded, {
            List<int>? waiting,
            String waitingName = 'lena.oldbrain',
          }) {
            final bundles = FakeBundles(Directory('build'));
            if (waiting != null) {
              final file = File('build/guide-sent/$waitingName')
                ..parent.createSync(recursive: true)
                ..writeAsBytesSync(waiting);
              bundles.waiting.add(file.path);
            }
            final dir = Directory('build/guide-folder')
              ..createSync(recursive: true);
            seeded.store.themeChoice = choice;
            return _start(
              tester,
              boundary,
              language,
              seeded.store,
              _guideDpr,
              bundles: bundles,
              folders: DirFolders(dir.path),
            );
          }

          /// The Profile page scrolled to [finder], which is tapped with
          /// [open].
          Future<void> onProfile(
            WidgetTester tester,
            AppLocalizations t,
            Finder finder, {
            bool open = true,
          }) async {
            await _tap(tester, find.byTooltip(t.profileTitle));
            await tester.scrollUntilVisible(
              finder,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
            if (open) await _tap(tester, finder);
          }

          /// Sync, opened from the Profile page, scrolled to [finder].
          Future<void> openSync(
            WidgetTester tester,
            AppLocalizations t, [
            Finder? finder,
          ]) async {
            await onProfile(tester, t, find.byKey(const ValueKey('sync')));
            expect(find.byType(SyncScreen), findsOneWidget);
            if (finder == null) return;
            await tester.scrollUntilVisible(
              finder,
              300,
              scrollable: notesList.last,
            );
            await tester.pumpAndSettle();
          }

          /// The share page of [slug]'s Area, opened from its ⋮.
          Future<void> shareArea(
            WidgetTester tester,
            _Seeded seeded,
            AppLocalizations t,
          ) async {
            final household = Places(seeded.store)
                .areas()
                .firstWhere((a) => a.starter == 'household');
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(
              tester,
              find.byKey(ValueKey('area-menu-${household.id}')),
            );
            await _tap(tester, find.text(t.shareArea));
            expect(find.byType(ShareAreaScreen), findsOneWidget);
          }

          /// The boiler's card in the Household with its ⋮ open.
          Future<AppLocalizations> boilerMenu(WidgetTester tester) async {
            final t = await sharingApp(tester, _Seeded(texts, people: true));
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
            final card = find.ancestor(
              of: find.text(texts.boiler),
              matching: find.byType(NoteCard),
            );
            await _tap(
              tester,
              find.descendant(
                of: card.first,
                matching: find.byKey(const ValueKey('note-menu')),
              ),
            );
            // Send and Share to an app are under Share… (#281).
            await _tap(tester, find.byKey(const ValueKey('note-share-menu')));
            return t;
          }

          boxFrame(sharing176, 'send-profile', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, people: true));
            await _tap(tester, find.byTooltip(t.profileTitle));
            expect(find.byKey(const ValueKey('sendProfile')), findsOneWidget);
          });

          boxFrame(sharing176, 'add-person', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, people: true));
            await onProfile(tester, t, find.byKey(const ValueKey('addPerson')));
            expect(find.byType(AddPersonScreen), findsOneWidget);
          });

          boxFrame(sharing176, 'verify', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, people: true));
            await onProfile(tester, t, find.text('Lena'));
            expect(find.byType(PersonScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('compare-codes')), findsOneWidget);
          });

          boxFrame(sharing176, 'share-in-person', (tester) async {
            final seeded = _Seeded(texts, people: true);
            final t = await sharingApp(tester, seeded);
            await shareArea(tester, seeded, t);
            expect(find.byKey(const ValueKey('share-scan')), findsOneWidget);
          });

          boxFrame(sharing176, 'share-afar', (tester) async {
            final seeded = _Seeded(texts, people: true);
            final t = await sharingApp(tester, seeded);
            await shareArea(tester, seeded, t);
            // The role texts push "Share with…" below the screen's edge.
            final afar = find.byKey(const ValueKey('share-afar'));
            await tester.scrollUntilVisible(
              afar,
              100,
              scrollable: notesList.last,
            );
            // Clear of the screen's edge for the tap.
            await tester.drag(notesList.last, const Offset(0, -200));
            await tester.pumpAndSettle();
            await _tap(tester, afar);
            expect(find.byType(SharePersonScreen), findsOneWidget);
          });

          boxFrame(sharing176, 'roles', (tester) async {
            final seeded = _Seeded(texts, shared: true);
            final t = await sharingApp(tester, seeded);
            final work = Places(seeded.store)
                .areas()
                .firstWhere((a) => a.starter == 'work');
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.byKey(ValueKey('area-menu-${work.id}')));
            await _tap(tester, find.text(t.peopleTitle).last);
            expect(find.byType(PeopleScreen), findsOneWidget);
            await _tap(
              tester,
              find
                  .byWidgetPredicate((w) => '${w.key}'.contains('person-menu-'))
                  .first,
            );
            expect(find.text(t.makeManager), findsOneWidget);
          });

          boxFrame(sharing176, 'share-to-app', (tester) async {
            await boilerMenu(tester);
            expect(find.byKey(const ValueKey('note-share')), findsOneWidget);
          });

          boxFrame(sharing176, 'send-note', (tester) async {
            await boilerMenu(tester);
            await _tap(tester, find.byKey(const ValueKey('note-send')));
            expect(find.byType(SendNoteDialog), findsOneWidget);
          });

          boxFrame(sharing176, 'receive-note', (tester) async {
            final seeded = _Seeded(texts, sends: true);
            await sharingApp(tester, seeded, waiting: seeded.sent);
            expect(find.byType(SentItemScreen), findsOneWidget);
          });

          // Packs (#164): the Household's tree, and Lena's Pack opened.
          boxFrame(sharing176, 'send-pack', (tester) async {
            final seeded = _Seeded(texts, people: true);
            final t = await sharingApp(tester, seeded);
            final household = Places(seeded.store)
                .areas()
                .firstWhere((a) => a.starter == 'household');
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(
              tester,
              find.byKey(ValueKey('area-menu-${household.id}')),
            );
            await _tap(tester, find.text(t.exportAsPack));
            expect(find.byType(PackExportScreen), findsOneWidget);
          });

          boxFrame(sharing176, 'open-pack', (tester) async {
            // The Pack's work inline, as the frame's clock is the test's.
            final work = packWork;
            packWork = <R>(work) async => work();
            addTearDown(() => packWork = work);
            final seeded = _Seeded(texts, packs: true);
            await sharingApp(
              tester,
              seeded,
              waiting: seeded.pack,
              waitingName: 'lena.obfpack',
            );
            expect(find.byType(PackImportScreen), findsOneWidget);
          });

          boxFrame(sharing176, 'pairing', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts));
            await openSync(tester, t);
            expect(find.byKey(const ValueKey('show-code')), findsOneWidget);
          });

          boxFrame(sharing176, 'shared-folder', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts));
            await openSync(
              tester,
              t,
              find.byKey(const ValueKey('pick-folder')),
            );
          });

          boxFrame(sharing176, 'bundle', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts));
            await openSync(
              tester,
              t,
              find.byKey(const ValueKey('import-bundle')),
            );
          });

          boxFrame(sharing176, 'send-changes', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, shared: true));
            await onProfile(tester, t, find.text('Lena'));
            expect(find.byType(PersonScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('sendChanges')), findsOneWidget);
          });

          boxFrame(sharing176, 'conflicts', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, conflict: true));
            await openSync(tester, t);
            await _tap(tester, find.byKey(const ValueKey('conflicts')));
            expect(find.byType(ConflictsScreen), findsOneWidget);
            expect(find.text(texts.boilerThere), findsOneWidget);
          });

          boxFrame(sharing176, 'ai-task', (tester) async {
            // The phone's Sync with the Laptop paired: an AI Task's
            // changes come from there once accepted.
            final t = await sharingApp(tester, _Seeded(texts, conflict: true));
            await openSync(tester, t, find.text('Laptop'));
          });

          // Backups, locks and privacy (#177): Backups before and after
          // setting up, the App Lock and the Area Locks in Settings, the
          // question before blocking Lena, and the top of Settings.
          const privacy177 = 'backups-locks-and-privacy';

          /// The Backups page opened from the Profile page.
          Future<void> openBackups(
            WidgetTester tester,
            AppLocalizations t,
          ) async {
            await _tap(tester, find.byTooltip(t.profileTitle));
            await _tap(tester, find.byKey(const ValueKey('backups')));
            expect(find.byType(BackupScreen), findsOneWidget);
          }

          /// Settings opened from the Profile page.
          Future<void> openSettings(
            WidgetTester tester,
            AppLocalizations t,
          ) async {
            await _tap(tester, find.byTooltip(t.profileTitle));
            final settings = find.byKey(const ValueKey('deviceSettings'));
            await tester.ensureVisible(settings);
            await _tap(tester, settings);
            expect(find.byType(SettingsScreen), findsOneWidget);
          }

          /// Settings scrolled to Privacy, its heading on top.
          Future<void> toPrivacy(
            WidgetTester tester,
            AppLocalizations t,
          ) async {
            final heading = find.text(t.settingsPrivacy);
            await tester.scrollUntilVisible(
              heading,
              300,
              scrollable: notesList.last,
            );
            await tester.pumpAndSettle();
            await Scrollable.ensureVisible(tester.element(heading));
            await tester.pumpAndSettle();
          }

          boxFrame(privacy177, 'set-up', (tester) async {
            final t = await start(tester, seed());
            await openBackups(tester, t);
            expect(find.byKey(const ValueKey('set-up')), findsOneWidget);
          });

          boxFrame(privacy177, 'folder', (tester) async {
            final seeded = seed();
            await setUpBackups(seeded.store, 'correct horse', derive: quickKey);
            final dir = Directory('build/guide-backups')
              ..createSync(recursive: true);
            seeded.store
              ..backupFolder = dir.path
              ..setLocalSetting(
                'backup:last',
                DateTime(2026, 10, 25, 19, 30).toUtc().toIso8601String(),
              );
            final t = await _start(
              tester,
              boundary,
              language,
              seeded.store,
              _guideDpr,
              folders: DirFolders(dir.path),
            );
            await openBackups(tester, t);
            expect(find.byKey(const ValueKey('backup-now')), findsOneWidget);
          });

          boxFrame(privacy177, 'app-lock', (tester) async {
            final seeded = seed();
            final t = await start(tester, seeded);
            // Switched on once the app shows, so it starts unlocked.
            seeded.store
              ..setPin('2468', iterations: 1)
              ..appLockOn = true;
            await openSettings(tester, t);
            await toPrivacy(tester, t);
            expect(find.byKey(const ValueKey('changePin')), findsOneWidget);
          });

          boxFrame(privacy177, 'area-locks', (tester) async {
            final seeded = seed();
            final places = Places(seeded.store);
            String area(String slug) =>
                places.areas().firstWhere((a) => a.starter == slug).id;
            seeded.store
              ..setPin('2468', iterations: 1)
              ..setAreaLocked(area('household'), true)
              ..setAreaLocked(area('work'), true)
              ..setAreaMasked(area('work'), false);
            final t = await start(tester, seeded);
            await openSettings(tester, t);
            await toPrivacy(tester, t);
            await _tap(tester, find.byKey(const ValueKey('areaLocks')));
            expect(find.byType(AreaLocksScreen), findsOneWidget);
          });

          boxFrame(privacy177, 'block', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, people: true));
            await onProfile(tester, t, find.text('Lena'));
            await _tap(tester, find.byKey(const ValueKey('block-person')));
            expect(find.byKey(const ValueKey('block-confirm')), findsOneWidget);
          });

          boxFrame(privacy177, 'privacy', (tester) async {
            final t = await start(tester, seed());
            await openSettings(tester, t);
            expect(find.text(t.settingsIntro), findsOneWidget);
          });

          // Send feedback (#214), with words and the phone's facts held:
          // the page as Settings opens it.
          boxFrame(privacy177, 'feedback', (tester) async {
            final t = await start(tester, seed());
            await openSettings(tester, t);
            final settings = tester.state<NavigatorState>(
              find.byType(Navigator).first,
            );
            unawaited(
              settings.push(
                MaterialPageRoute<void>(
                  builder: (_) => FeedbackScreen(
                    store: tester
                        .widget<SettingsScreen>(find.byType(SettingsScreen))
                        .store,
                    facts: () async => const FeedbackFacts(
                      version: '0.4.0+57',
                      system: 'Android 15 (SDK 35), Google Pixel 8',
                      install: 'F-Droid',
                    ),
                    crash: () => null,
                    mail: (_) async {},
                  ),
                ),
              ),
            );
            await tester.pumpAndSettle();
            await tester.enterText(
              find.byKey(const ValueKey('feedback-text')),
              language == 'de'
                  ? 'Die Agenda zeigt die Routine von gestern als verpasst.'
                  : "The Agenda shows yesterday's Routine as Missed.",
            );
            await tester.pumpAndSettle();
            // The ticks' end and the preview with its buttons in view.
            final preview = find.text(t.feedbackPreview);
            await tester.scrollUntilVisible(
              preview,
              200,
              scrollable: find.byType(Scrollable).last,
            );
            await tester.drag(
              find.byType(Scrollable).last,
              const Offset(0, -300),
            );
            await tester.pumpAndSettle();
            expect(find.byType(FeedbackScreen), findsOneWidget);
          });

          // The desktop app (#178): the phone's own way where it has one,
          // else the Agenda, as the box is the desktop's alone.
          const desktop178 = 'desktop-app';

          Future<void> agenda(WidgetTester tester) async {
            await start(tester, seed());
            expect(find.byType(AgendaScreen), findsOneWidget);
          }

          boxFrame(desktop178, 'install', (tester) async {
            await firstStart(tester);
            expect(find.byType(FirstStartScreen), findsOneWidget);
          });

          boxFrame(desktop178, 'window', (tester) async {
            await addFan(tester);
            expect(find.byKey(const ValueKey('addFan-tally')), findsOneWidget);
          });

          boxFrame(desktop178, 'cards', household);

          boxFrame(desktop178, 'note-dialog', (tester) async {
            await boilerEditor(tester);
            expect(find.byKey(const ValueKey('note-badges')), findsOneWidget);
          });

          boxFrame(desktop178, 'menu-bar', (tester) async {
            // A Profile of fixed keys: its short code is the same on
            // every run.
            final t = await sharingApp(tester, _Seeded(texts, people: true));
            await _tap(tester, find.byTooltip(t.profileTitle));
            expect(
              find.byKey(const ValueKey('deviceSettings')),
              findsOneWidget,
            );
          });

          boxFrame(desktop178, 'dialogs', (tester) async {
            final t = await start(tester, seed());
            await openBackups(tester, t);
          });

          boxFrame(desktop178, 'keys', (tester) async {
            final t = await start(tester, seed());
            await _tap(tester, find.byTooltip(t.browseTooltip));
            expect(find.text(t.starterHousehold), findsOneWidget);
          });

          boxFrame(desktop178, 'tray', agenda);

          boxFrame(desktop178, 'hotkey', (tester) async {
            await start(tester, seed());
            await openTextCapture(tester);
            await tester.enterText(find.byType(TextField), texts.captureText);
            await tester.pumpAndSettle();
            expect(find.byType(CaptureScreen), findsOneWidget);
          });

          boxFrame(desktop178, 'command-line', agenda);

          boxFrame(desktop178, 'drop', (tester) async {
            await start(tester, seed());
            await _tap(tester, find.byIcon(Icons.add));
            expect(fanEntry('gallery'), findsOneWidget);
          });

          boxFrame(desktop178, 'notifications', (tester) async {
            final t = await firstStart(tester);
            await _tap(tester, find.byKey(const ValueKey('first-new')));
            await _tap(tester, find.text(t.starterSetDone));
            expect(find.byType(SetupScreen), findsOneWidget);
          });

          boxFrame(desktop178, 'speech', speechModel);

          boxFrame(desktop178, 'sync', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts));
            await openSync(tester, t);
            expect(find.byKey(const ValueKey('show-code')), findsOneWidget);
          });

          boxFrame(desktop178, 'ai-port', (tester) async {
            final t = await sharingApp(tester, _Seeded(texts, conflict: true));
            await openSync(tester, t, find.text('Laptop'));
          });

          boxFrame(desktop178, 'look', (tester) async {
            final t = await start(tester, seed());
            await openSettings(tester, t);
            expect(find.byKey(const ValueKey('fatFingers')), findsOneWidget);
          });

          boxFrame(desktop178, 'missing', agenda);

          // Sync with family and friends (#198): Sync at its shared
          // folder, where Syncthing's or Autosync's folder is chosen, and
          // the question how changes travel with Lena, the folder picked.
          const family198 = 'sync-with-family-and-friends';

          Future<void> sharedFolder(WidgetTester tester) async {
            final t = await sharingApp(tester, _Seeded(texts));
            await openSync(
              tester,
              t,
              find.byKey(const ValueKey('pick-folder')),
            );
          }

          boxFrame(family198, 'syncthing-own', sharedFolder);

          boxFrame(family198, 'syncthing-family', (tester) async {
            final seeded = _Seeded(texts, people: true);
            final t = await sharingApp(tester, seeded);
            await shareArea(tester, seeded, t);
            final afar = find.byKey(const ValueKey('share-afar'));
            await tester.scrollUntilVisible(
              afar,
              100,
              scrollable: notesList.last,
            );
            await tester.drag(notesList.last, const Offset(0, -200));
            await tester.pumpAndSettle();
            await _tap(tester, afar);
            await _tap(tester, find.text('Lena'));
            expect(find.byType(TransportDialog), findsOneWidget);
            await _tap(tester, find.byKey(const ValueKey('transport-folder')));
          });

          boxFrame(family198, 'autosync', sharedFolder);

          boxFrame(family198, 'cloud-apps', sharedFolder);

          // Calendar apps like FamilyWall (#199): the Calendar page, where
          // the family calendar is picked for the Agenda, and All Routines
          // and Reminders, whose Export makes the file to import there.
          const calendar199 = 'calendar-apps';

          boxFrame(calendar199, 'family-calendar-on-agenda', (tester) async {
            await _start(
              tester,
              boundary,
              language,
              seed().store,
              _guideDpr,
              appointments: FakeAppointments(),
            );
            await _tap(tester, find.byKey(const ValueKey('calendarPage')));
            expect(find.byType(CalendarScreen), findsOneWidget);
          });

          boxFrame(calendar199, 'routines-into-family-calendar', (
            tester,
          ) async {
            await start(tester, seed());
            await _tap(tester, find.byKey(const ValueKey('allRoutines')));
            expect(find.byType(AllRoutinesScreen), findsOneWidget);
          });
        }
      }
    },
  );
}

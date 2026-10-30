// The phone store screenshots (docs/agents/store-kit.md): eight bare app
// screens per locale, 1080 × 1920, into the fastlane metadata that Play
// and F-Droid read. Skipped in normal runs; written with
//
//   flutter test test/screenshots --run-skipped --concurrency=1 \
//     --plain-name 'store screenshots'
//
// The user guide's phone frames (#134) come from the same seeded data:
// each screen in English and German, light and dark, 648 × 1152, under
// site/assets/frames/phone/<language>/<theme>/. They are committed; a run
// compares each frame with the committed one and fails when they differ,
// and UPDATE_FRAMES=1 writes them instead (tool/guide-frames.sh):
//
//   flutter test test/screenshots --run-skipped --concurrency=1 \
//     --plain-name 'guide frames'
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/backup_screen.dart';
import 'package:oldbrainfix/src/first_start_screen.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/lock_settings.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix/src/ring_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/routine_history_screen.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/table_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../backup_test.dart' show DirFolders, quickKey;
import '../bundle_test.dart' show FakeBundles;
import '../fake_alarms.dart';
import '../fake_photos.dart';
import '../fake_voice.dart';
import '../open_capture.dart';

/// The frame in physical pixels and the density of a common phone:
/// 432 × 768 logical pixels, which 2.5 scales to exactly 1080 × 1920.
const _frame = Size(1080, 1920);
const _dpr = 2.5;

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
  String formatted,
  String meters,
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
);

/// A store seeded with a believable household, work and car life in
/// [texts], as of [_now].
class _Seeded {
  final Store store;
  final Routine snack;

  _Seeded._(this.store, this.snack);

  /// With [lists], the Household also holds a Checklist, a Shopping List
  /// and a Tally with its entries, for the guide's frames of them.
  /// With [formatted], it holds a formatted Note and one with a table.
  /// With [people], the user's Profile is Sam's and Lena is in People;
  /// with [shared] too, Work is shared with her from afar.
  factory _Seeded(
    _Texts x, {
    bool lists = false,
    bool formatted = false,
    bool people = false,
    bool shared = false,
  }) {
    // Each Note and Done at its own moment while seeding, then [_now].
    var clock = _now;
    final store = Store.inMemory(clock: () => clock)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    final snack = _seed(store, x, (at) => clock = at);
    if (lists) _seedLists(store, x, (at) => clock = at);
    if (formatted) _seedFormatted(store, x, (at) => clock = at);
    if (people || shared) _seedPeople(store, shared: shared);
    clock = _now;
    // What the history Missed was seen already: no Missed Summary.
    Alarms(store).markSeen();
    return _Seeded._(store, snack);
  }

  static Routine _seed(
    Store store,
    _Texts x,
    void Function(DateTime) setClock,
  ) {
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

/// A formatted Note about the boiler's service and one with a table of
/// meter readings, both in the Household, the newest of its Notes.
void _seedFormatted(Store store, _Texts x, void Function(DateTime) setClock) {
  final notes = Notes(store);
  final household = Places(store)
      .areas()
      .firstWhere((a) => a.starter == 'household');
  setClock(DateTime(2026, 10, 27, 8, 50));
  notes.capture(x.meters, format: TextFormats.markdown, place: household.id);
  setClock(DateTime(2026, 10, 27, 9, 10));
  notes.capture(x.formatted, format: TextFormats.markdown, place: household.id);
}

/// The user's own Profile as Sam's, and Lena in People from her Profile
/// card, sent through a messenger, so not verified. With [shared], Work
/// is shared with her from afar as a Member, its changes travelling by
/// bundles.
void _seedPeople(Store store, {required bool shared}) {
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
  lena.close();
  store.openProfileCard(card);
  if (!shared) return;
  final work = Places(store).areas().firstWhere((a) => a.starter == 'work');
  shareAreaWith(
    store,
    work.id,
    id,
    role: AreaRole.member,
    transport: Transport.bundle,
  );
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
}) async {
  tester.view
    ..physicalSize = _frame / _dpr * dpr
    ..devicePixelRatio = dpr;
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
  String path,
) async {
  final shot = await _capture(tester, boundary, _guideDpr);
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
/// the test ends, which checks it.
void _shot(String name, Future<void> Function(WidgetTester) body) {
  testWidgets(name, (tester) async {
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
      debugDisableShadows = true;
      EditableText.debugDeterministicCursor = false;
      WidgetsApp.debugAllowBannerOverride = true;
      Store.idMaker = random;
    }
  });
}

void main() {
  group('store screenshots', skip: 'writes PNGs; run with --run-skipped', () {
    setUpAll(_loadFonts);

    for (final (locale, language, texts) in [
      ('en-US', 'en', _en),
      ('de-DE', 'de', _de),
    ]) {
      final out = Directory(
        'fastlane/metadata/android/$locale/images/phoneScreenshots',
      );
      final boundary = GlobalKey();

      Future<AppLocalizations> start(
        WidgetTester tester,
        _Seeded seeded, {
        FakeAlarms? alarms,
      }) => _start(
        tester,
        boundary,
        language,
        seeded.store,
        _dpr,
        alarms: alarms,
      );

      Future<void> tap(WidgetTester tester, Finder finder) =>
          _tap(tester, finder);

      /// Writes what the app shows now as [name].
      Future<void> shoot(WidgetTester tester, String name) async {
        final shot = await _capture(tester, boundary, _dpr);
        expect((shot.width, shot.height), (1080, 1920));
        out.createSync(recursive: true);
        File('${out.path}/$name').writeAsBytesSync(shot.png);
      }

      void frame(String name, Future<void> Function(WidgetTester) body) {
        _shot('$locale $name', (tester) async {
          await body(tester);
          await shoot(tester, name);
        });
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
      });

      frame('4-editor.png', (tester) async {
        final t = await start(tester, _Seeded(texts));
        await tap(tester, find.byTooltip(t.browseTooltip));
        await tap(tester, find.text(t.starterHousehold));
        await tester.longPress(find.text(texts.boiler));
        await tester.pumpAndSettle();
        expect(find.byType(NoteEditorScreen), findsOneWidget);
      });

      frame('6-alarm.png', (tester) async {
        final seeded = _Seeded(texts);
        final alarms = FakeAlarms()
          ..ringingNow = Ringing(
            routineId: seeded.snack.id,
            occurrenceKey: Occurrence(Day(2026, 10, 27), ClockTime(15, 0)).key,
            title: texts.snack,
            body: '15:00',
          );
        await start(tester, seeded, alarms: alarms);
        expect(find.byType(RingScreen), findsOneWidget);
      });

      frame('7-history.png', (tester) async {
        final t = await start(tester, _Seeded(texts));
        await tap(tester, find.text(texts.snack).first);
        await tap(tester, find.byTooltip(t.historyTitle));
        expect(find.byType(RoutineHistoryScreen), findsOneWidget);
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
  });

  group(
    'guide frames',
    skip: 'compares the frames; run with --run-skipped',
    () {
      setUpAll(_loadFonts);

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

          frame('agenda', (tester) async {
            await start(tester, seed());
            expect(find.byType(AgendaScreen), findsOneWidget);
          });

          frame('first-start', (tester) async {
            await firstStart(tester);
            expect(find.byType(FirstStartScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('restore-offer')), findsOneWidget);
          });

          frame('starter-set', (tester) async {
            await firstStart(tester);
            await _tap(tester, find.byKey(const ValueKey('first-new')));
            expect(find.byType(StarterSetScreen), findsOneWidget);
          });

          frame('capture-fan', (tester) async {
            await start(tester, seed());
            await _tap(tester, find.byIcon(Icons.add));
            for (final way in ['text', 'voice', 'photo', 'gallery', 'paste']) {
              expect(fanEntry(way), findsOneWidget);
            }
          });

          frame('capture', (tester) async {
            await start(tester, seed());
            await openTextCapture(tester);
            await tester.enterText(find.byType(TextField), texts.captureText);
            await tester.pumpAndSettle();
            expect(find.byType(CaptureScreen), findsOneWidget);
            expect(find.byType(NudgeButtons), findsOneWidget);
          });

          frame('routine-editor', (tester) async {
            await start(tester, seed());
            await _tap(tester, find.text(texts.snack).first);
            expect(find.byType(RoutineEditorScreen), findsOneWidget);
          });

          /// The Household with its Checklist, Shopping List and Tally.
          Future<void> household(WidgetTester tester) async {
            final t = await start(
              tester,
              _Seeded(texts, lists: true)..store.themeChoice = choice,
            );
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
          }

          /// The Household's list of Notes, scrolling down.
          final notesList = find.byWidgetPredicate(
            (w) => w is Scrollable && w.axisDirection == AxisDirection.down,
          );

          frame('lists', (tester) async {
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

          frame('tally', (tester) async {
            await household(tester);
            final reading = find.text(texts.reading);
            await tester.scrollUntilVisible(
              reading,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
            await tester.longPress(reading);
            await tester.pumpAndSettle();
            expect(find.byType(NoteEditorScreen), findsOneWidget);
            // The length of a bar and the chart of the pages read, with the
            // target as a line.
            expect(
              find.text('${texts.pagesRead} (${texts.pages})'),
              findsOneWidget,
            );
            Scrollable.ensureVisible(
              tester.element(find.byKey(const ValueKey('tally-period'))),
              alignment: 0.12,
            );
            await tester.pumpAndSettle();
          });

          /// The Household with its formatted Notes, the newest on top.
          Future<void> formattedNotes(WidgetTester tester) async {
            final t = await start(
              tester,
              _Seeded(texts, formatted: true)..store.themeChoice = choice,
            );
            await _tap(tester, find.byTooltip(t.browseTooltip));
            await _tap(tester, find.text(t.starterHousehold));
          }

          /// The card's text of [text], as drawn on the list.
          Finder cardText(String text) =>
              find.byWidgetPredicate((w) => w is MarkedText && w.text == text);

          frame('formatted-text', (tester) async {
            await formattedNotes(tester);
            final boiler = cardText(texts.formatted);
            await tester.scrollUntilVisible(
              boiler,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
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

          frame('table', (tester) async {
            await formattedNotes(tester);
            final meters = cardText(texts.meters);
            await tester.scrollUntilVisible(
              meters,
              300,
              scrollable: notesList.first,
            );
            await tester.pumpAndSettle();
            final table = find.descendant(
              of: meters.first,
              matching: find.byWidgetPredicate(
                (w) => w is GestureDetector && '${w.key}'.contains('table-'),
              ),
            );
            await _tap(tester, table.first);
            expect(find.byType(TableScreen), findsOneWidget);
          });

          frame('share-area', (tester) async {
            final seeded = _Seeded(texts, people: true)
              ..store.themeChoice = choice;
            final t = await _start(
              tester,
              boundary,
              language,
              seeded.store,
              _guideDpr,
              bundles: FakeBundles(Directory('build')),
            );
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
            expect(find.byKey(const ValueKey('share-afar')), findsOneWidget);
          });

          frame('person', (tester) async {
            final seeded = _Seeded(texts, shared: true)
              ..store.themeChoice = choice;
            final t = await _start(
              tester,
              boundary,
              language,
              seeded.store,
              _guideDpr,
              bundles: FakeBundles(Directory('build')),
            );
            await _tap(tester, find.byTooltip(t.profileTitle));
            final lena = find.text('Lena');
            await tester.scrollUntilVisible(
              lena,
              300,
              scrollable: notesList.first,
            );
            await _tap(tester, lena);
            expect(find.byType(PersonScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('sendChanges')), findsOneWidget);
          });

          frame('backups', (tester) async {
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
            await _tap(tester, find.byTooltip(t.profileTitle));
            await _tap(tester, find.byKey(const ValueKey('backups')));
            expect(find.byType(BackupScreen), findsOneWidget);
            expect(find.byKey(const ValueKey('backup-now')), findsOneWidget);
          });

          frame('area-locks', (tester) async {
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
            await _tap(tester, find.byTooltip(t.profileTitle));
            final settings = find.byKey(const ValueKey('deviceSettings'));
            await tester.ensureVisible(settings);
            await _tap(tester, settings);
            final locks = find.byKey(const ValueKey('areaLocks'));
            await tester.scrollUntilVisible(
              locks,
              300,
              scrollable: notesList.last,
            );
            await _tap(tester, locks);
            expect(find.byType(AreaLocksScreen), findsOneWidget);
          });
        }
      }
    },
  );
}

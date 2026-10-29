// The phone store screenshots (docs/agents/store-kit.md): eight bare app
// screens per locale, 1080 × 1920, into the fastlane metadata that Play
// and F-Droid read. Skipped in normal runs; written with
//
//   flutter test test/screenshots --run-skipped --concurrency=1
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
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/ring_screen.dart';
import 'package:oldbrainfix/src/routine_history_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../fake_alarms.dart';
import '../fake_photos.dart';
import '../fake_voice.dart';
import '../open_capture.dart';

/// The frame in physical pixels and the density of a common phone:
/// 432 × 768 logical pixels, which 2.5 scales to exactly 1080 × 1920.
const _frame = Size(1080, 1920);
const _dpr = 2.5;

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
);

/// A store seeded with a believable household, work and car life in
/// [texts], as of [_now].
class _Seeded {
  final Store store;
  final Routine snack;

  _Seeded._(this.store, this.snack);

  factory _Seeded(_Texts x) {
    // Each Note and Done at its own moment while seeding, then [_now].
    var clock = _now;
    final store = Store.inMemory(clock: () => clock)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    final snack = _seed(store, x, (at) => clock = at);
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

      /// Shows the app on [seeded] as a phone in [language] would.
      Future<AppLocalizations> start(
        WidgetTester tester,
        _Seeded seeded, {
        FakeAlarms? alarms,
      }) async {
        tester.view
          ..physicalSize = _frame
          ..devicePixelRatio = _dpr;
        tester.platformDispatcher.localesTestValue = [Locale(language)];
        addTearDown(tester.view.reset);
        addTearDown(tester.platformDispatcher.clearLocalesTestValue);
        addTearDown(seeded.store.close);
        await tester.pumpWidget(
          RepaintBoundary(
            key: boundary,
            child: OldBrainFixApp(
              store: seeded.store,
              alarms: alarms ?? FakeAlarms(),
              voice: FakeVoice(),
              photos: FakePhotos(),
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

      Future<void> tap(WidgetTester tester, Finder finder) async {
        await tester.tap(finder);
        await tester.pumpAndSettle();
      }

      /// Writes what the app shows now as [name].
      Future<void> shoot(WidgetTester tester, String name) async {
        await tester.pumpAndSettle();
        final render =
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        final png = await tester.runAsync(() async {
          final image = await render.toImage(pixelRatio: _dpr);
          try {
            expect((image.width, image.height), (1080, 1920));
            final data = await image.toByteData(format: ui.ImageByteFormat.png);
            return data!.buffer.asUint8List();
          } finally {
            image.dispose();
          }
        });
        out.createSync(recursive: true);
        File('${out.path}/$name').writeAsBytesSync(png!);
      }

      /// Runs [body] with real shadows: the embossed cards are flat
      /// otherwise. Restored before the test ends, which checks it.
      void frame(String name, Future<void> Function(WidgetTester) body) {
        testWidgets('$locale $name', (tester) async {
          debugDisableShadows = false;
          EditableText.debugDeterministicCursor = true;
          // Bare screens: no debug banner in the corner.
          WidgetsApp.debugAllowBannerOverride = false;
          // Counted ids: the own Profile's placeholder colour, made from
          // its id, is the same on every run.
          final random = Store.idMaker;
          var ids = 0;
          Store.idMaker = () => 'shot${++ids}';
          try {
            await body(tester);
            await shoot(tester, name);
          } finally {
            debugDisableShadows = true;
            EditableText.debugDeterministicCursor = false;
            WidgetsApp.debugAllowBannerOverride = true;
            Store.idMaker = random;
          }
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
}

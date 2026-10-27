import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_widget.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';

void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  late FakeAlarms alarms;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final en = lookupAppLocalizations(const Locale('en'));
  final de = lookupAppLocalizations(const Locale('de'));

  // The app's localizations load these; the snapshot tests run without.
  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    routines = Routines(store);
    alarms = FakeAlarms();
  });
  tearDown(() => store.close());

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  Schedule once(Day day, [List<ClockTime> times = const []]) =>
      Schedule(start: day, times: times);

  AgendaSnapshot snapshot([AppLocalizations? t, String locale = 'en']) =>
      AgendaSnapshot.of(store, deviceWallClock, t ?? en, locale);

  List<String> titles(List<WidgetRow>? rows) => [
    for (final row in rows ?? const <WidgetRow>[]) row.title,
  ];

  group('the snapshot', () {
    test('holds today\'s Occurrences in Agenda order: all day first, '
        'then by time', () {
      routines.create('evening pills', daily([ClockTime(20, 0)]));
      routines.create('bins out', once(Day(2026, 10, 24)));
      routines.create('morning pills', daily([ClockTime(8, 0)]));
      routines.create('water', daily([ClockTime(7, 0), ClockTime(13, 30)]));

      final today = snapshot().days['2026-10-24'];

      expect(titles(today), [
        'bins out',
        'water',
        'morning pills',
        'water',
        'evening pills',
      ]);
      expect(
        [for (final row in today ?? const []) row.time],
        ['All day', '07:00', '08:00', '13:30', '20:00'],
      );
    });

    test('names each row by its Routine and Occurrence, and tells what '
        'is Done', () {
      final pills = routines.create('pills', daily([ClockTime(8, 0)]));
      routines.markDone(
        pills.id,
        Occurrence(Day(2026, 10, 24), ClockTime(8, 0)),
      );

      final row = snapshot().days['2026-10-24']?.single;

      expect(row?.routineId, pills.id);
      expect(row?.occurrenceKey, '2026-10-24T08:00');
      expect(row?.done, isTrue);
      expect(row?.toMap(), {
        'routine': pills.id,
        'occurrence': '2026-10-24T08:00',
        'title': 'pills',
        'time': '08:00',
        'done': true,
      });
    });

    test('holds tomorrow as well, so the widget moves on at midnight '
        'without the app', () {
      routines.create('dentist', once(Day(2026, 10, 25), [ClockTime(10, 0)]));
      routines.create('pills', daily([ClockTime(8, 0)]));

      final days = snapshot().days;

      expect(days.keys, ['2026-10-24', '2026-10-25']);
      expect(titles(days['2026-10-24']), ['pills']);
      expect(titles(days['2026-10-25']), ['pills', 'dentist']);
      expect(days['2026-10-25']?.every((row) => !row.done), isTrue);
    });

    test('titles a row by its Note\'s first line, "Photo" or "Voice '
        'recording" for a photo or voice Note without words', () {
      routines.create('pills\ntwo of them', daily([ClockTime(8, 0)]));
      final photo = notes.capture('', photos: [onePixelPng]);
      routines.spawn(photo.id, daily([ClockTime(9, 0)]));
      final voice = notes.capture(
        '',
        recordings: [
          Uint8List.fromList([1]),
        ],
      );
      notes.setTranscript(voice.id, voice.recordings.single, '');
      routines.spawn(voice.id, daily([ClockTime(10, 0)]));

      expect(titles(snapshot().days['2026-10-24']), [
        'pills',
        'Photo',
        'Voice recording',
      ]);
      expect(titles(snapshot(de, 'de').days['2026-10-24']), [
        'pills',
        'Foto',
        'Sprachaufnahme',
      ]);
    });

    test('a Routine in the trash is gone from it', () {
      final pills = routines.create('pills', daily([ClockTime(8, 0)]));
      routines.trash(pills.id);

      expect(snapshot().days['2026-10-24'], isEmpty);
    });

    test('carries the Unsorted count and the texts', () {
      notes.capture('nees mor sauce');
      notes.capture('call mum');

      final s = snapshot();

      expect(s.unsorted, 2);
      expect(s.unsortedText, '2 Unsorted');
      expect(s.today, 'Today');
      expect(s.empty, 'Nothing due today.');
      expect(s.more, 'and {count} more');
      expect(s.done, 'Done');
      expect(s.toMap()['labels'], {
        'today': 'Today',
        'empty': 'Nothing due today.',
        'unsorted': '2 Unsorted',
        'more': 'and {count} more',
        'done': 'Done',
      });
    });

    test('speaks the reader\'s language', () {
      notes.capture('Soße kaufen');
      routines.create('Tonne raus', once(Day(2026, 10, 24)));

      final s = snapshot(de, 'de');

      expect(s.today, 'Heute');
      expect(s.empty, 'Heute steht nichts an.');
      expect(s.unsortedText, '1 unsortiert');
      expect(s.more, 'und {count} weitere');
      expect(s.done, 'Erledigt');
      expect(s.days['2026-10-24']?.single.time, de.allDay);
    });
  });

  group('handing it to the widget', () {
    Future<void> start(WidgetTester tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
      await tester.pumpAndSettle();
    }

    testWidgets('hands it over on start', (tester) async {
      routines.create('pills', daily([ClockTime(20, 0)]));
      notes.capture('nees mor sauce');
      await start(tester);

      expect(titles(alarms.agendas.last.days['2026-10-24']), ['pills']);
      expect(alarms.agendas.last.unsorted, 1);
    });

    testWidgets('hands it over again when Occurrences change', (tester) async {
      await start(tester);
      expect(alarms.agendas.last.days['2026-10-24'], isEmpty);

      final pills = routines.create('pills', daily([ClockTime(20, 0)]));
      await tester.pumpAndSettle();
      expect(titles(alarms.agendas.last.days['2026-10-24']), ['pills']);

      routines.markDone(
        pills.id,
        Occurrence(Day(2026, 10, 24), ClockTime(20, 0)),
      );
      await tester.pumpAndSettle();
      expect(alarms.agendas.last.days['2026-10-24']?.single.done, isTrue);

      notes.edit(pills.noteId, 'blue pills');
      await tester.pumpAndSettle();
      expect(titles(alarms.agendas.last.days['2026-10-24']), ['blue pills']);
    });

    testWidgets('hands it over again when the Unsorted count changes', (
      tester,
    ) async {
      await start(tester);
      expect(alarms.agendas.last.unsorted, 0);

      final note = notes.capture('nees mor sauce');
      await tester.pumpAndSettle();
      expect(alarms.agendas.last.unsorted, 1);
      expect(alarms.agendas.last.unsortedText, '1 Unsorted');

      notes.trash(note.id);
      await tester.pumpAndSettle();
      expect(alarms.agendas.last.unsorted, 0);
    });

    testWidgets('does not hand over the same snapshot again', (tester) async {
      final note = notes.capture('nees mor sauce');
      await start(tester);
      final handed = alarms.agendas.length;

      // Changes the log, not what the widget shows.
      notes.setImportance(note.id, Importance.important);
      await tester.pumpAndSettle();

      expect(alarms.agendas.length, handed);
    });

    testWidgets('a Done checked in the widget reaches the log, and the '
        'widget then shows it Done', (tester) async {
      final pills = routines.create('pills', daily([ClockTime(8, 0)]));
      await start(tester);
      expect(alarms.agendas.last.days['2026-10-24']?.single.done, isFalse);

      final tapped = DateTime(2026, 10, 24, 8, 5).toUtc();
      alarms.tap(DoneTapped(1, pills.id, '2026-10-24T08:00', tapped));
      await tester.pumpAndSettle();

      expect(
        routines
            .byId(pills.id)
            ?.doneAt(Occurrence(Day(2026, 10, 24), ClockTime(8, 0))),
        tapped,
      );
      expect(alarms.agendas.last.days['2026-10-24']?.single.done, isTrue);
      expect(alarms.acknowledged, 1);
      expect(alarms.pending, isEmpty);
    });

    testWidgets('tapping the Unsorted count opens the Unsorted list', (
      tester,
    ) async {
      notes.capture('nees mor sauce');
      await start(tester);
      expect(find.byType(UnsortedScreen), findsNothing);

      alarms.tapUnsorted();
      await tester.pumpAndSettle();

      expect(find.byType(UnsortedScreen), findsOneWidget);
      expect(find.text('nees mor sauce'), findsOneWidget);
    });

    testWidgets('tapping the Unsorted count starts the app on the Unsorted '
        'list', (tester) async {
      notes.capture('nees mor sauce');
      alarms.tapUnsorted();
      await start(tester);

      expect(find.byType(UnsortedScreen), findsOneWidget);
      expect(find.text('nees mor sauce'), findsOneWidget);
    });
  });
}

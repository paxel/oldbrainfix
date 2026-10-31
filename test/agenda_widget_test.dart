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
        'kind': 'occurrence',
        'title': 'pills',
        'time': '08:00',
        'done': true,
        'checks': true,
        'routine': pills.id,
        'occurrence': '2026-10-24T08:00',
        'block': 'timed',
        'at': 8 * 60,
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
      expect(s.done, 'Done');
      expect(s.toMap()['v'], 2);
      expect(s.toMap()['labels'], {
        'today': 'Today',
        'empty': 'Nothing due today.',
        'unsorted': '2 Unsorted',
        'done': 'Done',
        'allDay': 'All day',
        'noTitle': 'Appointment without a title',
        'since': 'from {time}',
        'until': 'until {time}',
      });
    });

    test('speaks the reader\'s language', () {
      notes.capture('Soße kaufen');
      routines.create('Tonne raus', once(Day(2026, 10, 24)));

      final s = snapshot(de, 'de');

      expect(s.today, 'Heute');
      expect(s.empty, 'Heute steht nichts an.');
      expect(s.unsortedText, '1 unsortiert');
      expect(s.done, 'Erledigt');
      expect(s.since, de.appointmentSince('{time}'));
      expect(s.days['2026-10-24']?.single.time, de.allDay);
    });
  });

  group('every row the Agenda shows (#153)', () {
    List<String> kinds(List<WidgetRow>? rows) => [
      for (final row in rows ?? const <WidgetRow>[]) row.kind,
    ];

    /// "Packing" with Bath, Tent and Map, a daily all-day Routine from it.
    Routine packing({String? place}) {
      final note = notes.capture('Packing\nBath\nTent\nMap', place: place);
      Checklists(store).make(note.id);
      return routines.spawn(note.id, daily(const []));
    }

    /// Two Tallies, Push-ups and Squats, and "Sport", a Session picking
    /// both at 07:00 every day.
    Session sport({String? place}) {
      final tallies = Tallies(store);
      final pool = [
        for (final name in ['Push-ups', 'Squats'])
          notes.capture(name, place: place).id,
      ];
      for (final id in pool) {
        tallies.addField(id, name: 'Done', unit: 'times', target: 20);
      }
      return Sessions(store).spawn(
        notes.capture('Sport', place: place).id,
        daily([ClockTime(7, 0)]),
        pool: pool,
        count: 2,
        rule: SessionRule.rolling,
      );
    }

    /// "Groceries" with Milk and Eggs checked a week ago, back today, and
    /// Rice open.
    String groceries({String? place}) {
      final lists = ShoppingLists(store);
      now = DateTime(2026, 10, 17, 10);
      final id = notes.capture('Groceries\nMilk\nEggs\nRice', place: place).id;
      lists.make(id);
      for (final text in ['Milk', 'Eggs']) {
        lists.check(
          lists
              .byNote(id, deviceWallClock)!
              .lines
              .firstWhere((l) => l.text == text)
              .id,
          deviceWallClock,
        );
      }
      now = DateTime(2026, 10, 24, 9);
      return id;
    }

    /// "Before the trip", a Checklist without a Routine: Passport,
    /// Tickets.
    String trip({String? place}) {
      final id = notes
          .capture('Before the trip\nPassport\nTickets', place: place)
          .id;
      Checklists(store).make(id);
      return id;
    }

    test('in the Agenda\'s order: all-day Occurrences, the Shopping '
        'Lists, the Checklists, the timed ones, then the Frogs', () {
      routines.create('pills', daily([ClockTime(8, 0)]));
      routines.create('bins out', once(Day(2026, 10, 24)));
      groceries();
      trip();
      final frogs = Frogs(store);
      frogs.spawn(notes.capture('taxes').id);

      final today = snapshot().days['2026-10-24'];

      expect(titles(today), [
        'bins out',
        'Groceries: 2 back',
        'Before the trip: 2 of 2 open',
        'pills',
        'Eat a frog',
        'taxes',
      ]);
      expect(kinds(today), [
        'occurrence',
        'shopping',
        'checklist',
        'occurrence',
        'heading',
        'frog',
      ]);
      expect(
        [for (final row in today ?? const <WidgetRow>[]) row.block],
        ['allDay', 'lists', 'lists', 'timed', 'frogs', 'frogs'],
      );
      // Today only, as on the Agenda.
      expect(titles(snapshot().days['2026-10-25']), ['pills']);
    });

    test('a Checklist Routine\'s Lines under its Occurrence, checkable '
        'for it', () {
      final routine = packing();
      final occurrence = Occurrence(Day(2026, 10, 24));
      final lists = Checklists(store);
      final bath = lists
          .forOccurrence((routineId: routine.id, occurrence: occurrence))!
          .lines
          .first;
      lists.check(bath.id, on: (routineId: routine.id, occurrence: occurrence));

      final today = snapshot().days['2026-10-24'] ?? const <WidgetRow>[];

      expect(titles(today), ['Packing', 'Bath', 'Tent', 'Map']);
      expect(kinds(today), ['occurrence', 'line', 'line', 'line']);
      expect([for (final row in today) row.done], [false, true, false, false]);
      final line = today[1];
      expect(line.toMap(), {
        'kind': 'line',
        'title': 'Bath',
        'done': true,
        'checks': true,
        'indent': 1,
        'routine': routine.id,
        'occurrence': '2026-10-24',
        'line': bath.id,
        'block': 'allDay',
      });
      // Tomorrow's Occurrence starts with none checked.
      expect([
        for (final row in snapshot().days['2026-10-25'] ?? const []) row.done,
      ], everyElement(isFalse));
    });

    test('a Session\'s picks under it today, how many it picks tomorrow, '
        'without writing the pick', () {
      final session = sport();
      final before = store.history(session.id).length;

      final s = snapshot();

      final today = s.days['2026-10-24'] ?? const <WidgetRow>[];
      expect(kinds(today), ['occurrence', 'text', 'text']);
      expect(today[1].title, contains('Push-ups'));
      expect(today[2].title, contains('Squats'));
      expect(today[1].indent, 1);
      expect(titles(s.days['2026-10-25']), ['Sport', 'Picks 2 Tallies']);
      expect(store.history(session.id).length, before);
    });

    test('the Frogs as "Eat a frog" shows them: once one is eaten, only '
        'that one', () {
      final frogs = Frogs(store);
      final taxes = frogs.spawn(notes.capture('taxes').id);
      frogs.spawn(notes.capture('dentist').id);
      // The Agenda writes today's pick as it shows.
      frogs.pickToday(deviceWallClock);

      final open = snapshot().days['2026-10-24'] ?? const <WidgetRow>[];
      expect(titles(open), ['Eat a frog', 'taxes', 'dentist']);
      expect(open[1].frogId, taxes.id);
      expect(open[1].checks, isTrue);

      frogs.markDone(taxes.id);
      final eaten = snapshot().days['2026-10-24'] ?? const <WidgetRow>[];
      expect(titles(eaten), ['Eat a frog', 'taxes']);
      expect(eaten[1].done, isTrue);
    });

    test('Shopping Lists the way this Device shows them', () {
      final id = groceries();

      expect(titles(snapshot().days['2026-10-24']), ['Groceries: 2 back']);
      expect(snapshot().days['2026-10-24']?.single.checks, isFalse);

      store.shoppingAgenda = ShoppingAgenda.open;
      expect(titles(snapshot().days['2026-10-24']), ['Groceries: 3 open']);

      store.shoppingAgenda = ShoppingAgenda.lines;
      final lines = snapshot().days['2026-10-24'] ?? const <WidgetRow>[];
      expect(titles(lines), ['Milk · Groceries', 'Eggs · Groceries']);
      expect(lines.every((row) => row.checks && row.lineId != null), isTrue);
      expect(
        lines.first.lineId,
        ShoppingLists(store)
            .byNote(id, deviceWallClock)
            ?.lines
            .firstWhere((l) => l.text == 'Milk')
            .id,
      );

      store.shoppingAgenda = ShoppingAgenda.off;
      expect(snapshot().days['2026-10-24'], isEmpty);
    });

    test('the Missed Summary first, as its card shows it', () {
      now = DateTime(2026, 10, 24, 7);
      routines.create('pills', daily([ClockTime(8, 0)]));
      // Each is Missed once the next one is due.
      now = DateTime(2026, 10, 29, 12);
      final summary = Alarms(store).missed(deviceWallClock);
      final missed = summary.routines.single;

      final rows = snapshot().missed;

      expect(rows.first.kind, 'heading');
      expect(rows.first.title, 'Missed');
      expect(rows[1].title, 'pills · ${en.missedTimes(missed.count)}');
      final checkable = [
        for (final row in rows)
          if (row.kind == 'missed') row,
      ];
      expect(checkable.length, missed.count < 3 ? missed.count : 3);
      expect(checkable.first.occurrenceKey, missed.missed.first.occurrence.key);
      expect(checkable.first.checks, isTrue);
      if (missed.count > 3) {
        expect(rows.last.title, en.missedMore(missed.count - 3));
      }

      Alarms(store).markSeen();
      expect(snapshot().missed, isEmpty);
    });

    test('names the calendars whose appointments the widget reads, none '
        'while that is off, and keeps no appointment', () {
      store.agendaCalendars = {'9', '3'};
      expect(snapshot().calendars, isEmpty);

      store.agendaCalendarOn = true;
      expect(snapshot().calendars, ['3', '9']);
      expect(snapshot().toMap()['calendars'], ['3', '9']);
    });

    test('a Masked Area tells nothing on the widget either', () {
      final doctor = Places(store).addArea('Doctor');
      packing(place: doctor.id);
      sport(place: doctor.id);
      groceries(place: doctor.id);
      trip(place: doctor.id);
      Frogs(store).spawn(notes.capture('taxes', place: doctor.id).id);
      routines.spawn(
        notes.capture('Pills', place: doctor.id).id,
        daily([ClockTime(8, 0)]),
      );
      store.setPin('1234', iterations: 10);
      store.setAreaLocked(doctor.id, true);
      store.setAreaMasked(doctor.id, true);
      store.shoppingAgenda = ShoppingAgenda.lines;

      final s = snapshot();

      expect(titles(s.days['2026-10-24']), [
        en.maskedRoutineAt(en.allDay),
        en.shoppingRowBack(en.maskedShoppingList, 2),
        en.checklistRowOpen(en.maskedChecklist, 2, 2),
        en.maskedRoutineAt('07:00'),
        en.maskedRoutineAt('08:00'),
        'Eat a frog',
        en.maskedFrog,
      ]);
      expect([
        for (final row in s.missed)
          if (row.kind == 'text' && row.indent == 0)
            row.title.split(' · ').first,
      ], everyElement(en.maskedRoutine));
      final text = '${s.toMap()}';
      for (final word in [
        'Packing',
        'Bath',
        'Push-ups',
        'Groceries',
        'Milk',
        'Before the trip',
        'taxes',
        'Pills',
        'Sport',
      ]) {
        expect(text, isNot(contains(word)), reason: word);
      }
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

    testWidgets('a Line, a Frog and a Shopping List\'s Line checked in the '
        'widget change the log as the Agenda\'s boxes do', (tester) async {
      final note = notes.capture('Packing\nBath\nTent');
      Checklists(store).make(note.id);
      final packing = routines.spawn(note.id, daily(const []));
      final frogs = Frogs(store);
      final taxes = frogs.spawn(notes.capture('taxes').id);
      final shopping = ShoppingLists(store);
      final groceries = notes.capture('Groceries\nMilk').id;
      shopping.make(groceries);
      store.shoppingAgenda = ShoppingAgenda.open;
      await start(tester);
      final today = alarms.agendas.last.days['2026-10-24'] ?? const [];
      final lines = [
        for (final row in today)
          if (row.kind == 'line') row,
      ];
      final milk = shopping.byNote(groceries, deviceWallClock)!.lines.single;
      final tapped = DateTime(2026, 10, 24, 8, 5).toUtc();

      alarms.tap(
        LineTapped(1, packing.id, '2026-10-24', lines[0].lineId!, tapped),
      );
      alarms.tap(FrogTapped(2, taxes.id, tapped));
      alarms.tap(ShoppingTapped(3, milk.id, tapped));
      await tester.pumpAndSettle();

      final occurrence = Occurrence(Day(2026, 10, 24));
      final checked = Checklists(store)
          .forOccurrence((routineId: packing.id, occurrence: occurrence));
      expect([for (final l in checked!.lines) l.checked], [true, false]);
      expect(routines.byId(packing.id)?.doneAt(occurrence), isNull);
      expect(frogs.byId(taxes.id)?.done, tapped);
      expect(
        shopping.byNote(groceries, deviceWallClock)?.lines.single.struck,
        isTrue,
      );
      expect(alarms.acknowledged, 3);
      final shown = alarms.agendas.last.days['2026-10-24'] ?? const [];
      expect(
        [
          for (final row in shown)
            if (row.kind != 'shopping') row.done,
        ],
        [false, true, false, false, true],
      );

      // The last Line checks the Occurrence off, as on the Agenda.
      alarms.tap(
        LineTapped(4, packing.id, '2026-10-24', lines[1].lineId!, tapped),
      );
      await tester.pumpAndSettle();
      expect(routines.byId(packing.id)?.doneAt(occurrence), isNotNull);
      expect(alarms.agendas.last.days['2026-10-24']?.first.done, isTrue);
    });

    testWidgets('a check of something gone changes nothing and is still '
        'acknowledged', (tester) async {
      await start(tester);

      final tapped = DateTime(2026, 10, 24, 8, 5).toUtc();
      alarms.tap(LineTapped(1, 'routine:gone', '2026-10-24', 'line:x', tapped));
      alarms.tap(FrogTapped(2, 'frog:gone', tapped));
      alarms.tap(ShoppingTapped(3, 'line:gone', tapped));
      await tester.pumpAndSettle();

      expect(alarms.acknowledged, 3);
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

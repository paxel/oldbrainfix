import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/calendar_export_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_calendar.dart';
import 'fake_mirror.dart';

void main() {
  late Store store;
  late Routines routines;
  late FakeMirror mirror;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  const delay = Duration(milliseconds: 100);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    routines = Routines(store);
    mirror = FakeMirror();
  });
  tearDown(() => store.close());

  Schedule daily(int hour) => Schedule(
    start: Day(2026, 10, 24),
    times: [ClockTime(hour, 0)],
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  String uid(Routine routine, int hour) =>
      '${routine.id}-${hour.toString().padLeft(2, '0')}00@oldbrainfix';

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        calendar: FakeCalendar(),
        mirror: mirror,
        mirrorDelay: delay,
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Lets the mirror's wait after a change run out, and its pass finish.
  Future<void> settle(WidgetTester tester) async {
    await tester.pump(delay);
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> openExport(WidgetTester tester) async {
    await tap(tester, find.byTooltip('Export to calendar'));
    expect(find.byType(CalendarExportScreen), findsOneWidget);
  }

  final mirrorSwitch = find.descendant(
    of: find.byType(MirrorSwitch),
    matching: find.byType(Switch),
  );

  bool switchedOn(WidgetTester tester) =>
      tester.widget<Switch>(mirrorSwitch).value;

  group('turning it on', () {
    testWidgets('explains, asks, picks a calendar and writes every event', (
      tester,
    ) async {
      final pills = routines.create('pills', daily(8));
      final foreign = mirror.addForeign('1', 'Dentist');
      await start(tester);
      await openExport(tester);
      expect(find.text('Keep in a calendar'), findsOneWidget);
      expect(switchedOn(tester), isFalse);

      await tap(tester, mirrorSwitch);
      expect(find.text('Keep them in your calendar?'), findsOneWidget);
      await tap(tester, find.text('Allow'));
      expect(mirror.requests, 1);
      // The calendars to pick from, where the finger is.
      expect(find.text('Family'), findsOneWidget);
      await tap(tester, find.text('Personal'));

      expect(switchedOn(tester), isTrue);
      expect(
        find.text('In “Personal”. Tap to choose another calendar.'),
        findsOneWidget,
      );
      expect(store.mirrorCalendar, (id: '1', name: 'Personal'));
      final event = mirror.mirroredIn('1')[uid(pills, 8)];
      expect(event?.title, 'pills');
      expect(event?.start, Occurrence(Day(2026, 10, 24), ClockTime(8, 0)));
      expect(event?.rule, 'FREQ=DAILY');
      expect(mirror.events[foreign]?.title, 'Dentist');
    });

    testWidgets('refused access leaves it off and says so at the top', (
      tester,
    ) async {
      routines.create('pills', daily(8));
      mirror.allowOnRequest = false;
      await start(tester);
      await openExport(tester);

      await tap(tester, mirrorSwitch);
      await tap(tester, find.text('Allow'));

      expect(switchedOn(tester), isFalse);
      expect(store.mirrorCalendar, isNull);
      expect(mirror.events, isEmpty);
      expect(
        find.text(
          'Old Brain Fix may not write to your calendar. '
          'You can allow it in the system settings.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('"Not now" asks nothing', (tester) async {
      await start(tester);
      await openExport(tester);
      await tap(tester, mirrorSwitch);
      await tap(tester, find.text('Not now'));
      expect(mirror.requests, 0);
      expect(switchedOn(tester), isFalse);
    });

    testWidgets('without a calendar to write into it stays off', (
      tester,
    ) async {
      mirror = FakeMirror(allowed: true, calendars: const []);
      await start(tester);
      await openExport(tester);
      await tap(tester, mirrorSwitch);
      expect(switchedOn(tester), isFalse);
      expect(
        find.text('There is no calendar on this device to write into.'),
        findsOneWidget,
      );
    });

    testWidgets('the only calendar is taken without asking which', (
      tester,
    ) async {
      mirror = FakeMirror(
        allowed: true,
        calendars: const [(id: '5', name: 'Phone', account: '')],
      );
      final pills = routines.create('pills', daily(8));
      await start(tester);
      await openExport(tester);
      await tap(tester, mirrorSwitch);
      expect(store.mirrorCalendar, (id: '5', name: 'Phone'));
      expect(mirror.mirroredIn('5').keys, [uid(pills, 8)]);
    });

    testWidgets('another calendar takes the events from the first', (
      tester,
    ) async {
      mirror.allowed = true;
      final pills = routines.create('pills', daily(8));
      await start(tester);
      await openExport(tester);
      await tap(tester, mirrorSwitch);
      await tap(tester, find.text('Personal'));
      expect(mirror.mirroredIn('1').keys, [uid(pills, 8)]);

      await tap(tester, find.text('Keep in a calendar'));
      await tap(tester, find.text('Family'));
      expect(store.mirrorCalendar, (id: '2', name: 'Family'));
      expect(mirror.mirroredIn('1'), isEmpty);
      expect(mirror.mirroredIn('2').keys, [uid(pills, 8)]);
    });
  });

  group('while on', () {
    setUp(() {
      mirror.allowed = true;
      store.mirrorCalendar = (id: '1', name: 'Personal');
    });

    testWidgets('the calendar is brought in step on start', (tester) async {
      final pills = routines.create('pills', daily(8));
      await start(tester);
      expect(mirror.mirroredIn('1').keys, [uid(pills, 8)]);
      expect(store.mirroredEvents.keys, [uid(pills, 8)]);
    });

    testWidgets('events follow every change to the Routines', (tester) async {
      final pills = routines.create('pills', daily(8));
      final foreign = mirror.addForeign('1', 'Dentist');
      await start(tester);
      final id = mirror.idOf(uid(pills, 8));
      expect(id, isNotNull);

      // A new Reminder comes in.
      final dentist = routines.create(
        'call the dentist',
        Schedule(start: Day(2026, 10, 30), times: [ClockTime(14, 0)]),
      );
      await settle(tester);
      expect(
        mirror.mirroredIn('1')[uid(dentist, 14)]?.title,
        'call the dentist',
      );
      expect(mirror.mirroredIn('1')[uid(dentist, 14)]?.rule, isNull);

      // The Note's text changes: the same event, written again.
      Notes(store).edit(pills.noteId, 'blue pills');
      await settle(tester);
      expect(mirror.idOf(uid(pills, 8)), id);
      expect(mirror.mirroredIn('1')[uid(pills, 8)]?.title, 'blue pills');

      // Another time: the old event goes, the new one comes.
      routines.reschedule(pills.id, daily(9));
      await settle(tester);
      expect(
        mirror.mirroredIn('1').keys,
        unorderedEquals([uid(pills, 9), uid(dentist, 14)]),
      );

      // Into the trash: out of the calendar.
      routines.trash(dentist.id);
      await settle(tester);
      expect(mirror.mirroredIn('1').keys, [uid(pills, 9)]);

      expect(mirror.events[foreign]?.title, 'Dentist');
    });

    testWidgets('an event removed by hand comes back', (tester) async {
      final pills = routines.create('pills', daily(8));
      await start(tester);
      final id = mirror.idOf(uid(pills, 8));
      mirror.events.remove(id);

      routines.create('walk', daily(7));
      await settle(tester);
      expect(mirror.mirroredIn('1').keys, contains(uid(pills, 8)));
    });

    testWidgets('a calendar gone turns the mirror off', (tester) async {
      routines.create('pills', daily(8));
      mirror.deviceCalendars.removeWhere((c) => c.id == '1');
      await start(tester);
      expect(store.mirrorCalendar, isNull);
      expect(store.mirroredEvents, isEmpty);
    });
  });

  group('turning it off', () {
    setUp(() {
      mirror.allowed = true;
      store.mirrorCalendar = (id: '1', name: 'Personal');
    });

    testWidgets('removes every mirrored event and nothing else', (
      tester,
    ) async {
      routines.create('pills', daily(8));
      routines.create('walk', daily(7));
      final foreign = mirror.addForeign('1', 'Dentist');
      await start(tester);
      expect(mirror.mirroredIn('1'), hasLength(2));
      await openExport(tester);
      expect(switchedOn(tester), isTrue);

      await tap(tester, mirrorSwitch);
      expect(switchedOn(tester), isFalse);
      expect(mirror.mirroredIn('1'), isEmpty);
      expect(mirror.events.keys, [foreign]);
      expect(store.mirrorCalendar, isNull);
      expect(store.mirroredEvents, isEmpty);

      // Off means off: changes no longer reach the calendar.
      routines.create('read', daily(21));
      await settle(tester);
      expect(mirror.mirroredIn('1'), isEmpty);
    });

    testWidgets('without access it asks, and stays on when refused', (
      tester,
    ) async {
      routines.create('pills', daily(8));
      await start(tester);
      await openExport(tester);
      mirror
        ..allowed = false
        ..allowOnRequest = false;

      await tap(tester, mirrorSwitch);
      expect(mirror.requests, 1);
      expect(switchedOn(tester), isTrue);
      expect(mirror.mirroredIn('1'), hasLength(1));
      expect(
        find.textContaining('The events could not be removed'),
        findsOneWidget,
      );
    });
  });
}

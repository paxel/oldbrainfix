import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_history_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announced.dart';
import 'fake_alarms.dart';

/// Snooze and Not today wherever an Occurrence is answered (#269): the
/// Agenda row's ⋮, the Note editor's Due line and the notification; a
/// skipped Occurrence struck on the Agenda, and its history.
void main() {
  late Store store;
  late Routines routines;
  late FakeAlarms alarms;
  late Announcements announcements;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    routines = Routines(store);
    alarms = FakeAlarms();
    announcements = Announcements();
  });
  tearDown(() {
    announcements.dispose();
    store.close();
  });

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  /// Makes a Routine as of [made], before its Occurrences were due.
  Routine madeAt(DateTime made, String text, Schedule schedule) {
    final saved = now;
    now = made;
    final routine = routines.create(text, schedule);
    now = saved;
    return routine;
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1200, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        alarms: alarms,
        announcements: announcements,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Finder key(String name) => find.byKey(ValueKey(name));
  Finder rowMenu(Routine routine) => key('row-menu-${routine.id}').first;
  final snooze30 = key('answer-snooze30');
  final skip = key('answer-skip');
  final tomorrow = key('answer-tomorrow');

  /// Lets the offer to undo go by itself, so no timer is left.
  Future<void> letUndoGo(WidgetTester tester) async {
    await tester.pump(Announcements.undoLasts);
    await tester.pumpAndSettle();
  }

  group("the Agenda row's ⋮", () {
    testWidgets('snoozes an Occurrence due now, armed at once', (tester) async {
      final pills = madeAt(DateTime(2026, 10, 24, 6), 'pills', daily([at(8)]));
      await start(tester);
      await tap(tester, rowMenu(pills));

      expect(find.text('Snooze'), findsOneWidget);
      expect(find.text('Not today'), findsOneWidget);
      for (final answer in [
        'snooze10',
        'snooze30',
        'snooze60',
        'skip',
        'tomorrow',
      ]) {
        expect(key('answer-$answer'), findsOneWidget);
      }
      expect(find.text('30 min'), findsOneWidget);
      expect(find.text('History'), findsOneWidget);
      await tap(tester, snooze30);

      expect(
        Alarms(store).snoozes['${pills.id}@2026-10-24T08:00'],
        DateTime(2026, 10, 24, 9, 30).toUtc(),
      );
      // Armed without the app coming back.
      expect(alarms.ids, contains('snooze:${pills.id}@2026-10-24T08:00'));
      expect(announcements.visible, isNull);
    });

    testWidgets('offers no Snooze for an Occurrence still to come today', (
      tester,
    ) async {
      final pills = routines.create('pills', daily([at(20)]));
      await start(tester);
      await tap(tester, rowMenu(pills));

      expect(snooze30, findsNothing);
      expect(find.text('Snooze'), findsNothing);
      expect(skip, findsOneWidget);
      expect(tomorrow, findsOneWidget);
    });

    testWidgets('Skip strikes the row as skipped for the day, stops its '
        'alarm, and the offer at the top takes it back', (tester) async {
      final pills = madeAt(DateTime(2026, 10, 24, 6), 'pills', daily([at(8)]));
      final morning = Occurrence(today, at(8));
      await start(tester);
      await tap(tester, rowMenu(pills));
      await tap(tester, skip);

      expect(routines.byId(pills.id)?.skippedAt(morning), now.toUtc());
      final under = key('row-skipped-${pills.id}@${morning.key}');
      expect(under, findsOneWidget);
      expect(tester.widget<Text>(under).data, 'Skipped');
      final title = find.descendant(
        of: key('row-${pills.id}@${morning.key}'),
        matching: find.text('pills'),
      );
      expect(
        tester.widget<Text>(title).style?.decoration,
        TextDecoration.lineThrough,
      );
      expect(
        alarms.ids,
        isNot(contains('occurrence:${pills.id}@2026-10-24T08:00')),
      );
      expect(alarms.done, contains('${pills.id}@${morning.key}'));
      expect(announcements.visible, 'Skipped');
      expect(announcements.visibleUndoes, isTrue);

      // Answered once, the ⋮ offers neither again.
      await tap(tester, rowMenu(pills));
      expect(skip, findsNothing);
      expect(snooze30, findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tapVisible(tester, announcements);
      expect(routines.byId(pills.id)?.skippedAt(morning), isNull);
      expect(under, findsNothing);
      expect(alarms.ids, contains('occurrence:${pills.id}@2026-10-24T08:00'));
    });

    testWidgets('its box still checks a skipped Occurrence off', (
      tester,
    ) async {
      final pills = madeAt(DateTime(2026, 10, 24, 6), 'pills', daily([at(8)]));
      final morning = Occurrence(today, at(8));
      routines.skip(pills.id, morning);
      await start(tester);

      await tap(tester, find.byType(Checkbox).first);
      expect(routines.byId(pills.id)?.doneAt(morning), isNotNull);
      expect(key('row-skipped-${pills.id}@${morning.key}'), findsNothing);
    });

    testWidgets('Tomorrow skips it and shows it the day after at the same '
        'time', (tester) async {
      // On Saturdays only, so the day after has none of its own.
      final chores = madeAt(
        DateTime(2026, 10, 24, 6),
        'clean the bathroom',
        Schedule(
          start: Day(2026, 10, 1),
          times: [at(8)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=SA'),
        ),
      );
      final morning = Occurrence(today, at(8));
      final moved = Occurrence(Day(2026, 10, 25), at(8));
      await start(tester);
      await tap(tester, rowMenu(chores));
      await tap(tester, tomorrow);

      final routine = routines.byId(chores.id)!;
      expect(routine.skippedAt(morning), isNotNull);
      expect(routine.isMoved(moved), isTrue);
      expect(
        tester
            .widget<Text>(key('row-skipped-${chores.id}@${morning.key}'))
            .data,
        'The day after instead',
      );
      expect(key('row-moved-${chores.id}@${moved.key}'), findsOneWidget);
      expect(find.text('From the day before'), findsOneWidget);
      expect(alarms.ids, contains('occurrence:${chores.id}@${moved.key}'));
      expect(announcements.visible, 'The day after instead');

      await tapVisible(tester, announcements);
      expect(routines.byId(chores.id)?.isMoved(moved), isFalse);
      expect(key('row-moved-${chores.id}@${moved.key}'), findsNothing);
    });

    testWidgets('a skipped Occurrence leaves the Agenda where this Device '
        'hides what is checked', (tester) async {
      final pills = madeAt(DateTime(2026, 10, 24, 6), 'pills', daily([at(8)]));
      store.agendaHidesChecked = true;
      routines.skip(pills.id, Occurrence(today, at(8)));
      await start(tester);

      expect(key('row-${pills.id}@2026-10-24T08:00'), findsNothing);
      expect(key('row-${pills.id}@2026-10-25T08:00'), findsOneWidget);
    });
  });

  group('the notification', () {
    testWidgets('hands the platform the words of Not today', (tester) async {
      routines.create('pills', daily([at(20)]));
      await start(tester);

      expect(alarms.labels?.notToday, 'Not today');
      expect(
        alarms.labels?.notTodayQuestion,
        'Skip it, or tomorrow at the same time?',
      );
      expect(alarms.labels?.skip, 'Skip');
      expect(alarms.labels?.tomorrow, 'Tomorrow');
    });

    testWidgets('Skip and Tomorrow tapped on it reach the log and are '
        'acknowledged', (tester) async {
      final pills = madeAt(DateTime(2026, 10, 24, 6), 'pills', daily([at(8)]));
      final chores = madeAt(
        DateTime(2026, 10, 24, 6),
        'chores',
        Schedule(
          start: Day(2026, 10, 1),
          times: [at(8, 30)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=SA'),
        ),
      );
      final tapped = DateTime.utc(2026, 10, 24, 7, 5);
      alarms.pending.addAll([
        SkipTapped(1, pills.id, '2026-10-24T08:00', tapped),
        TomorrowTapped(2, chores.id, '2026-10-24T08:30', tapped),
      ]);
      await start(tester);

      expect(
        routines.byId(pills.id)?.skippedAt(Occurrence(today, at(8))),
        tapped,
      );
      final moved = routines.byId(chores.id)!;
      expect(moved.skippedAt(Occurrence(today, at(8, 30))), tapped);
      expect(moved.isMoved(Occurrence(Day(2026, 10, 25), at(8, 30))), isTrue);
      expect(alarms.acknowledged, 2);
      // Made outside the app: no offer to undo.
      expect(announcements.visible, isNull);
      expect(key('row-skipped-${pills.id}@2026-10-24T08:00'), findsOneWidget);
    });
  });

  group("the Note editor's Due line", () {
    /// From the Agenda, opens the editor of the Note showing [text].
    Future<void> openNote(WidgetTester tester, String text) async {
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
      await tester.longPress(find.text(text).first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    }

    testWidgets('snoozes beside Done, armed at once', (tester) async {
      now = DateTime(2026, 10, 24, 7);
      final note = Notes(store).capture('call mum').id;
      final reminder = routines.spawn(
        note,
        Schedule(start: today, times: [at(8)]),
      );
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      await openNote(tester, 'call mum');

      expect(find.text('Due 08:00'), findsOneWidget);
      await tap(tester, key('open-snooze'));
      expect(find.text('10 min'), findsOneWidget);
      expect(skip, findsNothing);
      await tap(tester, key('answer-snooze10'));

      expect(alarms.ids, contains('snooze:${reminder.id}@2026-10-24T08:00'));
      // Still due: Snooze is no answer that takes it off.
      expect(find.text('Due 08:00'), findsOneWidget);
    });

    testWidgets('Not today ▸ Skip takes it off, and the offer at the top '
        'brings it back', (tester) async {
      now = DateTime(2026, 10, 24, 7);
      final note = Notes(store).capture('call mum').id;
      final reminder = routines.spawn(
        note,
        Schedule(start: today, times: [at(8)]),
      );
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      await openNote(tester, 'call mum');

      await tap(tester, key('open-not-today'));
      expect(tomorrow, findsOneWidget);
      expect(snooze30, findsNothing);
      await tap(tester, skip);

      expect(
        routines.byId(reminder.id)?.skippedAt(Occurrence(today, at(8))),
        isNotNull,
      );
      expect(find.text('Due 08:00'), findsNothing);
      expect(announcements.visible, 'Skipped');

      await tapVisible(tester, announcements);
      expect(find.text('Due 08:00'), findsOneWidget);
    });

    testWidgets('a Missed Occurrence of an earlier day offers no Snooze and '
        'no Tomorrow', (tester) async {
      final note = Notes(store).capture('call mum').id;
      now = DateTime(2026, 10, 23, 7);
      routines.spawn(note, Schedule(start: Day(2026, 10, 23), times: [at(8)]));
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      await openNote(tester, 'call mum');

      expect(find.textContaining('Missed'), findsWidgets);
      expect(key('open-snooze'), findsNothing);
      await tap(tester, key('open-not-today'));
      expect(skip, findsOneWidget);
      expect(tomorrow, findsNothing);
      await tap(tester, skip);
      await letUndoGo(tester);
      expect(key('open-not-today'), findsNothing);
    });
  });

  group('the history', () {
    /// Pills at 08:00 since the 20th: Done on the 20th and 21st, skipped
    /// on the 22nd, Missed on the 23rd; today's still due.
    Routine pills() {
      final pills = madeAt(DateTime(2026, 10, 20, 6), 'pills', daily([at(8)]));
      for (final day in [20, 21]) {
        routines.markDone(
          pills.id,
          Occurrence(Day(2026, 10, day), at(8)),
          at: DateTime(2026, 10, day, 8, 5),
        );
      }
      routines.skip(
        pills.id,
        Occurrence(Day(2026, 10, 22), at(8)),
        at: DateTime(2026, 10, 22, 8, 1),
      );
      return pills;
    }

    Finder dot(String outcome) => find.byWidgetPredicate(
      (w) =>
          w is Semantics &&
          (w.properties.label?.endsWith(': $outcome') ?? false),
    );

    testWidgets('labels a skipped Occurrence, keeps it out of the Adherence '
        'and counts it beside it', (tester) async {
      final routine = pills();
      await start(tester);
      await tap(tester, rowMenu(routine));
      await tap(tester, find.text('History'));
      expect(find.byType(RoutineHistoryScreen), findsOneWidget);

      // 2 of 3: the skip is not part of the share, and counted beside it.
      expect(find.text('2 of 3 Done'), findsNWidgets(3));
      expect(find.text('1 skipped'), findsNWidgets(3));
      expect(dot('skipped'), findsOneWidget);
      expect(dot('Missed'), findsOneWidget);
      expect(dot('all Done'), findsNWidgets(2));

      final scrollable = find.byType(Scrollable).first;
      await tester.scrollUntilVisible(
        find.text('Skipped'),
        200,
        scrollable: scrollable,
      );
      expect(find.text('Skipped'), findsOneWidget);
    });

    testWidgets('fits a phone 360 wide in German', (tester) async {
      final routine = pills();
      // The Missed Summary's card is not this test's.
      Alarms(store).markSeen();
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      // 360 dp wide, a common phone.
      tester.view.physicalSize = const Size(1080, 2340);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: alarms,
          announcements: announcements,
        ),
      );
      await tester.pumpAndSettle();
      await tap(tester, rowMenu(routine));
      await tap(tester, find.text('Verlauf'));

      // An overflow fails the test.
      expect(find.text('1 ausgelassen'), findsNWidgets(3));
      expect(find.text('2 von 3 erledigt'), findsNWidgets(3));
    });

    testWidgets('labels an Occurrence Tomorrow skipped, and the one it '
        'added', (tester) async {
      final routine = madeAt(
        DateTime(2026, 10, 10, 6),
        'chores',
        Schedule(
          start: Day(2026, 10, 1),
          times: [at(8)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=TH'),
        ),
      );
      routines.tomorrow(
        routine.id,
        Occurrence(Day(2026, 10, 22), at(8)),
        at: DateTime(2026, 10, 22, 7),
      );
      await start(tester);
      // Next Thursday's row on the Agenda.
      await tap(tester, rowMenu(routine));
      await tap(tester, find.text('History'));
      expect(find.byType(RoutineHistoryScreen), findsOneWidget);

      expect(find.text('Skipped, the day after instead'), findsOneWidget);
      expect(find.text('From the day before'), findsOneWidget);
    });
  });
}

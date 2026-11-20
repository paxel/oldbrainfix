import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// The Heads-ups in the Routine editor (obf-1046): listed under the
/// schedule, the earliest warning first, removed by their ✕, added from
/// a menu at the finger, whole days only for an all-day schedule, saved
/// by Save and by leaving, and set by a Member who may change nothing
/// else.
void main() {
  late Store store;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    routines = Routines(store);
  });
  tearDown(() => store.close());

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  final panel = find.byKey(const ValueKey('headsUps'));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor [screen] over the Agenda of [on].
  Future<void> open(
    WidgetTester tester,
    Store on,
    Widget Function(Notes notes, Places places) screen,
  ) async {
    await tester.pumpWidget(OldBrainFixApp(store: on, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    final context = tester.element(find.byType(AgendaScreen));
    unawaited(
      Navigator.of(context).push(
        MaterialPageRoute<bool>(builder: (_) => screen(Notes(on), Places(on))),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(RoutineEditorScreen), findsOneWidget);
  }

  Future<void> edit(WidgetTester tester, Routine routine, {Store? on}) => open(
    tester,
    on ?? store,
    (notes, places) => RoutineEditorScreen(
      notes: notes,
      places: places,
      routineId: routine.id,
    ),
  );

  /// Scrolls the editor's lazy page until the Heads-ups are built and
  /// on screen.
  Future<void> showPanel(WidgetTester tester) async {
    await tester.scrollUntilVisible(
      panel,
      200,
      scrollable: find
          .descendant(
            of: find.byType(RoutineEditorScreen),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    await tester.pumpAndSettle();
  }

  /// The Heads-ups listed, in their order, in minutes.
  List<int> listed(WidgetTester tester) => [
    for (final chip in tester.widgetList<InputChip>(
      find.descendant(of: panel, matching: find.byType(InputChip)),
    ))
      int.parse((chip.key! as ValueKey<String>).value.split('-').last),
  ];

  /// The Heads-ups Add heads-up offers, in their order, in minutes; its
  /// menu closes again.
  Future<List<int>> offered(WidgetTester tester) async {
    await showPanel(tester);
    await tap(tester, find.byKey(const ValueKey('addHeadsUp')));
    final items = [
      for (final item in tester.widgetList<PopupMenuItem<int>>(
        find.byType(PopupMenuItem<int>),
      ))
        item.value!,
    ];
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
    return items;
  }

  Future<void> add(WidgetTester tester, int minutes) async {
    await showPanel(tester);
    await tap(tester, find.byKey(const ValueKey('addHeadsUp')));
    await tap(tester, find.byKey(ValueKey('addHeadsUp-$minutes')));
  }

  Future<void> remove(WidgetTester tester, int minutes) => tap(
    tester,
    find.descendant(
      of: find.byKey(ValueKey('headsUp-$minutes')),
      matching: find.byTooltip('Remove heads-up'),
    ),
  );

  testWidgets('lists the Heads-ups earliest first, removes and adds them, '
      'and Save writes them', (tester) async {
    final pills = routines.create('pills', daily([at(20)]));
    routines.setHeadsUps(pills.id, [20, 1440]);
    await edit(tester, pills);
    await showPanel(tester);
    expect(find.text('Heads-ups'), findsOneWidget);
    expect(listed(tester), [1440, 20]);
    expect(find.text('1 day before'), findsOneWidget);
    expect(find.text('20 min before'), findsOneWidget);
    // The timed choices, without the one set already.
    expect(await offered(tester), [5, 10, 15, 30, 60, 120, 2880, 10080]);

    await remove(tester, 20);
    await add(tester, 60);
    expect(listed(tester), [1440, 60]);
    expect(find.text('1 h before'), findsOneWidget);
    expect(routines.headsUps(pills.id), [20, 1440], reason: 'until Save');

    await tap(tester, find.text('Save'));
    expect(routines.headsUps(pills.id), [60, 1440]);
  });

  testWidgets('an all-day schedule offers and keeps whole days only', (
    tester,
  ) async {
    final bins = routines.create('bins', daily(const []));
    routines.setHeadsUps(bins.id, [30, 1440]);
    await edit(tester, bins);
    await showPanel(tester);
    expect(listed(tester), [1440]);
    expect(await offered(tester), [2880, 4320, 10080]);

    await add(tester, 4320);
    expect(find.text('3 days before'), findsOneWidget);
    expect(listed(tester), [4320, 1440]);
    await tap(tester, find.text('Save'));
    expect(routines.headsUps(bins.id), [1440, 4320]);
  });

  testWidgets('leaving with only a Heads-up changed saves it (#275)', (
    tester,
  ) async {
    final pills = routines.create('pills', daily([at(20)]));
    await edit(tester, pills);
    await add(tester, 15);
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(RoutineEditorScreen), findsNothing);
    expect(routines.headsUps(pills.id), [15]);
  });

  testWidgets('a new Routine gets its Heads-ups once it is made', (
    tester,
  ) async {
    await open(
      tester,
      store,
      (notes, places) => RoutineEditorScreen(notes: notes, places: places),
    );
    await tester.enterText(find.byType(TextField).first, 'water');
    await tester.pumpAndSettle();
    // Every day, all day, as a new Routine starts.
    await add(tester, 1440);
    await tap(tester, find.text('Save'));
    final water = routines.all().singleWhere(
      (r) => routines.note(r)?.text == 'water',
    );
    expect(routines.headsUps(water.id), [1440]);
  });

  group('in a Shared Area', () {
    late Store ben;
    late Routine shared;

    setUp(() {
      store
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ann');
      ben = Store.inMemory(clock: () => now)
        ..starterSetOffered = true
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ben');
      final house = Places(store).addArea('Household');
      final note = Notes(store).capture('Bins out', place: house.id);
      shared = routines.spawn(note.id, daily([at(19)]));
      routines.setHeadsUps(shared.id, [60]);
      final area = store.share(house.id);
      // Ann's computer invites Ben's phone as a Member.
      final done = answerSync(
        store,
        wire(
          syncRequest(
            ben,
            store.versionVector(),
            name: 'Phone',
            hostDevice: store.deviceId,
            invited: true,
          ),
        ),
        name: 'Desk',
        inviting: Invite(area, role: AreaRole.member),
      )!;
      applyAnswer(ben, wire(done.answer), shared: true);
    });
    tearDown(() => ben.close());

    testWidgets('a Member sets their own Heads-ups and Save writes nothing '
        'else', (tester) async {
      final theirs = Routines(ben);
      expect(ben.mayChangeRoutine(shared.id), isFalse);
      expect(
        theirs.headsUps(shared.id),
        isEmpty,
        reason: "Ann's Heads-ups are hers",
      );
      await edit(tester, shared, on: ben);
      expect(
        find.text('Only a Manager of this Area can change this.'),
        findsOne,
      );
      // The schedule is shown, not changed: a tap where its Once shows
      // goes nowhere, as the locked schedule takes no touch.
      final once = find.text('Once');
      await tester.ensureVisible(once);
      await tester.pumpAndSettle();
      await tester.tapAt(tester.getCenter(once));
      await tester.pumpAndSettle();
      expect(find.text('Starts'), findsOneWidget);

      await add(tester, 30);
      expect(listed(tester), [30]);
      await tap(tester, find.text('Save'));
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(theirs.headsUps(shared.id), [30]);
      expect(theirs.byId(shared.id)?.schedule.times, [at(19)]);
      expect(routines.headsUps(shared.id), [60]);
    });
  });
}

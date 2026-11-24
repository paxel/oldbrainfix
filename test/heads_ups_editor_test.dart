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

  /// The Heads-ups Add heads-up offers, in their order, in minutes,
  /// before Custom… at the end (obf-1087); its menu closes again.
  Future<List<int>> offered(WidgetTester tester) async {
    await showPanel(tester);
    await tap(tester, find.byKey(const ValueKey('addHeadsUp')));
    final items = tester
        .widgetList<PopupMenuItem<int>>(find.byType(PopupMenuItem<int>))
        .toList();
    expect(items.last.key, const ValueKey('addHeadsUp-custom'));
    expect(find.text('Custom…'), findsOneWidget);
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
    return [for (final item in items.take(items.length - 1)) item.value!];
  }

  /// Opens Custom…'s dialog from Add heads-up.
  Future<void> openCustom(WidgetTester tester) async {
    await showPanel(tester);
    await tap(tester, find.byKey(const ValueKey('addHeadsUp')));
    await tap(tester, find.byKey(const ValueKey('addHeadsUp-custom')));
    expect(find.text('Custom heads-up'), findsOneWidget);
  }

  final customAdd = find.byKey(const ValueKey('customHeadsUp-add'));

  bool addEnabled(WidgetTester tester) =>
      tester.widget<TextButton>(customAdd).onPressed != null;

  /// Types [count] in Custom…'s dialog and picks [unit] when given.
  Future<void> type(WidgetTester tester, String count, [String? unit]) async {
    await tester.enterText(
      find.byKey(const ValueKey('customHeadsUp-count')),
      count,
    );
    await tester.pumpAndSettle();
    if (unit != null) {
      await tap(tester, find.byKey(ValueKey('customHeadsUp-$unit')));
    }
  }

  /// Adds a custom Heads-up of [count] [unit]s.
  Future<void> addCustom(WidgetTester tester, String count, String unit) async {
    await openCustom(tester);
    await type(tester, count, unit);
    await tap(tester, customAdd);
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

  testWidgets('Custom… adds a number of minutes, hours, days or weeks, '
      'shown in the list and removed by its ✕ (obf-1087)', (tester) async {
    final pills = routines.create('pills', daily([at(20)]));
    await edit(tester, pills);
    await openCustom(tester);
    expect(
      [
        for (final unit in HeadsUpUnit.values)
          find.byKey(ValueKey('customHeadsUp-${unit.name}')),
      ].every((f) => f.evaluate().length == 1),
      isTrue,
    );
    await type(tester, '45', 'minutes');
    await tap(tester, customAdd);
    expect(find.text('Custom heads-up'), findsNothing);
    expect(listed(tester), [45]);
    expect(find.text('45 min before'), findsOneWidget);

    await addCustom(tester, '3', 'hours');
    expect(listed(tester), [180, 45]);
    expect(find.text('3 h before'), findsOneWidget);
    await remove(tester, 180);
    expect(listed(tester), [45]);

    await tap(tester, find.text('Save'));
    expect(routines.headsUps(pills.id), [45]);
  });

  testWidgets('a custom Heads-up already set shows, and its ✕ removes it', (
    tester,
  ) async {
    final pills = routines.create('pills', daily([at(20)]));
    routines.setHeadsUps(pills.id, [45, 7200]);
    await edit(tester, pills);
    await showPanel(tester);
    expect(listed(tester), [7200, 45]);
    expect(find.text('5 days before'), findsOneWidget);
    expect(find.text('45 min before'), findsOneWidget);
    await remove(tester, 45);
    await tap(tester, find.text('Save'));
    expect(routines.headsUps(pills.id), [7200]);
  });

  testWidgets('Custom… adds only from 1 to the most its unit takes, and '
      'Cancel adds nothing', (tester) async {
    final pills = routines.create('pills', daily([at(20)]));
    await edit(tester, pills);
    await openCustom(tester);
    expect(addEnabled(tester), isFalse, reason: 'nothing typed');
    expect(find.text('From 1 to 43200.'), findsOneWidget);
    await type(tester, '0');
    expect(addEnabled(tester), isFalse);
    await type(tester, '43200');
    expect(addEnabled(tester), isTrue);
    await type(tester, '721', 'hours');
    expect(find.text('From 1 to 720.'), findsOneWidget);
    expect(addEnabled(tester), isFalse);
    await type(tester, '5', 'weeks');
    expect(addEnabled(tester), isFalse);
    await type(tester, '4');
    expect(addEnabled(tester), isTrue);
    await tap(tester, find.text('Cancel'));
    expect(find.text('Custom heads-up'), findsNothing);
    expect(listed(tester), isEmpty);
  });

  testWidgets('an all-day schedule takes a custom Heads-up in days and '
      'weeks only', (tester) async {
    final bins = routines.create('bins', daily(const []));
    await edit(tester, bins);
    await openCustom(tester);
    expect(find.byKey(const ValueKey('customHeadsUp-minutes')), findsNothing);
    expect(find.byKey(const ValueKey('customHeadsUp-hours')), findsNothing);
    expect(find.text('From 1 to 30.'), findsOneWidget, reason: 'days first');
    await type(tester, '2', 'weeks');
    await tap(tester, customAdd);
    expect(listed(tester), [20160]);
    expect(find.text('2 weeks before'), findsOneWidget);
    await tap(tester, find.text('Save'));
    expect(routines.headsUps(bins.id), [20160]);
  });

  testWidgets('with every choice set, Add heads-up still offers Custom…, '
      'and a custom one already set is not set twice', (tester) async {
    final bins = routines.create('bins', daily(const []));
    routines.setHeadsUps(bins.id, HeadsUpKeys.allDayChoices);
    await edit(tester, bins);
    expect(await offered(tester), isEmpty);
    await addCustom(tester, '2', 'days');
    expect(listed(tester), [10080, 4320, 2880, 1440]);
    await tap(tester, find.text('Save'));
    expect(routines.headsUps(bins.id), HeadsUpKeys.allDayChoices);
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

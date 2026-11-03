import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/schedule_text.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

void main() {
  late Store store;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  final now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);

  setUpAll(initializeDateFormatting);

  setUp(() {
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    routines = Routines(store);
  });
  tearDown(() => store.close());

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  final line = find.byKey(const ValueKey('scheduleSentence'));

  String shown(WidgetTester tester) =>
      tester.widget<TextField>(line).controller!.text;

  Future<void> open(WidgetTester tester, String name) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    await tester.tap(find.text(name).first);
    await tester.pumpAndSettle();
  }

  Future<void> type(WidgetTester tester, String text) async {
    await tester.enterText(line, text);
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// The chips of the times of day.
  Future<List<String>> times(WidgetTester tester) async {
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('renag')),
      200,
      scrollable: find
          .descendant(
            of: find.byType(RoutineEditorScreen),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    await tester.pumpAndSettle();
    return [
      for (final chip in tester.widgetList<InputChip>(find.byType(InputChip)))
        ((chip.label as Text).data ?? ''),
    ];
  }

  bool picked(WidgetTester tester, String chip) =>
      tester.widget<FilterChip>(find.widgetWithText(FilterChip, chip)).selected;

  testWidgets('the line shows the schedule as a sentence and follows the '
      'chips', (tester) async {
    routines.create(
      'pills',
      Schedule(
        start: Day(2026, 10, 1),
        times: [at(8)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    await open(tester, 'pills');
    expect(shown(tester), 'Every day at 08:00, from 10/1');
    await tap(tester, find.widgetWithText(ChoiceChip, 'Weekdays'));
    // Weekdays start with the start's, a Thursday.
    expect(shown(tester), 'Weekdays Thu at 08:00, from 10/1');
    // A chip changed while typing writes the line anew.
    await type(tester, 'Weekdays Thu at 08:00, from 10/1, until 12/1');
    await tap(tester, find.widgetWithText(FilterChip, weekdayText('en', 1)));
    expect(shown(tester), 'Weekdays Mon, Thu at 08:00, from 10/1, until 12/1');
  });

  testWidgets('typing fills the chips and times live, and Save keeps the '
      'schedule, not the sentence', (tester) async {
    final pills = routines.create(
      'pills',
      Schedule(start: today, rule: Recurrence.parse('FREQ=DAILY')),
    );
    await open(tester, 'pills');
    expect(shown(tester), 'Every day');
    await type(tester, 'Weekdays Mon, Thu at 08:00, 15:00, until 12/31');
    expect(picked(tester, weekdayText('en', 1)), isTrue);
    expect(picked(tester, weekdayText('en', 4)), isTrue);
    expect(picked(tester, weekdayText('en', 2)), isFalse);
    expect(await times(tester), ['08:00', '15:00']);
    await tap(tester, find.text('Save'));
    expect(
      routines.byId(pills.id)?.schedule.rule?.format(),
      'FREQ=WEEKLY;UNTIL=20261231;BYDAY=MO,TH',
    );
    expect(routines.byId(pills.id)?.schedule.times, [at(8), at(15)]);
  });

  testWidgets('unknown words are marked and change nothing', (tester) async {
    routines.create(
      'pills',
      Schedule(start: today, rule: Recurrence.parse('FREQ=DAILY')),
    );
    await open(tester, 'pills');
    await type(tester, 'Every 2 wekks at 08:00');
    expect(
      find.text('Marked words are not understood and change nothing.'),
      findsOneWidget,
    );
    // The repeat is still every day; the times too.
    expect(
      tester
          .widget<ChoiceChip>(find.widgetWithText(ChoiceChip, 'Every …'))
          .selected,
      isTrue,
    );
    expect(await times(tester), isEmpty);
    await tester.ensureVisible(line);
    await type(tester, 'Every day at 08:00 please');
    expect(await times(tester), ['08:00']);
    // Left, the line shows the schedule again, without the unknown word.
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(shown(tester), 'Every day at 08:00');
    expect(
      find.text('Marked words are not understood and change nothing.'),
      findsNothing,
    );
  });

  testWidgets('suggestions offer the next word and put it in', (tester) async {
    routines.create(
      'pills',
      Schedule(start: today, rule: Recurrence.parse('FREQ=DAILY')),
    );
    await open(tester, 'pills');
    await type(tester, 'Every 2 w');
    final suggestions = find.byKey(const ValueKey('scheduleSuggestions'));
    expect(
      find.descendant(of: suggestions, matching: find.byType(ActionChip)),
      findsOneWidget,
    );
    await tap(tester, find.widgetWithText(ActionChip, 'weeks'));
    expect(shown(tester), 'Every 2 weeks ');
    expect(
      tester
          .widget<ChoiceChip>(find.widgetWithText(ChoiceChip, 'weeks'))
          .selected,
      isTrue,
    );
    // Then what may follow the repeat.
    expect(find.widgetWithText(ActionChip, 'at'), findsOneWidget);
    expect(find.widgetWithText(ActionChip, 'until'), findsOneWidget);
  });

  testWidgets('an Advanced schedule shows a read-only summary', (tester) async {
    routines.create(
      'pills',
      Schedule(start: today, rule: Recurrence.parse('FREQ=DAILY;BYHOUR=8,20')),
    );
    await open(tester, 'pills');
    expect(find.text('Switch to Simple to type.'), findsOneWidget);
    expect(tester.widget<TextField>(line).readOnly, isTrue);
    expect(shown(tester), 'Rule: FREQ=DAILY;BYHOUR=8,20 · 08:00, 20:00');
  });

  testWidgets('in German, with the app\'s weekday names', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    routines.create(
      'pills',
      Schedule(
        start: today,
        times: [at(8)],
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO,TH'),
      ),
    );
    await open(tester, 'pills');
    expect(
      shown(tester),
      'Wochentags ${weekdayText('de', 1)}, ${weekdayText('de', 4)} um 08:00',
    );
    await type(tester, 'Alle 3 Tage um 07:30');
    expect(await times(tester), ['07:30']);
  });

  test('the sentence names weekdays as the app does', () {
    for (final locale in ['en', 'de']) {
      final all = SimpleSchedule(
        const SimpleRepeat.weekdays({1, 2, 3, 4, 5, 6, 7}),
        today,
        const [],
      );
      final names = [for (var d = 1; d <= 7; d++) weekdayText(locale, d)];
      expect(scheduleSentence(all, locale, today), endsWith(names.join(', ')));
      for (var d = 1; d <= 7; d++) {
        final nth = SimpleSchedule(
          SimpleRepeat.monthWeekday(WeekdayRule(d, 1)),
          today,
          const [],
        );
        expect(
          scheduleSentence(nth, locale, today),
          endsWith(weekdayName(locale, d)),
        );
      }
    }
  });
}

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A quiet day on the Agenda (obf-1044): "Quiet today" in its bar, or
/// in ⋮ when the bar is full, and while it holds the line "Quiet day
/// until midnight" on top with End.
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

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  final entry = find.byKey(const ValueKey('quietToday'));
  final line = find.byKey(const ValueKey('quietDayLine'));
  final more = find.byKey(const ValueKey('bar-more'));

  /// Whether "Quiet today" is offered, on the bar or in ⋮; leaves ⋮
  /// closed.
  Future<bool> offered(WidgetTester tester) async {
    if (entry.evaluate().isNotEmpty) return true;
    if (more.evaluate().isEmpty) return false;
    await tester.tap(more);
    await tester.pumpAndSettle();
    final found = entry.evaluate().isNotEmpty;
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
    return found;
  }

  testWidgets('Quiet today shows the line, End takes it away again', (
    tester,
  ) async {
    routines.create(
      'pills',
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(18, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    await start(tester);
    expect(line, findsNothing);
    expect(await offered(tester), isTrue);

    if (entry.evaluate().isEmpty) {
      await tester.tap(more);
      await tester.pumpAndSettle();
    }
    await tester.tap(entry);
    await tester.pumpAndSettle();
    expect(store.quietUntil(deviceWallClock), isNotNull);
    expect(line, findsOneWidget);
    expect(find.text('Quiet day until midnight'), findsOneWidget);
    expect(await offered(tester), isFalse);
    // Today's Occurrence stays on the Agenda, to be checked off.
    expect(find.text('pills'), findsOneWidget);
    expect(find.byType(Checkbox), findsWidgets);
    // First in the Agenda's order.
    expect(
      tester.getTopLeft(line).dy,
      lessThan(tester.getTopLeft(find.text('Today')).dy),
    );

    await tester.tap(find.byKey(const ValueKey('quietDayEnd')));
    await tester.pumpAndSettle();
    expect(store.quietUntil(deviceWallClock), isNull);
    expect(line, findsNothing);
    expect(await offered(tester), isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a quiet day made elsewhere shows, and goes at midnight', (
    tester,
  ) async {
    now = DateTime(2026, 10, 24, 23, 59);
    await start(tester);
    expect(line, findsNothing);

    // As a sync's writes do, from beside the screen.
    store.quietToday(deviceWallClock);
    await tester.pumpAndSettle();
    expect(line, findsOneWidget);
    expect(await offered(tester), isFalse);

    now = DateTime(2026, 10, 25, 0, 0, 30);
    await tester.pump(const Duration(minutes: 1, seconds: 1));
    await tester.pumpAndSettle();
    expect(line, findsNothing);
    expect(await offered(tester), isTrue);
    expect(tester.takeException(), isNull);
  });
}

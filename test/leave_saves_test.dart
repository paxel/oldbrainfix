import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/tally_entry_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Leaving the Routine editor and a Tally entry by Back (#275): left as
/// it opened it writes nothing, a change that can be saved is saved, and
/// one that can't asks "Discard changes?" once. The New Tally and New
/// Session pages are in browser_add_test.dart.
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  late Routines routines;
  late Tallies tallies;

  setUp(() {
    // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
    final now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    places = Places(store);
    routines = Routines(store);
    tallies = Tallies(store);
  });
  tearDown(() => store.close());

  Finder key(String k) => find.byKey(ValueKey(k));
  final discardDialog = find.text('Discard changes?');

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Starts the app on a tall phone; a context to open pages from.
  Future<BuildContext> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(2400, 3600);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    return tester.element(find.byType(Scaffold).first);
  }

  /// Back by the system, as the gesture or the button.
  Future<void> systemBack(WidgetTester tester) async {
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
  }

  /// Back by the bar's arrow.
  Future<void> barBack(WidgetTester tester) async {
    await tester.tap(find.byType(BackButton));
    await tester.pumpAndSettle();
  }

  Schedule daily() => Schedule(
    start: Day(2026, 10, 1),
    times: [ClockTime(20, 0)],
    rule: const SimpleRepeat.every(1, RepeatUnit.days).toRule(),
  );

  group('the Routine editor', () {
    testWidgets('left as it opened writes nothing', (tester) async {
      final pills = routines.create('pills', daily());
      final context = await start(tester);
      final done = openRoutine(context, notes, places, pills);
      await tester.pumpAndSettle();
      await systemBack(tester);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(discardDialog, findsNothing);
      expect(await done, isFalse);
      expect(routines.byId(pills.id)!.escalation, isNull);
    });

    testWidgets('a change is saved by the bar\'s arrow', (tester) async {
      final pills = routines.create('pills', daily());
      final context = await start(tester);
      final done = openRoutine(context, notes, places, pills);
      await tester.pumpAndSettle();
      await tap(tester, key('escalation'));
      await tap(tester, key('escalationOn'));
      await barBack(tester);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(await done, isTrue);
      expect(routines.byId(pills.id)!.escalation, isNotNull);
    });

    testWidgets('a new Routine is made by Back once it has a text', (
      tester,
    ) async {
      final context = await start(tester);
      final done = newRoutine(context, notes, places);
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).first, 'water');
      await tester.pumpAndSettle();
      await systemBack(tester);
      expect(await done, isTrue);
      expect(routines.all().single.isTrashed, isFalse);
      expect(notes.all().single.text, 'water');
    });

    testWidgets('a past Reminder whose schedule stays saves the rest', (
      tester,
    ) async {
      final dentist = routines.create(
        'dentist',
        Schedule(start: Day(2026, 10, 2), times: [ClockTime(10, 0)]),
      );
      final context = await start(tester);
      final done = openRoutine(context, notes, places, dentist);
      await tester.pumpAndSettle();
      await tap(tester, key('escalation'));
      await tap(tester, key('escalationOn'));
      await systemBack(tester);
      expect(discardDialog, findsNothing);
      expect(await done, isTrue);
      expect(routines.byId(dentist.id)!.escalation, isNotNull);
    });

    testWidgets('a changed schedule that never comes up asks; Discard keeps '
        'the one saved', (tester) async {
      final pills = routines.create('pills', daily());
      final context = await start(tester);
      final done = openRoutine(context, notes, places, pills);
      await tester.pumpAndSettle();
      await tester.enterText(
        key('scheduleSentence'),
        'Every day at 20:00, until 10/2',
      );
      await tester.pumpAndSettle();
      expect(find.text('This never comes up.'), findsOneWidget);

      await systemBack(tester);
      expect(discardDialog, findsOneWidget);
      await tap(tester, key('leave-discard'));
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(await done, isFalse);
      expect(
        routines.byId(pills.id)!.schedule.next(Day(2026, 10, 24), 1),
        isNotEmpty,
      );
    });

    testWidgets('a new Session without a pool asks; Keep editing shows the '
        'pool to fix, Discard makes nothing', (tester) async {
      final context = await start(tester);
      final done = newSession(context, notes, places);
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).first, 'Sport');
      await tester.pumpAndSettle();

      await systemBack(tester);
      expect(discardDialog, findsOneWidget);
      await tap(tester, key('leave-keep-editing'));
      expect(discardDialog, findsNothing);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(key('session-pool-error'), findsOneWidget);

      await barBack(tester);
      expect(discardDialog, findsOneWidget);
      await tap(tester, key('leave-discard'));
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(await done, isFalse);
      expect(notes.all(), isEmpty);
      expect(routines.all(), isEmpty);
    });
  });

  group('a Tally entry', () {
    late String noteId;
    late String pages;

    setUp(() {
      noteId = notes.capture('Reading').id;
      pages = tallies.addField(noteId, name: 'Pages', target: 10);
    });

    testWidgets('left as it opened, targets and all, writes nothing', (
      tester,
    ) async {
      final context = await start(tester);
      final done = addTallyEntry(context, notes, noteId);
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(key('tally-value-$pages')).controller?.text,
        '10',
      );
      await systemBack(tester);
      expect(find.byType(TallyEntryScreen), findsNothing);
      expect(await done, isFalse);
      expect(tallies.entries(noteId), isEmpty);
    });

    testWidgets('a value typed is saved by Back', (tester) async {
      final context = await start(tester);
      final done = addTallyEntry(context, notes, noteId);
      await tester.pumpAndSettle();
      await tester.enterText(key('tally-value-$pages'), '12');
      await systemBack(tester);
      expect(find.byType(TallyEntryScreen), findsNothing);
      expect(await done, isTrue);
      expect(tallies.entries(noteId).single.values[pages], 12);
    });

    testWidgets('an entry changed is saved by the bar\'s arrow', (
      tester,
    ) async {
      final entry = tallies.add(noteId, values: {pages: 5});
      final context = await start(tester);
      final done = editTallyEntry(context, notes, noteId, entry.id);
      await tester.pumpAndSettle();
      await tester.enterText(key('tally-entry-line'), 'In the train');
      await barBack(tester);
      expect(await done, isTrue);
      final saved = tallies.entryById(entry.id)!;
      expect(saved.line, 'In the train');
      expect(saved.values[pages], 5);
    });

    testWidgets('a tick alone is saved by Back, new or changed', (
      tester,
    ) async {
      final read = tallies.addField(noteId, name: 'Read aloud', check: true);
      final context = await start(tester);
      final added = addTallyEntry(context, notes, noteId);
      await tester.pumpAndSettle();
      await tap(tester, key('tally-check-$read'));
      await systemBack(tester);
      expect(await added, isTrue);
      final entry = tallies.entries(noteId).single;
      expect(entry.values[read], 1);

      final edited = editTallyEntry(context, notes, noteId, entry.id);
      await tester.pumpAndSettle();
      await tap(tester, key('tally-check-$read'));
      await systemBack(tester);
      expect(await edited, isTrue);
      expect(tallies.entryById(entry.id)!.values.containsKey(read), isFalse);
    });

    testWidgets('a value that is no number asks; Keep editing shows it, '
        'Discard writes nothing', (tester) async {
      final context = await start(tester);
      final done = addTallyEntry(context, notes, noteId);
      await tester.pumpAndSettle();
      await tester.enterText(key('tally-value-$pages'), 'many');

      await systemBack(tester);
      expect(discardDialog, findsOneWidget);
      await tap(tester, key('leave-keep-editing'));
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      expect(
        find.text('Not a number. Type it like 12, 2.5 or 1:30.'),
        findsOneWidget,
      );

      await systemBack(tester);
      await tap(tester, key('leave-discard'));
      expect(find.byType(TallyEntryScreen), findsNothing);
      expect(await done, isFalse);
      expect(tallies.entries(noteId), isEmpty);
    });
  });
}

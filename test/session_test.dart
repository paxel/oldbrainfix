import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/all_routines_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/session_editor.dart';
import 'package:oldbrainfix/src/tally_entry_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';
import 'reveal.dart';

void main() {
  late Store store;
  late Notes notes;
  late Tallies tallies;
  late Sessions sessions;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Occurrence(Day(2026, 10, 24), ClockTime(7, 0));

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    tallies = Tallies(store);
    sessions = Sessions(store);
  });
  tearDown(() => store.close());

  /// A Tally [name] with one field and its target, a minute after the
  /// last, as its Note.
  String tally(String name, {String unit = 'times', double target = 20}) {
    now = now.add(const Duration(minutes: 1));
    final id = notes.capture(name).id;
    tallies.addField(id, name: 'Done', unit: unit, target: target);
    return id;
  }

  /// "Sport", daily at 07:00 from today.
  Session session(
    List<String> pool, {
    int count = 2,
    SessionRule rule = SessionRule.rolling,
  }) => sessions.spawn(
    notes.capture('Sport').id,
    Schedule(
      start: Day(2026, 10, 24),
      times: [ClockTime(7, 0)],
      rule: Recurrence(frequency: Frequency.daily),
    ),
    pool: pool,
    count: count,
    rule: rule,
  );

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    if (finder.evaluate().isEmpty) await reveal(tester, finder);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> enter(WidgetTester tester, Finder finder, String text) async {
    if (finder.evaluate().isEmpty) await reveal(tester, finder);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.enterText(finder, text);
    await tester.pumpAndSettle();
  }

  Finder key(String value) => find.byKey(ValueKey(value));

  /// The picks of today's Occurrence as written.
  List<String> picked(Session s) => [
    for (final pick in sessions.on(s.id, today).picks) pick.noteId,
  ];

  /// Whether the text [text] shows struck through.
  bool struck(WidgetTester tester, String text) =>
      tester.widget<Text>(find.text(text)).style?.decoration ==
      TextDecoration.lineThrough;

  /// Opens the editor of the Note [id] from the Agenda.
  Future<void> edit(WidgetTester tester, String id) async {
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, Places(store), notes.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  group('where a Session starts', () {
    testWidgets('the Note editor offers Routine, Reminder, Frog and Session; '
        'Session opens the editor the card\'s menu opens', (tester) async {
      final pushUps = tally('Push-ups');
      final id = notes.capture('Sport').id;
      await start(tester);
      await edit(tester, id);
      for (final item in ['Routine', 'Reminder', 'Frog', 'Session']) {
        expect(find.widgetWithText(FilledButton, item), findsOneWidget);
      }

      await tap(tester, key('spawn-session'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('New Session'), findsOneWidget);
      await tap(tester, key('session-pool-$pushUps'));
      await tap(tester, find.text('Save'));

      final made = sessions.all().single;
      expect(routinesNote(store, made)?.id, id);
      expect(made.pool, [pushUps]);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('with no Tally, the Note editor and All Routines and '
        'Reminders offer a Session, whose editor says to make a Tally and '
        'saves nothing', (tester) async {
      const hint =
          'No Tallies yet. "Make it a Tally" in a Note\'s ⋮ makes one.';
      final id = notes.capture('Sport').id;
      await start(tester);
      await edit(tester, id);
      await tap(tester, key('spawn-session'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('New Session'), findsOneWidget);
      expect(find.text(hint), findsOneWidget);
      await tap(tester, find.text('Save'));
      expect(key('session-pool-error'), findsOneWidget);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(sessions.all(), isEmpty);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();

      await tap(tester, key('allRoutines'));
      expect(find.byType(AllRoutinesScreen), findsOneWidget);
      await tap(tester, key('new-session'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text(hint), findsOneWidget);
      await tester.enterText(
        find
            .descendant(
              of: find.byType(RoutineEditorScreen),
              matching: find.byType(TextField),
            )
            .first,
        'Morning sport',
      );
      await tester.pumpAndSettle();
      final before = notes.unsorted().length;
      await tap(tester, find.text('Save'));
      expect(key('session-pool-error'), findsOneWidget);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(sessions.all(), isEmpty);
      expect(notes.unsorted(), hasLength(before), reason: 'no stray Note');
    });

    testWidgets('"New Session" on All Routines and Reminders makes the '
        'Session with its Note from the text typed', (tester) async {
      final pushUps = tally('Push-ups');
      await start(tester);
      await tap(tester, key('allRoutines'));
      await tap(tester, key('new-session'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('New Session'), findsOneWidget);

      await tester.enterText(
        find
            .descendant(
              of: find.byType(RoutineEditorScreen),
              matching: find.byType(TextField),
            )
            .first,
        'Morning sport',
      );
      await tester.pumpAndSettle();
      await tap(tester, key('session-pool-$pushUps'));
      await tap(tester, find.text('Save'));

      final made = sessions.all().single;
      final note = routinesNote(store, made);
      expect(note?.text, 'Morning sport');
      expect(made.pool, [pushUps]);
      // Its Note is no Unsorted one, as a new Routine's is not.
      expect(notes.unsorted().map((n) => n.id), isNot(contains(note?.id)));
      expect(find.byType(AllRoutinesScreen), findsOneWidget);
      expect(find.text('Morning sport'), findsOneWidget);
    });
  });

  group('the Session editor', () {
    testWidgets('"Session" in the card\'s menu opens the Routine editor with '
        'the pool, the count, the weekdays\' counts and the rule; Save says '
        'under each field what to fix', (tester) async {
      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit', unit: 'min', target: 60);
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'Sport');
      await tap(tester, find.text('Save'));

      await tap(tester, find.byTooltip('Routine, Reminder, Frog or Session'));
      expect(find.byType(BottomSheet), findsNothing);
      for (final item in ['Routine', 'Reminder', 'Frog']) {
        expect(find.text(item), findsOneWidget);
      }
      await tap(tester, find.text('Session'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('New Session'), findsOneWidget);
      // The rule is a column of radio buttons.
      expect(
        find.descendant(
          of: key('session-rule'),
          matching: find.byType(RadioListTile<SessionRule>),
        ),
        findsNWidgets(3),
      );

      await tap(tester, find.text('Save'));
      expect(find.text('Choose at least one Tally.'), findsOneWidget);
      expect(sessions.all(), isEmpty);

      await enter(tester, key('session-count'), '0');
      expect(find.text('Type a number from 1 to 99.'), findsOneWidget);
      await enter(tester, key('session-count'), '2');
      expect(find.text('Type a number from 1 to 99.'), findsNothing);
      await tap(tester, key('session-pool-$pushUps'));
      await tap(tester, key('session-pool-$wallSit'));
      expect(find.text('Choose at least one Tally.'), findsNothing);
      await tap(tester, find.text('Random'));

      // A weekday's count is folded away, and fixed where it is typed.
      expect(key('session-count-6'), findsNothing);
      await tap(tester, find.text('Count per weekday'));
      await enter(tester, key('session-count-6'), '100');
      expect(find.text('Type a number from 1 to 99.'), findsWidgets);
      await tap(tester, find.text('Save'));
      expect(sessions.all(), isEmpty);
      await enter(tester, key('session-count-6'), '1');
      expect(find.text('1 weekday with its own count'), findsOneWidget);
      await tap(tester, find.text('Save'));

      final made = sessions.all().single;
      expect(routinesNote(store, made)?.text, 'Sport');
      // Newest capture first, as offered.
      expect(made.pool, [wallSit, pushUps]);
      expect(made.count, 2);
      expect(made.counts, {DateTime.saturday: 1});
      expect(made.rule, SessionRule.random);
      expect(store.localSetting(SessionDraft.weekdaysOpenKey), 'true');
      // Saturday picks one, at the editor's time of day.
      final occurrence = made.routine.schedule.on(Day(2026, 10, 24)).single;
      expect(sessions.on(made.id, occurrence).picks, hasLength(1));
    });

    testWidgets('a Session opens in its editor; the pool is ordered by its '
        'handles for Rolling and saves what changed', (tester) async {
      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit', unit: 'min', target: 60);
      final bridge = tally('Half bridge', target: 15);
      final made = session([pushUps, wallSit, bridge]);
      await start(tester);
      await tap(tester, find.text('Sport').first);
      expect(find.text('Session'), findsOneWidget);
      for (final id in [pushUps, wallSit, bridge]) {
        expect(
          tester.widget<CheckboxListTile>(key('session-pool-$id')).value,
          isTrue,
        );
      }

      final handle = key('session-move-$pushUps');
      await tester.ensureVisible(handle);
      await tester.pumpAndSettle();
      final drag = await tester.startGesture(tester.getCenter(handle));
      await tester.pump(const Duration(milliseconds: 100));
      for (var i = 0; i < 12; i++) {
        await drag.moveBy(const Offset(0, 15));
        await tester.pump(const Duration(milliseconds: 16));
      }
      await drag.up();
      await tester.pumpAndSettle();
      await tap(tester, key('session-pool-$bridge'));
      await tap(tester, find.text('Save'));

      final saved = sessions.byId(made.id);
      expect(saved?.pool, [wallSit, pushUps]);
      expect(saved?.counts, isEmpty);
      expect(
        store
            .history(made.id)
            .where((e) => e.field.startsWith('${SessionKeys.count}:')),
        isEmpty,
        reason: 'no weekday count written that was not typed',
      );
    });

    testWidgets('only Rolling has an order to drag', (tester) async {
      final pushUps = tally('Push-ups');
      session([pushUps, tally('Squats')]);
      await start(tester);
      await tap(tester, find.text('Sport').first);
      expect(key('session-move-$pushUps'), findsOneWidget);
      await tap(tester, find.text('Least selected'));
      expect(key('session-move-$pushUps'), findsNothing);
    });
  });

  group('the Agenda', () {
    testWidgets('today\'s row lists the picks with their targets; tapping one '
        'records an entry prefilled with them; "Another one" swaps; once '
        'each has an entry the box stays checked', (tester) async {
      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit', unit: 'min', target: 60);
      final bridge = tally('Half bridge', target: 15);
      final made = session([pushUps, wallSit, bridge]);
      await start(tester);

      expect(find.text('Push-ups 20 times'), findsOneWidget);
      expect(find.text('Wall sit 1:00 min'), findsOneWidget);
      expect(find.text('Half bridge 15 times'), findsNothing);
      // Written as the Agenda showed it, for every own Device.
      expect(
        store.currentFields(made.id)[SessionKeys.pick(today)],
        '$pushUps,$wallSit',
      );

      await tap(tester, key('session-another-$wallSit'));
      expect(picked(made), [pushUps, bridge]);
      expect(find.text('Wall sit 1:00 min'), findsNothing);
      expect(find.text('Half bridge 15 times'), findsOneWidget);

      await tap(tester, key('session-pick-$pushUps'));
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller?.text,
        '20',
      );
      expect(key('tally-entry-date'), findsNothing, reason: 'counts for now');
      await tap(tester, key('tally-entry-save'));

      final first = sessions.on(made.id, today).picks.first;
      expect(first.entries.single.values.values.single, 20);
      expect(struck(tester, 'Push-ups 20 times'), isTrue);
      expect(key('session-another-$pushUps'), findsNothing);
      expect(key('session-another-$bridge'), findsOneWidget);
      var box = tester.widget<Checkbox>(find.byType(Checkbox).first);
      expect(box.value, isFalse);

      await tap(tester, key('session-pick-$bridge'));
      await tap(tester, key('tally-entry-save'));
      expect(sessions.on(made.id, today).complete, isTrue);
      box = tester.widget<Checkbox>(find.byType(Checkbox).first);
      expect(box.value, isTrue);
      expect(box.onChanged, isNull, reason: 'stays checked');
      expect(find.text('Sport'), findsWidgets);
      expect(struck(tester, 'Half bridge 15 times'), isTrue);
    });

    testWidgets('a pick with an entry opens that entry; removing it undoes '
        'the Done', (tester) async {
      final pushUps = tally('Push-ups');
      final made = session([pushUps], count: 1);
      sessions.pickToday(deviceWallClock);
      final entry = sessions.record(
        made.id,
        today,
        pushUps,
        values: {tallies.byNote(pushUps)!.shown.single.id: 20},
      );
      await start(tester);
      expect(tester.widget<Checkbox>(find.byType(Checkbox).first).value, true);

      await tap(tester, key('session-pick-$pushUps'));
      expect(find.text('Entry'), findsOneWidget);
      await tap(tester, key('tally-entry-trash'));
      await tap(tester, find.text('Delete').last);

      expect(tallies.entryById(entry.id)?.isTrashed, isTrue);
      final box = tester.widget<Checkbox>(find.byType(Checkbox).first);
      expect(box.value, isFalse);
      expect(box.onChanged, isNotNull);
    });

    testWidgets('tomorrow and later days show the title and how many it '
        'picks, with no picks yet', (tester) async {
      final pushUps = tally('Push-ups');
      final made = session([pushUps, tally('Squats'), tally('Plank')]);
      sessions.setCountOn(made.id, DateTime.sunday, 1);
      await start(tester);

      // Tomorrow is a Sunday.
      expect(find.text('Picks 1 Tally'), findsOneWidget);
      expect(find.text('Picks 2 Tallies'), findsWidgets);
      expect(find.text('Push-ups 20 times'), findsOneWidget);
      final occurrences = sessions.store
          .currentFields(made.id)
          .keys
          .where((field) => field.startsWith(SessionKeys.pickPrefix));
      expect(occurrences, [SessionKeys.pick(today)]);
    });
  });
}

/// The Note the Session [session] was spawned from.
Note? routinesNote(Store store, Session session) =>
    Routines(store).note(session.routine);

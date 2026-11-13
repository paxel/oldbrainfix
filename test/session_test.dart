import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/all_routines_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/session_editor.dart';
import 'package:oldbrainfix/src/session_form_screen.dart';
import 'package:oldbrainfix/src/tally_form_screen.dart';
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

  Finder key(String value) => find.byKey(ValueKey(value));

  /// The minus or, with [plus], the plus button of the stepper [stepper].
  Finder step(String stepper, {bool plus = false}) => find.descendant(
    of: key(stepper),
    matching: find.widgetWithIcon(IconButton, plus ? Icons.add : Icons.remove),
  );

  /// Whether the button [button] can be pressed.
  bool enabled(WidgetTester tester, Finder button) =>
      tester.widget<IconButton>(button).onPressed != null;

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

  group('past runs (#264)', () {
    testWidgets('the card and the page list the past runs, newest first, '
        'each pick done or not done', (tester) async {
      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit', unit: 'min', target: 60);
      final made = session([pushUps, wallSit]);
      DateTime wall(DateTime utc) => utc.toLocal();
      sessions.pickToday(wall);
      sessions.record(made.id, today, pushUps, line: 'Morning');
      // The next day, the run of yesterday is past; today's is not.
      now = now.add(const Duration(days: 1));
      sessions.pickToday(wall);
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
      await reveal(tester, key('session-runs'));
      expect(key('session-run-0'), findsOneWidget);
      expect(key('session-run-1'), findsNothing);
      expect(
        find.descendant(
          of: key('session-run-0'),
          matching: find.text('Push-ups: done'),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: key('session-run-0'),
          matching: find.text('Wall sit: not done'),
        ),
        findsOneWidget,
      );

      // Its page too, from the Agenda.
      await tester.pumpWidget(const SizedBox());
      await start(tester);
      await edit(tester, made.noteId);
      await reveal(tester, key('session-runs'));
      expect(find.text('Push-ups: done'), findsOneWidget);
    });
  });

  group('where a Session starts', () {
    testWidgets('the Note editor\'s "+" offers Routine, Reminder, Frog and '
        'Session; Session opens the editor the card\'s menu opens', (
      tester,
    ) async {
      final pushUps = tally('Push-ups');
      final id = notes.capture('Sport').id;
      await start(tester);
      await edit(tester, id);
      await tap(tester, key('badge-add'));
      for (final item in ['Routine', 'Reminder', 'Frog', 'Session']) {
        expect(find.widgetWithText(PopupMenuItem<Spawn>, item), findsOneWidget);
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
      await tap(tester, key('badge-add'));
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
      // A new Session starts All day (#163).
      expect(made.routine.schedule.times, isEmpty);
      // Its Note is no Unsorted one, as a new Routine's is not.
      expect(notes.unsorted().map((n) => n.id), isNot(contains(note?.id)));
      expect(find.byType(AllRoutinesScreen), findsOneWidget);
      expect(find.text('Morning sport'), findsOneWidget);
    });

    testWidgets('"New Session" on the Agenda\'s bar opens the New Session '
        'page and makes the Session (#246)', (tester) async {
      final pushUps = tally('Push-ups');
      await start(tester);
      final button = key('newSession');
      expect(
        find.descendant(of: button, matching: find.byIcon(Icons.event_repeat)),
        findsOneWidget,
      );
      expect(find.byTooltip('New Session'), findsOneWidget);
      await tap(tester, button);
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
      expect(sessions.all().single.pool, [pushUps]);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('in Simple the Agenda\'s bar has no "New Session" (#197)', (
      tester,
    ) async {
      store.advanced = false;
      await start(tester);
      expect(key('newSession'), findsNothing);
      expect(find.byTooltip('New Session'), findsNothing);
      expect(find.byTooltip('New Routine or Reminder'), findsOneWidget);
    });
  });

  group('the Session editor', () {
    testWidgets('"Session" in the card\'s menu opens the Routine editor with '
        'the pool, the count and the weekdays\' counts as − / + steppers and '
        'the rule; Save asks for a Tally', (tester) async {
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

      // No typed number: the count starts at 1, which minus cannot go
      // below.
      expect(
        find.descendant(of: key('session-count'), matching: find.text('1')),
        findsOneWidget,
      );
      expect(enabled(tester, step('session-count')), isFalse);
      await tap(tester, step('session-count', plus: true));
      expect(
        find.descendant(of: key('session-count'), matching: find.text('2')),
        findsOneWidget,
      );
      expect(enabled(tester, step('session-count')), isTrue);
      await tap(tester, key('session-pool-$pushUps'));
      await tap(tester, key('session-pool-$wallSit'));
      expect(find.text('Choose at least one Tally.'), findsNothing);
      await tap(tester, find.text('Random'));

      // The weekdays are folded away; each picks as every day until its
      // switch gives it a count of its own, starting from every day's.
      expect(key('session-own-6'), findsNothing);
      await tap(tester, find.text('Count per weekday'));
      expect(find.text('As every day'), findsNWidgets(7));
      expect(key('session-count-6'), findsNothing);
      await tap(tester, key('session-own-6'));
      expect(find.text('As every day'), findsNWidgets(6));
      expect(
        find.descendant(of: key('session-count-6'), matching: find.text('2')),
        findsOneWidget,
      );
      await tap(tester, step('session-count-6'));
      expect(enabled(tester, step('session-count-6')), isFalse);
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
  });

  group('the Session Note\'s page', () {
    testWidgets('the kind badge makes a plain Note a Session through its '
        'setup page, which asks for a Tally, and makes it plain again', (
      tester,
    ) async {
      final pushUps = tally('Push-ups');
      final id = notes.capture('Sport\nMorning set').id;
      await start(tester);
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      expect(key('kind-session'), findsOneWidget);
      await tap(tester, key('kind-session'));
      expect(find.byType(SessionFormScreen), findsOneWidget);
      expect(key('session-form-name'), findsNothing);
      expect(key('session-form-schedule'), findsNothing);
      await tap(tester, key('session-form-save'));
      expect(key('session-pool-error'), findsOneWidget);
      expect(sessions.isSessionNote(id), isFalse);
      await tap(tester, key('session-pool-$pushUps'));
      await tap(tester, key('session-form-save'));

      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(NoteKinds(store).of(id), NoteKind.session);
      expect(sessions.setupOf(id)?.pool, [pushUps]);
      expect(sessions.ofNote(id), isEmpty, reason: 'no Routine yet');
      // Its page: the name, the pool and the count, then its badges.
      expect(key('note-name'), findsOneWidget);
      expect(key('session-pool'), findsOneWidget);
      expect(
        find.descendant(
          of: key('badge-kind'),
          matching: find.byIcon(Icons.event_repeat),
        ),
        findsOneWidget,
      );
      expect(key('badge-session-rule'), findsOneWidget);
      expect(key('badge-session-weekdays'), findsOneWidget);
      // The last Tally of the pool stays ticked.
      expect(
        tester.widget<CheckboxListTile>(key('session-pool-$pushUps')).onChanged,
        isNull,
      );

      await tap(tester, key('badge-kind'));
      await tap(tester, key('kind-plain'));
      expect(NoteKinds(store).of(id), NoteKind.plain);
      expect(key('session-pool'), findsNothing);
      expect(key('badge-session-rule'), findsNothing);
      expect(notes.byId(id)?.text, 'Sport\nMorning set');
    });

    testWidgets('the pool is ticked and ordered by its handles for Rolling, '
        'and the count changes with − / +, each written at once', (
      tester,
    ) async {
      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit', unit: 'min', target: 60);
      final bridge = tally('Half bridge', target: 15);
      final squats = tally('Squats');
      final made = session([pushUps, wallSit, bridge]);
      await start(tester);
      await edit(tester, made.noteId);
      for (final id in [pushUps, wallSit, bridge]) {
        expect(
          tester.widget<CheckboxListTile>(key('session-pool-$id')).value,
          isTrue,
        );
      }
      expect(
        tester.widget<CheckboxListTile>(key('session-pool-$squats')).value,
        isFalse,
      );

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
      final dragged = sessions.setupOf(made.noteId)!.pool;
      expect(dragged.first, wallSit);
      expect(dragged, unorderedEquals([pushUps, wallSit, bridge]));
      await tap(tester, key('session-pool-$bridge'));
      await tap(tester, key('session-pool-$squats'));
      expect(sessions.setupOf(made.noteId)?.pool, [
        for (final id in dragged)
          if (id != bridge) id,
        squats,
      ]);

      await tap(tester, step('session-count', plus: true));
      expect(sessions.setupOf(made.noteId)?.count, 3);
      await tap(tester, step('session-count'));
      await tap(tester, step('session-count'));
      expect(sessions.setupOf(made.noteId)?.count, 1);
      expect(enabled(tester, step('session-count')), isFalse);
      // Written on the Note, none on the Routine.
      expect(
        store.history(made.id).where((e) => e.field.startsWith('session:')),
        isEmpty,
      );
    });

    testWidgets('the rule and the weekdays\' counts are badges whose small '
        'editors write at once; only Rolling has an order to drag', (
      tester,
    ) async {
      final pushUps = tally('Push-ups');
      final made = session([pushUps, tally('Squats')], count: 99);
      sessions.setCountOn(made.noteId, DateTime.sunday, 3);
      await start(tester);
      await edit(tester, made.noteId);
      expect(key('session-move-$pushUps'), findsOneWidget);
      await reveal(tester, key('badge-session-weekdays'));
      expect(
        find.descendant(
          of: key('badge-session-weekdays'),
          matching: find.text('1 weekday with its own count'),
        ),
        findsOneWidget,
      );

      await tap(tester, key('badge-session-weekdays'));
      expect(find.byType(BottomSheet), findsNothing);
      expect(
        find.descendant(of: key('session-count-7'), matching: find.text('3')),
        findsOneWidget,
      );
      await tap(tester, key('session-own-1'));
      expect(
        find.descendant(of: key('session-count-1'), matching: find.text('99')),
        findsOneWidget,
      );
      expect(enabled(tester, step('session-count-1', plus: true)), isFalse);
      await tap(tester, key('session-own-7'));
      expect(key('session-count-7'), findsNothing);
      expect(sessions.setupOf(made.noteId)?.counts, {DateTime.monday: 99});
      await tap(tester, key('badge-close'));

      await tap(tester, key('badge-session-rule'));
      await tap(tester, key('session-rule-least'));
      expect(sessions.setupOf(made.noteId)?.rule, SessionRule.least);
      expect(
        find.descendant(
          of: key('badge-session-rule'),
          matching: find.byIcon(Icons.balance),
        ),
        findsOneWidget,
      );
      expect(key('session-move-$pushUps'), findsNothing);
    });

    testWidgets('"New Tally" in the pool comes back with that Tally ticked '
        'first', (tester) async {
      final made = session([tally('Push-ups')], count: 1);
      await start(tester);
      await edit(tester, made.noteId);
      await tap(tester, key('session-new-tally'));
      expect(find.byType(TallyFormScreen), findsOneWidget);
      await tester.enterText(key('tally-form-name'), 'Plank');
      await tester.enterText(key('tally-form-field-name-0'), 'Time');
      await tap(tester, key('tally-form-save'));

      expect(find.byType(NoteEditorScreen), findsOneWidget);
      final plank = notes.all().firstWhere((n) => n.text == 'Plank').id;
      expect(sessions.setupOf(made.noteId)?.pool.first, plank);
      expect(
        tester.widget<CheckboxListTile>(key('session-pool-$plank')).value,
        isTrue,
      );
    });

    testWidgets('the tags of the pool\'s Tallies filter it by any picked; '
        'ticks hidden stay and are counted, and New Tally carries the tags '
        '(#220)', (tester) async {
      final pushUps = tally('Push-ups #sport');
      final mop = tally('Mop #Cleaning');
      final dust = tally('Dust #cleaning #home');
      final made = session([pushUps], count: 1);
      await start(tester);
      await edit(tester, made.noteId);
      // Sorted by how many Tallies carry them, then by name.
      final chips = tester
          .widgetList<FilterChip>(find.byType(FilterChip))
          .map((c) => (c.label as Text).data)
          .toList();
      expect(chips, ['#cleaning', '#home', '#sport']);
      expect(key('session-pool-hidden'), findsNothing);
      expect(key('session-move-$mop'), findsOneWidget);

      await tap(tester, key('session-tag-#cleaning'));
      // Rolling's order is dragged only in the whole list.
      expect(key('session-move-$mop'), findsNothing);
      expect(key('session-pool-$pushUps'), findsNothing);
      expect(key('session-pool-$mop'), findsOneWidget);
      expect(key('session-pool-$dust'), findsOneWidget);
      expect(find.text('1 more ticked, hidden by the filter'), findsOneWidget);
      // A tick while filtered is written, and the filter stays.
      await tap(tester, key('session-pool-$mop'));
      expect(sessions.setupOf(made.noteId)?.pool, [pushUps, mop]);
      expect(key('session-pool-$pushUps'), findsNothing);
      // Any of the picked tags.
      await tap(tester, key('session-tag-#sport'));
      expect(key('session-pool-$pushUps'), findsOneWidget);
      expect(key('session-pool-$dust'), findsOneWidget);
      expect(key('session-pool-hidden'), findsNothing);

      await tap(tester, key('session-new-tally'));
      expect(
        tester.widget<TextField>(key('tally-form-name')).controller?.text,
        '#cleaning #sport ',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, key('session-tag-#cleaning'));
      await tap(tester, key('session-tag-#sport'));
      expect(key('session-pool-$pushUps'), findsOneWidget);
      expect(key('session-pool-$dust'), findsOneWidget);
      expect(sessions.setupOf(made.noteId)?.pool, [pushUps, mop]);
    });

    testWidgets('a tag picked filters for the Tallies tagged below it too, '
        'and no more (#267)', (tester) async {
      final run = tally('Run #sport/run');
      final swim = tally('Swim #Sport/Swim');
      final chess = tally('Chess #sportive');
      final plain = tally('Plank #sport');
      final made = session([plain], count: 1);
      await start(tester);
      await edit(tester, made.noteId);

      await tap(tester, key('session-tag-#sport'));
      expect(key('session-pool-$plain'), findsOneWidget);
      expect(key('session-pool-$run'), findsOneWidget);
      expect(key('session-pool-$swim'), findsOneWidget);
      expect(key('session-pool-$chess'), findsNothing);

      await tap(tester, key('session-tag-#sport'));
      await tap(tester, key('session-tag-#sport/run'));
      expect(key('session-pool-$run'), findsOneWidget);
      expect(key('session-pool-$swim'), findsNothing);
      expect(key('session-pool-$plain'), findsNothing);
    });

    testWidgets('the New Session page filters its pool by tag, and New Tally '
        'there carries the tags (#220)', (tester) async {
      final pushUps = tally('Push-ups #sport');
      final mop = tally('Mop #cleaning');
      final id = notes.capture('Morning').id;
      await start(tester);
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      await tap(tester, key('kind-session'));
      expect(find.byType(SessionFormScreen), findsOneWidget);
      await tap(tester, key('session-pool-$pushUps'));
      await tap(tester, key('session-tag-#cleaning'));
      expect(key('session-pool-$pushUps'), findsNothing);
      expect(find.text('1 more ticked, hidden by the filter'), findsOneWidget);
      await tap(tester, key('session-new-tally'));
      expect(
        tester.widget<TextField>(key('tally-form-name')).controller?.text,
        '#cleaning ',
      );
      await tester.enterText(key('tally-form-name'), '#cleaning Dust');
      await tester.enterText(key('tally-form-field-name-0'), 'Rooms');
      await tap(tester, key('tally-form-save'));
      final dust = notes.all().firstWhere((n) => n.text == '#cleaning Dust').id;
      // Back on the page, still filtered, the new Tally ticked in it.
      expect(key('session-pool-$pushUps'), findsNothing);
      expect(
        tester.widget<CheckboxListTile>(key('session-pool-$dust')).value,
        isTrue,
      );
      expect(key('session-pool-$mop'), findsOneWidget);
      await tap(tester, key('session-form-save'));
      expect(sessions.setupOf(id)?.pool, [dust, pushUps]);
    });

    testWidgets('"+" on a Session Note offers no Session; a Routine added '
        'there runs it, and a Routine\'s editor only schedules', (
      tester,
    ) async {
      final made = session([tally('Push-ups')], count: 1);
      await start(tester);
      await edit(tester, made.noteId);
      await tap(tester, key('badge-add'));
      expect(key('spawn-routine'), findsOneWidget);
      expect(key('spawn-session'), findsNothing);
      await tap(tester, key('spawn-routine'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(key('session-pool'), findsNothing);
      await tap(tester, find.text('Save'));

      expect(sessions.ofNote(made.noteId), hasLength(2));
      await tap(tester, key('badge-routine-${made.id}'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(key('session-pool'), findsNothing);
      expect(key('session-rule'), findsNothing);
      expect(find.text('Routine'), findsWidgets);
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

    testWidgets('with checked items hidden, a pick with its entry leaves and '
        'the heading goes once each has one (#217)', (tester) async {
      store.agendaHidesChecked = true;
      final pushUps = tally('Push-ups');
      final squats = tally('Squats');
      final made = session([pushUps, squats]);
      await start(tester);
      expect(key('session-pick-$pushUps'), findsOneWidget);

      await tap(tester, key('session-pick-$pushUps'));
      await tap(tester, key('tally-entry-save'));
      expect(sessions.on(made.id, today).picks.first.done, isTrue);
      expect(key('session-pick-$pushUps'), findsNothing);
      expect(key('session-another-$pushUps'), findsNothing);
      expect(key('session-pick-$squats'), findsOneWidget);
      expect(key('row-${made.id}@${today.key}'), findsOneWidget);

      await tap(tester, key('session-pick-$squats'));
      await tap(tester, key('tally-entry-save'));
      expect(sessions.on(made.id, today).complete, isTrue);
      expect(key('row-${made.id}@${today.key}'), findsNothing);
      expect(key('session-pick-$squats'), findsNothing);
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
      // Deleted without asking; the announcement at the top undoes it
      // (#268).
      expect(find.byType(AlertDialog), findsNothing);

      expect(tallies.entryById(entry.id)?.isTrashed, isTrue);
      final box = tester.widget<Checkbox>(find.byType(Checkbox).first);
      expect(box.value, isFalse);
      expect(box.onChanged, isNull, reason: 'it only shows Done (#208)');

      await tap(tester, find.text('Entry deleted'));
      expect(tallies.entryById(entry.id)?.isTrashed, isFalse);
      expect(tester.widget<Checkbox>(find.byType(Checkbox).first).value, true);
    });

    testWidgets('the heading\'s box only shows Done and a tap on it does '
        'nothing; each pick has a box of its own that opens its entry form '
        '(#208)', (tester) async {
      final pushUps = tally('Push-ups');
      final made = session([pushUps, tally('Squats')]);
      await start(tester);

      final heading = find.byType(Checkbox).first;
      expect(tester.widget<Checkbox>(heading).onChanged, isNull);
      await tap(tester, heading);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(
        Routines(store).agenda(deviceWallClock).todayRows.single.done,
        isNull,
      );

      final pick = key('session-check-$pushUps');
      expect(
        find.descendant(of: key('session-pick-$pushUps'), matching: pick),
        findsOneWidget,
      );
      expect(tester.widget<Checkbox>(pick).value, isFalse);
      expect(tester.getSize(pick).height, greaterThanOrEqualTo(40));
      // Indented under the heading's words.
      expect(
        tester.getTopLeft(pick).dx,
        greaterThanOrEqualTo(tester.getTopLeft(find.text('Sport').first).dx),
      );
      await tap(tester, pick);
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      await tap(tester, key('tally-entry-save'));
      expect(sessions.on(made.id, today).picks.first.done, isTrue);
      expect(tester.widget<Checkbox>(pick).value, isTrue);
    });

    testWidgets('tomorrow and later days show the title and how many it '
        'picks, with no picks yet', (tester) async {
      final pushUps = tally('Push-ups');
      final made = session([pushUps, tally('Squats'), tally('Plank')]);
      sessions.setCountOn(made.noteId, DateTime.sunday, 1);
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

    testWidgets('two Routines of one Session Note each show their '
        'Occurrence with its own picks, Rolling going on from the earlier', (
      tester,
    ) async {
      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit', unit: 'min', target: 60);
      final made = session([pushUps, wallSit, tally('Half bridge')], count: 1);
      final evening = Routines(store).spawn(
        made.noteId,
        Schedule(
          start: Day(2026, 10, 24),
          times: [ClockTime(18, 0)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      await start(tester);

      expect(find.text('Push-ups 20 times'), findsOneWidget);
      expect(find.text('Wall sit 1:00 min'), findsOneWidget);
      final later = Occurrence(Day(2026, 10, 24), ClockTime(18, 0));
      expect(store.currentFields(evening.id)[SessionKeys.pick(later)], wallSit);
      expect(picked(made), [pushUps]);
    });
  });
}

/// The Note the Session [session] was spawned from.
Note? routinesNote(Store store, Session session) =>
    Routines(store).note(session.routine);

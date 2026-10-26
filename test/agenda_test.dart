import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);
  final tomorrow = Day(2026, 10, 25);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    routines = Routines(store);
  });
  tearDown(() => store.close());

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// The Agenda's rows from top to bottom as "time text", with [x] when
  /// Done.
  List<String> agendaRows(WidgetTester tester) => [
    for (final box in tester.widgetList<Checkbox>(find.byType(Checkbox)))
      () {
        final row = find.ancestor(
          of: find.byWidget(box),
          matching: find.byType(Row),
        );
        final texts = tester
            .widgetList<Text>(
              find.descendant(of: row.first, matching: find.byType(Text)),
            )
            .map((t) => t.data)
            .join(' ');
        return '${box.value == true ? '[x] ' : ''}$texts';
      }(),
  ];

  /// Whether [text] is drawn faded, as tomorrow is.
  bool dimmed(WidgetTester tester, String text) => tester
      .widgetList<Opacity>(
        find.ancestor(of: find.text(text), matching: find.byType(Opacity)),
      )
      .any((o) => o.opacity == AgendaScreen.tomorrowOpacity);

  group('the Agenda', () {
    testWidgets('is the home screen, with "+" and the magnifier floating', (
      tester,
    ) async {
      await start(tester);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(FloatingActionButton), findsNWidgets(2));
      expect(find.byTooltip('Capture a Note'), findsOneWidget);
      expect(find.byTooltip('Search and browse'), findsOneWidget);
      expect(
        find.text('Nothing due today. Tap + to capture a thought.'),
        findsOneWidget,
      );
      expect(find.text('Nothing due tomorrow.'), findsOneWidget);
      final scaffold = tester.widget<Scaffold>(find.byType(Scaffold));
      expect(scaffold.bottomNavigationBar, isNull);
      expect(scaffold.bottomSheet, isNull);
    });

    for (final choice in [ThemeChoice.light, ThemeChoice.dark]) {
      testWidgets('shows the brain before "Old Brain Fix" in the top bar, '
          'read only as the name (${choice.name})', (tester) async {
        store.themeChoice = choice;
        final semantics = tester.ensureSemantics();
        await start(tester);

        final bar = find.byType(AppBar);
        expect(
          Theme.of(tester.element(bar)).brightness,
          choice == ThemeChoice.dark ? Brightness.dark : Brightness.light,
        );
        final brain = find.descendant(
          of: bar,
          matching: find.byKey(const ValueKey('brain')),
        );
        final title = find.descendant(
          of: bar,
          matching: find.text('Old Brain Fix'),
        );
        expect(brain, findsOneWidget);
        expect(title, findsOneWidget);
        final image = tester.widget<Image>(brain);
        expect(image.excludeFromSemantics, isTrue);
        expect((image.image as AssetImage).assetName, 'assets/brain.png');
        final brainBox = tester.getRect(brain);
        final titleBox = tester.getRect(title);
        expect(brainBox.right, lessThanOrEqualTo(titleBox.left));
        expect(brainBox.height, 28);
        expect(brainBox.center.dy, closeTo(titleBox.center.dy, 2));
        expect(
          tester.getSemantics(find.text('Old Brain Fix')),
          isSemantics(label: 'Old Brain Fix', isHeader: true, namesRoute: true),
        );
        expect(find.bySemanticsLabel('Old Brain Fix'), findsOneWidget);
        semantics.dispose();
      });
    }

    testWidgets('today all-day first, then by time; a gap; tomorrow dimmed', (
      tester,
    ) async {
      routines.create('pills', daily([at(20), at(8)]));
      routines.create('tidy up', daily([]));
      routines.create('dentist', Schedule(start: tomorrow, times: [at(10)]));
      routines.create('call Mum', Schedule(start: today, times: [at(12)]));
      await start(tester);

      expect(agendaRows(tester).take(8), [
        'All day tidy up',
        '08:00 pills',
        '12:00 call Mum',
        '20:00 pills',
        'All day tidy up',
        '08:00 pills',
        '10:00 dentist',
        '20:00 pills',
      ]);
      expect(find.text('Today'), findsOneWidget);
      expect(dimmed(tester, 'Tomorrow'), isTrue);
      expect(dimmed(tester, 'dentist'), isTrue);
      expect(dimmed(tester, 'call Mum'), isFalse);
      final todayEnd = tester.getRect(find.text('20:00').first).bottom;
      final tomorrowStart = tester.getRect(find.text('Tomorrow')).top;
      expect(tomorrowStart - todayEnd, greaterThan(32));
    });

    /// Starts on a page tall enough for the whole week to be built.
    Future<void> startTall(WidgetTester tester) async {
      tester.view.physicalSize = const Size(800, 4000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await start(tester);
    }

    /// The heading texts from top to bottom.
    List<String> headings(WidgetTester tester, List<String> texts) {
      final found = [
        for (final text in texts)
          if (find.text(text).evaluate().isNotEmpty) text,
      ];
      found.sort(
        (a, b) => tester
            .getTopLeft(find.text(a))
            .dy
            .compareTo(tester.getTopLeft(find.text(b)).dy),
      );
      return found;
    }

    testWidgets('a week: days three to seven under weekday and date, dimmed, '
        'across a month end', (tester) async {
      // 12:00 on Wednesday, 28 October 2026.
      now = DateTime(2026, 10, 28, 12);
      routines.create('dentist', Schedule(start: Day(2026, 11, 2)));
      routines.create(
        'bins',
        Schedule(start: Day(2026, 10, 30), times: [at(7)]),
      );
      await startTall(tester);

      const week = [
        'Today',
        'Tomorrow',
        'Friday, 30 Oct',
        'Saturday, 31 Oct',
        'Sunday, 1 Nov',
        'Monday, 2 Nov',
        'Tuesday, 3 Nov',
      ];
      expect(headings(tester, week), week);
      for (final day in week.skip(1)) {
        expect(dimmed(tester, day), isTrue, reason: day);
      }
      expect(dimmed(tester, 'Today'), isFalse);
      expect(agendaRows(tester), ['07:00 bins', 'All day dentist']);
      expect(dimmed(tester, 'bins'), isTrue);
      expect(dimmed(tester, 'dentist'), isTrue);
      // Each empty day after tomorrow says so, dimmed.
      expect(find.text('Nothing planned'), findsNWidgets(3));
      expect(dimmed(tester, 'Nothing planned'), isTrue);
      final bins = tester.getTopLeft(find.text('bins')).dy;
      expect(bins, greaterThan(tester.getTopLeft(find.text(week[2])).dy));
      expect(bins, lessThan(tester.getTopLeft(find.text(week[3])).dy));
      final dentist = tester.getTopLeft(find.text('dentist')).dy;
      expect(dentist, greaterThan(tester.getTopLeft(find.text(week[5])).dy));
      expect(dentist, lessThan(tester.getTopLeft(find.text(week[6])).dy));
    });

    testWidgets('a day after tomorrow is checked off today', (tester) async {
      final dentist = routines.create(
        'dentist',
        Schedule(start: Day(2026, 10, 27)),
      );
      await startTall(tester);
      expect(find.text('Tuesday, 27 Oct'), findsOneWidget);
      await tap(tester, find.byType(Checkbox));
      expect(agendaRows(tester), ['[x] All day dentist']);
      expect(
        routines.byId(dentist.id)?.doneAt(Occurrence(Day(2026, 10, 27))),
        now.toUtc(),
      );
    });

    testWidgets('the week in German', (tester) async {
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      now = DateTime(2026, 10, 28, 12);
      await startTall(tester);
      const week = [
        'Heute',
        'Morgen',
        'Freitag, 30. Okt.',
        'Samstag, 31. Okt.',
        'Sonntag, 1. Nov.',
        'Montag, 2. Nov.',
        'Dienstag, 3. Nov.',
      ];
      expect(headings(tester, week), week);
      expect(find.text('Nichts geplant'), findsNWidgets(5));
    });

    testWidgets('each time of day is checked off on its own', (tester) async {
      final pills = routines.create('pills', daily([at(8), at(14)]));
      await start(tester);
      await tap(tester, find.byType(Checkbox).first);
      expect(agendaRows(tester).take(2), ['[x] 08:00 pills', '14:00 pills']);
      expect(
        routines.byId(pills.id)?.doneAt(Occurrence(today, at(8))),
        isNotNull,
      );
      expect(
        routines.byId(pills.id)?.doneAt(Occurrence(today, at(14))),
        isNull,
      );
      // Unchecking takes it back.
      await tap(tester, find.byType(Checkbox).first);
      expect(agendaRows(tester).first, '08:00 pills');
    });

    testWidgets('moves on to the new day at midnight while open', (
      tester,
    ) async {
      now = DateTime(2026, 10, 24, 23, 59);
      routines.create('dentist', Schedule(start: tomorrow, times: [at(10)]));
      await start(tester);
      expect(dimmed(tester, 'dentist'), isTrue);

      now = DateTime(2026, 10, 25, 0, 0, 30);
      await tester.pump(const Duration(minutes: 1, seconds: 1));
      expect(dimmed(tester, 'dentist'), isFalse);
      expect(find.text('Nothing due tomorrow.'), findsOneWidget);
    });

    testWidgets('tomorrow is checked off today, and stays where it is', (
      tester,
    ) async {
      final report = routines.create(
        'weekly report',
        Schedule(start: tomorrow, rule: Recurrence.parse('FREQ=WEEKLY')),
      );
      await start(tester);
      await tap(tester, find.byType(Checkbox));
      expect(agendaRows(tester), ['[x] All day weekly report']);
      expect(
        routines.byId(report.id)?.doneAt(Occurrence(tomorrow)),
        now.toUtc(),
      );
      expect(routines.byId(report.id)?.schedule.next(tomorrow, 2), [
        Occurrence(tomorrow),
        Occurrence(Day(2026, 11, 1)),
      ]);
    });
  });

  group('spawning', () {
    testWidgets('a captured Note\'s card offers a Reminder; it leaves '
        'Unsorted and shows on the Agenda', (tester) async {
      await start(tester);
      await tap(tester, find.byTooltip('Capture a Note'));
      await tester.enterText(find.byType(TextField), 'call the plumber');
      await tap(tester, find.text('Save'));

      expect(find.text('Just captured'), findsOneWidget);
      expect(find.byType(NoteCard), findsOneWidget);
      await tap(tester, find.byTooltip('Routine or Reminder'));
      expect(find.byType(BottomSheet), findsNothing);
      await tap(tester, find.text('Reminder'));

      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('New Reminder'), findsOneWidget);
      expect(
        find.text('call the plumber'),
        findsOneWidget,
        reason: 'back-link',
      );
      // Once, at the next full hour.
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.isUnsorted, isFalse);
      final reminder = routines.ofNote(note.id).single;
      expect(reminder.isReminder, isTrue);
      expect(reminder.schedule.on(today), [Occurrence(today, at(10))]);
      expect(agendaRows(tester), ['10:00 call the plumber']);
      expect(find.text('Unsorted'), findsNothing, reason: 'nor on its card');
      await tap(tester, find.text('call the plumber').first);
      expect(find.text('Not filed'), findsOneWidget);
    });

    testWidgets('the captured card stays through a dialog, not a return', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, find.byTooltip('Capture a Note'));
      await tester.enterText(find.byType(TextField), 'milk');
      await tap(tester, find.text('Save'));
      final binding = tester.binding;
      // A system dialog or the notification shade over the app.
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pumpAndSettle();
      expect(find.text('Just captured'), findsOneWidget);
      // The app left for the background and came back.
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
      expect(find.text('Just captured'), findsNothing);
      expect(find.byType(NoteCard), findsNothing);
    });

    testWidgets('a Note spawns several from its editor, each linking back', (
      tester,
    ) async {
      final dentist = notes.capture('dentist');
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
      await tap(tester, find.text('dentist'));

      await tap(tester, find.text('Reminder'));
      await tap(tester, find.text('Save'));
      await reveal(tester, find.text('Routine'));
      await tap(tester, find.text('Routine'));
      expect(find.text('New Routine'), findsOneWidget);
      await tap(tester, find.text('Save'));

      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(find.text('Every day · 08:00'), findsOneWidget);
      expect(find.textContaining('Once, '), findsOneWidget);
      final spawned = routines.ofNote(dentist.id);
      expect(spawned, hasLength(2));
      expect(spawned.where((r) => r.isReminder), hasLength(1));

      await tap(tester, find.text('Every day · 08:00'));
      expect(find.text('From the Note'), findsOneWidget);
      await tap(tester, find.text('dentist'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('a browser card offers a Routine from a menu at the finger', (
      tester,
    ) async {
      notes.capture('water the plants');
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
      await tap(tester, find.byTooltip('Routine or Reminder'));
      expect(find.byType(BottomSheet), findsNothing);
      await tap(tester, find.text('Routine').last);
      await tap(tester, find.text('Save'));
      expect(routines.all().single.isReminder, isFalse);
      expect(notes.all().single.isUnsorted, isFalse);
    });

    testWidgets('a Routine made directly makes its Note, not Unsorted', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, find.byTooltip('New Routine or Reminder'));
      expect(
        tester
            .widget<TextButton>(
              find.ancestor(
                of: find.text('Save'),
                matching: find.byType(TextButton),
              ),
            )
            .onPressed,
        isNull,
        reason: 'nothing typed yet',
      );
      await tester.enterText(find.byType(TextField), 'pills');
      await tester.pumpAndSettle();
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.text, 'pills');
      expect(note.isUnsorted, isFalse);
      expect(routines.note(routines.all().single)?.id, note.id);
      expect(agendaRows(tester).take(2), ['08:00 pills', '08:00 pills']);
    });
  });

  group('schedule editor', () {
    Future<void> openNew(WidgetTester tester) async {
      await start(tester);
      await tap(tester, find.byTooltip('New Routine or Reminder'));
      await tester.enterText(find.byType(TextField), 'pills');
      await tester.pumpAndSettle();
    }

    testWidgets('every N weeks, and several times a day', (tester) async {
      await openNew(tester);
      await reveal(tester, find.byTooltip('More'));
      await tap(tester, find.byTooltip('More'));
      await tap(tester, find.text('days'));
      await tap(tester, find.text('weeks').last);
      // A second time of day: the picker offers 08:00, after 08:00 went.
      await reveal(tester, find.byTooltip('Remove 08:00'));
      await tap(tester, find.byTooltip('Remove 08:00'));
      expect(find.text('All day'), findsOneWidget);
      await reveal(tester, find.text('Add a time'));
      await tap(tester, find.text('Add a time'));
      await tap(tester, find.text('OK'));
      await tap(tester, find.text('Save'));

      final schedule = routines.all().single.schedule;
      expect(schedule.rule?.format(), 'FREQ=WEEKLY;INTERVAL=2');
      expect(schedule.times, [at(8)]);
      expect(schedule.start, today);
    });

    testWidgets('its choices are radio buttons, one per line', (tester) async {
      await openNew(tester);
      expect(find.byType(SegmentedButton<bool>), findsNothing);
      expect(find.byType(SegmentedButton<RepeatKind>), findsNothing);
      expect(find.byType(RadioListTile<bool>), findsNWidgets(2));
      expect(find.byType(RadioListTile<RepeatKind>), findsNWidgets(5));
      // One option per line: each sits below the one before.
      final tops = [
        for (final tile in find.byType(RadioListTile<RepeatKind>).evaluate())
          tester.getTopLeft(find.byWidget(tile.widget)),
      ];
      expect(tops.map((o) => o.dx).toSet(), hasLength(1));
      expect(tops[1].dy, greaterThan(tops[0].dy));
      expect(tops[2].dy, greaterThan(tops[1].dy));
      expect(tops[4].dy, greaterThan(tops[3].dy));

      await tap(
        tester,
        find.descendant(
          of: find.widgetWithText(RadioListTile<RepeatKind>, 'Weekdays'),
          matching: find.byType(Radio<RepeatKind>),
        ),
      );
      await tester.ensureVisible(find.text('Save'));
      await tap(tester, find.text('Save'));
      expect(
        routines.all().single.schedule.rule?.format(),
        'FREQ=WEEKLY;BYDAY=SA',
      );
    });

    testWidgets('chosen weekdays', (tester) async {
      await openNew(tester);
      await tap(tester, find.text('Weekdays'));
      // Saturday, the start's weekday, comes ticked; add Monday.
      await tester.ensureVisible(find.text('Mon'));
      await tap(tester, find.text('Mon'));
      await tap(tester, find.text('Save'));
      expect(
        routines.all().single.schedule.rule?.format(),
        'FREQ=WEEKLY;BYDAY=MO,SA',
      );
    });

    testWidgets('the advanced mode takes an RRULE and shows what comes', (
      tester,
    ) async {
      await openNew(tester);
      await tap(tester, find.text('Advanced'));
      expect(find.text('FREQ=DAILY'), findsOneWidget);
      await tester.enterText(
        find.byType(TextField).last,
        'FREQ=DAILY;BYSETPOS=1',
      );
      await tester.pumpAndSettle();
      expect(
        find.text('Not a rule this app understands: BYSETPOS'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<TextButton>(
              find.ancestor(
                of: find.text('Save'),
                matching: find.byType(TextButton),
              ),
            )
            .onPressed,
        isNull,
      );
      await tester.enterText(
        find.byType(TextField).last,
        'FREQ=MONTHLY;BYDAY=1MO,-1FR',
      );
      await tester.pumpAndSettle();
      expect(find.text('Mon, 11/2/2026  08:00'), findsOneWidget);
      await tap(tester, find.text('Save'));

      final routine = routines.all().single;
      expect(routine.schedule.rule?.format(), 'FREQ=MONTHLY;BYDAY=1MO,-1FR');
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
      await tap(tester, find.text('pills'));
      await tap(tester, find.text('Rule: FREQ=MONTHLY;BYDAY=1MO,-1FR · 08:00'));
      // A rule the simple editor cannot show opens in the advanced mode.
      expect(find.text('FREQ=MONTHLY;BYDAY=1MO,-1FR'), findsOneWidget);
    });

    testWidgets('monthly on the last Friday, and it reopens as built', (
      tester,
    ) async {
      await openNew(tester);
      await tap(tester, find.text('Monthly on a weekday'));
      // Saturday the 24th is the month's fourth Saturday.
      await reveal(tester, find.text('Last'));
      await tap(tester, find.text('Last'));
      await reveal(tester, find.text('Friday'));
      await tap(tester, find.text('Friday'));
      await tester.ensureVisible(find.text('Save'));
      await tap(tester, find.text('Save'));
      expect(
        routines.all().single.schedule.rule?.format(),
        'FREQ=MONTHLY;BYDAY=-1FR',
      );

      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
      await tap(tester, find.text('pills'));
      await tap(tester, find.text('Every month on the last Friday · 08:00'));
      // In the simple editor, as it was built.
      expect(find.text('FREQ=MONTHLY;BYDAY=-1FR'), findsNothing);
      // Whether the option [text] is the one its radio group holds.
      bool picked(String text) {
        final tile = find.ancestor(
          of: find.text(text),
          matching: find.byWidgetPredicate((w) => w is RadioListTile),
        );
        final group = find.ancestor(
          of: tile,
          matching: find.byWidgetPredicate((w) => w is RadioGroup),
        );
        return (tester.widget(group.first) as RadioGroup).groupValue ==
            (tester.widget(tile) as RadioListTile).value;
      }

      expect(picked('Monthly on a weekday'), isTrue);
      await reveal(tester, find.text('Friday'));
      expect(picked('Last'), isTrue);
      expect(picked('Friday'), isTrue);
    });

    testWidgets('an end after a number of times', (tester) async {
      await openNew(tester);
      await reveal(tester, find.text('After a number of times'));
      await tap(tester, find.text('After a number of times'));
      await reveal(tester, find.text('How many times'));
      await tap(tester, find.byTooltip('More').last);
      expect(find.text('11'), findsOneWidget);
      await tester.ensureVisible(find.text('Save'));
      await tap(tester, find.text('Save'));
      expect(
        routines.all().single.schedule.rule?.format(),
        'FREQ=DAILY;COUNT=11',
      );
    });

    testWidgets('a Routine is trashed after a confirm; its Note stays', (
      tester,
    ) async {
      final routine = routines.create('pills', daily([at(8)]));
      await start(tester);
      await tap(tester, find.text('pills').first);
      await tap(tester, find.byTooltip('Delete'));
      expect(
        find.text(
          'Delete this Routine? It stays in the trash for 30 days. '
          'Its Note stays.',
        ),
        findsOneWidget,
      );
      await tap(tester, find.text('Delete').last);
      expect(routines.byId(routine.id)?.isTrashed, isTrue);
      expect(notes.all().single.isUnsorted, isFalse);
      expect(find.byType(Checkbox), findsNothing);
    });

    testWidgets('editor and Agenda fit a phone in German', (tester) async {
      tester.view.physicalSize = const Size(1080, 2340);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      routines.create('Tabletten', daily([at(8), at(20)]));
      await start(tester);
      expect(find.text('Morgen'), findsOneWidget);
      await tap(tester, find.byTooltip('Neue Routine oder Erinnerung'));
      await tap(tester, find.text('Alle …'));
      await tap(tester, find.text('Wochentage'));
      await tap(tester, find.text('Monatlich an einem Wochentag'));
      await reveal(tester, find.text('Nach einer bestimmten Anzahl'));
      await tap(tester, find.text('Nach einer bestimmten Anzahl'));
      expect(tester.takeException(), isNull);
      // Back to the top, where the mode is chosen.
      await tester.drag(find.byType(Scrollable).first, const Offset(0, 3000));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Erweitert'));
      expect(find.text('Wiederholregel (RRULE)'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });
}

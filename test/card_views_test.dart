import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/tally_entry_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Every kind has a compact and a detailed view (#189): the list's switch
/// sets the view of all cards, a tap switches one card, remembered on this
/// Device until the switch changes again; a compact card has no buttons at
/// the bottom, its ⋮ top right, and no Capture Context.
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> openAllNotes(WidgetTester tester) async {
    await tap(tester, find.byKey(const ValueKey('browse')));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// All Notes, its cards compact.
  Future<void> openCompact(WidgetTester tester) async {
    store.setLocalSetting('compactNotes:allNotes', 'true');
    await start(tester);
    await openAllNotes(tester);
  }

  /// The card of [text]'s Note.
  Finder cardOf(String text) => find.ancestor(
    of: find.textContaining(text, findRichText: true),
    matching: find.byType(InkWell),
  );

  /// Whether the card holding [text] is compact: no action row in it.
  bool compactCard(String text) => find
      .descendant(of: cardOf(text).first, matching: key('note-actions'))
      .evaluate()
      .isEmpty;

  group('a compact card', () {
    testWidgets('of a plain Note shows its first line and the two lines '
        'after it, ⋮ top right, no buttons at the bottom and no Capture '
        'Context', (tester) async {
      final note = notes.capture(
        'Call the plumber\nabout the tap\nand the '
        'boiler\nnot the roof',
      );
      await openCompact(tester);

      expect(find.text('Call the plumber', findRichText: true), findsOneWidget);
      expect(
        find.text('about the tap\nand the boiler', findRichText: true),
        findsOneWidget,
      );
      expect(find.textContaining('not the roof'), findsNothing);
      expect(key('note-actions'), findsNothing);
      expect(key('captured-line:${note.id}'), findsNothing);
      // Neither the time nor the place line over the text.
      expect(find.textContaining('10/24/2026'), findsNothing);
      final menu = tester.getTopRight(key('note-menu'));
      final line = tester.getTopRight(
        find.text('Call the plumber', findRichText: true),
      );
      expect(menu.dx, greaterThan(line.dx));
      expect(menu.dy, lessThanOrEqualTo(line.dy));

      await tap(tester, key('note-menu'));
      expect(key('note-edit'), findsOneWidget);
    });

    testWidgets('of a Checklist shows its name and how many Lines are open', (
      tester,
    ) async {
      final id = notes.capture('Before the trip').id;
      final lists = Checklists(store)..make(id);
      final first = lists.add(id, 'Passport');
      lists
        ..add(id, 'Tickets')
        ..add(id, 'Charger')
        ..check(first);
      // A description, written once the Note is a list.
      notes.edit(id, 'Before the trip\nPack early');
      await openCompact(tester);

      expect(find.text('Before the trip', findRichText: true), findsOneWidget);
      expect(find.text('2 of 3 open'), findsOneWidget);
      expect(find.textContaining('Pack early'), findsNothing);
      expect(find.text('Passport', findRichText: true), findsNothing);
      expect(key('note-actions'), findsNothing);
    });

    testWidgets('of a Shopping List shows its name and how many Lines are '
        'open', (tester) async {
      final id = notes.capture('Groceries').id;
      final lists = ShoppingLists(store)..make(id);
      final milk = lists.add(id, 'Milk', deviceWallClock);
      lists
        ..add(id, 'Bread', deviceWallClock)
        ..check(milk, deviceWallClock);
      await openCompact(tester);

      expect(find.text('Groceries', findRichText: true), findsOneWidget);
      expect(find.text('1 open'), findsOneWidget);
      expect(find.text('Bread', findRichText: true), findsNothing);
    });

    testWidgets('of a Tally shows its name and each field\'s total today '
        'against its target', (tester) async {
      final id = notes.capture('Reading').id;
      final tallies = Tallies(store);
      final pages = tallies.addField(
        id,
        name: 'Pages',
        unit: 'pages',
        target: 50,
      );
      final minutes = tallies.addField(id, name: 'Books', unit: 'books');
      tallies
        ..add(id, values: {pages: 20, minutes: 1})
        ..add(id, values: {pages: 10})
        // Yesterday's counts for yesterday.
        ..add(
          id,
          values: {pages: 99},
          at: now.subtract(const Duration(days: 1)),
        );
      await openCompact(tester);

      expect(find.text('Reading', findRichText: true), findsOneWidget);
      expect(find.text('Pages 30 / 50 pages'), findsOneWidget);
      expect(find.text('Books 1 books'), findsOneWidget);
    });

    testWidgets('of a Note that runs a Session shows its next run and this '
        'week\'s done and skipped', (tester) async {
      final pool = notes.capture('Push-ups').id;
      Tallies(store).addField(pool, name: 'Done', unit: 'times', target: 20);
      Sessions(store).spawn(
        notes.capture('Sport\nmorning moves').id,
        Schedule(
          start: Day(2026, 10, 25),
          times: [ClockTime(7, 0)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
        pool: [pool],
        count: 1,
        rule: SessionRule.rolling,
      );
      await openCompact(tester);

      expect(find.text('Sport', findRichText: true), findsOneWidget);
      expect(find.text('Next: Sun, 10/25/2026 07:00'), findsOneWidget);
      expect(find.text('done 0 / skipped 0 this week'), findsOneWidget);
      expect(find.textContaining('morning moves'), findsNothing);
    });
    testWidgets('of a Session Note shows the next run of any of its Routines '
        'and this week\'s done and skipped over all of them', (tester) async {
      final pool = notes.capture('Push-ups').id;
      Tallies(store).addField(pool, name: 'Done', unit: 'times', target: 20);
      final made = Sessions(store).spawn(
        notes.capture('Sport\nmorning moves').id,
        Schedule(
          start: Day(2026, 10, 26),
          times: [ClockTime(7, 0)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
        pool: [pool],
        count: 1,
        rule: SessionRule.rolling,
      );
      // A second Routine whose run comes first.
      Routines(store).spawn(
        made.noteId,
        Schedule(
          start: Day(2026, 10, 25),
          times: [ClockTime(18, 0)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      await openCompact(tester);

      expect(find.text('Next: Sun, 10/25/2026 18:00'), findsOneWidget);
      expect(find.text('done 0 / skipped 0 this week'), findsOneWidget);
    });

    testWidgets('of a Session Note without a Routine shows how many it picks', (
      tester,
    ) async {
      final pool = notes.capture('Push-ups').id;
      Tallies(store).addField(pool, name: 'Done', unit: 'times', target: 20);
      Sessions(store).make(
        notes.capture('Sport').id,
        pool: [pool],
        count: 1,
        rule: SessionRule.random,
      );
      await openCompact(tester);

      expect(find.text('Sport', findRichText: true), findsOneWidget);
      expect(find.text('Picks 1 Tally'), findsOneWidget);
      expect(find.textContaining('this week'), findsNothing);
    });

    testWidgets('of a Session an older version left on its Routine reads as '
        'one', (tester) async {
      final pool = notes.capture('Push-ups').id;
      Tallies(store).addField(pool, name: 'Done', unit: 'times', target: 20);
      final note = notes.capture('Stretch').id;
      Routines(store).spawn(
        note,
        Schedule(
          start: Day(2026, 10, 25),
          times: [ClockTime(7, 0)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
        within: (id) {
          store.append(id, SessionKeys.pool, pool);
          store.append(id, SessionKeys.count, '1');
          store.append(id, SessionKeys.rule, 'rolling');
        },
      );
      await openCompact(tester);

      expect(find.text('Next: Sun, 10/25/2026 07:00'), findsOneWidget);
      expect(find.text('done 0 / skipped 0 this week'), findsOneWidget);

      await tap(tester, find.text('Stretch', findRichText: true));
      expect(key('card-session'), findsOneWidget);
      expect(
        find.descendant(
          of: key('card-session'),
          matching: find.text('Push-ups'),
        ),
        findsOneWidget,
      );
    });
  });

  group('the ⋮ of a compact card', () {
    testWidgets('offers File into… and Merge into…, which open their pages', (
      tester,
    ) async {
      notes.capture('Call the plumber');
      notes.capture('Buy milk');
      await openCompact(tester);

      final menu = find.descendant(
        of: cardOf('Call the plumber'),
        matching: key('note-menu'),
      );
      await tap(tester, menu);
      expect(find.text('File into…'), findsOneWidget);
      expect(find.text('Merge into…'), findsOneWidget);
      await tap(tester, key('note-file'));
      expect(find.byType(FileIntoScreen), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();

      await tap(tester, menu);
      await tap(tester, key('note-merge'));
      expect(find.byType(MergeIntoScreen), findsOneWidget);
    });

    testWidgets('of a detailed card leaves them to the action row', (
      tester,
    ) async {
      notes.capture('Call the plumber');
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, key('note-menu'));
      expect(key('note-file'), findsNothing);
      expect(key('note-merge'), findsNothing);
      expect(find.byTooltip('File into…'), findsOneWidget);
      expect(find.byTooltip('Merge into…'), findsOneWidget);
    });
  });

  group('a fold on a card is kept on this Device', () {
    testWidgets('the struck Lines of a Shopping List', (tester) async {
      final id = notes.capture('Groceries').id;
      final lists = ShoppingLists(store)..make(id);
      final milk = lists.add(id, 'Milk', deviceWallClock);
      lists.check(milk, deviceWallClock);
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, key('shopping-struck'));
      expect(store.localSetting('shoppingStruckShown:$id'), 'true');
      // Shown again, it is still open.
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('All Notes'));
      expect(find.text('Milk', findRichText: true), findsOneWidget);
      await tap(tester, key('shopping-struck'));
      expect(store.localSetting('shoppingStruckShown:$id'), isNull);
      expect(find.text('Milk', findRichText: true), findsNothing);
    });

    testWidgets('the description of a Note of a kind', (tester) async {
      final id = notes.capture('Before the trip').id;
      Checklists(store).make(id);
      notes.edit(id, 'Before the trip\nPack early\nand light');
      await start(tester);
      await openAllNotes(tester);
      expect(find.textContaining('and light'), findsNothing);

      await tap(tester, key('card-description'));
      expect(store.localSetting('descriptionUnfolded:$id'), 'true');
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('All Notes'));
      expect(
        find.textContaining('and light', findRichText: true),
        findsOneWidget,
      );
      await tap(tester, key('card-description'));
      expect(store.localSetting('descriptionUnfolded:$id'), isNull);
    });
  });

  group('a detailed card', () {
    testWidgets('of a plain Note shows its whole text, its Capture Context '
        'and the buttons at the bottom', (tester) async {
      final note = notes.capture(
        'Call the plumber\nabout the tap\nand the '
        'boiler\nnot the roof',
      );
      await start(tester);
      await openAllNotes(tester);

      expect(find.text(note.text, findRichText: true), findsOneWidget);
      expect(key('captured-line:${note.id}'), findsOneWidget);
      expect(key('note-actions'), findsOneWidget);
    });

    testWidgets('of a Shopping List shows its open Lines, tickable, the '
        'struck ones folded until a tap', (tester) async {
      final id = notes.capture('Groceries').id;
      final lists = ShoppingLists(store)..make(id);
      final milk = lists.add(id, 'Milk', deviceWallClock);
      lists
        ..add(id, 'Bread', deviceWallClock)
        ..check(milk, deviceWallClock);
      await start(tester);
      await openAllNotes(tester);

      expect(find.text('Bread', findRichText: true), findsOneWidget);
      expect(find.text('Milk', findRichText: true), findsNothing);
      expect(find.text('1 struck through'), findsOneWidget);
      await tap(tester, key('shopping-struck'));
      expect(find.text('Milk', findRichText: true), findsOneWidget);
      await tap(tester, key('shopping-struck'));
      expect(find.text('Milk', findRichText: true), findsNothing);

      await tap(tester, find.byType(Checkbox).first);
      expect(lists.byNote(id, deviceWallClock)!.openCount, 0);
    });

    testWidgets('of a Tally shows its chart, its entries and "Add entry"; '
        'the compact one none of them', (tester) async {
      final id = notes.capture('Reading').id;
      final tallies = Tallies(store);
      final pages = tallies.addField(
        id,
        name: 'Pages',
        unit: 'pages',
        target: 50,
      );
      tallies.add(id, values: {pages: 30}, line: 'on the train');
      await start(tester);
      await openAllNotes(tester);

      expect(key('tally-card'), findsOneWidget);
      expect(key('tally-chart-$pages'), findsOneWidget);
      // The bar length is the Tally's, set in its setup (#264).
      expect(key('tally-period'), findsNothing);
      expect(find.text('Pages 30 pages\non the train'), findsOneWidget);
      expect(key('tally-add-entry'), findsOneWidget);
      // The setup stays in the editor.
      expect(key('tally-fields'), findsNothing);
      expect(key('tally-add-field'), findsNothing);

      await tap(tester, find.text('Reading', findRichText: true));
      expect(key('tally-card'), findsNothing);
      expect(key('tally-add-entry'), findsNothing);
      expect(find.text('Pages 30 / 50 pages'), findsOneWidget);
    });
    testWidgets('of a Session Note shows its pool, today\'s picks of each '
        'Routine, recorded there as on the Agenda, and its last four weeks', (
      tester,
    ) async {
      final tallies = Tallies(store);
      String tally(String name) {
        now = now.add(const Duration(minutes: 1));
        final id = notes.capture(name).id;
        tallies.addField(id, name: 'Done', unit: 'times', target: 20);
        return id;
      }

      final pushUps = tally('Push-ups');
      final wallSit = tally('Wall sit');
      final bridge = tally('Half bridge');
      final today = Day.today(now, deviceWallClock);
      final sessions = Sessions(store);
      final made = sessions.spawn(
        notes.capture('Sport').id,
        Schedule(
          start: today,
          rule: Recurrence(frequency: Frequency.daily),
        ),
        pool: [pushUps, wallSit, bridge],
        count: 1,
        rule: SessionRule.rolling,
      );
      final evening = Routines(store).spawn(
        made.noteId,
        Schedule(
          start: today,
          times: [ClockTime(23, 0)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      await start(tester);
      await openAllNotes(tester);

      final card = key('card-session');
      expect(card, findsOneWidget);
      Finder inCard(Finder finder) =>
          find.descendant(of: card, matching: finder);
      for (final name in ['Push-ups', 'Wall sit', 'Half bridge']) {
        expect(inCard(find.text(name)), findsOneWidget);
      }
      expect(inCard(find.text('Picks 1 Tally · Rolling')), findsOneWidget);
      // Each Routine's own pick, Rolling going on from the earlier.
      final allDay = made.routine.schedule.on(today).single;
      final late = evening.schedule.on(today).single;
      expect(
        inCard(key('card-session-today-${made.id}@${allDay.key}')),
        findsOneWidget,
      );
      expect(
        inCard(key('card-session-today-${evening.id}@${late.key}')),
        findsOneWidget,
      );
      expect(inCard(key('session-pick-$pushUps')), findsOneWidget);
      expect(inCard(key('session-pick-$wallSit')), findsOneWidget);
      expect(inCard(key('session-pick-$bridge')), findsNothing);

      await tap(tester, inCard(key('session-pick-$pushUps')));
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      await tap(tester, key('tally-entry-save'));
      expect(sessions.on(made.id, allDay).complete, isTrue);
      expect(
        tester
            .widget<Text>(inCard(find.text('Push-ups 20 times')))
            .style
            ?.decoration,
        TextDecoration.lineThrough,
      );

      for (var week = 0; week < 4; week++) {
        expect(inCard(key('card-session-week-$week')), findsOneWidget);
      }
      expect(inCard(key('card-session-week-4')), findsNothing);
      expect(
        tester.widget<Text>(inCard(key('card-session-week-0'))).data,
        endsWith('done 1 / skipped 0'),
      );
    });

    testWidgets('of a Session Note without a Routine shows its pool and setup '
        'and no picks', (tester) async {
      final pool = notes.capture('Push-ups').id;
      Tallies(store).addField(pool, name: 'Done', unit: 'times', target: 20);
      Sessions(store).make(
        notes.capture('Sport').id,
        pool: [pool],
        count: 1,
        rule: SessionRule.random,
      );
      await start(tester);
      await openAllNotes(tester);

      final card = key('card-session');
      expect(
        find.descendant(of: card, matching: find.text('Push-ups')),
        findsOneWidget,
      );
      expect(key('card-session-setup'), findsOneWidget);
      expect(key('session-pick-$pool'), findsNothing);
      expect(key('card-session-week-0'), findsNothing);
    });
    testWidgets('of a Tally an older Session was spawned from stays a Tally, '
        'and its spawn menu offers no Session', (tester) async {
      final pool = notes.capture('Push-ups').id;
      Tallies(store).addField(pool, name: 'Done', unit: 'times', target: 20);
      final id = notes.capture('Reading').id;
      Tallies(store).addField(id, name: 'Pages', unit: 'pages');
      Routines(store).spawn(
        id,
        Schedule(
          start: Day(2026, 10, 25),
          rule: Recurrence(frequency: Frequency.daily),
        ),
        within: (routine) {
          store.append(routine, SessionKeys.pool, pool);
          store.append(routine, SessionKeys.count, '1');
          store.append(routine, SessionKeys.rule, 'rolling');
        },
      );
      await start(tester);
      await openAllNotes(tester);

      expect(key('tally-card'), findsNWidgets(2));
      expect(key('card-session'), findsNothing);
      await tap(
        tester,
        find.byTooltip('Routine, Reminder, Frog or Session').first,
      );
      expect(find.text('Routine'), findsOneWidget);
      expect(find.text('Session'), findsNothing);
    });
  });

  group('a tap', () {
    testWidgets('switches only that card, both ways; a long press opens the '
        'editor in either view', (tester) async {
      notes.capture('First Note\nits second line\nits third\nits fourth');
      now = now.add(const Duration(minutes: 1));
      notes.capture('Second Note\nmore\nmore still\nthe end');
      await start(tester);
      await openAllNotes(tester);
      expect(compactCard('First Note'), isFalse);

      await tap(tester, find.textContaining('First Note', findRichText: true));
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(compactCard('First Note'), isTrue);
      expect(compactCard('Second Note'), isFalse);

      await tap(tester, find.textContaining('First Note', findRichText: true));
      expect(compactCard('First Note'), isFalse);

      await tap(tester, find.textContaining('First Note', findRichText: true));
      await tester.longPress(
        find.textContaining('First Note', findRichText: true),
      );
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('is remembered on this Device until the list\'s switch '
        'changes', (tester) async {
      final first = notes.capture('First Note\nmore');
      now = now.add(const Duration(minutes: 1));
      final second = notes.capture('Second Note\nmore');
      await openCompact(tester);

      await tap(tester, find.text('Second Note', findRichText: true));
      expect(compactCard('Second Note'), isFalse);
      expect(store.localSetting('switchedNotes:allNotes'), second.id);
      await tap(tester, find.text('First Note', findRichText: true));
      expect(store.localSetting('switchedNotes:allNotes')!.split(',').toSet(), {
        first.id,
        second.id,
      });
      await tap(tester, find.textContaining('First Note', findRichText: true));
      expect(store.localSetting('switchedNotes:allNotes'), second.id);

      // Shown again, the list keeps it.
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('All Notes'));
      expect(compactCard('Second Note'), isFalse);
      expect(compactCard('First Note'), isTrue);

      // The switch sets every card again and forgets the taps.
      await tap(tester, find.byTooltip('Show detailed views'));
      expect(store.localSetting('switchedNotes:allNotes'), isNull);
      expect(compactCard('Second Note'), isFalse);
      expect(compactCard('First Note'), isFalse);
      await tap(tester, find.byTooltip('Show compact views'));
      expect(compactCard('Second Note'), isTrue);
      expect(compactCard('First Note'), isTrue);
    });
  });
}

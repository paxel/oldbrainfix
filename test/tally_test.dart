import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/tally_entry_screen.dart';
import 'package:oldbrainfix/src/tally_field_screen.dart';
import 'package:oldbrainfix/src/tally_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

void main() {
  late Store store;
  late Notes notes;
  late Tallies tallies;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    tallies = Tallies(store);
  });
  tearDown(() => store.close());

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

  /// Starts the app and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester, [Store? on]) async {
    await tester.pumpWidget(OldBrainFixApp(store: on ?? store));
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// Opens the ⋮ of the only card.
  Future<void> cardMenu(WidgetTester tester) =>
      tap(tester, find.byKey(const ValueKey('note-menu')));

  /// Shows the phone's region as [region], for the default unit system.
  void region(WidgetTester tester, String region) {
    tester.platformDispatcher.localeTestValue = Locale('en', region);
    addTearDown(tester.platformDispatcher.clearLocaleTestValue);
  }

  Finder key(String value) => find.byKey(ValueKey(value));

  group('making a Tally', () {
    testWidgets('"Make it a Tally" in the card\'s ⋮ opens a page for its '
        'first field; Save makes the Note a Tally out of Unsorted', (
      tester,
    ) async {
      final note = notes.capture('Push-ups');
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(key('note-add-entry'), findsNothing);
      await tap(tester, key('note-make-tally'));
      expect(find.byType(TallyFieldScreen), findsOneWidget);
      expect(find.byType(BottomSheet), findsNothing);
      expect(find.text('New field'), findsOneWidget);
      // Save sits in the top bar.
      expect(
        find.descendant(
          of: find.byType(AppBar),
          matching: key('tally-field-save'),
        ),
        findsOneWidget,
      );

      await enter(tester, key('tally-field-name'), 'Push-ups');
      await enter(tester, key('tally-unit-text'), 'times');
      await enter(tester, key('tally-field-target'), '20');
      await tap(tester, key('tally-field-save'));

      expect(find.byType(TallyFieldScreen), findsNothing);
      final tally = tallies.byNote(note.id);
      expect(tally, isNotNull);
      final field = tally!.fields.single;
      expect(field.name, 'Push-ups');
      expect(field.unit, 'times');
      expect(field.target, 20);
      expect(field.rule, TallyRule.sum);
      expect(notes.unsorted(), isEmpty);

      await cardMenu(tester);
      expect(key('note-make-tally'), findsNothing);
      expect(key('note-add-entry'), findsOneWidget);
    });

    testWidgets('a field needs a name, and a target is a number; both say '
        'so under the field', (tester) async {
      final note = notes.capture('Reading');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-make-tally'));
      await enter(tester, key('tally-field-target'), 'lots');
      await tap(tester, key('tally-field-save'));

      expect(find.byType(TallyFieldScreen), findsOneWidget);
      // The name to fix comes into view.
      expect(find.text('Give the field a name.'), findsOneWidget);
      await enter(tester, key('tally-field-name'), 'Pages');
      await tap(tester, key('tally-field-save'));
      expect(
        find.text('Not a number. Type it like 12, 2.5 or 1:30.'),
        findsOneWidget,
      );
      expect(find.byType(SnackBar), findsNothing);
      expect(tallies.isTally(note.id), isFalse);
    });

    testWidgets('known units show in the region\'s system and are stored in '
        'metric; picking one sets the rule its kind counts by', (tester) async {
      region(tester, 'US');
      final note = notes.capture('Lifting');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-make-tally'));

      // The unit is a column of radio buttons, shown in imperial.
      expect(
        find.descendant(of: key('tally-unit'), matching: find.text('lb')),
        findsOneWidget,
      );
      expect(find.text('kg'), findsNothing);
      await enter(tester, key('tally-field-name'), 'Weight');
      await tap(tester, key('tally-unit-kg'));
      await enter(tester, key('tally-field-target'), '100');
      await tap(tester, key('tally-field-save'));

      final field = tallies.byNote(note.id)!.fields.single;
      expect(field.unit, 'kg');
      expect(field.target, closeTo(45.359237, 1e-6));
      expect(field.rule, TallyRule.max);

      // Shown back in imperial.
      await tester.longPress(find.text('Lifting'));
      await tester.pumpAndSettle();
      expect(find.text('lb · Target 100 lb'), findsOneWidget);
    });

    testWidgets('a field hidden keeps its values, which return when it is '
        'shown again', (tester) async {
      final note = notes.capture('Running');
      final id = tallies.addField(note.id, name: 'Distance', unit: 'km');
      tallies.add(note.id, values: {id: 5000});
      await startAtNotes(tester);
      await tester.longPress(find.text('Running'));
      await tester.pumpAndSettle();
      expect(find.byType(TallySection), findsOneWidget);
      await tap(tester, key('tally-field-$id'));
      // Entries hold values: only distances are left.
      expect(
        find.text(
          'Entries hold values for this field, so its kind of unit stays.',
        ),
        findsOneWidget,
      );
      expect(key('tally-unit-kg'), findsNothing);
      expect(key('tally-unit-typed'), findsNothing);
      expect(key('tally-unit-m'), findsOneWidget);

      await tap(tester, key('tally-field-hide'));
      expect(tallies.byNote(note.id)!.field(id)!.hidden, isTrue);
      expect(key('tally-chart-$id'), findsNothing);

      await tap(tester, key('tally-field-$id'));
      await reveal(tester, key('tally-field-hide'));
      expect(find.text('Show field again'), findsOneWidget);
      await tap(tester, key('tally-field-hide'));
      expect(tallies.byNote(note.id)!.field(id)!.hidden, isFalse);
      expect(tallies.entries(note.id).single.values[id], 5000);
      expect(key('tally-chart-$id'), findsOneWidget);
    });
  });

  group('entries', () {
    testWidgets('"Add entry" in the card\'s ⋮ starts with the targets, so '
        'saving them is one tap', (tester) async {
      final note = notes.capture('Push-ups');
      final reps = tallies.addField(note.id, name: 'Push-ups', target: 20);
      final weight = tallies.addField(note.id, name: 'Weight', unit: 'kg');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-add-entry'));
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      expect(
        tester.widget<TextField>(key('tally-value-$reps')).controller?.text,
        '20',
      );
      expect(
        tester.widget<TextField>(key('tally-value-$weight')).controller?.text,
        '',
      );

      await tap(tester, key('tally-entry-save'));
      final entry = tallies.entries(note.id).single;
      expect(entry.values, {reps: 20});
      expect(entry.line, '');
      expect(entry.at, now.toUtc());
    });

    testWidgets('an entry holds a value or a line; a value is a number', (
      tester,
    ) async {
      final note = notes.capture('Pages');
      final id = tallies.addField(note.id, name: 'Pages');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-add-entry'));
      await tap(tester, key('tally-entry-save'));
      expect(find.text('Enter a value or a line of text.'), findsOneWidget);
      expect(tallies.entries(note.id), isEmpty);

      await enter(tester, key('tally-value-$id'), 'twelve');
      await tap(tester, key('tally-entry-save'));
      expect(
        find.text('Not a number. Type it like 12, 2.5 or 1:30.'),
        findsOneWidget,
      );
      expect(tallies.entries(note.id), isEmpty);

      await enter(tester, key('tally-value-$id'), '12,5');
      await enter(tester, key('tally-entry-line'), 'on the train');
      await tap(tester, key('tally-entry-save'));
      final entry = tallies.entries(note.id).single;
      expect(entry.values, {id: 12.5});
      expect(entry.line, 'on the train');
    });

    testWidgets('the Note shows its entries as a table and a chart per field '
        'with the day\'s sum; Day, Week, Month and Year are radio buttons', (
      tester,
    ) async {
      final note = notes.capture('Push-ups');
      final id = tallies.addField(note.id, name: 'Push-ups', unit: 'times');
      tallies.add(note.id, values: {id: 10}, line: 'morning');
      now = now.add(const Duration(hours: 2));
      tallies.add(note.id, values: {id: 15});
      await startAtNotes(tester);
      await tester.longPress(find.text('Push-ups'));
      await tester.pumpAndSettle();

      await reveal(tester, key('tally-entries'));
      expect(find.text('Push-ups 10 times\nmorning'), findsOneWidget);
      expect(find.text('Push-ups 15 times'), findsOneWidget);
      await reveal(tester, key('tally-chart-$id'));
      expect(find.text('Highest bar: 25 times'), findsOneWidget);
      expect(
        find.descendant(
          of: key('tally-period'),
          matching: find.byType(RadioListTile<TallyPeriod>),
        ),
        findsNWidgets(4),
      );
      expect(find.byType(TabBar), findsNothing);
      expect(find.byType(SegmentedButton<TallyPeriod>), findsNothing);
      await tap(tester, key('tally-period-month'));
      expect(find.text('Highest bar: 25 times'), findsOneWidget);
    });

    testWidgets('tapping an entry edits it; a long press deletes it after '
        'asking once, and "Removed entries" restores it', (tester) async {
      final note = notes.capture('Pages');
      final id = tallies.addField(note.id, name: 'Pages');
      final entry = tallies.add(note.id, values: {id: 12});
      await startAtNotes(tester);
      await tester.longPress(find.text('Pages'));
      await tester.pumpAndSettle();

      await tap(tester, key('tally-entry-${entry.id}'));
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      expect(find.text('Entry'), findsOneWidget);
      await enter(tester, key('tally-value-$id'), '30');
      await tap(tester, key('tally-entry-save'));
      expect(tallies.entryById(entry.id)!.values[id], 30);
      expect(find.text('Pages 30'), findsOneWidget);

      await reveal(tester, key('tally-entry-${entry.id}'));
      await tester.longPress(key('tally-entry-${entry.id}'));
      await tester.pumpAndSettle();
      expect(find.byType(BottomSheet), findsNothing);
      await tap(tester, key('tally-entry-delete'));
      expect(
        find.text(
          'Delete this entry? It can be restored under "Removed entries".',
        ),
        findsOneWidget,
      );
      final confirm = tester.widget<TextButton>(key('tally-delete-confirm'));
      expect(
        confirm.style?.foregroundColor?.resolve({}),
        Theme.of(tester.element(key('tally-delete-confirm'))).colorScheme.error,
      );
      await tap(tester, key('tally-delete-confirm'));
      expect(tallies.entryById(entry.id)!.isTrashed, isTrue);
      expect(tallies.entries(note.id), isEmpty);
      expect(find.text('No entries yet.'), findsOneWidget);

      await tap(tester, key('tally-removed'));
      expect(store.localSetting('tallyRemovedOpen'), 'true');
      await tap(tester, key('tally-restore-${entry.id}'));
      expect(tallies.entryById(entry.id)!.isTrashed, isFalse);
      expect(key('tally-removed'), findsNothing);
      expect(find.text('Pages 30'), findsOneWidget);
    });

    testWidgets('a new entry counts for the day picked', (tester) async {
      final note = notes.capture('Pages');
      final id = tallies.addField(note.id, name: 'Pages');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final context = tester.element(find.byType(Scaffold).first);
      final done = addTallyEntry(context, notes, note.id);
      await tester.pumpAndSettle();
      await enter(tester, key('tally-value-$id'), '5');
      await tap(tester, key('tally-entry-date'));
      expect(find.byType(DatePickerDialog), findsOneWidget);
      await tap(tester, find.text('22'));
      await tap(tester, find.text('OK'));
      await tap(tester, key('tally-entry-save'));
      expect(await done, isTrue);
      expect(
        tallies.entries(note.id).single.at,
        DateTime(2026, 10, 22, 9).toUtc(),
      );
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;
    late String note;
    late String field;

    setUp(() {
      store
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ann');
      ben = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ben');
      final house = Places(store).addArea('Household');
      note = notes.capture('Push-ups', place: house.id).id;
      field = tallies.addField(note, name: 'Push-ups', unit: 'times');
      tallies.add(note, values: {field: 10});
      area = store.share(house.id);
    });
    tearDown(() => ben.close());

    /// Ann's computer invites Ben's phone as a Member, as a LAN session
    /// would.
    void invite() {
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
      for (final (store, peer) in [(store, ben), (ben, store)]) {
        store.rememberDevice(
          PairedDevice(
            id: peer.deviceId,
            name: '',
            fingerprint: '',
            host: null,
            port: null,
            lastSync: DateTime.utc(2026, 3, 1),
            shared: true,
          ),
        );
      }
    }

    testWidgets('entries carry their author; the user\'s own show by '
        'default, everyone\'s or one person\'s on a radio button', (
      tester,
    ) async {
      invite();
      Tallies(ben).add(note, values: {field: 7}, line: 'Ben did some');
      final done = answerSync(
        store,
        wire(
          syncRequest(
            ben,
            store.versionVector(),
            name: 'Phone',
            hostDevice: store.deviceId,
          ),
        ),
        name: 'Desk',
      )!;
      applyAnswer(ben, wire(done.answer), shared: true);
      // Ben's entry reached Ann.
      expect(tallies.entries(note), hasLength(2));

      await startAtNotes(tester);
      await tester.longPress(find.text('Push-ups'));
      await tester.pumpAndSettle();
      await reveal(tester, key('tally-whose'));
      expect(
        tester
            .widget<RadioGroup<String>>(
              find.descendant(
                of: key('tally-whose'),
                matching: find.byType(RadioGroup<String>),
              ),
            )
            .groupValue,
        '',
      );
      await reveal(tester, key('tally-entries'));
      expect(find.text('Push-ups 10 times'), findsOneWidget);
      expect(find.textContaining('Ben did some'), findsNothing);

      await tap(tester, key('tally-whose-everyone'));
      await reveal(tester, key('tally-entries'));
      expect(find.text('Ben\nPush-ups 7 times\nBen did some'), findsOneWidget);
      expect(find.text('Push-ups 10 times'), findsOneWidget);

      final bens = tallies.entries(note).firstWhere((e) => e.line.isNotEmpty);
      final author = store.authorOf(bens.id)!;
      expect(author.name, 'Ben');
      await tap(tester, key('tally-whose-${author.id}'));
      await reveal(tester, key('tally-entries'));
      expect(find.text('Ben\nPush-ups 7 times\nBen did some'), findsOneWidget);
      expect(find.text('Push-ups 10 times'), findsNothing);
    });
  });
}

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

      // The unit is one field; the known units it may mean show under it
      // as chips, in imperial.
      expect(find.byType(RadioListTile<String>), findsNothing);
      await enter(tester, key('tally-field-name'), 'Weight');
      await enter(tester, key('tally-unit-text'), 'p');
      expect(
        find.descendant(of: key('tally-unit'), matching: find.text('lb')),
        findsOneWidget,
      );
      expect(key('tally-unit-g'), findsNothing);
      expect(find.text('kg'), findsNothing);
      await tap(tester, key('tally-unit-kg'));
      expect(
        tester.widget<TextField>(key('tally-unit-text')).controller!.text,
        'lb',
      );
      await enter(tester, key('tally-field-target'), '100');
      await tap(tester, key('tally-field-save'));

      final field = tallies.byNote(note.id)!.fields.single;
      expect(field.unit, 'kg');
      expect(field.target, closeTo(45.359237, 1e-6));
      expect(field.rule, TallyRule.max);

      // Shown back in imperial.
      await tester.longPress(find.text('Lifting'));
      await tester.pumpAndSettle();
      for (final (badge, value) in [('unit', 'lb'), ('target', '100 lb')]) {
        expect(
          find.descendant(
            of: key('tally-setup-$badge-${field.id}'),
            matching: find.text(value),
          ),
          findsOneWidget,
        );
      }
    });

    testWidgets('typing filters the known units, in English or German; a '
        'chip picks one, and a unit typed out is that unit', (tester) async {
      region(tester, 'GB');
      final note = notes.capture('Video');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-make-tally'));
      await enter(tester, key('tally-field-name'), 'Watched');

      // Every known unit until something is typed.
      for (final known in TallyUnits.known) {
        expect(key('tally-unit-${known.code}'), findsOneWidget);
      }
      await enter(tester, key('tally-unit-text'), 'k');
      expect(key('tally-unit-kg'), findsOneWidget);
      expect(key('tally-unit-km'), findsOneWidget);
      expect(key('tally-unit-min'), findsNothing);
      await enter(tester, key('tally-unit-text'), 'Minu');
      expect(key('tally-unit-min'), findsOneWidget);
      expect(key('tally-unit-kg'), findsNothing);
      await tap(tester, key('tally-unit-min'));
      expect(
        tester.widget<TextField>(key('tally-unit-text')).controller!.text,
        'min',
      );
      await tap(tester, key('tally-field-save'));
      expect(tallies.byNote(note.id)!.fields.single.unit, 'min');
    });

    testWidgets('in imperial, a symbol typed out without its chip is that '
        'known unit', (tester) async {
      region(tester, 'US');
      final note = notes.capture('Running');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-make-tally'));
      await enter(tester, key('tally-field-name'), 'Distance');
      await enter(tester, key('tally-unit-text'), 'mi');
      expect(key('tally-unit-km'), findsOneWidget);
      await tap(tester, key('tally-field-save'));
      expect(tallies.byNote(note.id)!.fields.single.unit, 'km');
    });

    testWidgets('anything else typed stays a unit of the user\'s own, and '
        'the field opens with it as typed', (tester) async {
      region(tester, 'US');
      final note = notes.capture('Reading');
      // Typed on another Device: it reads like a known symbol here.
      final id = tallies.addField(note.id, name: 'Weight', unit: 'lb');
      tallies.add(note.id, values: {id: 3});
      await startAtNotes(tester);
      await tester.longPress(find.text('Reading'));
      await tester.pumpAndSettle();
      await tap(tester, key('tally-field-$id'));
      expect(
        tester.widget<TextField>(key('tally-unit-text')).controller!.text,
        'lb',
      );
      // A unit of its own takes no known one.
      expect(find.byType(ActionChip), findsNothing);
      await enter(tester, key('tally-field-name'), 'Load');
      await tap(tester, key('tally-field-save'));
      final field = tallies.byNote(note.id)!.field(id)!;
      expect(field.name, 'Load');
      expect(field.unit, 'lb');
    });

    testWidgets('a chip makes a unit of the user\'s own that reads like a '
        'known symbol that known unit', (tester) async {
      region(tester, 'US');
      final note = notes.capture('Lifting');
      final id = tallies.addField(note.id, name: 'Weight', unit: 'lb');
      await startAtNotes(tester);
      await tester.longPress(find.text('Lifting'));
      await tester.pumpAndSettle();
      await tap(tester, key('tally-field-$id'));
      // No entries yet: every known unit is a tap away.
      await tap(tester, key('tally-unit-kg'));
      await tap(tester, key('tally-field-save'));
      expect(tallies.byNote(note.id)!.field(id)!.unit, 'kg');
    });

    testWidgets('once entries hold values, a unit of another kind is not '
        'saved and the field says why', (tester) async {
      region(tester, 'DE');
      final note = notes.capture('Running');
      final id = tallies.addField(note.id, name: 'Distance', unit: 'km');
      tallies.add(note.id, values: {id: 5000});
      await startAtNotes(tester);
      await tester.longPress(find.text('Running'));
      await tester.pumpAndSettle();
      await tap(tester, key('tally-field-$id'));

      await enter(tester, key('tally-unit-text'), 'kilo');
      expect(key('tally-unit-km'), findsOneWidget);
      expect(key('tally-unit-kg'), findsNothing);
      expect(key('tally-unit-km/h'), findsNothing);
      await enter(tester, key('tally-unit-text'), 'pages');
      await tap(tester, key('tally-field-save'));
      expect(find.byType(TallyFieldScreen), findsOneWidget);
      expect(
        find.text(
          'Entries hold values for this field, so its kind of unit stays.',
        ),
        findsNWidgets(2),
      );
      expect(tallies.byNote(note.id)!.field(id)!.unit, 'km');

      // Within the kind it changes; the values stay in metres.
      await enter(tester, key('tally-unit-text'), 'Met');
      await tap(tester, key('tally-unit-m'));
      await tap(tester, key('tally-field-save'));
      expect(tallies.byNote(note.id)!.field(id)!.unit, 'm');
      expect(tallies.entries(note.id).single.values[id], 5000);
    });

    testWidgets('a field hidden keeps its values, which return when it is '
        'shown again', (tester) async {
      final note = notes.capture('Running');
      final id = tallies.addField(note.id, name: 'Distance', unit: 'km');
      tallies.add(note.id, values: {id: 5000});
      await startAtNotes(tester);
      expect(key('tally-chart-$id'), findsOneWidget);
      await tester.longPress(find.text('Running'));
      await tester.pumpAndSettle();
      expect(find.byType(TallySetup), findsOneWidget);
      await tap(tester, key('tally-field-$id'));
      // Entries hold values: only distances are left.
      expect(
        find.text(
          'Entries hold values for this field, so its kind of unit stays.',
        ),
        findsOneWidget,
      );
      expect(key('tally-unit-kg'), findsNothing);
      expect(key('tally-unit-km'), findsOneWidget);
      expect(key('tally-unit-m'), findsOneWidget);

      await tap(tester, key('tally-field-hide'));
      expect(tallies.byNote(note.id)!.field(id)!.hidden, isTrue);
      expect(key('tally-setup-hidden-$id'), findsOneWidget);

      await tap(tester, key('tally-setup-hidden-$id'));
      await reveal(tester, key('tally-field-hide'));
      expect(find.text('Show field again'), findsOneWidget);
      await tap(tester, key('tally-field-hide'));
      expect(tallies.byNote(note.id)!.field(id)!.hidden, isFalse);
      expect(tallies.entries(note.id).single.values[id], 5000);
      expect(key('tally-setup-hidden-$id'), findsNothing);
    });
  });

  group('the editor', () {
    testWidgets('holds only the setup: each field\'s name with its rule, '
        'unit and target as badges, and "Add field"; no chart, no entries', (
      tester,
    ) async {
      final note = notes.capture('Push-ups');
      final id = tallies.addField(
        note.id,
        name: 'Push-ups',
        unit: 'times',
        target: 20,
      );
      tallies.add(note.id, values: {id: 10});
      await startAtNotes(tester);
      await tester.longPress(find.text('Push-ups'));
      await tester.pumpAndSettle();

      expect(find.byType(TallySetup), findsOneWidget);
      expect(find.byType(CardTally), findsNothing);
      expect(find.byType(TallyChart), findsNothing);
      expect(key('tally-entries'), findsNothing);
      expect(key('tally-period'), findsNothing);
      expect(key('tally-add-entry'), findsNothing);
      // The rule as its icon (#192), the others as text.
      expect(
        find.descendant(
          of: key('tally-setup-rule-$id'),
          matching: find.byIcon(Icons.add_circle_outline),
        ),
        findsOneWidget,
      );
      for (final (badge, value) in [
        ('unit', 'times'),
        ('target', '20 times'),
      ]) {
        expect(
          find.descendant(
            of: key('tally-setup-$badge-$id'),
            matching: find.text(value),
          ),
          findsOneWidget,
        );
      }

      // A badge opens the field's page.
      await tap(tester, key('tally-setup-target-$id'));
      expect(find.byType(TallyFieldScreen), findsOneWidget);
      expect(find.text('Field'), findsOneWidget);
      await enter(tester, key('tally-field-target'), '25');
      await tap(tester, key('tally-field-save'));
      expect(tallies.byNote(note.id)!.field(id)!.target, 25);
      expect(
        find.descendant(
          of: key('tally-setup-target-$id'),
          matching: find.text('25 times'),
        ),
        findsOneWidget,
      );

      await tap(tester, key('tally-add-field'));
      expect(find.text('New field'), findsOneWidget);
      await enter(tester, key('tally-field-name'), 'Sets');
      await tap(tester, key('tally-field-save'));
      expect(tallies.byNote(note.id)!.fields, hasLength(2));
      expect(find.text('Sets'), findsOneWidget);
    });
  });

  group('sets and Check fields (#264)', () {
    testWidgets('the setup\'s badges set the bar length and whether it '
        'counts in sets, on the Note', (tester) async {
      final note = notes.capture('Push-ups');
      tallies.addField(note.id, name: 'Reps');
      await startAtNotes(tester);
      await tester.longPress(find.text('Push-ups'));
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: key('tally-setup-bar'), matching: find.text('Day')),
        findsOneWidget,
      );
      await tap(tester, key('tally-setup-bar'));
      await tap(tester, key('tally-bar-week'));
      expect(tallies.byNote(note.id)!.bar, TallyPeriod.week);
      expect(
        find.descendant(
          of: key('tally-setup-bar'),
          matching: find.text('Week'),
        ),
        findsOneWidget,
      );
      await tap(tester, key('tally-setup-sets'));
      await tap(tester, key('badge-switch'));
      await tap(tester, key('badge-close'));
      expect(tallies.byNote(note.id)!.countsInSets, isTrue);
    });

    testWidgets('"Add set" adds a set that starts with the set before; each '
        'field totals the sets by its rule', (tester) async {
      region(tester, 'DE');
      final note = notes.capture('Bench');
      final weight = tallies.addField(
        note.id,
        name: 'Weight',
        unit: 'kg',
        rule: TallyRule.max,
      );
      // The fields show in the order they were added.
      now = now.add(const Duration(seconds: 1));
      final reps = tallies.addField(note.id, name: 'Reps');
      tallies.setCountsInSets(note.id, true);
      await startAtNotes(tester);
      // No period to pick on the card: the bar length is the Tally's.
      expect(key('tally-period'), findsNothing);
      await tap(tester, key('tally-add-entry'));
      expect(find.text('Set 1'), findsOneWidget);
      await enter(tester, key('tally-value-$weight'), '60');
      await enter(tester, key('tally-value-$reps'), '10');
      await tap(tester, key('tally-add-set'));
      expect(find.text('Set 2'), findsOneWidget);
      String text(String value) =>
          tester.widget<TextField>(key(value)).controller?.text ?? '';
      expect(text('tally-value-1-$weight'), '60');
      expect(text('tally-value-1-$reps'), '10');
      await enter(tester, key('tally-value-1-$reps'), '8');
      await tap(tester, key('tally-add-set'));
      await tap(tester, key('tally-set-remove-2'));
      expect(find.text('Set 3'), findsNothing);
      await tap(tester, key('tally-entry-save'));

      final entry = tallies.entries(note.id).single;
      expect(entry.values, {weight: 60, reps: 10});
      expect(entry.more, [
        {weight: 60, reps: 8},
      ]);
      await reveal(tester, key('tally-entries'));
      expect(
        find.text('1. Weight 60 kg · Reps 10\n2. Weight 60 kg · Reps 8'),
        findsOneWidget,
      );
      await reveal(tester, key('tally-chart-$reps'));
      expect(find.text('Highest bar: 18'), findsOneWidget);
      expect(find.text('Highest bar: 60 kg'), findsOneWidget);
    });

    testWidgets('a Check field is a box per entry and charts its ticks', (
      tester,
    ) async {
      final note = notes.capture('Kitchen');
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-make-tally'));
      await enter(tester, key('tally-field-name'), 'Wiped');
      await tap(tester, key('tally-field-kind-check'));
      expect(key('tally-unit-text'), findsNothing);
      expect(key('tally-rule'), findsNothing);
      await tap(tester, key('tally-field-save'));
      final field = tallies.byNote(note.id)!.fields.single;
      expect(field.check, isTrue);

      for (var i = 0; i < 2; i++) {
        await tap(tester, key('tally-add-entry'));
        await tap(tester, key('tally-check-${field.id}'));
        await tap(tester, key('tally-entry-save'));
        now = now.add(const Duration(hours: 1));
      }
      expect(
        [for (final e in tallies.entries(note.id)) e.values],
        [
          {field.id: 1},
          {field.id: 1},
        ],
      );
      await reveal(tester, key('tally-entries'));
      expect(find.text('✓ Wiped'), findsNWidgets(2));
      await reveal(tester, key('tally-chart-${field.id}'));
      expect(find.text('Highest bar: 2'), findsOneWidget);
    });

    testWidgets('a Tally a multiplier made shows its entries as sets', (
      tester,
    ) async {
      final note = notes.capture('Push-ups');
      final reps = tallies.addField(note.id, name: 'Reps', unit: 'reps');
      final rounds = tallies.addField(note.id, name: 'Rounds');
      // As #200 wrote it.
      store.append(
        note.id,
        TallyKeys.field(rounds, TallyKeys.multiplies),
        reps,
      );
      final entry = tallies.add(note.id, values: {reps: 20, rounds: 3});
      await startAtNotes(tester);
      await reveal(tester, key('tally-entries'));
      expect(
        find.text('1. Reps 20 reps\n2. Reps 20 reps\n3. Reps 20 reps'),
        findsOneWidget,
      );
      expect(key('tally-chart-$rounds'), findsNothing);

      await tap(tester, key('tally-entry-${entry.id}'));
      expect(find.text('Set 3'), findsOneWidget);
      expect(key('tally-add-set'), findsOneWidget);
      await enter(tester, key('tally-value-2-$reps'), '15');
      await tap(tester, key('tally-entry-save'));
      final edited = tallies.entryById(entry.id)!;
      expect(edited.values, {reps: 20});
      expect(edited.more, [
        {reps: 20},
        {reps: 15},
      ]);
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
        'with the sum of the Tally\'s bar', (tester) async {
      final note = notes.capture('Push-ups');
      final id = tallies.addField(note.id, name: 'Push-ups', unit: 'times');
      tallies.add(note.id, values: {id: 10}, line: 'morning');
      now = now.add(const Duration(hours: 2));
      tallies.add(note.id, values: {id: 15});
      await startAtNotes(tester);

      // On the detailed card, as the list starts.
      expect(find.byType(CardTally), findsOneWidget);
      await reveal(tester, key('tally-entries'));
      expect(find.text('Push-ups 10 times\nmorning'), findsOneWidget);
      expect(find.text('Push-ups 15 times'), findsOneWidget);
      await reveal(tester, key('tally-chart-$id'));
      expect(find.text('Highest bar: 25 times'), findsOneWidget);
      expect(key('tally-period'), findsNothing);
      // The bar length lies on the Note (#264): a month's bar holds both.
      tallies.setBar(note.id, TallyPeriod.month);
      await tap(tester, find.text('Push-ups').first);
      await tap(tester, find.text('Push-ups').first);
      await reveal(tester, key('tally-chart-$id'));
      expect(find.text('Highest bar: 25 times'), findsOneWidget);
    });

    testWidgets('"Add entry" on the card adds one, its total shows on the '
        'compact card against the target', (tester) async {
      final note = notes.capture('Steps');
      final id = tallies.addField(
        note.id,
        name: 'Steps',
        unit: 'steps',
        target: 10000,
      );
      await startAtNotes(tester);
      await tap(tester, key('tally-add-entry'));
      expect(find.byType(TallyEntryScreen), findsOneWidget);
      await enter(tester, key('tally-value-$id'), '7000');
      await tap(tester, key('tally-entry-save'));
      expect(tallies.entries(note.id).single.values[id], 7000);
      expect(find.text('Steps 7000 steps'), findsOneWidget);

      // A tap on the card shows it compact.
      await tap(tester, find.text('Steps').first);
      expect(find.byType(CardTally), findsNothing);
      expect(find.text('Steps 7000 / 10000 steps'), findsOneWidget);
    });

    testWidgets('the card shows the latest five entries, newest first, the '
        'older folded under how many until a tap, kept on this Device', (
      tester,
    ) async {
      final note = notes.capture('Pages');
      final id = tallies.addField(note.id, name: 'Pages');
      final added = <TallyEntry>[];
      for (var i = 1; i <= 7; i++) {
        added.add(tallies.add(note.id, values: {id: i.toDouble()}));
        now = now.add(const Duration(minutes: 1));
      }
      await startAtNotes(tester);
      await reveal(tester, key('tally-older'));
      for (final entry in added.skip(2)) {
        expect(key('tally-entry-${entry.id}'), findsOneWidget);
      }
      for (final entry in added.take(2)) {
        expect(key('tally-entry-${entry.id}'), findsNothing);
      }
      expect(
        tester.getTopLeft(key('tally-entry-${added[6].id}')).dy,
        lessThan(tester.getTopLeft(key('tally-entry-${added[2].id}')).dy),
      );
      expect(find.text('2 older entries'), findsOneWidget);

      await tap(tester, key('tally-older'));
      expect(key('tally-entry-${added[0].id}'), findsOneWidget);
      expect(store.localSetting(tallyEntriesShownSetting(note.id)), 'true');
      await tap(tester, key('tally-older'));
      expect(key('tally-entry-${added[0].id}'), findsNothing);
      expect(store.localSetting(tallyEntriesShownSetting(note.id)), isNull);
    });

    testWidgets('tapping an entry edits it; a long press deletes it after '
        'asking once, and "Removed entries" restores it', (tester) async {
      final note = notes.capture('Pages');
      final id = tallies.addField(note.id, name: 'Pages');
      final entry = tallies.add(note.id, values: {id: 12});
      await startAtNotes(tester);

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
      // Today on Ann's clock too.
      Tallies(ben)
          .add(note, values: {field: 7}, line: 'Ben did some', at: now.toUtc());
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
      // The compact card counts the user's own entries, as the detailed
      // view does by default.
      await tap(tester, find.byKey(const ValueKey('compactNotes')));
      expect(find.text('Push-ups 10 times'), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('compactNotes')));
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

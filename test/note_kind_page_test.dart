import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/line_editing.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

/// A Note is its kind (#188): a Checklist, Shopping List or Tally shows
/// its name on top, its description folded under it, then its content;
/// a plain Note keeps its one text field. Nothing is rewritten to show it.
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// The text field of the field keyed [name].
  Finder field(String name) =>
      find.descendant(of: key(name), matching: find.byType(TextField));

  TextField widgetOf(WidgetTester tester, String name) =>
      tester.widget<TextField>(field(name));

  String typedIn(WidgetTester tester, String name) =>
      widgetOf(tester, name).controller!.text;

  /// Opens the editor of the Note [id] from the Agenda, on a phone-tall
  /// page.
  Future<void> edit(WidgetTester tester, String id) async {
    tester.view
      ..physicalSize = const Size(1080, 2340)
      ..devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, places, notes.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  Future<void> leave(WidgetTester tester) async {
    await tester.pageBack();
    await tester.pumpAndSettle();
  }

  const description = 'Take the early train\nBook seats';

  /// A Note of [kind] named "Trip" with [description], its text set after
  /// it became one, and a Section.
  String kindNote(NoteKind kind) {
    final id = notes.capture('Trip\nPassport').id;
    switch (kind) {
      case NoteKind.checklist:
        Checklists(store).make(id);
      case NoteKind.shoppingList:
        ShoppingLists(store).make(id);
      case NoteKind.tally:
        Tallies(store).addField(id, name: 'Pages', unit: 'pages');
      case NoteKind.session:
        final pool = notes.capture('Stretch').id;
        Tallies(store).addField(pool, name: 'Times', unit: 'times');
        Sessions(store)
            .make(id, pool: [pool], count: 1, rule: SessionRule.rolling);
      case NoteKind.plain:
    }
    notes.edit(id, 'Trip\n$description');
    now = now.add(const Duration(minutes: 5));
    notes.amend(id, 'Later on the way');
    return id;
  }

  for (final (kind, content) in [
    (NoteKind.checklist, 'checklist'),
    (NoteKind.shoppingList, 'shopping'),
    (NoteKind.tally, 'tally-fields'),
    (NoteKind.session, 'session-pool'),
  ]) {
    testWidgets('a ${kind.name} shows its name on top, its description '
        'folded under it, then its content, the badges, and its Sections and '
        'Amend below', (tester) async {
      final id = kindNote(kind);
      await edit(tester, id);
      expect(key('note-text'), findsNothing);
      expect(typedIn(tester, 'note-name'), 'Trip');
      // Folded to its first line, read as such.
      expect(key('note-description'), findsNothing);
      final folded = key('note-description-folded');
      expect(
        find.descendant(
          of: folded,
          matching: find.text('Take the early train'),
        ),
        findsOneWidget,
      );
      expect(find.textContaining('Book seats'), findsNothing);
      final name = tester.getBottomLeft(key('note-name')).dy;
      expect(tester.getTopLeft(folded).dy, greaterThanOrEqualTo(name));
      expect(
        tester.getTopLeft(key(content)).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(folded).dy),
      );
      // A list or Tally is one card, its content in the card of its name
      // and in no panel of its own (obf-1071).
      final card = find.ancestor(
        of: key('note-name'),
        matching: find.byType(Embossed),
      );
      expect(
        find.descendant(of: card, matching: key(content)),
        kind == NoteKind.session ? findsNothing : findsOneWidget,
      );
      if (kind != NoteKind.session) {
        expect(
          find.ancestor(of: key(content), matching: find.byType(EditorPanel)),
          findsNothing,
        );
      }
      // Measured on the page, as This Note's heading may push the
      // badges out of sight of the content (obf-1073).
      final page = tester.state<ScrollableState>(find.byType(Scrollable).first);
      final contentEnd =
          tester.getBottomLeft(key(content)).dy + page.position.pixels;
      await reveal(tester, key('note-badges'));
      expect(
        tester.getTopLeft(key('note-badges')).dy + page.position.pixels,
        greaterThanOrEqualTo(contentEnd),
      );
      final section = notes.sections(id).single.id;
      await reveal(tester, key('section-text:$section'));
      expect(
        tester.getTopLeft(key('section-text:$section')).dy,
        greaterThan(tester.getBottomLeft(key('note-badges')).dy),
      );
      // In no box, under a tear line (#307).
      expect(
        find.ancestor(
          of: key('section-text:$section'),
          matching: find.byType(Embossed),
        ),
        findsNothing,
      );
      expect(
        tester.getBottomLeft(key('tear:$section')).dy,
        lessThanOrEqualTo(tester.getTopLeft(key('section-text:$section')).dy),
      );
      // No Amend box or button under it: "Add a part" opens it
      // (obf-1072).
      expect(find.text('Add to Note'), findsNothing);
      expect(find.text("What's new?"), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('a tap unfolds the description into its field, the cursor in '
      'it; leaving it, or its fold button, folds it again', (tester) async {
    final semantics = tester.ensureSemantics();
    final id = kindNote(NoteKind.checklist);
    await edit(tester, id);
    expect(
      tester.getSemantics(key('note-description-folded')),
      matchesSemantics(
        hint: 'Show the whole description',
        isButton: true,
        hasTapAction: true,
        hasFocusAction: true,
        isFocusable: true,
        label: 'Take the early train',
      ),
    );
    await tap(tester, key('note-description-folded'));
    expect(key('note-description-folded'), findsNothing);
    expect(typedIn(tester, 'note-description'), description);
    expect(widgetOf(tester, 'note-description').readOnly, isFalse);
    expect(widgetOf(tester, 'note-description').focusNode!.hasFocus, isTrue);
    expect(find.byTooltip('Fold the description'), findsOneWidget);

    await tap(tester, key('note-description-fold'));
    expect(key('note-description'), findsNothing);
    expect(key('note-description-folded'), findsOneWidget);

    await tap(tester, key('note-description-folded'));
    // The name takes the cursor; the description folds.
    await tap(tester, field('note-name'));
    expect(key('note-description-folded'), findsOneWidget);
    semantics.dispose();
  });

  testWidgets('a plain Note keeps its one text field', (tester) async {
    final id = notes.capture('Call the dentist\nAsk about Tuesday').id;
    await edit(tester, id);
    expect(typedIn(tester, 'note-text'), 'Call the dentist\nAsk about Tuesday');
    expect(key('note-name'), findsNothing);
    expect(key('note-description'), findsNothing);
    expect(key('note-description-folded'), findsNothing);
  });

  group('saving', () {
    testWidgets('a changed name or description writes the name and the '
        'description as the Note\'s text', (tester) async {
      final id = kindNote(NoteKind.shoppingList);
      await edit(tester, id);
      await tester.enterText(field('note-name'), 'Train trip');
      await tap(tester, key('note-description-folded'));
      await tester.enterText(field('note-description'), 'Seats 41 and 42');
      await leave(tester);
      expect(notes.byId(id)!.text, 'Train trip\nSeats 41 and 42');

      // A description emptied leaves the name alone.
      await edit(tester, id);
      await tap(tester, key('note-description-folded'));
      await tester.enterText(field('note-description'), '');
      await leave(tester);
      expect(notes.byId(id)!.text, 'Train trip');
    });

    testWidgets('an emptied name gets the stored one back, nothing of the '
        'description becoming the name', (tester) async {
      final id = kindNote(NoteKind.shoppingList);
      final stored = notes.byId(id)!.text;
      await edit(tester, id);
      await tester.enterText(field('note-name'), '');
      await leave(tester);
      expect(notes.byId(id)!.text, stored);

      // With the description changed, the stored name stays above it.
      await edit(tester, id);
      await tester.enterText(field('note-name'), '  ');
      await tap(tester, key('note-description-folded'));
      await tester.enterText(field('note-description'), 'Seats 41 and 42');
      await leave(tester);
      expect(
        notes.byId(id)!.text,
        '${NoteKinds.name(stored)}\nSeats 41 and 42',
      );
    });

    testWidgets('opening and leaving a Note of a kind writes nothing, its '
        'text stored with another line break included', (tester) async {
      final id = notes.capture('Trip\nPassport').id;
      Checklists(store).make(id);
      notes.edit(id, 'Trip\r\nTake the early train');
      final setAt = notes.textSetAt(id);
      now = now.add(const Duration(hours: 1));
      await edit(tester, id);
      expect(typedIn(tester, 'note-name'), 'Trip');
      await tap(tester, key('note-description-folded'));
      expect(typedIn(tester, 'note-description'), 'Take the early train');
      await leave(tester);
      expect(notes.textSetAt(id), setAt);
      expect(notes.byId(id)!.text, 'Trip\r\nTake the early train');
    });

    testWidgets('Return in the name moves what follows into the description, '
        'a line before what it held, and the cursor there', (tester) async {
      final id = kindNote(NoteKind.checklist);
      await edit(tester, id);
      await tester.enterText(field('note-name'), 'Trip\nfor two');
      await tester.pumpAndSettle();
      expect(typedIn(tester, 'note-name'), 'Trip');
      expect(typedIn(tester, 'note-description'), 'for two\n$description');
      expect(widgetOf(tester, 'note-description').focusNode!.hasFocus, isTrue);
      await leave(tester);
      expect(notes.byId(id)!.text, 'Trip\nfor two\n$description');
    });

    testWidgets('an empty description is "Add a description", which opens '
        'its field, the cursor in it; left empty it is the button again', (
      tester,
    ) async {
      final id = notes.capture('Trip\nPassport').id;
      Checklists(store).make(id);
      await edit(tester, id);
      expect(key('note-description'), findsNothing);
      expect(
        find.descendant(
          of: key('note-description-add'),
          matching: find.text('Add a description'),
        ),
        findsOneWidget,
      );
      await tap(tester, key('note-description-add'));
      expect(key('note-description-add'), findsNothing);
      expect(typedIn(tester, 'note-description'), '');
      expect(
        widgetOf(tester, 'note-description').decoration?.hintText,
        'Description',
      );
      expect(widgetOf(tester, 'note-description').focusNode!.hasFocus, isTrue);

      // Left empty: the button again.
      await tap(tester, field('note-name'));
      expect(key('note-description'), findsNothing);
      expect(key('note-description-add'), findsOneWidget);

      await tap(tester, key('note-description-add'));
      await tester.enterText(field('note-description'), 'Bring snacks');
      await leave(tester);
      expect(notes.byId(id)!.text, 'Trip\nBring snacks');
    });

    testWidgets('a Session\'s empty description stays a field with its '
        'hint', (tester) async {
      final id = notes.capture('Morning').id;
      final pool = notes.capture('Stretch').id;
      Tallies(store).addField(pool, name: 'Times', unit: 'times');
      Sessions(store)
          .make(id, pool: [pool], count: 1, rule: SessionRule.rolling);
      await edit(tester, id);
      expect(key('note-description-add'), findsNothing);
      expect(typedIn(tester, 'note-description'), '');
    });

    testWidgets('a text only read shows no "Add a description"', (
      tester,
    ) async {
      final id = notes.capture('Trip\nPassport').id;
      Checklists(store).make(id);
      store.append(id, NoteKeys.text, '{"format":"x-future","text":"Trip"}');
      await edit(tester, id);
      expect(widgetOf(tester, 'note-name').readOnly, isTrue);
      expect(key('note-description-add'), findsNothing);
      expect(key('note-description'), findsNothing);
    });
  });

  testWidgets('a formatted description folds to its first line and '
      'unfolds into its field as stored; leaving writes nothing', (
    tester,
  ) async {
    final id = notes.capture('Trip\nPassport', format: TextFormats.markdown).id;
    Checklists(store).make(id);
    notes.edit(
      id,
      'Trip\n**Early** train\nBook seats',
      format: TextFormats.markdown,
    );
    final setAt = notes.textSetAt(id);
    now = now.add(const Duration(hours: 1));
    await edit(tester, id);
    expect(typedIn(tester, 'note-name'), 'Trip');
    expect(find.textContaining('Book seats'), findsNothing);
    await tap(tester, key('note-description-folded'));
    expect(typedIn(tester, 'note-description'), '**Early** train\nBook seats');
    await leave(tester);
    expect(notes.textSetAt(id), setAt);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a text of a newer format shows folded, a tap unfolds it and '
      'nothing is editable', (tester) async {
    final id = notes.capture('Trip\nPassport').id;
    Checklists(store).make(id);
    store.append(
      id,
      NoteKeys.text,
      '{"format":"x-future","text":"Trip\\nTake the early train\\nBook seats"}',
    );
    final setAt = notes.textSetAt(id);
    await edit(tester, id);
    expect(widgetOf(tester, 'note-name').readOnly, isTrue);
    await tap(tester, key('note-description-folded'));
    expect(widgetOf(tester, 'note-description').readOnly, isTrue);
    expect(typedIn(tester, 'note-description'), description);
    await tap(tester, key('note-description-fold'));
    expect(key('note-description-folded'), findsOneWidget);
    await leave(tester);
    expect(notes.textSetAt(id), setAt);
  });

  group('"Add a Line"', () {
    /// The field of the Line [shown] keyed [prefix], `new-1` for the
    /// first new one.
    Finder lineField(String prefix, String shown) => find.descendant(
      of: key('$prefix-field-$shown'),
      matching: find.byType(TextField),
    );

    for (final prefix in ['checklist', 'shopping']) {
      testWidgets(
        'of a ${prefix == 'checklist' ? 'Checklist' : 'Shopping '
                  'List'} puts the cursor into a new Line after the Lines, once',
        (tester) async {
          final semantics = tester.ensureSemantics();
          final id = notes.capture('Trip\nPassport\nTickets').id;
          if (prefix == 'checklist') {
            Checklists(store).make(id);
          } else {
            ShoppingLists(store).make(id);
          }

          /// The ids and texts of its Lines.
          List<(String, String)> stored() => prefix == 'checklist'
              ? [
                  for (final l in Checklists(store).byNote(id)!.lines)
                    (l.id, l.text),
                ]
              : [
                  for (final l in ShoppingLists(
                    store,
                  ).byNote(id, deviceWallClock)!.lines)
                    (l.id, l.text),
                ];
          final tickets = stored().last.$1;
          await edit(tester, id);
          final add = key('$prefix-add-line');
          expect(
            find.descendant(of: key(prefix), matching: add),
            findsOneWidget,
          );
          final hint = tester
              .widget<TextField>(
                find.descendant(of: add, matching: find.byType(TextField)),
              )
              .decoration
              ?.hintText;
          expect(hint, 'Add a Line');
          expect(
            tester.getSemantics(add),
            containsSemantics(label: 'Add a Line', hasTapAction: true),
          );
          // Under the Lines.
          final last = key('$prefix-field-$tickets');
          expect(
            tester.getTopLeft(add).dy,
            greaterThanOrEqualTo(tester.getBottomLeft(last).dy),
          );
          await tap(tester, add);
          final typed = lineField(prefix, 'new-1');
          expect(typed, findsOneWidget);
          expect(tester.widget<TextField>(typed).focusNode!.hasFocus, isTrue);
          expect(
            tester.getTopLeft(typed).dy,
            greaterThanOrEqualTo(tester.getBottomLeft(last).dy),
          );
          expect(
            tester.getTopLeft(add).dy,
            greaterThanOrEqualTo(tester.getBottomLeft(typed).dy),
          );
          // Again while the new Line is empty: the cursor stays there.
          await tap(tester, add);
          expect(lineField(prefix, 'new-2'), findsNothing);
          expect(tester.widget<TextField>(typed).focusNode!.hasFocus, isTrue);

          await tester.enterText(typed, '${lineStart}Snacks');
          await tester.pumpAndSettle();
          await leave(tester);
          expect(stored().map((l) => l.$2), ['Passport', 'Tickets', 'Snacks']);
          semantics.dispose();
        },
      );
    }

    testWidgets('of a Checklist puts the new Line after the open ones', (
      tester,
    ) async {
      final id = notes.capture('Trip\nPassport\nTickets').id;
      final lists = Checklists(store)..make(id);
      String lineId(String text) =>
          lists.byNote(id)!.lines.firstWhere((l) => l.text == text).id;
      lists.check(lineId('Tickets'));
      final passport = key('checklist-field-${lineId('Passport')}');
      final tickets = key('checklist-field-${lineId('Tickets')}');
      await edit(tester, id);
      await tap(tester, key('checklist-add-line'));
      final typed = lineField('checklist', 'new-1');
      expect(
        tester.getTopLeft(typed).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(passport).dy),
      );
      expect(
        tester.getBottomLeft(typed).dy,
        lessThanOrEqualTo(tester.getTopLeft(tickets).dy),
      );
      await tester.enterText(typed, '${lineStart}Snacks');
      await tester.pumpAndSettle();
      await leave(tester);
      expect(lists.byNote(id)!.lines.map((l) => l.text), [
        'Passport',
        'Snacks',
        'Tickets',
      ]);
    });

    testWidgets('of a Shopping List puts the new Line before the struck '
        'ones', (tester) async {
      final id = notes.capture('Trip\nPassport\nTickets').id;
      final lists = ShoppingLists(store)..make(id);
      final passport = lists.byNote(id, deviceWallClock)!.lines.first.id;
      lists.check(passport, deviceWallClock);
      final tickets = lists
          .byNote(id, deviceWallClock)!
          .lines
          .firstWhere((l) => l.text == 'Tickets')
          .id;
      await edit(tester, id);
      final open = key('shopping-field-$tickets');
      final struck = key('shopping-field-$passport');
      expect(
        tester.getTopLeft(struck).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(open).dy),
      );
      await tap(tester, key('shopping-add-line'));
      final typed = lineField('shopping', 'new-1');
      expect(
        tester.getTopLeft(typed).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(open).dy),
      );
      expect(
        tester.getBottomLeft(typed).dy,
        lessThanOrEqualTo(tester.getTopLeft(struck).dy),
      );
      await tester.enterText(typed, '${lineStart}Snacks');
      await tester.pumpAndSettle();
      await leave(tester);
      expect(
        [
          for (final l in lists.byNote(id, deviceWallClock)!.lines)
            if (!l.struck) l.text,
        ],
        ['Tickets', 'Snacks'],
      );
    });
  });

  group('the full card', () {
    Future<void> openAllNotes(WidgetTester tester) async {
      tester.view
        ..physicalSize = const Size(800, 2400)
        ..devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tap(tester, find.byKey(const ValueKey('browse')));
      await tap(tester, find.byTooltip('All Notes'));
    }

    testWidgets('of a list shows its name as the title and its description '
        'folded under it, a tap unfolding it and nothing else', (tester) async {
      final id = notes.capture('Trip\nPassport').id;
      Checklists(store).make(id);
      notes.edit(id, 'Trip\n$description');
      await openAllNotes(tester);
      final name = key('card-name');
      expect(find.descendant(of: name, matching: find.text('Trip')), findsOne);
      final title = tester.widget<Text>(
        find.descendant(of: name, matching: find.byType(Text)),
      );
      final body = Theme.of(tester.element(name)).textTheme;
      expect(title.style?.fontSize, body.titleMedium?.fontSize);
      final folded = key('card-description');
      expect(
        find.descendant(
          of: folded,
          matching: find.text('Take the early train'),
        ),
        findsOneWidget,
      );
      expect(find.textContaining('Book seats'), findsNothing);
      // The Lines under it.
      expect(
        tester.getTopLeft(find.text('Passport')).dy,
        greaterThan(tester.getBottomLeft(folded).dy),
      );

      await tap(tester, folded);
      // Its tap is its own; the card's long press still opens the editor.
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(
        find.descendant(of: folded, matching: find.text(description)),
        findsOneWidget,
      );
      await tap(tester, folded);
      expect(find.textContaining('Book seats'), findsNothing);
      // Nothing was written to show it.
      expect(notes.byId(id)!.text, 'Trip\n$description');

      await tester.longPress(folded);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('of a formatted list shows its name and description '
        'formatted, folded and unfolded', (tester) async {
      final id = notes
          .capture('Trip\nPassport', format: TextFormats.markdown)
          .id;
      Checklists(store).make(id);
      notes.edit(
        id,
        '# Trip\n**Early** train\nBook seats',
        format: TextFormats.markdown,
      );
      await openAllNotes(tester);
      expect(
        find.descendant(of: key('card-name'), matching: find.text('Trip')),
        findsOneWidget,
      );
      expect(find.textContaining('Book seats'), findsNothing);
      expect(find.textContaining('Early', findRichText: true), findsWidgets);
      await tap(tester, key('card-description'));
      expect(find.textContaining('Book seats'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('of a plain Note is unchanged', (tester) async {
      notes.capture('Call the dentist\nAsk about Tuesday');
      await openAllNotes(tester);
      expect(key('card-name'), findsNothing);
      expect(key('card-description'), findsNothing);
      expect(find.text('Call the dentist\nAsk about Tuesday'), findsOneWidget);
    });

    testWidgets('of a Tally shows its name as the title', (tester) async {
      final id = notes.capture('Reading\nTwenty pages a day').id;
      Tallies(store).addField(id, name: 'Pages', unit: 'pages');
      await openAllNotes(tester);
      expect(
        find.descendant(of: key('card-name'), matching: find.text('Reading')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: key('card-description'),
          matching: find.text('Twenty pages a day'),
        ),
        findsOneWidget,
      );
      // Its content under them on the detailed card (#190).
      expect(
        tester.getTopLeft(key('tally-card')).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(key('card-description')).dy),
      );
    });
  });
}

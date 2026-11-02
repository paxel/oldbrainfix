import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
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
        greaterThan(tester.getBottomLeft(folded).dy),
      );
      await reveal(tester, key('note-badges'));
      expect(
        tester.getTopLeft(key('note-badges')).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(key(content)).dy),
      );
      final section = notes.sections(id).single.id;
      await reveal(tester, key('section-text:$section'));
      expect(
        tester.getTopLeft(key('section-text:$section')).dy,
        greaterThan(tester.getBottomLeft(key('note-badges')).dy),
      );
      await reveal(tester, find.text('Add to Note'));
      expect(
        tester.getTopLeft(find.text('Add to Note')).dy,
        greaterThan(tester.getBottomLeft(key('section-text:$section')).dy),
      );
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

    testWidgets('an empty description is a field with its hint', (
      tester,
    ) async {
      final id = notes.capture('Trip\nPassport').id;
      Checklists(store).make(id);
      await edit(tester, id);
      expect(typedIn(tester, 'note-description'), '');
      expect(
        widgetOf(tester, 'note-description').decoration?.hintText,
        'Description',
      );
      await tap(tester, field('note-description'));
      await tester.enterText(field('note-description'), 'Bring snacks');
      await leave(tester);
      expect(notes.byId(id)!.text, 'Trip\nBring snacks');
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

  group('the full card', () {
    Future<void> openAllNotes(WidgetTester tester) async {
      tester.view
        ..physicalSize = const Size(800, 2400)
        ..devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tap(tester, find.byIcon(Icons.search));
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

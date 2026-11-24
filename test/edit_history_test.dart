import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/edit_history.dart';
import 'package:oldbrainfix/src/line_editing.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Undo, Redo and "Revert all changes" of the Note editor (obf-1051):
/// step by step back to the Note as it was opened, for its text, its
/// Lines and its badges.
void main() {
  late Store store;
  late Notes notes;
  late Places places;

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 24, 8))
      ..starterSetOffered = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor of the Note [id] from the Agenda on a phone 360
  /// wide.
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

  Finder field() => find.byType(TextField).first;

  String typed(WidgetTester tester) =>
      tester.widget<TextField>(field()).controller!.text;

  /// Types [letters] one by one after [start], as a keyboard does.
  Future<String> type(WidgetTester tester, String start, String letters) async {
    var text = start;
    for (final letter in letters.split('')) {
      text += letter;
      await tester.enterText(field(), text);
      await tester.pump();
    }
    return text;
  }

  bool enabled(WidgetTester tester, String name) =>
      tester
          .widget<IconButton>(
            find.descendant(of: key(name), matching: find.byType(IconButton)),
          )
          .onPressed !=
      null;

  Future<void> undo(WidgetTester tester) => tap(tester, key('editor-undo'));
  Future<void> redo(WidgetTester tester) => tap(tester, key('editor-redo'));

  /// Leaves the editor, the text kept.
  Future<void> leave(WidgetTester tester) async {
    await tester.pageBack();
    await tester.pumpAndSettle();
  }

  group('the bar', () {
    for (final (size, fat) in [
      (TextSize.normal, false),
      (TextSize.largest, false),
      (TextSize.normal, true),
    ]) {
      testWidgets('Undo and Redo lead it at 360 wide, text size '
          '${size.name}${fat ? ', Fat fingers' : ''}; Revert is in ⋮', (
        tester,
      ) async {
        store
          ..textSize = size
          ..fatFingers = fat;
        await edit(tester, notes.capture('oil the hinge').id);
        final bar = find.byType(AppBar);
        for (final name in ['editor-undo', 'editor-redo']) {
          expect(
            find.descendant(of: bar, matching: key(name)),
            findsOneWidget,
            reason: name,
          );
          // Nothing changed yet: greyed.
          expect(enabled(tester, name), isFalse, reason: name);
        }
        expect(key('editor-revert'), findsNothing);
        await tap(tester, key('bar-more'));
        expect(key('editor-revert'), findsOneWidget);
        expect(find.text('Revert all changes'), findsOneWidget);
      });
    }
  });

  group('typing', () {
    testWidgets('goes back a word at a time, and Redo makes it again', (
      tester,
    ) async {
      final id = notes.capture('buy').id;
      await edit(tester, id);
      var text = await type(tester, 'buy', ' fresh');
      text = await type(tester, text, ' milk');
      expect(typed(tester), 'buy fresh milk');
      expect(enabled(tester, 'editor-undo'), isTrue);

      await undo(tester);
      expect(typed(tester), 'buy fresh ');
      await undo(tester);
      expect(typed(tester), 'buy ');
      await undo(tester);
      expect(typed(tester), 'buy');
      expect(enabled(tester, 'editor-undo'), isFalse);
      expect(enabled(tester, 'editor-redo'), isTrue);

      await redo(tester);
      await redo(tester);
      expect(typed(tester), 'buy fresh ');
      await leave(tester);
      expect(notes.byId(id)?.text, 'buy fresh');
    });

    testWidgets('what is taken back never reaches the log', (tester) async {
      final id = notes.capture('soup').id;
      await edit(tester, id);
      await type(tester, 'soup', ' xq');
      await undo(tester);
      await undo(tester);
      await leave(tester);
      expect(
        store
            .history(id)
            .where((e) => e.field == NoteKeys.text)
            .map((e) => StoredText.read(e.value).text),
        ['soup'],
      );
    });

    testWidgets('a pause makes a step of its own', (tester) async {
      await edit(tester, notes.capture('call').id);
      var text = await type(tester, 'call', ' An');
      await tester.pump(EditHistory.typingPause);
      text = await type(tester, text, 'na');
      expect(typed(tester), 'call Anna');
      await undo(tester);
      expect(typed(tester), 'call An');
    });

    testWidgets('deleting after typing is a step of its own', (tester) async {
      await edit(tester, notes.capture('tea').id);
      await type(tester, 'tea', 's');
      await tester.enterText(field(), 'te');
      await tester.pump();
      await undo(tester);
      expect(typed(tester), 'teas');
    });

    testWidgets('a text with a box goes back and forth, its box kept', (
      tester,
    ) async {
      final id = notes.capture('list').id;
      await edit(tester, id);
      await tester.enterText(field(), 'list\n☐ eggs');
      await tester.pump(EditHistory.typingPause);
      await tester.enterText(field(), 'list\n☐ eggs\n☐ flour');
      await tester.pump(EditHistory.typingPause);
      await undo(tester);
      await undo(tester);
      expect(typed(tester), 'list');
      await redo(tester);
      await redo(tester);
      await leave(tester);
      final note = notes.byId(id)!;
      expect(note.shownText, 'list\n☐ eggs\n☐ flour');
      expect(note.boxes, hasLength(2));
    });
  });

  group('a table', () {
    late String id;

    setUp(() {
      id = notes
          .capture('Shop\n\n| Item | Count |\n| --- | --- |\n| tea | 1 |')
          .id;
      notes.setFormat(id, TextFormats.markdown);
    });

    Finder cell(int row, int column) => find.descendant(
      of: key('grid-cell-$row-$column'),
      matching: find.byType(TextField),
    );

    testWidgets('a cell typed in goes back, and what is typed next goes '
        'on from there', (tester) async {
      await edit(tester, id);
      await tap(tester, key('grid-cell-1-0'));
      await tester.enterText(cell(1, 0), 'teab');
      await tester.pump();
      expect(typed(tester), contains('| teab |'));

      await undo(tester);
      expect(typed(tester), contains('| tea |'));
      expect(
        find.descendant(of: key('grid-cell-1-0'), matching: find.text('tea')),
        findsOneWidget,
      );
      await tap(tester, key('grid-cell-1-0'));
      await tester.enterText(cell(1, 0), 'teas');
      await tester.pump();
      await leave(tester);
      expect(notes.byId(id)?.text, contains('| teas |'));
    });

    testWidgets('a row added by "+" goes again', (tester) async {
      await edit(tester, id);
      await tap(tester, key('grid-add-row'));
      expect(typed(tester).split('\n'), hasLength(6));

      await undo(tester);
      expect(typed(tester).split('\n'), hasLength(5));
      await leave(tester);
      expect(
        notes.byId(id)?.text,
        'Shop\n\n| Item | Count |\n| --- | --- |\n| tea | 1 |',
      );
    });

    testWidgets('a table moved or deleted from its ⋮ goes back and comes '
        'again, a step each (obf-1050)', (tester) async {
      const table = '| Item | Count |\n| --- | --- |\n| tea | 1 |';
      await edit(tester, id);
      await tap(tester, key('grid-menu'));
      await tap(tester, key('table-menu-up'));
      expect(typed(tester), '$table\n\nShop');
      await tap(tester, key('grid-menu'));
      await tap(tester, key('table-menu-delete'));
      await tap(tester, key('table-delete-confirm'));
      // The table and the line break after it go, as its last row would.
      expect(typed(tester), '\nShop');

      await undo(tester);
      expect(typed(tester), '$table\n\nShop');
      await undo(tester);
      expect(typed(tester), 'Shop\n\n$table');
      await redo(tester);
      expect(typed(tester), '$table\n\nShop');
      await leave(tester);
      expect(notes.byId(id)?.text, '$table\n\nShop');
    });

    testWidgets('rows sorted by a column\'s menu go back, rows already in '
        'that order are no step (obf-1052)', (tester) async {
      const table = '| Item | Count |\n| --- | --- |\n| tea | 1 |\n| oat | 2 |';
      final sorted = notes.capture('Shop\n\n$table').id;
      notes.setFormat(sorted, TextFormats.markdown);
      await edit(tester, sorted);
      await tap(tester, key('grid-column-1'));
      await tap(tester, key('table-sortUp'));
      expect(typed(tester), 'Shop\n\n$table');
      expect(enabled(tester, 'editor-undo'), isFalse);

      await tap(tester, key('grid-column-1'));
      await tap(tester, key('table-sortDown'));
      expect(
        typed(tester),
        'Shop\n\n| Item | Count |\n| --- | --- |\n| oat | 2 |\n| tea | 1 |',
      );
      expect(enabled(tester, 'editor-undo'), isTrue);
      await undo(tester);
      expect(typed(tester), 'Shop\n\n$table');
      expect(enabled(tester, 'editor-undo'), isFalse);
      await leave(tester);
      expect(notes.byId(sorted)?.text, 'Shop\n\n$table');
    });

    testWidgets('a box ticked after it was typed keeps its tick when the '
        'typing after it goes back', (tester) async {
      final plain = notes.capture('list').id;
      await edit(tester, plain);
      await tester.enterText(field(), 'list\n☐ eggs');
      await tester.pump(EditHistory.typingPause);
      await tester.enterText(field(), 'list\n☐ eggs\n☐ milk');
      await tester.pump(EditHistory.typingPause);
      // Gone to the background: the text is stored, its boxes get ids.
      tester.binding
        ..handleAppLifecycleStateChanged(AppLifecycleState.inactive)
        ..handleAppLifecycleStateChanged(AppLifecycleState.hidden)
        ..handleAppLifecycleStateChanged(AppLifecycleState.paused);
      await tester.pump();
      tester.binding
        ..handleAppLifecycleStateChanged(AppLifecycleState.hidden)
        ..handleAppLifecycleStateChanged(AppLifecycleState.inactive)
        ..handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pump();
      final eggs = notes.byId(plain)!.boxes.first.id;
      Boxes(store).tick(plain, eggs, BoxState.done);
      await tester.pump();

      await undo(tester);
      expect(typed(tester), contains('eggs'));
      expect(typed(tester), isNot(contains('milk')));
      await leave(tester);
      final note = notes.byId(plain)!;
      expect(note.boxes, hasLength(1));
      expect(note.boxes.first.id, eggs);
      expect(note.boxes.first.state, BoxState.done);
    });
  });

  group('badges', () {
    testWidgets('the Importance goes back and comes again', (tester) async {
      final id = notes.capture('pay the rent').id;
      await edit(tester, id);
      await tap(tester, key('badge-importance'));
      await tap(tester, key('importance-important'));
      expect(notes.byId(id)?.importance, Importance.important);

      await undo(tester);
      expect(notes.byId(id)?.importance, Importance.normal);
      await redo(tester);
      expect(notes.byId(id)?.importance, Importance.important);
    });

    testWidgets('filed into a Shared Area after Show to the AI, Undo takes '
        'it out and hides it from the AI again', (tester) async {
      final diary = notes.capture('Diary of today').id;
      store.setHiddenFromAi(diary, true);
      final family = places.addArea('Family');
      final shared = store.share(family.id);
      await edit(tester, diary);
      await tap(tester, key('badge-place'));
      await tap(tester, find.text('Family'));
      await tap(tester, key('hidden-from-ai-show'));
      expect(notes.byId(diary)?.place, shared);
      expect(store.isMarkedHiddenFromAi(diary), isFalse);

      await undo(tester);
      expect(notes.byId(diary)?.place, isNull);
      expect(store.isMarkedHiddenFromAi(diary), isTrue);
      await redo(tester);
      expect(notes.byId(diary)?.place, shared);
      expect(store.isMarkedHiddenFromAi(diary), isFalse);
    });

    testWidgets('a Plain Note made a Checklist is plain again, its text as '
        'it was', (tester) async {
      final id = notes.capture('Trip\ntickets\npassport').id;
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      await tap(tester, key('kind-checklist'));
      expect(Checklists(store).isChecklist(id), isTrue);
      List<String> lines() =>
          Checklists(store).byNote(id)!.lines.map((l) => l.text).toList();
      final made = lines();

      await undo(tester);
      expect(Checklists(store).isChecklist(id), isFalse);
      expect(notes.byId(id)?.text, 'Trip\ntickets\npassport');
      expect(typed(tester), 'Trip\ntickets\npassport');

      await redo(tester);
      // Made again as it was made.
      expect(lines(), made);
    });
  });

  group('Lines', () {
    late String id;
    late Checklists lists;

    setUp(() {
      id = notes.capture('Packing\nsocks\nshirts').id;
      lists = Checklists(store)..make(id);
    });

    String lineId(String text) =>
        lists.byNote(id)!.lines.firstWhere((l) => l.text == text).id;

    testWidgets('a Line checked is unchecked again', (tester) async {
      final socks = lineId('socks');
      await edit(tester, id);
      await tap(tester, key('checklist-check-$socks'));
      expect(lists.byNote(id)?.line(socks)?.checked, isTrue);

      await undo(tester);
      expect(lists.byNote(id)?.line(socks)?.checked, isFalse);
      await redo(tester);
      expect(lists.byNote(id)?.line(socks)?.checked, isTrue);
    });

    testWidgets('a Line deleted comes back, and one moved goes back', (
      tester,
    ) async {
      final socks = lineId('socks');
      await edit(tester, id);
      await tap(tester, key('checklist-menu-$socks'));
      await tap(tester, key('checklist-down'));
      expect(lists.byNote(id)?.lines.map((l) => l.text), ['shirts', 'socks']);
      await tap(tester, key('checklist-menu-$socks'));
      await tap(tester, key('checklist-delete'));
      expect(lists.byNote(id)?.lines.map((l) => l.text), ['shirts']);

      await undo(tester);
      expect(lists.byNote(id)?.lines.map((l) => l.text), ['shirts', 'socks']);
      await undo(tester);
      expect(lists.byNote(id)?.lines.map((l) => l.text), ['socks', 'shirts']);
      expect(enabled(tester, 'editor-undo'), isFalse);
    });

    testWidgets('"Add a Line" is no step; the Line typed there goes again '
        '(obf-1071)', (tester) async {
      await edit(tester, id);
      await tap(tester, key('checklist-add-line'));
      expect(enabled(tester, 'editor-undo'), isFalse);
      final typed = find.descendant(
        of: key('checklist-field-new-1'),
        matching: find.byType(TextField),
      );
      await tester.enterText(typed, '${lineStart}hats');
      await tester.pumpAndSettle();

      await undo(tester);
      expect(lists.byNote(id)?.lines.map((l) => l.text), ['socks', 'shirts']);
      await redo(tester);
      expect(lists.byNote(id)?.lines.map((l) => l.text), [
        'socks',
        'shirts',
        'hats',
      ]);
    });
  });

  testWidgets('a struck Line of a Shopping List typed again is struck again '
      'by Undo', (tester) async {
    final id = notes.capture('Shop\nMilk\nEggs').id;
    final shop = ShoppingLists(store)..make(id);
    String lineOf(String text) => shop
        .byNote(id, deviceWallClock)!
        .lines
        .firstWhere((l) => l.text == text)
        .id;
    final milk = lineOf('Milk');
    shop.check(milk, deviceWallClock);
    Finder lineField(String shown) => find.descendant(
      of: key('shopping-field-$shown'),
      matching: find.byType(TextField),
    );
    await edit(tester, id);
    await tester.enterText(lineField(lineOf('Eggs')), '${lineStart}Eggs\n');
    await tester.pumpAndSettle();
    await tester.enterText(lineField('new-1'), '${lineStart}milk');
    await tester.pump();
    // Typed, not stored yet: Undo stores it and takes it back.
    expect(enabled(tester, 'editor-undo'), isTrue);

    await undo(tester);
    expect(shop.byNote(id, deviceWallClock)?.line(milk)?.struck, isTrue);
    expect(enabled(tester, 'editor-redo'), isTrue);
    expect(enabled(tester, 'editor-undo'), isFalse);
    await redo(tester);
    expect(shop.byNote(id, deviceWallClock)?.line(milk)?.struck, isFalse);
  });

  testWidgets('a text History brought back goes again', (tester) async {
    final id = notes.capture('Dentist on Tuesday').id;
    notes.edit(id, 'Dentist on Wednesday');
    await edit(tester, id);
    await tap(tester, key('bar-more'));
    await tap(tester, key('note-history'));
    // The capture, last: its text is the one before.
    final use = find.byKey(const ValueKey('use-version-1'));
    await tester.ensureVisible(use);
    await tap(tester, use);
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(notes.byId(id)?.text, 'Dentist on Tuesday');

    await undo(tester);
    expect(notes.byId(id)?.text, 'Dentist on Wednesday');
    expect(typed(tester), 'Dentist on Wednesday');
  });

  testWidgets('Revert all changes asks once and goes back to what was '
      'opened; Redo brings it back step by step', (tester) async {
    final id = notes.capture('fix the gate').id;
    await edit(tester, id);
    await type(tester, 'fix the gate', ' hinge');
    await tap(tester, key('badge-importance'));
    await tap(tester, key('importance-important'));

    await tap(tester, key('bar-more'));
    await tap(tester, key('editor-revert'));
    expect(find.text('Revert all changes?'), findsOneWidget);
    // Cancel changes nothing.
    await tap(tester, find.text('Cancel'));
    expect(notes.byId(id)?.importance, Importance.important);

    await tap(tester, key('bar-more'));
    await tap(tester, key('editor-revert'));
    await tap(tester, key('revert-all-confirm'));
    expect(typed(tester), 'fix the gate');
    expect(notes.byId(id)?.importance, Importance.normal);
    expect(enabled(tester, 'editor-undo'), isFalse);

    await redo(tester);
    expect(typed(tester), 'fix the gate ');
    await leave(tester);
    expect(notes.byId(id)?.text, 'fix the gate');
    expect(notes.byId(id)?.importance, Importance.normal);
  });

  testWidgets('a whole-text step stays apart from the typing around it', (
    tester,
  ) async {
    final history = EditHistory();
    addTearDown(history.dispose);
    final text = TextEditingController(text: 'a');
    addTearDown(text.dispose);
    late BuildContext inside;
    await tester.pumpWidget(
      EditHistoryScope(
        history: history,
        child: Builder(
          builder: (context) {
            inside = context;
            return const SizedBox();
          },
        ),
      ),
    );
    void put(String value) {
      final before = text.text;
      text.text = value;
      history.typed(
        undo: () => text.text = before,
        redo: () => text.text = value,
        change: TypedChange.insert,
      );
    }

    put('ab');
    EditHistory.wholeStep(inside, () => put('ab|sorted'));
    put('ab|sortedc');
    history.undo();
    expect(text.text, 'ab|sorted');
    history.undo();
    expect(text.text, 'ab');
    history.undo();
    expect(text.text, 'a');
    expect(history.canUndo, isFalse);
  });
}

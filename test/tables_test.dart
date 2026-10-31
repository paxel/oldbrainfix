import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/table_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Tables in formatted text (#128, ADR 0033): drawn smaller to fit in
/// the text, opened full screen with the header row, and the first
/// column when it holds headings, kept on screen; their cells edited in
/// place, their rows and columns changed from their headings' menus; and
/// a new one put in from the bar of formats.
void main() {
  Finder key(String value) => find.byKey(ValueKey(value));

  const shop =
      '# Shop\n\n| Item | Qty |\n| --- | --: |\n| ☐ **milk** #dairy | 2 |\n'
      '| oat | 1 |';

  /// A table wider than any phone, with [rows] rows under its header.
  String wide({int rows = 3, bool headings = false}) {
    String row(String name) =>
        '| $name | ${[for (var c = 1; c < 8; c++) 'column$c words'].join(' | ')} |';
    return [
      if (headings) MarkdownTables.headingColumnLine,
      row('Name'),
      '| ${List.filled(8, '---').join(' | ')} |',
      for (var r = 1; r <= rows; r++) row('row$r'),
    ].join('\n');
  }

  Widget app(Widget home) => MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: home,
  );

  group('in the text', () {
    Future<void> pump(
      WidgetTester tester,
      String text, {
      List<ShownBox>? boxes,
      BoxTicking? ticking,
      int? start,
      int? end,
      int? maxLines,
    }) async {
      await tester.pumpWidget(
        app(
          Scaffold(
            body: Center(
              child: SizedBox(
                width: 300,
                child: MarkedText(
                  text,
                  key: const ValueKey('text'),
                  format: TextFormats.markdown,
                  start: start,
                  end: end,
                  maxLines: maxLines,
                  style: const TextStyle(fontSize: 16),
                  boxes: boxes,
                  ticking: ticking,
                ),
              ),
            ),
          ),
        ),
      );
    }

    testWidgets('is a grid made smaller to fit, its header row bold', (
      tester,
    ) async {
      await pump(tester, '${wide()}\nafter');
      final table = find.byType(Table);
      expect(table, findsOneWidget);
      expect(tester.widget<Table>(table).border, isNotNull);
      // Drawn smaller: its own width is more than the room it shows in.
      expect(tester.getSize(key('table-0')).width, lessThanOrEqualTo(300));
      expect(
        tester.renderObject<RenderBox>(table).size.width,
        greaterThan(300),
      );
      final scaled = tester.getRect(table);
      expect(scaled.width, lessThanOrEqualTo(300));

      final name = tester
          .widgetList<RichText>(
            find.descendant(of: table, matching: find.byType(RichText)),
          )
          .first;
      expect(name.text.toPlainText(), 'Name');
      var bold = false;
      name.text.visitChildren((span) {
        bold = bold || span.style?.fontWeight == FontWeight.bold;
        return true;
      });
      expect(bold, isTrue);
      // The pipes and the delimiter row show nowhere.
      expect(find.textContaining('|'), findsNothing);
      expect(find.textContaining('---'), findsNothing);
      expect(find.text('after'), findsOneWidget);
    });

    testWidgets('a cell\'s box is a button that ticks it beside the text', (
      tester,
    ) async {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final id = notes.capture(shop, format: TextFormats.markdown).id;
      final note = notes.byId(id)!;
      final box = note.boxes.single;
      var ticked = 0;
      await pump(
        tester,
        note.shownText,
        boxes: note.boxes,
        ticking: BoxTicking(
          notes: notes,
          entity: id,
          ticks: const {},
          mayTick: true,
          changed: () => ticked++,
        ),
      );
      // Bold and a tag inside a cell.
      final spans = <TextSpan>[];
      for (final paragraph in tester.widgetList<RichText>(
        find.descendant(
          of: find.byType(Table),
          matching: find.byType(RichText),
        ),
      )) {
        paragraph.text.visitChildren((span) {
          if (span is TextSpan) spans.add(span);
          return true;
        });
      }
      expect(
        spans.firstWhere((s) => s.text == 'milk').style?.fontWeight,
        FontWeight.bold,
      );
      expect(spans.any((s) => s.text == '#dairy'), isTrue);

      await tester.tap(key('box-${box.id}'));
      await tester.pumpAndSettle();
      await tester.tap(key('box-done'));
      await tester.pumpAndSettle();
      expect(ticked, 1);
      expect(notes.byId(id)!.text, note.text);
      expect(notes.byId(id)!.boxes.single.state, BoxState.done);
      expect(notes.byId(id)!.shownText, contains('| ✅ **milk**'));
    });

    testWidgets('a compact line shows the words of the header row', (
      tester,
    ) async {
      final text = wide(headings: true);
      final (start, end) = NoteCard.firstLine(text);
      await pump(tester, text, start: start, end: end, maxLines: 1);
      expect(find.byType(Table), findsNothing);
      final shown = tester
          .widget<RichText>(
            find.descendant(of: key('text'), matching: find.byType(RichText)),
          )
          .text
          .toPlainText();
      expect(shown, startsWith('Name column1 words'));
    });
  });

  group('full screen', () {
    /// Opens the table of [text] full screen from a button, as a tap on
    /// it does; what it writes is in the list given back.
    Future<List<MarkdownChange>> open(
      WidgetTester tester,
      String text, {
      bool edits = false,
      BoxTicking? ticking,
      ({int row, int column})? cell,
    }) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final written = <MarkdownChange>[];
      await tester.pumpWidget(
        app(
          Builder(
            builder: (context) => Scaffold(
              body: Center(
                child: TextButton(
                  onPressed: () => openTable(
                    context,
                    text: text,
                    at: MarkdownTables.all(text).first.start,
                    cell: cell,
                    ticking: ticking,
                    edit: edits
                        ? TableEditing(
                            write: (change) {
                              written.add(change);
                              return null;
                            },
                            twoRows: true,
                          )
                        : null,
                  ),
                  child: const Text('open'),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
      return written;
    }

    testWidgets('opens with a tap on the table in the text', (tester) async {
      await tester.pumpWidget(
        app(
          const Scaffold(body: MarkedText(shop, format: TextFormats.markdown)),
        ),
      );
      await tester.tap(key('table-2'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
      // Only read: no headings to change it by.
      expect(key('row-1'), findsNothing);
      expect(find.text('oat'), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsNothing);
    });

    testWidgets('keeps the header row while the rows scroll', (tester) async {
      await open(tester, wide(rows: 40));
      final header = tester.getTopLeft(key('cell-0-0'));
      final row = tester.getTopLeft(key('cell-10-0'));
      await tester.drag(key('table-rows'), const Offset(0, -300));
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(key('cell-0-0')), header);
      expect(tester.getTopLeft(key('cell-10-0')).dy, lessThan(row.dy));
    });

    testWidgets('a drag on the cells moves the table under the finger, the '
        'header row with it and the first column staying when it holds '
        'headings', (tester) async {
      await open(tester, wide(headings: true));
      final first = tester.getTopLeft(key('cell-1-0'));
      final third = tester.getTopLeft(key('cell-1-2'));
      final header = tester.getTopLeft(key('cell-0-2'));

      await tester.drag(key('cell-1-1'), const Offset(-100, 0));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget, reason: 'it stays');
      expect(tester.getTopLeft(key('cell-1-0')), first);
      final dragged = tester.getTopLeft(key('cell-1-2'));
      expect(dragged.dx, lessThan(third.dx));
      expect(
        tester.getTopLeft(key('cell-0-2')).dx - header.dx,
        dragged.dx - third.dx,
      );
      // Back the other way, no further than the start.
      await tester.drag(key('cell-0-2'), const Offset(400, 0));
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(key('cell-1-2')), third);
    });

    testWidgets('moves sideways by its bar, the first column staying when '
        'it holds headings', (tester) async {
      await open(tester, wide(headings: true));
      final first = tester.getTopLeft(key('cell-1-0'));
      final third = tester.getTopLeft(key('cell-1-2'));
      final header = tester.getTopLeft(key('cell-0-2'));

      await tester.drag(key('table-sideways-thumb'), const Offset(100, 0));
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(key('cell-1-0')), first);
      final moved = tester.getTopLeft(key('cell-1-2'));
      expect(moved.dx, lessThan(third.dx));
      // The header row moves with it.
      expect(
        tester.getTopLeft(key('cell-0-2')).dx - header.dx,
        moved.dx - third.dx,
      );
    });

    testWidgets('without headings the first column moves too', (tester) async {
      await open(tester, wide());
      final first = tester.getTopLeft(key('cell-1-0'));
      await tester.drag(key('table-sideways-thumb'), const Offset(100, 0));
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(key('cell-1-0')).dx, lessThan(first.dx));
    });

    testWidgets('a tapped cell is edited in place and written when another '
        'is tapped', (tester) async {
      final written = await open(tester, shop, edits: true);
      await tester.tap(key('cell-2-1'));
      await tester.pumpAndSettle();
      final field = find.descendant(
        of: key('cell-2-1'),
        matching: find.byType(TwoRowTextField),
      );
      expect(field, findsOneWidget);
      expect(tester.widget<TwoRowTextField>(field).controller.text, '1');
      await tester.enterText(find.byType(TextField), 'a | b');
      await tester.tap(key('cell-1-1'));
      await tester.pumpAndSettle();
      final text = written.single.text;
      expect(text, startsWith('# Shop\n\n| Item | Qty |\n| --- | --: |\n'));
      expect(MarkdownTables.cell(text, written.single.start, 2, 1), 'a | b');
      expect(text, contains(r'| oat | a \| b |'));
      // The other one is edited now; Back writes it.
      await tester.enterText(find.byType(TextField), '3');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(
        MarkdownTables.cell(written.last.text, written.last.start, 1, 1),
        '3',
      );
    });

    testWidgets('the empty row and column add one only with words', (
      tester,
    ) async {
      final written = await open(tester, shop, edits: true);
      await tester.tap(key('cell-3-0'));
      await tester.pumpAndSettle();
      await tester.tap(key('cell-1-0'));
      await tester.pumpAndSettle();
      expect(written, isEmpty);

      await tester.tap(key('cell-3-0'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'tea\n');
      await tester.pumpAndSettle();
      var table = MarkdownTables.at(written.last.text, written.last.start)!;
      expect(table.rows.length, 4);
      expect(table.rows[3].visible, 'tea');

      await tester.tap(key('cell-0-2'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'Shop');
      await tester.tap(key('cell-1-0'));
      await tester.pumpAndSettle();
      table = MarkdownTables.at(written.last.text, written.last.start)!;
      expect(table.columns, 3);
      expect(table.rows[0].visible, 'Item Qty Shop');
    });

    testWidgets('a row\'s and a column\'s heading open their menu', (
      tester,
    ) async {
      final written = await open(tester, shop, edits: true);
      MarkdownTable last() =>
          MarkdownTables.at(written.last.text, written.last.start)!;

      await tester.longPress(key('row-2'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-before'));
      await tester.pumpAndSettle();
      expect(
        [for (final r in last().rows) r.visible],
        ['Item Qty', '☐ milk #dairy 2', '', 'oat 1'],
      );

      await tester.longPress(key('row-3'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-back'));
      await tester.pumpAndSettle();
      expect(last().rows[2].visible, 'oat 1');

      // An empty row goes without asking.
      await tester.longPress(key('row-3'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-delete'));
      await tester.pumpAndSettle();
      expect(last().rows.length, 3);

      await tester.longPress(key('column-1'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-back'));
      await tester.pumpAndSettle();
      expect(last().rows[0].visible, 'Qty Item');
      expect(last().aligns.first, MarkdownAlign.right);

      await tester.longPress(key('column-0'));
      await tester.pumpAndSettle();
      expect(key('table-headings'), findsOneWidget);
      await tester.tap(key('table-headings'));
      await tester.pumpAndSettle();
      expect(last().headingColumn, isTrue);
      expect(written.last.text, contains(MarkdownTables.headingColumnLine));

      await tester.longPress(key('column-1'));
      await tester.pumpAndSettle();
      expect(key('table-headings'), findsNothing);
      await tester.tap(key('table-after'));
      await tester.pumpAndSettle();
      expect(last().columns, 3);

      // A column with words asks first.
      await tester.longPress(key('column-0'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-delete'));
      await tester.pumpAndSettle();
      expect(find.text('Delete this column?'), findsOneWidget);
      await tester.tap(key('table-delete-confirm'));
      await tester.pumpAndSettle();
      expect(last().rows[0].visible, 'Item');
    });

    testWidgets('deleting the last column deletes the table and closes', (
      tester,
    ) async {
      const text = 'before\n\n| A |\n| --- |\n| b |\n\nafter';
      final written = await open(tester, text, edits: true);
      await tester.longPress(key('column-0'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-delete'));
      await tester.pumpAndSettle();
      expect(find.text('Delete the table?'), findsOneWidget);
      await tester.tap(key('table-delete-confirm'));
      await tester.pumpAndSettle();
      expect(
        written.single.text,
        MarkdownTables.deleteColumn(text, text.indexOf('|'), 0).text,
      );
      expect(written.single.text, isNot(contains('|')));
      expect(find.byType(TableScreen), findsNothing);
    });

    testWidgets('a cell\'s box is ticked in place, the text unchanged', (
      tester,
    ) async {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final id = notes.capture(shop, format: TextFormats.markdown).id;
      final stored = notes.byId(id)!.text;
      final box = notes.byId(id)!.boxes.single;
      final written = await open(
        tester,
        stored,
        edits: true,
        ticking: BoxTicking(
          notes: notes,
          entity: id,
          ticks: const {},
          mayTick: true,
        ),
      );
      expect(find.text('☐'), findsOneWidget);
      await tester.tap(key('box-${box.id}'));
      await tester.pumpAndSettle();
      await tester.tap(key('box-failed'));
      await tester.pumpAndSettle();
      expect(written, isEmpty);
      expect(notes.byId(id)!.boxes.single.state, BoxState.failed);
      // Drawn anew in the state it was ticked to, its id never seen.
      expect(find.text('❌'), findsOneWidget);
      expect(find.text('☐'), findsNothing);
    });
  });

  group('the bar of formats', () {
    testWidgets('puts a new table at the cursor and edits its first cell', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1000, 1600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final controller = TagTextController(text: 'Shop', hideMarks: true);
      addTearDown(controller.dispose);
      var formatted = 0;
      await tester.pumpWidget(
        app(
          Scaffold(
            body: ListView(
              children: [
                TwoRowTextField(
                  controller: controller,
                  twoRows: true,
                  boxes: true,
                  formats: true,
                  onFormatted: () => formatted++,
                ),
              ],
            ),
          ),
        ),
      );
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 600);
      addTearDown(tester.view.resetViewInsets);
      controller.selection = const TextSelection.collapsed(offset: 4);
      await tester.pumpAndSettle();
      await tester.tap(key('format-table'));
      await tester.pumpAndSettle();
      expect(formatted, 1);
      expect(controller.markdown, isTrue);
      expect(controller.text, 'Shop\n\n|  |  |\n| --- | --- |\n|  |  |');
      expect(find.byType(TableScreen), findsOneWidget);
      expect(
        find.descendant(
          of: key('cell-0-0'),
          matching: find.byType(TwoRowTextField),
        ),
        findsOneWidget,
      );
      await tester.enterText(find.byType(TextField).last, 'Item');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(controller.text, 'Shop\n\n| Item |  |\n| --- | --- |\n|  |  |');
      // In the field its pipes and delimiter row are marks, drawn as
      // nothing in two rows.
      final spans = <TextSpan>[];
      tester
          .state<EditableTextState>(find.byType(EditableText).first)
          .buildTextSpan()
          .visitChildren((span) {
            if (span is TextSpan) spans.add(span);
            return true;
          });
      expect(
        spans.firstWhere((s) => s.text == '| --- | --- |').style?.fontSize,
        0.01,
      );

      // In a table the button opens it.
      controller.selection = const TextSelection.collapsed(offset: 8);
      await tester.pumpAndSettle();
      await tester.tap(key('format-table'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
      expect(find.text('Item'), findsOneWidget);

      // So does a tap on it in the field; the cursor stays.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsNothing);
      final editable = tester
          .state<EditableTextState>(find.byType(EditableText))
          .renderEditable;
      final word = editable
          .getBoxesForSelection(
            const TextSelection(baseOffset: 8, extentOffset: 12),
          )
          .first
          .toRect();
      final cursor = controller.selection;
      await tester.tapAt(editable.localToGlobal(word.center));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
      expect(controller.selection, cursor);
    });
  });

  group('on a card', () {
    late Store store;
    late Notes notes;

    setUpAll(initializeDateFormatting);

    setUp(() {
      store = Store.inMemory(clock: () => DateTime.utc(2026, 10, 30, 8))
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      notes = Notes(store);
    });
    tearDown(() => store.close());

    testWidgets('a table opens full screen and its cell is written as an '
        'edit of the Note', (tester) async {
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final id = notes.capture(shop, format: TextFormats.markdown).id;
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      await tester.tap(key('table-2'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
      await tester.tap(key('cell-2-0'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).last, 'oats');
      await tester.pageBack();
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.text, contains('| oats | 1 |'));
      expect(note.textFormat, TextFormats.markdown);
      expect(find.text('oats'), findsOneWidget);
    });

    /// Opens the Note editor on [card] with the keyboard up, the bar of
    /// formats shown in full.
    Future<TagTextController> openEditor(
      WidgetTester tester,
      String card,
    ) async {
      tester.view.physicalSize = const Size(1000, 2000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      await tester.longPress(find.textContaining(card).first);
      await tester.pumpAndSettle();
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 600);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
      return tester.widget<TextField>(find.byType(TextField)).controller!
          as TagTextController;
    }

    /// The table button with the cursor at [cursor]: the table it opens
    /// is the one in the text there, [cell] being edited.
    Future<void> tableButton(
      WidgetTester tester,
      TagTextController field,
      int cursor, {
      bool editing = true,
    }) async {
      field.selection = TextSelection.collapsed(offset: cursor);
      await tester.pumpAndSettle();
      await tester.tap(key('format-table'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
      expect(key('cell-0-0'), findsOneWidget);
      expect(
        find.descendant(
          of: key('cell-0-0'),
          matching: find.byType(TwoRowTextField),
        ),
        editing ? findsOneWidget : findsNothing,
      );
    }

    testWidgets('the table button opens the table it put in, also where '
        'keeping the first format gives a new box its id (#154)', (
      tester,
    ) async {
      final id = notes.capture('Milk').id;
      final field = await openEditor(tester, 'Milk');
      field.selection = const TextSelection.collapsed(offset: 4);
      await tester.pumpAndSettle();
      await tester.tap(key('two-row-insert-box'));
      await tester.pumpAndSettle();
      expect(field.text, 'Milk☐');

      await tableButton(tester, field, field.text.length);
      await tester.enterText(find.byType(TextField).last, 'Item');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsNothing);
      expect(field.markdown, isTrue);
      expect(field.text, contains('| Item |  |'));

      // In that table the button opens it, the words there.
      await tableButton(
        tester,
        field,
        field.text.indexOf('Item'),
        editing: false,
      );
      expect(find.text('Item'), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();

      await tester.pageBack();
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.textFormat, TextFormats.markdown);
      expect(note.boxes, hasLength(1));
      expect(note.shownText, contains('Milk☐'));
      expect(note.shownText, contains('| Item |  |'));
    });

    testWidgets('in a formatted Note the table button opens the table it '
        'put in after a new box, and a second one beside the first', (
      tester,
    ) async {
      final id = notes
          .capture('$shop\n\nBread', format: TextFormats.markdown)
          .id;
      final field = await openEditor(tester, 'Bread');
      field.selection = TextSelection.collapsed(offset: field.text.length);
      await tester.pumpAndSettle();
      await tester.tap(key('two-row-insert-box'));
      await tester.pumpAndSettle();

      await tableButton(tester, field, field.text.length);
      await tester.enterText(find.byType(TextField).last, 'Second');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(field.text, contains('| Second |  |'));

      // The first table opens on itself.
      await tableButton(
        tester,
        field,
        field.text.indexOf('Item'),
        editing: false,
      );
      expect(find.text('Qty'), findsOneWidget);
      expect(find.text('Second'), findsNothing);
      await tester.pageBack();
      await tester.pumpAndSettle();

      await tester.pageBack();
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.boxes, hasLength(2));
      expect(note.shownText, contains('| Second |  |'));
    });

    testWidgets('a box ticked on the card keeps its tick when a cell is '
        'written; one typed into a cell becomes a box', (tester) async {
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final id = notes.capture(shop, format: TextFormats.markdown).id;
      final box = notes.byId(id)!.boxes.single;
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      await tester.tap(key('box-${box.id}'));
      await tester.pumpAndSettle();
      await tester.tap(key('box-done'));
      await tester.pumpAndSettle();

      await tester.tap(key('table-2'));
      await tester.pumpAndSettle();
      expect(find.text('✅'), findsOneWidget);
      await tester.tap(key('cell-2-0'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).last, '✅ oats');
      await tester.tap(key('cell-2-1'));
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.boxes.map((b) => b.state), [BoxState.done, BoxState.done]);
      expect(note.boxes.first.id, box.id);
      expect(note.shownText, contains('| ✅ **milk** #dairy | 2 |'));
      expect(note.shownText, contains('| ✅ oats | 1 |'));
    });
  });
}

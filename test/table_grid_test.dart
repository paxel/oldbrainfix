import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/table_grid.dart';
import 'package:oldbrainfix/src/table_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A table of a formatted text edited as a grid in place of its lines in
/// the editor's field (#257): its cells typed into there, its rows and
/// columns added, moved, deleted and lined up, the cursor going into it
/// and out of it with the keys and ◀ ▶, and its lines shown on request.
void main() {
  Finder key(String value) => find.byKey(ValueKey(value));

  const shop =
      'Shop\n\n| Item | Qty |\n| --- | --: |\n| milk | 2 |\n| oat | 1 |'
      '\n\nBread';

  Widget app(Widget home) => MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: home,
  );

  /// A field holding [text], formatted unless [plain], in two rows or
  /// plain, with the bar of formats when [bar]; the keyboard up.
  Future<TagTextController> pumpField(
    WidgetTester tester,
    String text, {
    bool twoRows = true,
    bool bar = true,
    bool plain = false,
    double width = 400,
  }) async {
    tester.view.physicalSize = const Size(1000, 1600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final controller = TagTextController(text: text, hideMarks: twoRows)
      ..markdown = !plain;
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      app(
        Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: width,
              child: ListView(
                shrinkWrap: true,
                children: [
                  TwoRowTextField(
                    controller: controller,
                    twoRows: twoRows,
                    boxes: true,
                    formats: true,
                    bar: bar,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(TextField));
    tester.view.viewInsets = const FakeViewPadding(bottom: 600);
    addTearDown(tester.view.resetViewInsets);
    await tester.pumpAndSettle();
    return controller;
  }

  MarkdownTable tableOf(TagTextController controller) =>
      MarkdownTables.all(controller.text).single;

  /// The field of the cell being edited in [row] and [column].
  Finder cellField(int row, int column) => find.descendant(
    of: key('grid-cell-$row-$column'),
    matching: find.byType(TextField),
  );

  /// The controller of the cell being edited.
  TextEditingController editing(WidgetTester tester) => tester
      .widget<TextField>(
        find.descendant(
          of: find.byType(TableGrid),
          matching: find.byType(TextField),
        ),
      )
      .controller!;

  /// The spans the field draws visibly.
  String drawn(WidgetTester tester) {
    final out = StringBuffer();
    tester
        .state<EditableTextState>(find.byType(EditableText).first)
        .buildTextSpan()
        .visitChildren((span) {
          if (span is TextSpan &&
              span.text != null &&
              span.style?.fontSize != 0.01) {
            out.write(span.text);
          }
          return true;
        });
    return out.toString();
  }

  Future<void> tapCell(WidgetTester tester, int row, int column) async {
    await tester.tap(key('grid-cell-$row-$column'));
    await tester.pumpAndSettle();
  }

  /// Types [text] into the field with the keyboard's focus, as the
  /// keyboard does, the cursor after it.
  Future<void> type(WidgetTester tester, String text) async {
    tester.testTextInput.updateEditingValue(
      TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: text.length),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> key_(WidgetTester tester, LogicalKeyboardKey k) async {
    await tester.sendKeyEvent(k);
    await tester.pumpAndSettle();
  }

  for (final twoRows in [true, false]) {
    final mode = twoRows ? 'two rows' : 'plain';

    testWidgets('a pasted table shows as a grid, not as its lines ($mode)', (
      tester,
    ) async {
      final controller = await pumpField(
        tester,
        'Shop',
        twoRows: twoRows,
        plain: true,
      );
      expect(controller.markdown, isFalse);
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: shop,
          selection: TextSelection.collapsed(offset: shop.length),
        ),
      );
      await tester.pumpAndSettle();
      expect(controller.markdown, isTrue);
      expect(find.byType(TableGrid), findsOneWidget);
      expect(find.byType(TableScreen), findsNothing);
      final shown = drawn(tester);
      expect(shown, contains('Shop'));
      expect(shown, contains('Bread'));
      expect(shown, isNot(contains('|')));
      expect(
        find.descendant(
          of: find.byType(TableGrid),
          matching: find.text('milk'),
        ),
        findsOneWidget,
      );
    });

    testWidgets('a tapped cell is typed into in place, each letter written '
        'into the table\'s line ($mode)', (tester) async {
      final controller = await pumpField(tester, shop, twoRows: twoRows);
      await tapCell(tester, 2, 0);
      expect(find.byType(TableScreen), findsNothing);
      expect(cellField(2, 0), findsOneWidget);
      expect(editing(tester).text, 'oat');
      // The editor's field gave the keyboard up to the cell's.
      expect(
        tester
            .widget<TextField>(find.byType(TextField).first)
            .focusNode
            ?.hasFocus,
        isFalse,
      );
      expect(
        tester.widget<TextField>(cellField(2, 0)).focusNode?.hasPrimaryFocus,
        isTrue,
      );
      await type(tester, 'oat');
      // Spaces typed at the end stay in the cell while it is edited,
      // its line written without them.
      await type(tester, 'oats ');
      expect(editing(tester).text, 'oats ');
      expect(controller.text, contains('| oats | 1 |'));
      await type(tester, 'oats m|n');
      expect(controller.text, contains(r'| oats m\|n | 1 |'));
      expect(controller.text, startsWith('Shop\n\n| Item | Qty |'));
      expect(controller.text, endsWith('\n\nBread'));
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('a hardware key in a cell edits the cell, not the field', (
    tester,
  ) async {
    final controller = await pumpField(tester, shop, twoRows: false);
    await tapCell(tester, 2, 0);
    await key_(tester, LogicalKeyboardKey.backspace);
    expect(editing(tester).text, 'oa');
    expect(controller.text, contains('| oa | 1 |'));
    expect(controller.text, endsWith('\n\nBread'));
  });

  testWidgets('Tab, Shift+Tab, Enter and the arrows go from cell to cell, '
      'and out of the grid into the text past its ends', (tester) async {
    final controller = await pumpField(tester, shop, twoRows: false);
    final table = tableOf(controller);
    // From before the table → goes into its first cell, at its start.
    controller.selection = TextSelection.collapsed(offset: table.start);
    await tester.pumpAndSettle();
    await key_(tester, LogicalKeyboardKey.arrowRight);
    expect(cellField(0, 0), findsOneWidget);
    expect(editing(tester).selection, const TextSelection.collapsed(offset: 0));

    await key_(tester, LogicalKeyboardKey.tab);
    expect(cellField(0, 1), findsOneWidget);
    await key_(tester, LogicalKeyboardKey.tab);
    expect(cellField(1, 0), findsOneWidget, reason: 'row by row');
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await key_(tester, LogicalKeyboardKey.tab);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(cellField(0, 1), findsOneWidget);
    await key_(tester, LogicalKeyboardKey.enter);
    expect(cellField(1, 1), findsOneWidget, reason: 'Enter: the cell below');
    await key_(tester, LogicalKeyboardKey.arrowDown);
    expect(cellField(2, 1), findsOneWidget);
    await key_(tester, LogicalKeyboardKey.arrowUp);
    expect(cellField(1, 1), findsOneWidget);
    // ← at the start of a cell goes to the one before, at its end.
    editing(tester).selection = const TextSelection.collapsed(offset: 0);
    await key_(tester, LogicalKeyboardKey.arrowLeft);
    expect(cellField(1, 0), findsOneWidget);
    expect(editing(tester).selection.baseOffset, 'milk'.length);
    // In the middle of the words ← moves the cursor.
    await key_(tester, LogicalKeyboardKey.arrowLeft);
    expect(cellField(1, 0), findsOneWidget);
    expect(editing(tester).selection.baseOffset, 'milk'.length - 1);
    editing(tester).selection = const TextSelection.collapsed(offset: 4);
    await key_(tester, LogicalKeyboardKey.arrowRight);
    expect(cellField(1, 1), findsOneWidget);

    // The soft keyboard's ↵ is "next": the cell below.
    await tester.tap(key('grid-cell-2-0'));
    await tester.pumpAndSettle();
    expect(cellField(2, 0), findsOneWidget);
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await tester.pumpAndSettle();
    expect(find.byType(TextField), findsOneWidget);
    // Past the last row the cursor is after the table, in the text.
    expect(controller.selection, TextSelection.collapsed(offset: table.end));
    expect(
      tester
          .widget<TextField>(find.byType(TextField))
          .focusNode
          ?.hasPrimaryFocus,
      isTrue,
    );
    // The text around it is typed as before.
    await type(tester, '$shop!');
    expect(controller.text, '$shop!');

    // From after the table ← goes into its last cell, at its end.
    controller.selection = TextSelection.collapsed(offset: table.end);
    await tester.pumpAndSettle();
    await key_(tester, LogicalKeyboardKey.arrowLeft);
    expect(cellField(2, 1), findsOneWidget);
    expect(editing(tester).selection.baseOffset, 1);
    await key_(tester, LogicalKeyboardKey.tab);
    expect(find.byType(TextField), findsOneWidget);
    expect(controller.selection, TextSelection.collapsed(offset: table.end));

    // ↑ from the line after it, ↓ from the line before it go in too.
    controller.selection = TextSelection.collapsed(offset: table.end + 1);
    await tester.pumpAndSettle();
    await key_(tester, LogicalKeyboardKey.arrowUp);
    expect(cellField(2, 1), findsOneWidget);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await key_(tester, LogicalKeyboardKey.tab);
    await key_(tester, LogicalKeyboardKey.tab);
    await key_(tester, LogicalKeyboardKey.tab);
    await key_(tester, LogicalKeyboardKey.tab);
    await key_(tester, LogicalKeyboardKey.tab);
    await key_(tester, LogicalKeyboardKey.tab);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    // Past the first cell the cursor is before the table.
    expect(find.byType(TextField), findsOneWidget);
    expect(controller.selection, TextSelection.collapsed(offset: table.start));
    controller.selection = TextSelection.collapsed(offset: table.start - 1);
    await tester.pumpAndSettle();
    await key_(tester, LogicalKeyboardKey.arrowDown);
    expect(cellField(0, 0), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('◀ and ▶ of the bottom row go into the grid, not past it', (
    tester,
  ) async {
    final controller = await pumpField(tester, shop);
    final table = tableOf(controller);
    controller.selection = TextSelection.collapsed(offset: table.start);
    await tester.pumpAndSettle();
    await tester.tap(key('two-row-forward'));
    await tester.pumpAndSettle();
    expect(cellField(0, 0), findsOneWidget);
    // While a cell is edited the bottom row is hidden.
    expect(key('two-row-bar'), findsNothing);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await key_(tester, LogicalKeyboardKey.tab);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(controller.selection, TextSelection.collapsed(offset: table.start));
    expect(key('two-row-bar'), findsOneWidget);
    await tester.tap(key('two-row-back'));
    await tester.pumpAndSettle();
    expect(controller.selection.baseOffset, table.start - 1);

    controller.selection = TextSelection.collapsed(offset: table.end);
    await tester.pumpAndSettle();
    await tester.tap(key('two-row-back'));
    await tester.pumpAndSettle();
    expect(cellField(2, 1), findsOneWidget);
    expect(editing(tester).selection.baseOffset, 1);
  });

  testWidgets('"+" adds a row and a column, its first cell edited', (
    tester,
  ) async {
    final controller = await pumpField(tester, shop);
    await tester.tap(key('grid-add-row'));
    await tester.pumpAndSettle();
    expect(controller.text, contains('| oat | 1 |\n|  |  |\n\nBread'));
    expect(cellField(3, 0), findsOneWidget);
    await type(tester, 'eggs');
    expect(controller.text, contains('| oat | 1 |\n| eggs |  |\n\nBread'));

    await tester.tap(key('grid-add-column'));
    await tester.pumpAndSettle();
    expect(controller.text, contains('| Item | Qty |  |\n| --- | --: | --- |'));
    expect(cellField(0, 2), findsOneWidget);
    await type(tester, 'Shop');
    expect(controller.text, contains('| Item | Qty | Shop |'));
    expect(controller.text, contains('| eggs |  |  |'));
  });

  testWidgets('a screen reader presses a cell and "+" in two rows', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final controller = await pumpField(tester, shop);
    tester.semantics.tap(find.semantics.byLabel(RegExp('^Row 2, column A')));
    await tester.pumpAndSettle();
    expect(cellField(1, 0), findsOneWidget);
    tester.semantics.tap(find.semantics.byLabel('Add a row'));
    await tester.pumpAndSettle();
    expect(controller.text, contains('| oat | 1 |\n|  |  |\n\nBread'));
    expect(cellField(3, 0), findsOneWidget);
    semantics.dispose();
  });

  /// Opens the menu of [heading] and picks [item].
  Future<void> pick(WidgetTester tester, String heading, String item) async {
    await tester.tap(key(heading));
    await tester.pumpAndSettle();
    await tester.tap(key(item));
    await tester.pumpAndSettle();
  }

  testWidgets('a row\'s and a column\'s menu insert, move and delete it, '
      'asking once while it holds words', (tester) async {
    final controller = await pumpField(tester, shop);
    String lines() => controller.text.split('\n\n')[1];

    await pick(tester, 'grid-row-1', 'table-forward');
    expect(lines(), '| Item | Qty |\n| --- | --: |\n| oat | 1 |\n| milk | 2 |');
    await pick(tester, 'grid-row-2', 'table-before');
    expect(
      lines(),
      '| Item | Qty |\n| --- | --: |\n| oat | 1 |\n|  |  |\n| milk | 2 |',
    );
    // An empty row goes without asking.
    await pick(tester, 'grid-row-2', 'table-delete');
    expect(find.byType(AlertDialog), findsNothing);
    expect(lines(), '| Item | Qty |\n| --- | --: |\n| oat | 1 |\n| milk | 2 |');
    await pick(tester, 'grid-row-1', 'table-delete');
    expect(find.text('Delete this row?'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(lines(), contains('| oat | 1 |'));
    await pick(tester, 'grid-row-1', 'table-delete');
    await tester.tap(key('table-delete-confirm'));
    await tester.pumpAndSettle();
    expect(lines(), '| Item | Qty |\n| --- | --: |\n| milk | 2 |');

    await pick(tester, 'grid-column-1', 'table-back');
    expect(lines(), '| Qty | Item |\n| --: | --- |\n| 2 | milk |');
    await pick(tester, 'grid-column-0', 'table-after');
    expect(lines(), '| Qty |  | Item |\n| --: | --- | --- |\n| 2 |  | milk |');
    await pick(tester, 'grid-column-1', 'table-delete');
    expect(find.byType(AlertDialog), findsNothing);
    expect(lines(), '| Qty | Item |\n| --: | --- |\n| 2 | milk |');
    await pick(tester, 'grid-column-0', 'table-headings');
    expect(
      lines(),
      '${MarkdownTables.headingColumnLine}\n'
      '| Qty | Item |\n| --: | --- |\n| 2 | milk |',
    );

    // The last row deletes the table with it.
    await pick(tester, 'grid-row-1', 'table-delete');
    await tester.tap(key('table-delete-confirm'));
    await tester.pumpAndSettle();
    await pick(tester, 'grid-row-0', 'table-delete');
    expect(find.text('Delete the table?'), findsOneWidget);
    await tester.tap(key('table-delete-confirm'));
    await tester.pumpAndSettle();
    expect(find.byType(TableGrid), findsNothing);
    expect(MarkdownTables.all(controller.text), isEmpty);
    expect(controller.text, 'Shop\n\n\nBread');
  });

  testWidgets('a column\'s menu lines it up, the way it does checked', (
    tester,
  ) async {
    final controller = await pumpField(tester, shop);
    Icon? check(String item) => tester
        .widgetList<Icon>(
          find.descendant(of: key(item), matching: find.byType(Icon)),
        )
        .where((icon) => icon.icon == Icons.check)
        .firstOrNull;

    await tester.tap(key('grid-column-1'));
    await tester.pumpAndSettle();
    expect(check('table-alignRight'), isNotNull);
    expect(check('table-alignLeft'), isNull);
    expect(check('table-alignCenter'), isNull);
    await tester.tap(key('table-alignCenter'));
    await tester.pumpAndSettle();
    expect(controller.text, contains('| --- | :-: |'));
    await pick(tester, 'grid-column-0', 'table-alignRight');
    expect(controller.text, contains('| --: | :-: |'));
    await pick(tester, 'grid-column-0', 'table-alignLeft');
    expect(controller.text, contains('| :-- | :-: |'));
    // Its cells line up so.
    final cell = tester.widget<MarkedCell>(
      find.descendant(
        of: key('grid-cell-1-1'),
        matching: find.byType(MarkedCell),
      ),
    );
    expect(cell.textAlign, TextAlign.center);
  });

  testWidgets('a column lining up on the left without saying so has Align '
      'left checked, and picking it keeps its line', (tester) async {
    final controller = await pumpField(tester, shop);
    final text = controller.text;
    await tester.tap(key('grid-column-0'));
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: key('table-alignLeft'),
        matching: find.byIcon(Icons.check),
      ),
      findsOneWidget,
    );
    await tester.tap(key('table-alignLeft'));
    await tester.pumpAndSettle();
    expect(controller.text, text);
  });

  testWidgets('"Show as lines" shows the table\'s lines as text until the '
      'cursor leaves them or "Show as table" is pressed', (tester) async {
    for (final bar in [true, false]) {
      final controller = await pumpField(tester, shop, bar: bar);
      final table = tableOf(controller);
      await tester.tap(key('table-show-lines'));
      await tester.pumpAndSettle();
      expect(find.byType(TableGrid), findsNothing);
      expect(controller.rawTable, 0);
      expect(
        controller.selection,
        TextSelection.collapsed(offset: table.start),
      );
      // Its lines are typed into as text, kept nowhere.
      final at = controller.text.indexOf('oat') + 3;
      controller.selection = TextSelection.collapsed(offset: at);
      await tester.pumpAndSettle();
      expect(controller.selection.baseOffset, at);
      tester.testTextInput.updateEditingValue(
        TextEditingValue(
          text: controller.text.replaceRange(at, at, 's'),
          selection: TextSelection.collapsed(offset: at + 1),
        ),
      );
      await tester.pumpAndSettle();
      expect(controller.text, contains('| oats | 1 |'));
      expect(find.byType(TableGrid), findsNothing);
      // "Show as table" is on the bar, also without the formats.
      expect(key('table-show-grid'), findsOneWidget);
      await tester.tap(key('table-show-grid'));
      await tester.pumpAndSettle();
      expect(find.byType(TableGrid), findsOneWidget);
      expect(controller.rawTable, isNull);
      expect(key('table-show-grid'), findsNothing);

      // The cursor leaving the lines shows the grid again.
      await tester.tap(key('table-show-lines'));
      await tester.pumpAndSettle();
      expect(find.byType(TableGrid), findsNothing);
      controller.selection = const TextSelection.collapsed(offset: 2);
      await tester.pumpAndSettle();
      expect(find.byType(TableGrid), findsOneWidget);

      // A dialog leaves them lines: a link set on a word of a cell.
      await tester.tap(key('table-show-lines'));
      await tester.pumpAndSettle();
      if (bar) {
        final oat = controller.text.indexOf('oats');
        controller.selection = TextSelection(
          baseOffset: oat,
          extentOffset: oat + 4,
        );
        await tester.pumpAndSettle();
        await tester.tap(key('format-link'));
        await tester.pumpAndSettle();
        await tester.enterText(key('link-address'), 'example.org');
        await tester.tap(key('link-done'));
        await tester.pumpAndSettle();
        expect(find.byType(AlertDialog), findsNothing);
        expect(
          controller.text,
          contains('| [oats](https://example.org) | 1 |'),
        );
        expect(controller.rawTable, 0);
        expect(find.byType(TableGrid), findsNothing);
      }
      await tester.tap(key('table-show-grid'));
      await tester.pumpAndSettle();

      // The field losing the focus shows the grid again.
      await tester.tap(key('table-show-lines'));
      await tester.pumpAndSettle();
      FocusManager.instance.primaryFocus?.unfocus();
      await tester.pumpAndSettle();
      expect(find.byType(TableGrid), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });

  testWidgets('while a cell is edited the bar has only the formats of its '
      'words, which format them; without the bar none', (tester) async {
    final controller = await pumpField(tester, shop);
    await tapCell(tester, 2, 0);
    expect(key('format-bar'), findsOneWidget);
    for (final format in [
      'two-row-insert-box',
      'format-bold',
      'format-italic',
      'format-strike',
      'format-link',
      'format-code',
    ]) {
      expect(key(format), findsOneWidget, reason: format);
    }
    for (final format in [
      'format-heading1',
      'format-bullet',
      'format-quote',
      'format-code-block',
      'format-table',
    ]) {
      expect(key(format), findsNothing, reason: format);
    }
    editing(tester).selection = const TextSelection(
      baseOffset: 0,
      extentOffset: 3,
    );
    await tester.pumpAndSettle();
    await tester.tap(key('format-bold'));
    await tester.pumpAndSettle();
    expect(editing(tester).text, '**oat**');
    expect(controller.text, contains('| **oat** | 1 |'));
    await tester.tap(key('two-row-insert-box'));
    await tester.pumpAndSettle();
    expect(controller.text, contains('| **☐oat** | 1 |'));
    await tester.pumpWidget(const SizedBox.shrink());

    // Simple: no bar while a cell is edited.
    await pumpField(tester, shop, bar: false);
    await tapCell(tester, 2, 0);
    expect(cellField(2, 0), findsOneWidget);
    expect(key('format-bar'), findsNothing);
    expect(key('format-bold'), findsNothing);
  });

  testWidgets('the table button puts a table in and edits its first cell '
      'in the grid; at a table it goes into it', (tester) async {
    final controller = await pumpField(tester, 'Shop', plain: true);
    controller.selection = const TextSelection.collapsed(offset: 4);
    await tester.pumpAndSettle();
    await tester.tap(key('format-table'));
    await tester.pumpAndSettle();
    expect(find.byType(TableScreen), findsNothing);
    expect(controller.markdown, isTrue);
    expect(cellField(0, 0), findsOneWidget);
    await type(tester, 'Item');
    expect(controller.text, 'Shop\n\n| Item |  |\n| --- | --- |\n|  |  |');
    await key_(tester, LogicalKeyboardKey.enter);
    await key_(tester, LogicalKeyboardKey.enter);
    expect(find.byType(TextField), findsOneWidget);
    final table = tableOf(controller);
    expect(controller.selection, TextSelection.collapsed(offset: table.end));
    await tester.tap(key('format-table'));
    await tester.pumpAndSettle();
    expect(cellField(1, 1), findsOneWidget);
  });

  testWidgets('a grid wider than the field moves sideways under a drag, the '
      'cursor staying where it was', (tester) async {
    String row(String name) =>
        '| $name | ${[for (var c = 1; c < 8; c++) 'column$c words'].join(' | ')} |';
    final wide = [
      'Wide',
      '',
      row('Name'),
      '| ${List.filled(8, '---').join(' | ')} |',
      row('row1'),
    ].join('\n');
    final controller = await pumpField(tester, wide, width: 300);
    controller.selection = const TextSelection.collapsed(offset: 2);
    await tester.pumpAndSettle();
    final sideways = tester.widget<SingleChildScrollView>(
      key('table-grid-sideways'),
    );
    expect(sideways.controller?.offset, 0);
    await tester.drag(key('grid-cell-1-1'), const Offset(-200, 0));
    await tester.pumpAndSettle();
    expect(sideways.controller?.offset, greaterThan(100));
    expect(controller.selection, const TextSelection.collapsed(offset: 2));
    expect(find.byType(TextField), findsOneWidget);
    // A cell cut off at the edge moves into view when edited.
    await tester.drag(key('grid-cell-1-1'), const Offset(400, 0));
    await tester.pumpAndSettle();
    await tester.tap(find.text('row1'));
    await tester.pumpAndSettle();
    for (var i = 0; i < 7; i++) {
      await key_(tester, LogicalKeyboardKey.tab);
    }
    expect(cellField(1, 7), findsOneWidget);
    final rect = tester.getRect(cellField(1, 7));
    expect(rect.right, lessThanOrEqualTo(300));
  });

  testWidgets('a box typed into a cell taking its id while the cell is '
      'edited keeps the cell edited and the id', (tester) async {
    final controller = await pumpField(tester, shop, twoRows: false);
    await tapCell(tester, 2, 0);
    await type(tester, '☐ oat');
    expect(controller.text, contains('| ☐ oat | 1 |'));
    // The editor keeps the text: the new box gets its id.
    final id = String.fromCharCodes([
      for (final c in 'ab12'.codeUnits) ...[0xDB40, 0xDC00 + c],
      0xDB40,
      0xDC7F,
    ]);
    final kept = controller.text.replaceFirst('☐ oat', '☐$id oat');
    controller.value = TextEditingValue(
      text: kept,
      selection: controller.selection,
    );
    await tester.pumpAndSettle();
    expect(cellField(2, 0), findsOneWidget);
    expect(editing(tester).text, '☐$id oat');
    expect(editing(tester).selection.baseOffset, '☐$id oat'.length);
    await type(tester, '☐$id oats');
    expect(controller.text, contains('| ☐$id oats | 1 |'));

    // Changed otherwise from elsewhere, it is no longer edited.
    controller.value = TextEditingValue(
      text: controller.text.replaceFirst('oats', 'rye'),
      selection: controller.selection,
    );
    await tester.pumpAndSettle();
    expect(cellField(2, 0), findsNothing);
  });
}

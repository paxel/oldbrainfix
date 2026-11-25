import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/box_ids.dart';
import 'package:oldbrainfix/src/cell_selection.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/selection_toolbar.dart';
import 'package:oldbrainfix/src/table_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';

/// Cells of a table selected and copied (obf-1054): a long press on a
/// cell selects it, a drag on without lifting the finger spans a
/// rectangle of cells, and Copy in the toolbar above it puts them on the
/// clipboard as a table of their own, HTML and Markdown, one cell alone
/// as its words; in a text being read, on the editor's grid and full
/// screen. A selection that starts outside a table runs through it as
/// text.
void main() {
  Finder key(String value) => find.byKey(ValueKey(value));
  Finder words(String text) => find.text(text, findRichText: true);

  const table = '| Item | Qty |\n| --- | --: |\n| ☐ milk | 2 |\n| oat | 1 |';
  const shop = 'Shop\n\n$table\n\nBread';

  Widget app(Widget home) => MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: home,
  );

  /// What is put on the plain clipboard.
  List<String?> clipboard(WidgetTester tester) {
    final copied = <String?>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied.add((call.arguments as Map)['text'] as String?);
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    return copied;
  }

  /// A long press on [from], the finger then moved to [to] and lifted.
  Future<void> span(WidgetTester tester, Finder from, [Finder? to]) async {
    final gesture = await tester.startGesture(tester.getCenter(from));
    await tester.pump(kLongPressTimeout + const Duration(milliseconds: 50));
    if (to != null) {
      await gesture.moveTo(tester.getCenter(to));
      await tester.pump();
    }
    await gesture.up();
    await tester.pumpAndSettle();
  }

  /// The labels of the toolbar shown, in their order.
  List<String> toolbar(WidgetTester tester) => [
    for (final button in tester.widgetList<TextSelectionToolbarTextButton>(
      find.byType(TextSelectionToolbarTextButton),
    ))
      switch (button.child) {
        final Text text => text.data ?? '',
        _ => '',
      },
  ];

  group('the cells as a table of their own', () {
    final at = shop.indexOf('|');

    test('all of them copy as the whole table does', () {
      for (final text in [
        shop,
        '${MarkdownTables.headingColumnLine}\n| a | b \\| c |\n| :-: | --- |'
            '\n| x | y |',
      ]) {
        final start = MarkdownTables.all(text).single.start;
        final cells = tableCells(text, start, [0, 1, 2], 0, 1);
        expect(cells, isNotNull);
        final whole = MarkdownTables.copy(text, start);
        final copied = MarkdownTables.copy(cells!, 0);
        expect(copied?.markdown, whole?.markdown);
        expect(copied?.html, whole?.html);
      }
    });

    test('the rows in the order given, the first the header row, each '
        'column lining up as it does', () {
      expect(tableCells(shop, at, [2, 1], 1, 1), '| 1 |\n| --: |\n| 2 |');
      expect(tableCells(shop, at, [1], 0, 1), '| ☐ milk | 2 |\n| --- | --: |');
    });

    test('the first column holds headings only while it is copied', () {
      const text =
          '${MarkdownTables.headingColumnLine}\n| a | b |\n| --- | --- |'
          '\n| x | y |';
      expect(
        tableCells(text, 0, [0, 1], 0, 0),
        '${MarkdownTables.headingColumnLine}\n| a |\n| --- |\n| x |',
      );
      expect(tableCells(text, 0, [0, 1], 1, 1), '| b |\n| --- |\n| y |');
    });

    test('none where there is no table or no cell', () {
      expect(tableCells('Shop', 0, [0], 0, 0), isNull);
      expect(tableCells(shop, at, [], 0, 0), isNull);
      expect(tableCells(shop, at, [9], 0, 0), isNull);
      expect(tableCells(shop, at, [0], 2, 3), isNull);
    });
  });

  group('in a text being read', () {
    /// [text] read as a Section's text in the Note editor, its words
    /// selectable, Search noting into [searched]; the text selected in
    /// [selected].
    Future<FakePhotos> pumpRead(
      WidgetTester tester, {
      String text = shop,
      bool cells = true,
      List<String>? searched,
      List<String?>? selected,
      Note? note,
      BoxTicking? ticking,
    }) async {
      tester.view.physicalSize = const Size(800, 1200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final photos = FakePhotos();
      final read = MarkedText(
        note?.shownText ?? text,
        format: TextFormats.markdown,
        boxes: note?.boxes,
        source: note?.text,
        ticking: ticking,
        selectCells: cells,
      );
      await tester.pumpWidget(
        PhotoScope(
          photos: photos,
          bytes: (_) => null,
          child: app(
            Scaffold(
              body: SelectionSearchScope(
                onSearch: (words) => searched?.add(words),
                child: ListView(
                  children: [
                    if (cells)
                      SelectionArea(
                        onSelectionChanged: (content) =>
                            selected?.add(content?.plainText),
                        child: read,
                      )
                    else
                      read,
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      return photos;
    }

    testWidgets('a long press on a cell and a drag on select a rectangle of '
        'cells, which Copy puts on the clipboard as a table', (tester) async {
      final selected = <String?>[];
      final photos = await pumpRead(tester, selected: selected);
      final plain = clipboard(tester);
      await span(tester, words('oat'), words('2'));
      // Several cells: Copy alone, no text selected beside them.
      expect(toolbar(tester), ['Copy']);
      expect(selected.where((s) => s?.isNotEmpty ?? false), isEmpty);
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(photos.copiedHtml, [
        (
          text: '| ☐ milk | 2 |\n| --- | --: |\n| oat | 1 |',
          html:
              '<table><thead><tr><th>☐ milk</th><th align="right">2</th>'
              '</tr></thead><tbody><tr><td>oat</td><td align="right">1</td>'
              '</tr></tbody></table>\n',
        ),
      ]);
      expect(plain, isEmpty);
      expect(toolbar(tester), isEmpty);
      // Copied ends it: a tap opens the table again.
      await tester.tap(words('oat'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
    });

    testWidgets('one cell copies as a table of one cell and offers Search '
        'with its words; a tap ends the selection before opening the table', (
      tester,
    ) async {
      final searched = <String>[];
      final photos = await pumpRead(tester, searched: searched);
      clipboard(tester);
      await span(tester, words('oat'));
      expect(toolbar(tester), ['Copy', 'Find in Note']);
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(photos.copiedHtml.single.text, '| oat |\n| --- |');

      await span(tester, words('Item'));
      await tester.tap(find.text('Find in Note'));
      await tester.pumpAndSettle();
      expect(searched, ['Item']);

      await span(tester, words('oat'));
      await tester.tap(words('oat'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsNothing);
      expect(toolbar(tester), isEmpty);
      await tester.tap(words('oat'));
      await tester.pumpAndSettle();
      expect(find.byType(TableScreen), findsOneWidget);
    });

    testWidgets('rows sorted in the view copy in the order they show', (
      tester,
    ) async {
      final photos = await pumpRead(tester);
      clipboard(tester);
      await tester.tap(key('table-2-sort-1'));
      await tester.pumpAndSettle();
      await span(tester, words('Item'), words('1'));
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(
        photos.copiedHtml.single.text,
        '| Item | Qty |\n| --- | --: |\n| oat | 1 |',
      );
    });

    testWidgets('a selection that starts outside the table runs through it '
        'and copies as text', (tester) async {
      final selected = <String?>[];
      await pumpRead(tester, selected: selected);
      final plain = clipboard(tester);
      await tester.longPress(words('Shop'));
      await tester.pumpAndSettle();
      expect(selected.last, 'Shop');
      await tester.tap(find.text('Select all'));
      await tester.pumpAndSettle();
      expect(selected.last, allOf(contains('milk'), contains('Bread')));
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(plain.single, allOf(startsWith('Shop'), contains('oat')));
    });

    testWidgets('a Note\'s stored text copies each box as it shows, '
        'without its id', (tester) async {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final id = notes.capture(shop, format: TextFormats.markdown).id;
      final box = notes.byId(id)!.boxes.single;
      Boxes(store).tick(id, box.id, BoxState.done);
      final note = notes.byId(id)!;
      expect(withoutIds(note.text), isNot(note.text));
      final photos = await pumpRead(
        tester,
        note: note,
        ticking: BoxTicking.read(notes, id, mayTick: true),
      );
      clipboard(tester);
      await span(
        tester,
        find.textContaining('milk', findRichText: true).last,
        words('1'),
      );
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(
        photos.copiedHtml.single.text,
        '| ✅ milk | 2 |\n| --- | --: |\n| oat | 1 |',
      );
    });

    testWidgets('a drag that does not start with a long press still moves a '
        'wide table sideways, and ends the cells selected', (tester) async {
      String row(String name) =>
          '| $name | ${[for (var c = 1; c < 8; c++) 'column$c words'].join(' | ')} |';
      final wide = [
        row('Name'),
        '| ${List.filled(8, '---').join(' | ')} |',
        row('first'),
      ].join('\n');
      await pumpRead(tester, text: wide);
      clipboard(tester);
      await span(tester, words('first'));
      expect(toolbar(tester), ['Copy', 'Find in Note']);
      final before = tester.getTopLeft(words('first'));
      await tester.drag(words('first'), const Offset(-200, 0));
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(words('first')).dx, lessThan(before.dx));
      expect(toolbar(tester), isEmpty);
    });

    testWidgets('where no text selects, as on a card, a long press on a '
        'cell selects none', (tester) async {
      await pumpRead(tester, cells: false);
      await span(tester, words('oat'), words('2'));
      expect(toolbar(tester), isEmpty);
      expect(find.byType(CellSelection), findsNothing);
    });
  });

  group('on the editor\'s grid', () {
    Future<TagTextController> pumpField(
      WidgetTester tester, {
      String text = shop,
      BoxTicking? ticking,
      List<String>? searched,
    }) async {
      tester.view.physicalSize = const Size(1000, 1600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final controller = TagTextController(text: text, hideMarks: true)
        ..markdown = true
        ..boxes = ticking;
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        app(
          Scaffold(
            body: SelectionSearchScope(
              onSearch: (words) => searched?.add(words),
              child: Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 400,
                  child: ListView(
                    shrinkWrap: true,
                    children: [
                      TwoRowTextField(
                        controller: controller,
                        twoRows: true,
                        boxes: true,
                        formats: true,
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      return controller;
    }

    Finder cellField(int row, int column) => find.descendant(
      of: key('grid-cell-$row-$column'),
      matching: find.byType(TextField),
    );

    testWidgets('a long press on a cell not edited and a drag on select '
        'cells to copy; a tap ends it before editing', (tester) async {
      final controller = await pumpField(tester);
      final plain = clipboard(tester);
      await span(tester, key('grid-cell-1-0'), key('grid-cell-2-1'));
      expect(cellField(1, 0), findsNothing);
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      // No HTML here: the Markdown alone.
      expect(plain, ['| ☐ milk | 2 |\n| --- | --: |\n| oat | 1 |']);
      expect(controller.text, shop);

      await span(tester, key('grid-cell-2-0'));
      await tester.tap(key('grid-cell-2-0'));
      await tester.pumpAndSettle();
      expect(cellField(2, 0), findsNothing);
      await tester.tap(key('grid-cell-2-0'));
      await tester.pumpAndSettle();
      expect(cellField(2, 0), findsOneWidget);
    });

    testWidgets('a stored text copies each box as it shows, and Search on '
        'one cell holds no id', (tester) async {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final id = notes.capture(shop, format: TextFormats.markdown).id;
      final box = notes.byId(id)!.boxes.single;
      Boxes(store).tick(id, box.id, BoxState.done);
      final searched = <String>[];
      await pumpField(
        tester,
        text: notes.byId(id)!.text,
        ticking: BoxTicking.read(notes, id, mayTick: true),
        searched: searched,
      );
      final plain = clipboard(tester);
      await span(tester, key('grid-cell-1-0'), key('grid-cell-2-1'));
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(plain, ['| ✅ milk | 2 |\n| --- | --: |\n| oat | 1 |']);

      await span(tester, key('grid-cell-1-0'));
      await tester.tap(find.text('Find in Note'));
      await tester.pumpAndSettle();
      expect(searched, hasLength(1));
      expect(searched.single, contains('milk'));
      expect(withoutIds(searched.single), searched.single);
    });

    testWidgets('a long press in the cell being edited selects its words', (
      tester,
    ) async {
      await pumpField(tester);
      await tester.tap(key('grid-cell-2-0'));
      await tester.pumpAndSettle();
      await tester.longPressAt(
        tester.getTopLeft(cellField(2, 0)) + const Offset(8, 8),
      );
      await tester.pumpAndSettle();
      final field = tester.widget<TextField>(cellField(2, 0));
      expect(field.controller?.selection.isCollapsed, isFalse);
      expect(find.text('Copy'), findsOneWidget);
    });
  });

  group('full screen', () {
    Future<List<MarkdownChange>> open(
      WidgetTester tester,
      FakePhotos photos, {
      bool edits = false,
    }) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final written = <MarkdownChange>[];
      await tester.pumpWidget(
        PhotoScope(
          photos: photos,
          bytes: (_) => null,
          child: app(
            Builder(
              builder: (context) => Scaffold(
                body: Center(
                  child: TextButton(
                    onPressed: () => openTable(
                      context,
                      text: shop,
                      at: shop.indexOf('|'),
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
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      return written;
    }

    testWidgets('where it is only read, cells sorted in the view copy in '
        'the order they show', (tester) async {
      final photos = FakePhotos();
      clipboard(tester);
      await open(tester, photos);
      await tester.tap(key('cell-0-1'));
      await tester.pumpAndSettle();
      await span(tester, key('cell-2-0'), key('cell-1-1'));
      expect(toolbar(tester), ['Copy']);
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(
        photos.copiedHtml.single.text,
        '| oat | 1 |\n| --- | --: |\n| ☐ milk | 2 |',
      );
    });

    testWidgets('a scroll of the rows ends the cells selected', (tester) async {
      final long = [
        '| Item | Qty |',
        '| --- | --: |',
        for (var r = 1; r <= 40; r++) '| item$r | $r |',
      ].join('\n');
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(app(TableScreen(text: long, at: 0)));
      await tester.pumpAndSettle();
      clipboard(tester);
      await span(tester, key('cell-2-0'));
      expect(toolbar(tester), ['Copy']);
      await tester.drag(key('cell-3-0'), const Offset(0, -200));
      await tester.pumpAndSettle();
      expect(toolbar(tester), isEmpty);
    });

    testWidgets('where it is edited, the cell edited is written first', (
      tester,
    ) async {
      final photos = FakePhotos();
      clipboard(tester);
      final written = await open(tester, photos, edits: true);
      await tester.tap(key('cell-2-1'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '3');
      await span(tester, key('cell-1-0'), key('cell-2-1'));
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(written, hasLength(1));
      expect(
        photos.copiedHtml.single.text,
        '| ☐ milk | 2 |\n| --- | --: |\n| oat | 3 |',
      );
    });
  });
}

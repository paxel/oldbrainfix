import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/selection_toolbar.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The toolbar above a selection on the phone (obf-1055): Copy, Paste and
/// Search in place of the system's menu, in reading and in editing; only
/// Paste on an empty spot of a field, never Paste where nothing can be
/// pasted; Search opens the Note's find bar with the words selected, and
/// the find bar's "Search all Notes" the search over every Note.
void main() {
  /// The clipboard holds [held]; what is copied goes into [copied].
  void clipboard(WidgetTester tester, {String? held, List<String?>? copied}) {
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async => switch (call.method) {
        'Clipboard.hasStrings' => {'value': held != null},
        'Clipboard.getData' => held == null ? null : {'text': held},
        'Clipboard.setData' => () {
          copied?.add((call.arguments as Map)['text'] as String?);
          return null;
        }(),
        _ => null,
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
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

  group('in a field', () {
    /// A field holding [text] under a Note's Search, which notes the
    /// words it is given into [searched].
    Future<TagTextController> pumpField(
      WidgetTester tester, {
      required String text,
      bool readOnly = false,
      bool twoRows = true,
      bool markdown = false,
      List<String>? searched,
    }) async {
      final controller = TagTextController(text: text)..markdown = markdown;
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SelectionSearchScope(
              onSearch: (words) => searched?.add(words),
              child: ListView(
                children: [
                  TwoRowTextField(
                    controller: controller,
                    twoRows: twoRows,
                    readOnly: readOnly,
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      return controller;
    }

    Offset caretAt(WidgetTester tester, int offset) {
      final editable = tester
          .state<EditableTextState>(find.byType(EditableText))
          .renderEditable;
      final rect = editable.getLocalRectForCaret(TextPosition(offset: offset));
      return editable.localToGlobal(rect.center);
    }

    for (final twoRows in [true, false]) {
      testWidgets('a word selected offers Copy, Paste, Search and Delete, '
          'no Cut or Select all (two rows: $twoRows)', (tester) async {
        clipboard(tester, held: 'pasted');
        final searched = <String>[];
        await pumpField(
          tester,
          text: 'buy oat milk',
          twoRows: twoRows,
          searched: searched,
        );

        await tester.longPressAt(caretAt(tester, 5));
        await tester.pumpAndSettle();
        expect(toolbar(tester), ['Copy', 'Paste', 'Search', 'Delete']);

        await tester.tap(find.text('Search'));
        await tester.pumpAndSettle();
        expect(searched, ['oat']);
        expect(find.text('Copy'), findsNothing, reason: 'the toolbar closed');
      });
    }

    testWidgets('a long press on an empty spot offers only Paste', (
      tester,
    ) async {
      clipboard(tester, held: 'pasted');
      final controller = await pumpField(tester, text: 'milk\n\n\nbread');

      await tester.longPressAt(caretAt(tester, 6));
      await tester.pumpAndSettle();
      expect(controller.selection.textInside(controller.text), '\n');
      expect(toolbar(tester), ['Paste']);

      await tester.tap(find.text('Paste'));
      await tester.pumpAndSettle();
      expect(controller.text, 'milk\n\npasted\nbread');
    });

    for (final twoRows in [true, false]) {
      testWidgets('a long press in an empty field offers only Paste '
          '(two rows: $twoRows)', (tester) async {
        clipboard(tester, held: 'pasted');
        final controller = await pumpField(tester, text: '', twoRows: twoRows);

        await tester.longPressAt(caretAt(tester, 0));
        await tester.pumpAndSettle();
        expect(toolbar(tester), ['Paste']);
        await tester.tap(find.text('Paste'));
        await tester.pumpAndSettle();
        expect(controller.text, 'pasted');
      });
    }

    testWidgets('a long press past the end of the text offers only Paste', (
      tester,
    ) async {
      clipboard(tester, held: 'pasted');
      final controller = await pumpField(tester, text: 'milk');

      await tester.longPressAt(caretAt(tester, 4) + const Offset(200, 0));
      await tester.pumpAndSettle();
      expect(toolbar(tester), ['Paste']);
      expect(controller.selection.isCollapsed, isTrue);
      expect(controller.selection.baseOffset, 4);
    });

    testWidgets('with nothing to paste an empty spot shows no toolbar', (
      tester,
    ) async {
      clipboard(tester);
      await pumpField(tester, text: 'milk\n\n\nbread');

      await tester.longPressAt(caretAt(tester, 6));
      await tester.pumpAndSettle();
      expect(toolbar(tester), isEmpty);
    });

    testWidgets('a text only read offers Copy and Search, never Paste', (
      tester,
    ) async {
      final copied = <String?>[];
      clipboard(tester, held: 'pasted', copied: copied);
      await pumpField(tester, text: 'buy oat milk', readOnly: true);

      await tester.longPressAt(caretAt(tester, 9));
      await tester.pumpAndSettle();
      expect(toolbar(tester), ['Copy', 'Search']);
      await tester.tap(find.text('Copy'));
      await tester.pumpAndSettle();
      expect(copied, ['milk']);
    });

    testWidgets('Search finds the words as they read, without marks or a '
        "box's id", (tester) async {
      clipboard(tester);
      final searched = <String>[];
      final controller = await pumpField(
        tester,
        text: 'say **hello** there',
        markdown: true,
        searched: searched,
      );
      await tester.showKeyboard(find.byType(TextField));
      controller.selection = const TextSelection(
        baseOffset: 4,
        extentOffset: 19,
      );
      tester.state<EditableTextState>(find.byType(EditableText)).showToolbar();
      await tester.pumpAndSettle();
      await tester.tap(find.text('Search'));
      await tester.pumpAndSettle();
      expect(searched, ['hello there']);
    });

    test('a selection over lines finds its first line that holds words', () {
      expect(selectionWords('\n  tea \n coffee', TextFormats.plain), 'tea');
      expect(
        selectionWords('**bold**  word', TextFormats.markdown),
        'bold word',
      );
      expect(selectionWords(' \n ', TextFormats.plain), '');
      // A formatted span cut by the selection keeps none of its marks.
      expect(selectionWords('say **hello', TextFormats.markdown), 'say hello');
      expect(selectionWords('lo** there', TextFormats.markdown), 'lo there');
    });
  });

  group('in the Note editor', () {
    late Store store;
    late Notes notes;

    setUp(() {
      store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 23, 8))
        ..starterSetOffered = true;
      notes = Notes(store);
    });
    tearDown(() => store.close());

    Future<void> edit(WidgetTester tester, Note note) async {
      tester.view
        ..physicalSize = const Size(1080, 2340)
        ..devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), note));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    }

    Offset caretAt(WidgetTester tester, int offset) {
      final editable = tester
          .state<EditableTextState>(
            find
                .descendant(
                  of: find.byKey(const ValueKey('note-text')),
                  matching: find.byType(EditableText),
                )
                .first,
          )
          .renderEditable;
      final rect = editable.getLocalRectForCaret(TextPosition(offset: offset));
      return editable.localToGlobal(rect.center);
    }

    String count(WidgetTester tester) =>
        tester.widget<Text>(find.byKey(const ValueKey('find-count'))).data ??
        '';

    testWidgets('Search opens the find bar with the word selected, whose '
        '"Search all Notes" opens the search over every Note', (tester) async {
      clipboard(tester);
      final note = notes.capture('Boiler service\ncheck the boiler');
      notes.capture('boiler manual in the cellar');
      await edit(tester, note);
      expect(find.byKey(const ValueKey('find-bar')), findsNothing);

      await tester.longPressAt(caretAt(tester, 2));
      await tester.pumpAndSettle();
      expect(toolbar(tester), ['Copy', 'Search', 'Delete']);
      await tester.tap(find.text('Search'));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('find-bar')), findsOneWidget);
      expect(
        tester
            .widget<TextField>(find.byKey(const ValueKey('find-field')))
            .controller
            ?.text,
        'Boiler',
      );
      expect(count(tester), '1 of 2');

      await tester.tap(find.byKey(const ValueKey('find-search-all')));
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(NoteCard), findsNWidgets(2));

      // Back is back in the Note, its find bar still open.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(count(tester), '1 of 2');
    });

    testWidgets('"Search all Notes" is greyed while the find bar holds no '
        'words', (tester) async {
      final note = notes.capture('Boiler service');
      await edit(tester, note);
      await tester.tap(find.byKey(const ValueKey('find-in-note')));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<IconButton>(find.byKey(const ValueKey('find-search-all')))
            .onPressed,
        isNull,
      );
      await tester.enterText(find.byKey(const ValueKey('find-field')), 'boil');
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<IconButton>(find.byKey(const ValueKey('find-search-all')))
            .onPressed,
        isNotNull,
      );
    });
  });
}

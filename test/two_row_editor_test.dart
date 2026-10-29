import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  final bar = find.byKey(const ValueKey('two-row-bar'));
  final back = find.byKey(const ValueKey('two-row-back'));
  final forward = find.byKey(const ValueKey('two-row-forward'));
  final line = find.byKey(const ValueKey('two-row-line'));

  /// A two-row field holding [text] in a scrolling page, as in the editor.
  Future<TextEditingController> pumpField(
    WidgetTester tester, {
    String text = '',
    bool twoRows = true,
  }) async {
    final controller = TextEditingController(text: text);
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: ListView(
            children: [
              TwoRowTextField(controller: controller, twoRows: twoRows),
            ],
          ),
        ),
      ),
    );
    return controller;
  }

  /// Focuses the field without a tap and lets the keyboard cover the
  /// lower part of the screen.
  Future<void> openKeyboard(WidgetTester tester) async {
    await tester.showKeyboard(find.byType(TextField));
    tester.view.viewInsets = const FakeViewPadding(bottom: 900);
    addTearDown(tester.view.resetViewInsets);
    await tester.pumpAndSettle();
  }

  /// Where the caret before [offset] sits in the top row, on screen.
  Offset caretAt(WidgetTester tester, int offset) {
    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    final rect = editable.getLocalRectForCaret(TextPosition(offset: offset));
    return editable.localToGlobal(rect.center);
  }

  Future<void> press(WidgetTester tester, Finder button) async {
    await tester.tap(button);
    await tester.pump();
  }

  /// A finger sliding by [by] from [from] as a phone reports it: in small
  /// moves, a frame apart.
  Future<void> slide(WidgetTester tester, Offset from, Offset by) async {
    final finger = await tester.startGesture(from);
    for (var i = 1; i <= 40; i++) {
      await finger.moveBy(by / 40);
      await tester.pump(const Duration(milliseconds: 8));
    }
    await finger.up();
    await tester.pumpAndSettle();
  }

  TextSelection selectionOf(TextEditingController controller) =>
      controller.selection;

  group('the bottom row', () {
    testWidgets('shows above the keyboard only while it is open', (
      tester,
    ) async {
      await pumpField(tester, text: 'hello');
      expect(bar, findsNothing);

      await openKeyboard(tester);
      expect(bar, findsOneWidget);
      final keyboardTop =
          tester.view.physicalSize.height / tester.view.devicePixelRatio -
          900 / tester.view.devicePixelRatio;
      expect(tester.getBottomLeft(bar).dy, keyboardTop);

      tester.view.resetViewInsets();
      await tester.pumpAndSettle();
      expect(bar, findsNothing);
    });

    testWidgets('steps one character per tap and keeps the keyboard open', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'abc');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 1);
      await tester.pump();

      await press(tester, forward);
      expect(selectionOf(controller), const TextSelection.collapsed(offset: 2));
      await press(tester, back);
      await press(tester, back);
      expect(selectionOf(controller), const TextSelection.collapsed(offset: 0));
      await press(tester, back);
      expect(selectionOf(controller), const TextSelection.collapsed(offset: 0));
      expect(
        tester
            .state<EditableTextState>(find.byType(EditableText))
            .widget
            .focusNode
            .hasFocus,
        isTrue,
      );
      expect(tester.testTextInput.isVisible, isTrue);
      expect(bar, findsOneWidget);
    });

    testWidgets('steps over an emoji with its skin tone as one character', (
      tester,
    ) async {
      // a, thumbs up with a skin tone (four UTF-16 units), b.
      final controller = await pumpField(tester, text: 'a👍🏽b');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 1);
      await tester.pump();

      await press(tester, forward);
      expect(controller.selection.baseOffset, 5);
      await press(tester, forward);
      expect(controller.selection.baseOffset, 6);
      await press(tester, back);
      await press(tester, back);
      expect(controller.selection.baseOffset, 1);
    });

    testWidgets('shows the text around the cursor and follows typing', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'one two');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 3);
      await tester.pump();
      String shown(String key) => tester
          .widget<RichText>(
            find.descendant(
              of: find.byKey(ValueKey(key)),
              matching: find.byType(RichText),
            ),
          )
          .text
          .toPlainText();
      expect(shown('two-row-before'), 'one');
      expect(shown('two-row-after'), ' two');

      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'one, two',
          selection: TextSelection.collapsed(offset: 4),
        ),
      );
      await tester.pump();
      expect(shown('two-row-before'), 'one,');
      expect(shown('two-row-after'), ' two');
    });

    testWidgets('repeats while held, faster the longer it is held', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'x' * 200);
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 0);
      await tester.pump();

      final hold = await tester.startGesture(tester.getCenter(forward));
      await tester.pump();
      expect(controller.selection.baseOffset, 1, reason: 'a step at once');
      await tester.pump(
        TwoRowTextField.holdDelay - const Duration(milliseconds: 1),
      );
      expect(controller.selection.baseOffset, 1);
      await tester.pump(const Duration(milliseconds: 1));
      expect(controller.selection.baseOffset, 2, reason: 'the first repeat');

      // The second half-second of repeats steps further than the first.
      for (var i = 0; i < 50; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }
      final afterFirst = controller.selection.baseOffset;
      for (var i = 0; i < 50; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }
      final afterSecond = controller.selection.baseOffset;
      expect(afterSecond - afterFirst, greaterThan(afterFirst - 2));

      await hold.up();
      await tester.pump(const Duration(seconds: 1));
      expect(controller.selection.baseOffset, afterSecond, reason: 'stopped');
    });

    testWidgets('keeps a composing word while inside it and finishes it '
        'when leaving it', (tester) async {
      final controller = await pumpField(tester);
      await openKeyboard(tester);
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'hello wor',
          selection: TextSelection.collapsed(offset: 9),
          composing: TextRange(start: 6, end: 9),
        ),
      );
      await tester.pump();

      await press(tester, back);
      expect(controller.selection, const TextSelection.collapsed(offset: 8));
      expect(controller.value.composing, const TextRange(start: 6, end: 9));

      await press(tester, back);
      await press(tester, back);
      await press(tester, back);
      expect(controller.selection, const TextSelection.collapsed(offset: 5));
      expect(controller.value.composing, TextRange.empty);
      expect(controller.text, 'hello wor');
      // The keyboard was told: its composing word is done.
      expect(tester.testTextInput.editingState?['composingBase'], -1);
      expect(tester.testTextInput.editingState?['selectionBase'], 5);

      // Typing goes on where the cursor is.
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'hello, wor',
          selection: TextSelection.collapsed(offset: 6),
        ),
      );
      await tester.pump();
      expect(controller.text, 'hello, wor');
      await press(tester, forward);
      expect(controller.selection, const TextSelection.collapsed(offset: 7));
    });
  });

  group('the top row', () {
    testWidgets('a tap puts the cursor at the nearest word boundary', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello wonderful world');
      await openKeyboard(tester);

      // Two letters into "wonderful": its start is nearer.
      await tester.tapAt(caretAt(tester, 8));
      await tester.pump(const Duration(milliseconds: 500));
      expect(controller.selection, const TextSelection.collapsed(offset: 6));

      // Two letters before its end: its end is nearer.
      await tester.tapAt(caretAt(tester, 13));
      await tester.pump(const Duration(milliseconds: 500));
      expect(controller.selection, const TextSelection.collapsed(offset: 15));
    });

    testWidgets('has no magnifier and no handle under the cursor', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello');
      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.magnifierConfiguration, TextMagnifierConfiguration.disabled);
      controller.selection = const TextSelection.collapsed(offset: 2);
      expect(field.selectionControls?.getHandleSize(20), Size.zero);
      controller.selection = const TextSelection(
        baseOffset: 0,
        extentOffset: 2,
      );
      expect(field.selectionControls?.getHandleSize(20), isNot(Size.zero));
    });

    testWidgets('a drag across the text does not move the cursor', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello wonderful world');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 0);
      await tester.pump();

      await slide(tester, caretAt(tester, 2), const Offset(120, 0));
      expect(controller.selection, const TextSelection.collapsed(offset: 0));
    });

    testWidgets('a fast sideways swipe does not move the cursor either', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello wonderful world');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 0);
      await tester.pump();

      // The first move already goes 20 px, past the usual slop.
      await tester.dragFrom(caretAt(tester, 2), const Offset(120, 0));
      await tester.pumpAndSettle();
      expect(controller.selection, const TextSelection.collapsed(offset: 0));
      final finger = await tester.startGesture(caretAt(tester, 3));
      await finger.moveBy(const Offset(60, 0));
      await tester.pump();
      await finger.moveBy(const Offset(60, 0));
      await finger.up();
      await tester.pumpAndSettle();
      expect(controller.selection, const TextSelection.collapsed(offset: 0));
    });

    testWidgets('a drag up the text still scrolls the page', (tester) async {
      await pumpField(tester, text: List.filled(80, 'line').join('\n'));
      await openKeyboard(tester);
      final page = find.byType(Scrollable).first;
      final position = tester.state<ScrollableState>(page).position;
      tester.testTextInput.updateEditingValue(
        tester
            .widget<TextField>(find.byType(TextField))
            .controller!
            .value
            .copyWith(selection: const TextSelection.collapsed(offset: 0)),
      );
      await tester.pumpAndSettle();
      position.jumpTo(0);
      await tester.pump();

      await slide(tester, caretAt(tester, 2), const Offset(5, -200));
      expect(
        tester.state<ScrollableState>(page).position.pixels,
        greaterThan(100),
      );
    });

    testWidgets('a long press still selects a word, with Delete in the menu', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello wonderful world');
      await openKeyboard(tester);

      await tester.longPressAt(caretAt(tester, 9));
      await tester.pumpAndSettle();
      expect(
        controller.selection,
        const TextSelection(baseOffset: 6, extentOffset: 15),
      );
      expect(find.text('Copy'), findsOneWidget);
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();
      expect(controller.text, 'hello  world');
      expect(controller.selection, const TextSelection.collapsed(offset: 6));
    });
  });

  group('selecting in the bottom row', () {
    testWidgets('starts without moving the cursor and moves only one end', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello world');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 6);
      await tester.pump();
      Color? background(Finder button) =>
          (tester
                      .widget<DecoratedBox>(
                        find.descendant(
                          of: button,
                          matching: find.byType(DecoratedBox),
                        ),
                      )
                      .decoration
                  as BoxDecoration)
              .color;
      expect(background(forward), isNull);

      // Wherever the long press lands, the cursor stays.
      await tester.longPressAt(tester.getTopLeft(line) + const Offset(4, 8));
      await tester.pump();
      expect(controller.selection, const TextSelection.collapsed(offset: 6));
      final primary = Theme.of(tester.element(line)).colorScheme.primary;
      expect(background(forward), primary);
      expect(background(back), primary);

      await press(tester, forward);
      await press(tester, forward);
      expect(
        controller.selection,
        const TextSelection(baseOffset: 6, extentOffset: 8),
      );
      await tester.pumpAndSettle();
      expect(find.text('Copy'), findsOneWidget);
      expect(find.text('Cut'), findsOneWidget);
      expect(find.text('Delete'), findsOneWidget);

      await press(tester, back);
      await press(tester, back);
      await press(tester, back);
      expect(
        controller.selection,
        const TextSelection(baseOffset: 6, extentOffset: 5),
      );

      // Another long press ends it with the cursor at the moving end.
      await tester.longPress(line);
      await tester.pumpAndSettle();
      expect(controller.selection, const TextSelection.collapsed(offset: 5));
      expect(background(forward), isNull);
      await press(tester, forward);
      expect(controller.selection, const TextSelection.collapsed(offset: 6));
    });

    testWidgets('typing replaces the selection and ends selecting', (
      tester,
    ) async {
      final controller = await pumpField(tester, text: 'hello world');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 6);
      await tester.pump();
      await tester.longPress(line);
      await tester.pump();
      for (var i = 0; i < 5; i++) {
        await press(tester, forward);
      }
      expect(
        controller.selection,
        const TextSelection(baseOffset: 6, extentOffset: 11),
      );

      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'hello there',
          selection: TextSelection.collapsed(offset: 11),
        ),
      );
      await tester.pump();
      expect(controller.text, 'hello there');
      await press(tester, back);
      expect(
        controller.selection,
        const TextSelection.collapsed(offset: 10),
        reason: 'no longer selecting',
      );
    });

    testWidgets('Delete in the menu removes the selection', (tester) async {
      final controller = await pumpField(tester, text: 'hello world');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 5);
      await tester.pump();
      await tester.longPress(line);
      await tester.pump();
      for (var i = 0; i < 6; i++) {
        await press(tester, forward);
      }
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();
      expect(controller.text, 'hello');
      await press(tester, back);
      expect(controller.selection, const TextSelection.collapsed(offset: 4));
    });
  });

  group('with a screen reader', () {
    testWidgets('the rows and buttons are labelled and usable', (tester) async {
      final semantics = tester.ensureSemantics();
      final spoken = <String>[];
      tester.binding.defaultBinaryMessenger.setMockDecodedMessageHandler<
        dynamic
      >(SystemChannels.accessibility, (message) async {
        final data = (message as Map)['data'] as Map?;
        if (message['type'] == 'announce') spoken.add('${data?['message']}');
        return null;
      });
      addTearDown(
        () => tester.binding.defaultBinaryMessenger
            .setMockDecodedMessageHandler<dynamic>(
              SystemChannels.accessibility,
              null,
            ),
      );
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(accessibleNavigation: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      final controller = await pumpField(tester, text: 'a b');
      await openKeyboard(tester);
      controller.selection = const TextSelection.collapsed(offset: 0);
      await tester.pump();

      expect(
        tester.getSemantics(find.byType(EditableText)),
        isSemantics(isTextField: true, value: 'a b'),
      );
      tester.semantics.tap(
        find.semantics.byLabel('Cursor forward one character'),
      );
      await tester.pump();
      tester.semantics.tap(
        find.semantics.byLabel('Cursor forward one character'),
      );
      await tester.pump();
      expect(controller.selection.baseOffset, 2);
      expect(spoken, ['a', 'Space']);
      expect(
        tester.getSemantics(line),
        matchesSemantics(
          label: 'Text around the cursor',
          value: 'a  | b',
          hasLongPressAction: true,
        ),
      );

      tester.semantics.longPress(
        find.semantics.byLabel('Text around the cursor'),
      );
      await tester.pump();
      tester.semantics.tap(
        find.semantics.byLabel('Selection end back one character'),
      );
      await tester.pump();
      expect(
        controller.selection,
        const TextSelection(baseOffset: 2, extentOffset: 1),
      );
      semantics.dispose();
    });
  });

  group('the setting', () {
    testWidgets('off gives the plain system text field back', (tester) async {
      await pumpField(tester, text: 'hello', twoRows: false);
      await openKeyboard(tester);
      expect(bar, findsNothing);
      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.magnifierConfiguration, isNull);
      expect(field.selectionControls, isNull);
    });

    late Store store;
    late Notes notes;
    setUp(() {
      store = Store.inMemory()..starterSetOffered = true;
      notes = Notes(store);
    });
    tearDown(() => store.close());

    Future<void> openEditor(WidgetTester tester) async {
      notes.capture('buy sauce');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      await tester.longPress(find.text('buy sauce'));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    }

    testWidgets('is on by default in the Note editor and in Amend', (
      tester,
    ) async {
      await openEditor(tester);
      await openKeyboard(tester);
      expect(bar, findsOneWidget);
      await press(tester, back);
      expect(
        tester
            .widget<TextField>(find.byType(TextField))
            .controller
            ?.selection
            .baseOffset,
        'buy sauce'.length - 1,
      );

      // Amend asks in a dialog, its field focused at once.
      tester.view.resetViewInsets();
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('Add to Note'),
        200,
        scrollable: find
            .descendant(
              of: find.byType(NoteEditorScreen),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      await tester.tap(find.text('Add to Note'));
      await tester.pumpAndSettle();
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      await tester.pumpAndSettle();
      expect(bar, findsOneWidget);
      final dialog = find
          .descendant(
            of: find.byType(AlertDialog),
            matching: find.byType(Material),
          )
          .first;
      expect(
        tester.getBottomLeft(dialog).dy,
        lessThanOrEqualTo(tester.getTopLeft(bar).dy),
        reason: 'the dialog stays clear of the row',
      );
      await tester.enterText(find.byType(TextField).last, 'bought');
      await press(tester, back);
      final amend = tester.widget<TextField>(find.byType(TextField).last);
      expect(amend.controller?.selection.baseOffset, 5);
    });

    testWidgets('off in this Device\'s settings, the editor has one row', (
      tester,
    ) async {
      store.twoRowEditorOn = false;
      await openEditor(tester);
      await openKeyboard(tester);
      expect(bar, findsNothing);
    });
  });

  test('grapheme and word boundaries', () {
    expect(graphemeAfter('a👍🏽b', 1), 5);
    expect(graphemeBefore('a👍🏽b', 5), 1);
    expect(graphemeBefore('a👍🏽b', 0), 0);
    expect(graphemeAfter('ab', 2), 2);
    expect(wordEdgeNear('hello world', 2), 0);
    expect(wordEdgeNear('hello world', 4), 5);
    expect(wordEdgeNear('hello world', 7), 6);
    expect(wordEdgeNear('hello world', 10), 11);
    expect(wordEdgeNear('hello', 0), 0);
    expect(wordEdgeNear('', 0), 0);
  });
}

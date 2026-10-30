import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';

/// Formatted text (#127, ADR 0033): drawn formatted on cards, in Sections
/// and in the editor, whose bar above the keyboard formats the selection
/// or the word at the cursor; a plain text becomes formatted at its first
/// format, a typed shortcut or a paste that uses one, and stays plain
/// otherwise. Links open only to the web or e-mail.
void main() {
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

  Finder key(String value) => find.byKey(ValueKey(value));

  /// Every span drawn under [of], depth first.
  List<InlineSpan> spansUnder(WidgetTester tester, Finder of) {
    final spans = <InlineSpan>[];
    for (final paragraph in tester.widgetList<RichText>(
      find.descendant(of: of, matching: find.byType(RichText)),
    )) {
      paragraph.text.visitChildren((span) {
        spans.add(span);
        return true;
      });
    }
    return spans;
  }

  /// The style of the span under [of] that shows exactly [text].
  TextStyle? styleOf(WidgetTester tester, Finder of, String text) => spansUnder(
    tester,
    of,
  ).whereType<TextSpan>().firstWhere((s) => s.text == text).style;

  /// The style of the span of the text field [of] that shows exactly
  /// [text].
  TextStyle? fieldStyleOf(WidgetTester tester, Finder of, String text) {
    final spans = <TextSpan>[];
    tester.state<EditableTextState>(of).buildTextSpan().visitChildren((span) {
      if (span is TextSpan) spans.add(span);
      return true;
    });
    return spans.firstWhere((s) => s.text == text).style;
  }

  /// Everything drawn under [of] as one string.
  String shownUnder(WidgetTester tester, Finder of) => [
    for (final paragraph in tester.widgetList<RichText>(
      find.descendant(of: of, matching: find.byType(RichText)),
    ))
      paragraph.text.toPlainText(includeSemanticsLabels: false),
  ].join('|');

  group('drawn formatted', () {
    final opened = <Uri>[];

    Future<void> pump(
      WidgetTester tester,
      String text, {
      String format = TextFormats.markdown,
      MarkSwap? onMark,
      TagSearch? onSearch,
      int? start,
      int? end,
      int? maxLines,
    }) async {
      opened.clear();
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: LinkOpenScope(
              onOpen: (address) async => opened.add(address),
              child: TagSearchScope(
                onSearch: onSearch,
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: MarkedText(
                    text,
                    key: const ValueKey('text'),
                    format: format,
                    start: start,
                    end: end,
                    maxLines: maxLines,
                    style: const TextStyle(fontSize: 16),
                    onMark: onMark,
                  ),
                ),
              ),
            ),
          ),
        ),
      );
    }

    final text = find.byKey(const ValueKey('text'));

    testWidgets('each format shows without its marks', (tester) async {
      final semantics = tester.ensureSemantics();
      await pump(
        tester,
        '# Big\n## Small\n**bold** *italic* ~~gone~~ `x*y`\n> said\n\nend',
      );
      expect(styleOf(tester, text, 'Big')?.fontSize, 24);
      expect(styleOf(tester, text, 'Big')?.fontWeight, FontWeight.bold);
      expect(styleOf(tester, text, 'Small')?.fontSize, 20);
      expect(styleOf(tester, text, 'bold')?.fontWeight, FontWeight.bold);
      expect(styleOf(tester, text, 'italic')?.fontStyle, FontStyle.italic);
      expect(
        styleOf(tester, text, 'gone')?.decoration,
        TextDecoration.lineThrough,
      );
      final code = styleOf(tester, text, 'x*y');
      expect(code?.fontFamily, 'monospace');
      expect(code?.backgroundColor, isNotNull);
      expect(styleOf(tester, text, 'said'), isNotNull);
      final shown = shownUnder(tester, text);
      expect(shown, isNot(contains('**')));
      expect(shown, isNot(contains('#')));
      expect(shown, isNot(contains('~~')));
      expect(shown, isNot(contains('>')));
      // A blank line keeps its height.
      final lines = tester.widgetList<RichText>(
        find.descendant(of: text, matching: find.byType(RichText)),
      );
      expect(lines.length, 6);
      final blank = find
          .descendant(of: text, matching: find.byType(RichText))
          .at(4);
      expect(tester.getSize(blank).height, greaterThan(10));
      // A heading is read as one.
      expect(
        tester.getSemantics(find.text('Big', findRichText: true)),
        isSemantics(isHeader: true),
      );
      semantics.dispose();
    });

    testWidgets('a list shows bullets and its numbers counted on, one '
        'level in', (tester) async {
      await pump(tester, '- milk\n  - oat\n1. one\n1. two\n  1. inner');
      final shown = shownUnder(tester, text);
      expect(shown, contains('•|milk'));
      expect(shown, contains('◦|oat'));
      expect(shown, contains('1.|one'));
      expect(shown, contains('2.|two'));
      expect(shown, contains('1.|inner'));
      expect(shown, isNot(contains('- ')));
      expect(
        tester.getTopLeft(find.text('◦')).dx,
        greaterThan(tester.getTopLeft(find.text('•')).dx),
      );
    });

    testWidgets('a plain text shows as written', (tester) async {
      await pump(tester, '# not\n**kept**', format: TextFormats.plain);
      expect(find.text('# not\n**kept**'), findsOneWidget);
    });

    testWidgets('a compact line keeps the size of the text', (tester) async {
      const source = '# Big title\nmore';
      final (start, end) = NoteCard.firstLine(source);
      await pump(tester, source, start: start, end: end, maxLines: 1);
      expect(shownUnder(tester, text), 'Big title');
      expect(styleOf(tester, text, 'Big title')?.fontSize, 16);
      expect(styleOf(tester, text, 'Big title')?.fontWeight, FontWeight.bold);
    });

    testWidgets('a link to the web or e-mail opens; any other stays text', (
      tester,
    ) async {
      await pump(
        tester,
        '[site](https://example.org/a) [mail](MAILTO:a@b.c) '
        '[app](intent://x) [file](file:///etc/passwd) [js](javascript:x)',
      );
      final linked = spansUnder(tester, text)
          .whereType<TextSpan>()
          .where((s) => s.recognizer != null)
          .map((s) => s.text)
          .toList();
      expect(linked, ['site', 'mail']);
      // A finger on the start of the text: on "site".
      final paragraph = find
          .descendant(of: text, matching: find.byType(RichText))
          .first;
      await tester.tapAt(tester.getTopLeft(paragraph) + const Offset(6, 8));
      await tester.pump();
      expect(opened, [Uri.parse('https://example.org/a')]);
      expect(
        styleOf(tester, text, 'site')?.decoration,
        TextDecoration.underline,
      );
      for (final inert in ['app', 'file', 'js']) {
        expect(
          styleOf(tester, text, inert)?.decoration,
          isNot(TextDecoration.underline),
        );
      }
      final mail = spansUnder(
        tester,
        text,
      ).whereType<TextSpan>().firstWhere((s) => s.text == 'mail');
      (mail.recognizer! as TapGestureRecognizer).onTap!();
      await tester.pump();
      expect(opened, [
        Uri.parse('https://example.org/a'),
        Uri.parse('MAILTO:a@b.c'),
      ]);
    });

    testWidgets('status marks stay buttons and tags stay links inside it', (
      tester,
    ) async {
      final marked = <(int, TextMark)>[];
      final searched = <String>[];
      await pump(
        tester,
        '# Plan #Garten\n- ☐ **seeds** for #Beet\n`#code` [#tag](https://x.y)',
        onMark: (line, to, {column}) => marked.add((line, to)),
        onSearch: (_, query) => searched.add(query),
      );
      expect(key('mark-1'), findsOneWidget);
      await tester.tap(key('mark-1'));
      await tester.pumpAndSettle();
      await tester.tap(key('mark-checked'));
      await tester.pumpAndSettle();
      expect(marked, [(1, TextMark.checked)]);

      // Tags in code and in a link's label are none of their own.
      final tags = find.byWidgetPredicate(
        (w) => w.key is ValueKey<String> && '${w.key}'.contains("'tag-"),
      );
      expect(tags, findsNWidgets(2));
      await tester.tap(key('tag-7'));
      expect(searched, ['#Garten']);
    });
  });

  group('on a card', () {
    Future<void> start(WidgetTester tester) async {
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
    }

    testWidgets('a formatted Note and its Section show formatted, a plain '
        'one as written', (tester) async {
      final id = notes
          .capture('## Shop\n- **milk**', format: TextFormats.markdown)
          .id;
      notes.amend(id, '*later*', format: TextFormats.markdown);
      notes.capture('plain **as is**');
      await start(tester);
      final cards = find.byType(NoteCard);
      final shown = shownUnder(tester, cards);
      expect(shown, contains('Shop'));
      expect(shown, contains('•|milk'));
      expect(shown, contains('later'));
      expect(shown, isNot(contains('## ')));
      expect(shown, contains('plain **as is**'));
      expect(styleOf(tester, cards, 'milk')?.fontWeight, FontWeight.bold);
      expect(styleOf(tester, cards, 'later')?.fontStyle, FontStyle.italic);
    });
  });

  group('Capture', () {
    Future<void> captureText(WidgetTester tester, String text) async {
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), text);
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
    }

    testWidgets('a text that uses a format is kept formatted, and its card '
        'on the Agenda shows it so', (tester) async {
      await captureText(tester, '- milk\n- **oat**');
      final note = notes.all().single;
      expect(note.text, '- milk\n- **oat**');
      expect(note.textFormat, TextFormats.markdown);
      final card = find.byType(NoteCard);
      expect(shownUnder(tester, card), contains('•|milk'));
    });

    testWidgets('any other stays plain', (tester) async {
      await captureText(tester, 'call 2*3 people');
      expect(notes.all().single.textFormat, TextFormats.plain);
    });
  });

  group('the bar of formats', () {
    final bar = key('format-bar');

    /// A field with formats holding [text], the keyboard open.
    Future<TagTextController> pumpField(
      WidgetTester tester, {
      String text = '',
      bool twoRows = true,
      bool markdown = false,
      void Function()? onFormatted,
    }) async {
      final controller = TagTextController(text: text, hideMarks: twoRows)
        ..markdown = markdown;
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ListView(
              children: [
                TwoRowTextField(
                  controller: controller,
                  twoRows: twoRows,
                  marks: true,
                  formats: true,
                  onFormatted: onFormatted,
                ),
              ],
            ),
          ),
        ),
      );
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
      return controller;
    }

    Future<void> press(WidgetTester tester, String button) async {
      await tester.tap(key(button));
      await tester.pumpAndSettle();
    }

    testWidgets('sits above the bottom row on the keyboard, ☐ first', (
      tester,
    ) async {
      await pumpField(tester, text: 'hello');
      expect(bar, findsOneWidget);
      final row = key('two-row-bar');
      expect(tester.getBottomLeft(bar).dy, tester.getTopLeft(row).dy);
      expect(
        tester.getTopLeft(key('two-row-insert-mark')).dx,
        lessThan(tester.getTopLeft(key('format-bold')).dx),
      );
    });

    testWidgets('wraps the word at the cursor, which makes the text '
        'formatted, and unwraps it again', (tester) async {
      final semantics = tester.ensureSemantics();
      var formatted = 0;
      final controller = await pumpField(
        tester,
        text: 'buy milk now',
        onFormatted: () => formatted++,
      );
      controller.selection = const TextSelection.collapsed(offset: 6);
      await tester.pump();
      await press(tester, 'format-bold');
      expect(controller.text, 'buy **milk** now');
      expect(controller.markdown, isTrue);
      expect(formatted, 1);
      expect(
        tester.getSemantics(key('format-bold')),
        isSemantics(
          label: 'Bold',
          isButton: true,
          hasToggledState: true,
          isToggled: true,
        ),
      );
      expect(
        tester.getSemantics(key('format-italic')),
        isSemantics(
          label: 'Italic',
          isButton: true,
          hasToggledState: true,
          isToggled: false,
        ),
      );
      await press(tester, 'format-bold');
      expect(controller.text, 'buy milk now');
      // Formatted for good.
      expect(controller.markdown, isTrue);
      expect(formatted, 1);
      await press(tester, 'format-strike');
      expect(controller.text, 'buy ~~milk~~ now');
      semantics.dispose();
    });

    testWidgets('wraps a selection and formats the lines it touches', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      final controller = await pumpField(tester, text: 'one two\nthree');
      controller.selection = const TextSelection(
        baseOffset: 0,
        extentOffset: 7,
      );
      await tester.pump();
      await press(tester, 'format-italic');
      expect(controller.text, '*one two*\nthree');
      controller.selection = const TextSelection(
        baseOffset: 0,
        extentOffset: 12,
      );
      await tester.pump();
      await press(tester, 'format-bullet');
      expect(controller.text, '- *one two*\n- three');
      expect(
        tester.getSemantics(key('format-bullet')),
        isSemantics(hasToggledState: true, isToggled: true),
      );
      await press(tester, 'format-indent');
      expect(controller.text, '  - *one two*\n  - three');
      await press(tester, 'format-outdent');
      expect(controller.text, '- *one two*\n- three');
      controller.selection = const TextSelection.collapsed(offset: 1);
      await tester.pump();
      await press(tester, 'format-heading1');
      expect(controller.text, '# *one two*\n- three');
      await press(tester, 'format-heading2');
      expect(controller.text, '## *one two*\n- three');
      await press(tester, 'format-quote');
      expect(controller.text, '> *one two*\n- three');
      await press(tester, 'format-numbered');
      expect(controller.text, '1. *one two*\n- three');
      controller.selection = const TextSelection.collapsed(offset: 17);
      await tester.pump();
      await press(tester, 'format-code');
      expect(controller.text, '1. *one two*\n- `three`');
      semantics.dispose();
    });

    testWidgets('asks for a link\'s address, taking one written without '
        'its kind as the web\'s', (tester) async {
      final controller = await pumpField(tester, text: 'see the site');
      controller.selection = const TextSelection.collapsed(offset: 10);
      await tester.pump();
      await press(tester, 'format-link');
      expect(find.text('Web or e-mail address'), findsOneWidget);
      await tester.enterText(key('link-address'), 'example.org');
      await press(tester, 'link-done');
      expect(controller.text, 'see the [site](https://example.org)');
      expect(controller.markdown, isTrue);

      // In the link: its address offered, an empty one removes it.
      controller.selection = const TextSelection.collapsed(offset: 11);
      await tester.pump();
      await press(tester, 'format-link');
      expect(find.text('https://example.org'), findsOneWidget);
      await tester.enterText(key('link-address'), '');
      await press(tester, 'link-done');
      expect(controller.text, 'see the site');
    });

    testWidgets('a narrow screen keeps the rest in ⋮, opened at the '
        'finger', (tester) async {
      tester.view.physicalSize = const Size(360, 2000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final controller = await pumpField(tester, text: 'line');
      expect(key('format-more'), findsOneWidget);
      expect(key('format-outdent'), findsNothing);
      controller.selection = const TextSelection.collapsed(offset: 2);
      await tester.pump();
      await press(tester, 'format-more');
      await press(tester, 'format-code-more');
      expect(controller.text, '`line`');
    });

    testWidgets('a shortcut typed makes the text formatted; plain typing '
        'keeps it plain', (tester) async {
      final controller = await pumpField(tester);
      await tester.enterText(find.byType(TextField), 'buy 2*3');
      await tester.pump();
      expect(controller.markdown, isFalse);
      await tester.enterText(find.byType(TextField), 'buy 2*3 milk');
      expect(controller.markdown, isFalse);
      await tester.enterText(find.byType(TextField), 'buy 2*3 milk\n- ');
      await tester.pump();
      expect(controller.markdown, isTrue);
    });

    testWidgets('an inline shortcut ends where the cursor is', (tester) async {
      final controller = await pumpField(tester, text: 'it is *big');
      controller.selection = const TextSelection.collapsed(offset: 10);
      await tester.pump();
      await tester.enterText(find.byType(TextField), 'it is *big*');
      await tester.pump();
      expect(controller.markdown, isTrue);
    });

    Future<void> paste(WidgetTester tester, String text) async {
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async => switch (call.method) {
          'Clipboard.getData' => {'text': text},
          'Clipboard.hasStrings' => {'value': true},
          _ => null,
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await tester
          .state<EditableTextState>(find.byType(EditableText))
          .pasteText(SelectionChangedCause.toolbar);
      await tester.pumpAndSettle();
    }

    testWidgets('a paste that uses a format makes the text formatted, as '
        'it is', (tester) async {
      final controller = await pumpField(tester, text: 'list 2*3*4: ');
      controller.selection = const TextSelection.collapsed(offset: 12);
      await tester.pump();
      await paste(tester, 'plain words');
      expect(controller.markdown, isFalse);
      await paste(tester, '\n- **milk**');
      expect(controller.text, 'list 2*3*4: plain words\n- **milk**');
      expect(controller.markdown, isTrue);
    });

    testWidgets('in two rows the marks are drawn as nothing on top and the '
        'bottom row shows the source of the cursor\'s line', (tester) async {
      final controller = await pumpField(
        tester,
        text: 'first line\nbuy **milk** today',
        markdown: true,
      );
      controller.selection = const TextSelection.collapsed(offset: 17);
      await tester.pump();
      final field = find.byType(EditableText);
      final hidden = fieldStyleOf(tester, field, '**');
      expect(hidden?.color, Colors.transparent);
      expect(hidden?.fontSize, lessThan(1));
      expect(fieldStyleOf(tester, field, 'milk')?.fontWeight, FontWeight.bold);
      final before = tester.widget<RichText>(
        find.descendant(
          of: key('two-row-before'),
          matching: find.byType(RichText),
        ),
      );
      final after = tester.widget<RichText>(
        find.descendant(
          of: key('two-row-after'),
          matching: find.byType(RichText),
        ),
      );
      expect(before.text.toPlainText(), 'buy **');
      expect(after.text.toPlainText(), 'milk** today');
    });

    testWidgets('with two rows off the marks are dimmed, and the bar is '
        'there', (tester) async {
      await pumpField(tester, text: '**milk**', markdown: true, twoRows: false);
      expect(bar, findsOneWidget);
      expect(key('two-row-bar'), findsNothing);
      final field = find.byType(EditableText);
      final dimmed = fieldStyleOf(tester, field, '**');
      expect(dimmed?.color, isNot(Colors.transparent));
      expect(dimmed?.fontSize, isNull);
    });
  });

  group('the Note editor', () {
    Future<void> openEditor(WidgetTester tester) async {
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      await tester.longPress(find.byType(NoteCard).first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      await tester.showKeyboard(find.byType(TextField).first);
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
    }

    TextEditingController field(WidgetTester tester) =>
        tester.widget<TextField>(find.byType(TextField).first).controller!;

    testWidgets('a plain Note stays plain without a format', (tester) async {
      final id = notes.capture('buy milk').id;
      await openEditor(tester);
      await tester.enterText(find.byType(TextField).first, 'buy oat milk');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, 'buy oat milk');
      expect(notes.byId(id)!.textFormat, TextFormats.plain);
    });

    testWidgets('its first format makes it formatted at once, and it stays '
        'so', (tester) async {
      final id = notes.capture('buy milk').id;
      await openEditor(tester);
      field(tester).selection = const TextSelection.collapsed(offset: 5);
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('format-bold')));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, 'buy **milk**');
      expect(notes.byId(id)!.textFormat, TextFormats.markdown);

      await tester.tap(find.byKey(const ValueKey('format-bold')));
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, 'buy milk');
      expect(notes.byId(id)!.textFormat, TextFormats.markdown);
    });

    testWidgets('a formatted Note opens formatted', (tester) async {
      notes.capture('- **milk**', format: TextFormats.markdown);
      await openEditor(tester);
      expect((field(tester) as TagTextController).markdown, isTrue);
      expect(
        fieldStyleOf(
          tester,
          find.byType(EditableText).first,
          'milk',
        )?.fontWeight,
        FontWeight.bold,
      );
    });

    testWidgets('Amend writes a formatted Section', (tester) async {
      final id = notes.capture('buy milk').id;
      await openEditor(tester);
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
      await tester.enterText(find.byType(TextField).last, 'and *oat*');
      await tester.pump();
      await tester.tap(find.text('Add'));
      await tester.pumpAndSettle();
      final section = notes.sections(id).single;
      expect(section.text, 'and *oat*');
      expect(section.textFormat, TextFormats.markdown);
    });
  });
}

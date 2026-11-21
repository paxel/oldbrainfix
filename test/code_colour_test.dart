import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Code coloured by its language (obf-1036): a Code text by the language
/// its Note stores, a code block by the one its fence names; without
/// one, or with one not known, in one colour as before.
void main() {
  late Store store;
  late Notes notes;

  setUpAll(initializeDateFormatting);

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 20, 8))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  const rust = 'fn main() {\n    // a comment\n    let n = 42;\n}';

  Widget app(Widget child) => MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );

  final palette = {
    for (final colour in CodeColour.values)
      codeColour(colour, Brightness.light),
  };

  /// Each part of [span] in a colour of code, its text and colour.
  List<(String, Color)> coloured(InlineSpan span) {
    final parts = <(String, Color)>[];
    span.visitChildren((child) {
      if (child case TextSpan(:final text?, :final style?)
          when palette.contains(style.color)) {
        parts.add((text, style.color!));
      }
      return true;
    });
    return parts;
  }

  /// What [scope]'s grammar colours in [text], in the light look.
  List<(String, Color)> expected(String scope, String text) => [
    for (final span in Grammars.builtIn.byScope(scope)!.colour(text))
      (
        text.substring(span.start, span.end),
        codeColour(span.colour, Brightness.light),
      ),
  ];

  InlineSpan drawn(WidgetTester tester, Finder of) => tester
      .widget<RichText>(
        find.descendant(of: of, matching: find.byType(RichText)).first,
      )
      .text;

  group('a Code text', () {
    testWidgets('in Rust shows its parts in their colours', (tester) async {
      expect(expected('source.rust', rust), isNotEmpty);
      await tester.pumpWidget(
        app(
          const MarkedText(
            rust,
            format: TextFormats.code,
            language: 'source.rust',
          ),
        ),
      );
      final span = drawn(tester, key('code-text'));
      expect(span.toPlainText(), rust);
      expect(coloured(span), expected('source.rust', rust));
      // Only the glyphs' colour changes: monospace on the tint as before.
      expect(
        tester.widget<Text>(find.byType(Text)).style?.fontFamily,
        'monospace',
      );
    });

    testWidgets('without a language or with one not known is in one '
        'colour', (tester) async {
      for (final language in [null, 'source.nothing-known']) {
        await tester.pumpWidget(
          app(MarkedText(rust, format: TextFormats.code, language: language)),
        );
        final text = tester.widget<Text>(
          find.descendant(of: key('code-text'), matching: find.byType(Text)),
        );
        expect(text.data, rust);
        expect(coloured(drawn(tester, key('code-text'))), isEmpty);
      }
    });
  });

  group('a code block', () {
    testWidgets('whose fence names a known language is coloured by it', (
      tester,
    ) async {
      await tester.pumpWidget(
        app(
          const MarkedText(
            'Before\n```rs\n$rust\n```',
            format: TextFormats.markdown,
          ),
        ),
      );
      final span = drawn(tester, key('code-1'));
      expect(span.toPlainText(), rust);
      expect(coloured(span), expected('source.rust', rust));
    });

    testWidgets('whose fence names none or one not known is in one colour', (
      tester,
    ) async {
      for (final fence in ['```', '```nothing-known']) {
        await tester.pumpWidget(
          app(
            MarkedText(
              'Before\n$fence\n$rust\n```',
              format: TextFormats.markdown,
            ),
          ),
        );
        final text = tester.widget<Text>(
          find.descendant(of: key('code-1'), matching: find.byType(Text)),
        );
        expect(text.data, rust);
      }
    });

    testWidgets('is coloured in the editor field too', (tester) async {
      final text = '```rust\n$rust\n```';
      final controller = TagTextController(text: text)..markdown = true;
      addTearDown(controller.dispose);
      await tester.pumpWidget(app(TextField(controller: controller)));
      final span = tester
          .state<EditableTextState>(find.byType(EditableText))
          .buildTextSpan();
      expect(span.toPlainText(), text);
      final colours = coloured(span).map((p) => p.$2).toSet();
      for (final (_, colour) in expected('source.rust', rust)) {
        expect(colours, contains(colour));
      }
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
    }

    TagTextController field(WidgetTester tester) =>
        tester.widget<TextField>(find.byType(TextField).first).controller!
            as TagTextController;

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
          .state<EditableTextState>(find.byType(EditableText).first)
          .pasteText(SelectionChangedCause.toolbar);
      await tester.pumpAndSettle();
    }

    testWidgets('the text format badge picks the language and stores it', (
      tester,
    ) async {
      final id = notes.capture(rust, format: TextFormats.code).id;
      await openEditor(tester);
      expect(field(tester).codeLanguage, isNull);
      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      expect(find.text('Language'), findsOneWidget);
      expect(key('code-language-none'), findsOneWidget);
      for (final grammar in Grammars.builtIn.all) {
        expect(key('code-language-${grammar.scope}'), findsOneWidget);
      }
      await tester.ensureVisible(key('code-language-source.rust'));
      await tester.tap(key('code-language-source.rust'));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.codeLanguage, 'source.rust');
      expect(notes.byId(id)!.textFormat, TextFormats.code);
      expect(field(tester).codeLanguage, 'source.rust');
      final span = tester
          .state<EditableTextState>(find.byType(EditableText).first)
          .buildTextSpan();
      expect(span.toPlainText(), rust);
      expect(coloured(span), isNotEmpty);

      // No language again.
      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      await tester.tap(key('code-language-none'));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.codeLanguage, isNull);
      expect(field(tester).codeLanguage, isNull);
    });

    testWidgets('a text not code offers no language', (tester) async {
      notes.capture('words');
      await openEditor(tester);
      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      expect(key('code-language-none'), findsNothing);
    });

    testWidgets('Keep as code picks the language a #!/bin/bash names', (
      tester,
    ) async {
      final id = notes.capture('x').id;
      await openEditor(tester);
      await tester.showKeyboard(find.byType(TextField).first);
      field(tester).selection = const TextSelection.collapsed(offset: 1);
      await tester.pump();
      // Its first line names the language, the text's does not.
      await paste(tester, '#!/bin/bash\necho hi');
      expect(key('code-offer'), findsOneWidget);
      await tester.tap(key('keep-as-code'));
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.textFormat, TextFormats.code);
      expect(note.codeLanguage, 'source.shell');
      expect(field(tester).codeLanguage, 'source.shell');
    });

    testWidgets('Keep as code without a guess picks no language', (
      tester,
    ) async {
      final id = notes.capture('x').id;
      await openEditor(tester);
      await tester.showKeyboard(find.byType(TextField).first);
      field(tester).selection = const TextSelection.collapsed(offset: 1);
      await tester.pump();
      await paste(tester, '\nif (x) {\n\treturn;\n}');
      await tester.tap(key('keep-as-code'));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.textFormat, TextFormats.code);
      expect(notes.byId(id)!.codeLanguage, isNull);
    });
  });
}

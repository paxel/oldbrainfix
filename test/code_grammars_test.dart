import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/l10n/app_localizations_en.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_files.dart';
import 'fake_profile.dart';
import 'open_settings.dart';

/// The person's own grammars (obf-1037): one added colours code of its
/// language and is offered in the text format badge; one of a built-in
/// language's scope colours in its place until Remove; Settings lists
/// them with Remove, adds one picked with the system's picker and says
/// in red why a file was refused.
void main() {
  final t = AppLocalizationsEn();
  late Store store;
  late Notes notes;
  late Directory dir;
  late FakeFiles files;

  setUpAll(initializeDateFormatting);

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 20, 8))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('code_grammars_');
    files = FakeFiles(dir);
  });
  tearDown(() {
    store.close();
    dir.deleteSync(recursive: true);
  });

  Finder key(String value) => find.byKey(ValueKey(value));

  const lua = '''
%YAML 1.2
---
name: Lua
scope: source.lua
file_extensions: [lua]
contexts:
  main:
    - match: '--.*'
      scope: comment.line.lua
    - match: '\\b(local|function|end|return)\\b'
      scope: keyword.control.lua
''';

  // Rust's scope: in the built-in one's place, `fn` a string.
  const myRust = '''
%YAML 1.2
---
name: My Rust
scope: source.rust
file_extensions: [rs, rust]
contexts:
  main:
    - match: '\\bfn\\b'
      scope: string.quoted.rust
''';

  // A possessive quantifier: beyond what the cores read.
  const beyond = '''
%YAML 1.2
---
name: Beyond
scope: source.beyond
contexts:
  main:
    - match: 'a++'
      scope: keyword.beyond
''';

  const luaCode = 'local n = 1 -- one\nreturn n';
  const rustCode = 'fn main() {\n    // a comment\n    let n = 42;\n}';

  Uint8List bytesOf(String text) => Uint8List.fromList(utf8.encode(text));

  String reasonOf(String source) {
    try {
      Grammar.parse(source);
    } on GrammarRefused catch (refused) {
      return refused.reason;
    }
    fail('$source was read');
  }

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

  /// What [grammar] colours in [text], in the light look.
  List<(String, Color)> expected(Grammar grammar, String text) => [
    for (final span in grammar.colour(text))
      (
        text.substring(span.start, span.end),
        codeColour(span.colour, Brightness.light),
      ),
  ];

  InlineSpan drawn(WidgetTester tester) => tester
      .widget<RichText>(
        find
            .descendant(
              of: find.byType(MarkedText),
              matching: find.byType(RichText),
            )
            .first,
      )
      .text;

  /// [child] in the app's place of the person's store, as the app has it.
  Future<void> show(WidgetTester tester, Widget child) => tester.pumpWidget(
    MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => PersonScope(
        store: store,
        platform: FakeProfilePlatform(),
        child: child ?? const SizedBox.shrink(),
      ),
      home: Scaffold(body: child),
    ),
  );

  group('an added grammar', () {
    testWidgets('colours a Code text of its language', (tester) async {
      // Not known before it is added: one colour.
      await show(
        tester,
        MarkedText(luaCode, format: TextFormats.code, language: 'source.lua'),
      );
      expect(coloured(drawn(tester)), isEmpty);

      final grammar = store.addGrammar(bytesOf(lua));
      expect(expected(grammar, luaCode), isNotEmpty);
      await show(
        tester,
        MarkedText(luaCode, format: TextFormats.code, language: 'source.lua'),
      );
      expect(coloured(drawn(tester)), expected(grammar, luaCode));
    });

    testWidgets('colours a code block whose fence names it', (tester) async {
      final grammar = store.addGrammar(bytesOf(lua));
      await show(
        tester,
        MarkedText('```lua\n$luaCode\n```', format: TextFormats.markdown),
      );
      final colours = <InlineSpan>[];
      for (final rich in tester.widgetList<RichText>(
        find.descendant(
          of: find.byType(MarkedText),
          matching: find.byType(RichText),
        ),
      )) {
        colours.add(rich.text);
      }
      final parts = [for (final span in colours) ...coloured(span)];
      expect(parts, expected(grammar, luaCode));
    });

    testWidgets('is offered in the text format badge and colours the '
        'editor field', (tester) async {
      store.addGrammar(bytesOf(lua));
      final id = notes.capture(luaCode, format: TextFormats.code).id;
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

      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      for (final grammar in Grammars.builtIn.all) {
        expect(key('code-language-${grammar.scope}'), findsOneWidget);
      }
      await tester.ensureVisible(key('code-language-source.lua'));
      await tester.tap(key('code-language-source.lua'));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.codeLanguage, 'source.lua');
      final span = tester
          .state<EditableTextState>(find.byType(EditableText).first)
          .buildTextSpan();
      expect(span.toPlainText(), luaCode);
      expect(
        coloured(span).map((part) => part.$2).toSet(),
        expected(
          store.grammars.byScope('source.lua')!,
          luaCode,
        ).map((part) => part.$2).toSet(),
      );
    });
  });

  testWidgets('a grammar of a built-in language colours in its place; '
      'Remove brings the built-in one back', (tester) async {
    final builtIn = Grammars.builtIn.byScope('source.rust')!;
    Widget code() =>
        MarkedText(rustCode, format: TextFormats.code, language: 'source.rust');

    await show(tester, code());
    expect(coloured(drawn(tester)), expected(builtIn, rustCode));

    store.addGrammar(bytesOf(myRust));
    final mine = store.grammars.byScope('source.rust')!;
    expect(mine.name, 'My Rust');
    expect(expected(mine, rustCode), isNot(expected(builtIn, rustCode)));
    // The same text: what the built-in one coloured is not shown again.
    await show(tester, code());
    expect(coloured(drawn(tester)), expected(mine, rustCode));

    store.removeGrammar('source.rust');
    expect(store.grammars.byScope('source.rust'), same(builtIn));
    await show(tester, code());
    expect(coloured(drawn(tester)), expected(builtIn, rustCode));
  });

  group('Settings ▸ Code languages', () {
    Future<void> openSettings(WidgetTester tester) async {
      await tester.pumpWidget(
        OldBrainFixApp(store: store, alarms: FakeAlarms(), files: files),
      );
      await tester.pumpAndSettle();
      await openDeviceSettings(tester);
    }

    Future<void> tap(WidgetTester tester, Finder finder) async {
      await tester.ensureVisible(finder);
      await tester.pumpAndSettle();
      await tester.tap(finder);
      await tester.pumpAndSettle();
    }

    testWidgets('lists each added grammar with Remove, which removes it at '
        'once', (tester) async {
      store
        ..addGrammar(bytesOf(lua))
        ..addGrammar(bytesOf(myRust));
      await openSettings(tester);
      expect(find.text(t.grammarsTitle), findsOneWidget);
      expect(find.text(t.grammarsHint), findsOneWidget);
      expect(find.text('Lua'), findsOneWidget);
      expect(find.text('source.lua'), findsOneWidget);
      expect(find.text('My Rust'), findsOneWidget);
      expect(find.text(t.grammarInPlaceOf('Rust')), findsOneWidget);

      await tap(tester, key('grammar-remove-source.rust'));
      expect(store.grammars.added.map((g) => g.scope), ['source.lua']);
      expect(find.text('My Rust'), findsNothing);
      expect(key('grammar-source.rust'), findsNothing);
      expect(store.grammars.byScope('source.rust')!.name, 'Rust');
    });

    testWidgets('Add grammar… adds the picked file', (tester) async {
      files.nextPicked = [
        (name: 'Lua.sublime-syntax', bytes: bytesOf(lua), type: ''),
      ];
      await openSettings(tester);
      await tap(tester, key('grammarAdd'));
      expect(files.picks, 1);
      expect(store.grammars.added.map((g) => g.scope), ['source.lua']);
      expect(key('grammar-source.lua'), findsOneWidget);
      expect(key('grammarRefused'), findsNothing);
      // The picker's copy goes once read.
      expect(dir.listSync().whereType<File>(), isEmpty);
    });

    testWidgets('a refused file is said in red under the button, nothing is '
        'added; the next add clears it', (tester) async {
      files.nextPicked = [
        (name: 'Beyond.sublime-syntax', bytes: bytesOf(beyond), type: ''),
      ];
      await openSettings(tester);
      await tap(tester, key('grammarAdd'));
      expect(store.grammars.added, isEmpty);
      final said = t.grammarRefused('Beyond.sublime-syntax', reasonOf(beyond));
      expect(find.text(said), findsOneWidget);
      final line = tester.widget<Text>(key('grammarRefused'));
      expect(
        line.style?.color,
        Theme.of(tester.element(key('grammarRefused'))).colorScheme.error,
      );

      files.nextPicked = [
        (name: 'Lua.sublime-syntax', bytes: bytesOf(lua), type: ''),
      ];
      await tap(tester, key('grammarAdd'));
      expect(store.grammars.added.map((g) => g.scope), ['source.lua']);
      expect(key('grammarRefused'), findsNothing);
    });
  });
}

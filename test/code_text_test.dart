import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/setting_badges.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A Code text (#213): a Plain Note's whole text in monospace exactly as
/// written, unwrapped, nothing in it a format, a box or a tag; set in the
/// editor's text format badge, and offered for a paste that looks like
/// code.
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

  const script =
      '#!/bin/sh\n# **not bold** ☐ #tag\necho "a very long line '
      'that would wrap on any phone screen if it were wrapped at all"';

  Widget app(Widget child) => MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );

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

  group('shown', () {
    testWidgets('as written in monospace, unwrapped, on the ground of code', (
      tester,
    ) async {
      await tester.pumpWidget(
        app(
          const Padding(
            padding: EdgeInsets.all(16),
            child: MarkedText(script, format: TextFormats.code),
          ),
        ),
      );
      final text = tester.widget<Text>(
        find.descendant(of: key('code-text'), matching: find.byType(Text)),
      );
      expect(text.data, script);
      expect(text.softWrap, isFalse);
      expect(text.style?.fontFamily, 'monospace');
      // No tag is a link, no box a button: nothing but the one text.
      expect(find.byType(RichText), findsOneWidget);
      // Three lines, the long one cut off and not wrapped.
      final lineHeight = tester.getSize(find.byType(RichText)).height / 3;
      expect(lineHeight, lessThan(30));
    });

    testWidgets('a compact line in monospace without the ground', (
      tester,
    ) async {
      await tester.pumpWidget(
        app(
          const MarkedText(
            script,
            format: TextFormats.code,
            end: 9,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      );
      expect(key('code-text'), findsNothing);
      final text = tester.widget<Text>(find.byType(Text));
      expect(text.data, '#!/bin/sh');
      expect(text.style?.fontFamily, 'monospace');
    });

    testWidgets('a card shows a code Note in monospace', (tester) async {
      notes.capture(script, format: TextFormats.code);
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      final first = tester.widget<MarkedText>(
        find
            .descendant(
              of: find.byType(NoteCard),
              matching: find.byType(MarkedText),
            )
            .first,
      );
      expect(first.format, TextFormats.code);
      final shown = tester
          .widgetList<Text>(
            find.descendant(
              of: find.byType(NoteCard),
              matching: find.byType(Text),
            ),
          )
          .firstWhere((t) => t.data?.startsWith('#!/bin/sh') ?? false);
      expect(shown.style?.fontFamily, 'monospace');
      expect(shown.softWrap, isFalse);
    });
  });

  group('the field', () {
    Future<TagTextController> pumpField(
      WidgetTester tester, {
      String text = '',
      bool code = false,
      bool twoRows = true,
      VoidCallback? onCodePasted,
    }) async {
      final controller = TagTextController(text: text, hideMarks: twoRows)
        ..code = code;
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        app(
          ListView(
            children: [
              TwoRowTextField(
                controller: controller,
                twoRows: twoRows,
                boxes: true,
                formats: true,
                onCodePasted: onCodePasted,
              ),
            ],
          ),
        ),
      );
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
      return controller;
    }

    for (final twoRows in [true, false]) {
      testWidgets('a Code text is monospace, unwrapped and sideways, without '
          'the bar of formats (two rows: $twoRows)', (tester) async {
        await pumpField(tester, text: script, code: true, twoRows: twoRows);
        final span = tester
            .state<EditableTextState>(find.byType(EditableText))
            .buildTextSpan();
        expect(span.style?.fontFamily, 'monospace');
        expect(span.toPlainText(), script);
        expect(key('format-bar'), findsNothing);
        expect(key('code-sideways'), findsOneWidget);
        final screen = tester.getSize(find.byType(ListView)).width;
        expect(
          tester.getSize(find.byType(EditableText)).width,
          greaterThan(screen),
        );
      });
    }

    testWidgets('a plain text wraps and keeps the bar', (tester) async {
      await pumpField(tester, text: script);
      expect(key('format-bar'), findsOneWidget);
      expect(key('code-sideways'), findsNothing);
    });

    testWidgets('a paste that looks like code offers it and does not '
        'format the text; any other paste does as before', (tester) async {
      var offered = 0;
      final controller = await pumpField(tester, onCodePasted: () => offered++);
      await paste(tester, '# heading\nint a;\nint b;');
      expect(offered, 1);
      expect(controller.markdown, isFalse);
      await paste(tester, '\n- **milk**');
      expect(offered, 1);
      expect(controller.markdown, isTrue);
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

    SettingBadge badge(WidgetTester tester) =>
        tester.widget<SettingBadge>(key('badge-format'));

    TagTextController field(WidgetTester tester) =>
        tester.widget<TextField>(find.byType(TextField).first).controller!
            as TagTextController;

    testWidgets('the text format badge shows the format and sets it', (
      tester,
    ) async {
      final id = notes.capture('# comment\necho hi').id;
      await openEditor(tester);
      expect(badge(tester).value, 'Plain text');
      expect(badge(tester).valueIcon, Icons.short_text);
      expect(badge(tester).icon, Icons.text_format);
      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      expect(key('format-plain'), findsOneWidget);
      expect(key('format-markdown'), findsOneWidget);
      await tester.tap(key('format-code'));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.textFormat, TextFormats.code);
      expect(badge(tester).value, 'Code');
      expect(badge(tester).valueIcon, Icons.code);
      expect(field(tester).code, isTrue);
      expect(key('code-sideways'), findsOneWidget);
      // A Code text becomes no other kind, nor a Session from "+".
      expect(tester.widget<SettingBadge>(key('badge-kind')).onTap, isNull);
      await tester.tap(key('badge-add'));
      await tester.pumpAndSettle();
      expect(key('spawn-routine'), findsOneWidget);
      expect(key('spawn-session'), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      await tester.tap(key('format-markdown'));
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.textFormat, TextFormats.markdown);
      expect(field(tester).code, isFalse);
      expect(field(tester).markdown, isTrue);
      expect(key('code-sideways'), findsNothing);
      // A formatted text is never made plain again.
      await tester.tap(key('badge-format'));
      await tester.pumpAndSettle();
      expect(key('format-plain'), findsNothing);
      expect(key('format-code'), findsOneWidget);
    });

    testWidgets('an edit of a Code text keeps it code', (tester) async {
      final id = notes.capture('ls', format: TextFormats.code).id;
      await openEditor(tester);
      await tester.enterText(find.byType(TextField).first, 'ls **-l**');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, 'ls **-l**');
      expect(notes.byId(id)!.textFormat, TextFormats.code);
    });

    testWidgets('a paste that looks like code offers Keep as code', (
      tester,
    ) async {
      final id = notes.capture('Script:').id;
      await openEditor(tester);
      expect(key('code-offer'), findsNothing);
      await tester.showKeyboard(find.byType(TextField).first);
      field(tester).selection = const TextSelection.collapsed(offset: 7);
      await tester.pump();
      await paste(tester, '\nif (x) {\n\treturn;\n}');
      expect(key('code-offer'), findsOneWidget);
      expect(find.text('This looks like code.'), findsOneWidget);
      await tester.tap(key('keep-as-code'));
      await tester.pumpAndSettle();
      expect(key('code-offer'), findsNothing);
      final note = notes.byId(id)!;
      expect(note.textFormat, TextFormats.code);
      expect(note.text, 'Script:\nif (x) {\n\treturn;\n}');
    });

    testWidgets('Close lets the paste be', (tester) async {
      final id = notes.capture('x').id;
      await openEditor(tester);
      await tester.showKeyboard(find.byType(TextField).first);
      await paste(tester, '#!/bin/sh\nls');
      await tester.tap(key('code-offer-close'));
      await tester.pumpAndSettle();
      expect(key('code-offer'), findsNothing);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.textFormat, TextFormats.plain);
    });
  });
}

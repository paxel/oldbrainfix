import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';

/// Tags in the text of a Note and its Sections (#126, ADR 0032): drawn
/// in the accent colour on full and compact cards, in the editor and its
/// Sections; a tap opens the search for the tag as written, not the card.
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

  /// Every tag drawn as a link, by key.
  final links = find.byWidgetPredicate(
    (w) => w.key is ValueKey<String> && '${w.key}'.contains("'tag-"),
  );

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> startAtNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  Color accent(WidgetTester tester) =>
      Theme.of(tester.element(find.byType(Scaffold).last)).colorScheme.primary;

  /// The colour the tag link [finder] is drawn in.
  Color? colourOf(WidgetTester tester, Finder finder) => tester
      .widget<Text>(find.descendant(of: finder, matching: find.byType(Text)))
      .style
      ?.color;

  /// What the search field of the search on top holds.
  String searched(WidgetTester tester) => tester
      .widget<TextField>(
        find.descendant(
          of: find.byType(AreasScreen),
          matching: find.byType(TextField),
        ),
      )
      .controller!
      .text;

  group('on a card', () {
    testWidgets('a tag in the text and in a Section is highlighted; "C#", '
        '"#1" and a heading are not', (tester) async {
      final id = notes.capture('Mow the #Garten\nC# and #1 only\n# Title').id;
      notes.amend(id, '#garten tools');
      await startAtNotes(tester);

      expect(links, findsNWidgets(2));
      expect(key('tag-8'), findsOneWidget);
      expect(find.text('#Garten'), findsOneWidget);
      expect(find.text('#garten'), findsOneWidget);
      expect(colourOf(tester, key('tag-8')), accent(tester));
      // The Section's tag counts from its own text's start.
      expect(colourOf(tester, key('tag-0')), accent(tester));
    });

    testWidgets('a tap opens the search for the tag as written, which finds '
        'every Note holding it and not the word alone', (tester) async {
      notes.capture('Mow the #Garten');
      notes.capture('#GARTEN tools');
      notes.capture('Plan the #Gartenarbeit');
      notes.capture('Garten without the sign');
      await startAtNotes(tester);

      await tap(
        tester,
        find.ancestor(of: find.text('#Garten'), matching: links),
      );
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(searched(tester), '#Garten');
      expect(find.byType(NoteCard), findsNWidgets(3));
      expect(find.text('#GARTEN'), findsOneWidget);
      expect(find.text('#Gartenarbeit'), findsOneWidget);
      expect(find.textContaining('without', findRichText: true), findsNothing);

      // A tag on a hit searches for it in place.
      await tap(
        tester,
        find.ancestor(of: find.text('#Gartenarbeit'), matching: links),
      );
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(searched(tester), '#Gartenarbeit');
      expect(find.byType(NoteCard), findsOneWidget);
    });

    testWidgets('a tap on a tag of a compact card opens the search, not the '
        'card', (tester) async {
      notes.capture('Seeds for the #Balkon\nand the pots');
      await startAtNotes(tester);
      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(key('note-first-line'), findsOneWidget);
      expect(colourOf(tester, key('tag-14')), accent(tester));

      await tap(tester, key('tag-14'));
      expect(searched(tester), '#Balkon');
      expect(find.byType(NoteEditorScreen), findsNothing);

      await tester.pageBack();
      await tester.pumpAndSettle();
      // Still folded: the tap was the tag's alone.
      expect(key('note-first-line'), findsOneWidget);
      expect(find.byType(NoteEditorScreen), findsNothing);
    });

    testWidgets('the Agenda\'s just-captured card opens the search too', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'Call about #Wasser');
      await tap(tester, find.text('Save'));
      expect(find.text('Just captured'), findsOneWidget);

      await tap(tester, key('tag-11'));
      expect(searched(tester), '#Wasser');
      expect(find.byType(NoteCard), findsOneWidget);
    });

    testWidgets('a screen reader reads a tag as a link to its search', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      notes.capture('Mow the #Garten');
      await startAtNotes(tester);

      expect(
        tester.getSemantics(key('tag-8')),
        matchesSemantics(
          label: 'Search for #Garten',
          isLink: true,
          hasTapAction: true,
        ),
      );
      semantics.dispose();
    });
  });

  group('in the editor', () {
    testWidgets('the text\'s tags are highlighted as typed, a Section\'s tag '
        'opens the search', (tester) async {
      final id = notes.capture('Mow the #Garten, C# too').id;
      notes.amend(id, 'Water the #Beet');
      await startAtNotes(tester);
      await tester.longPress(find.byType(NoteCard).first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);

      final field = tester.widget<EditableText>(find.byType(EditableText));
      final span = field.controller.buildTextSpan(
        context: tester.element(find.byType(EditableText)),
        withComposing: false,
      );
      final coloured = [
        for (final child in span.children ?? const <InlineSpan>[])
          if (child is TextSpan && child.style?.color == accent(tester))
            child.text,
      ];
      expect(coloured, ['#Garten']);

      await tap(tester, key('tag-10'));
      expect(searched(tester), '#Beet');
      expect(find.byType(NoteCard), findsOneWidget);
    });
  });
}

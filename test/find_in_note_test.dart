import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/attachment_views.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/found_marks.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Search leads to what it found (obf-1047): a card of the search lists
/// the places found in its Note, three and how many more, each its line
/// with the place on a tint; a tap on one opens the Note there with the
/// find bar, which counts, steps, finds anew and closes; the magnifier
/// opens it without a search; a table holding a place shows all its
/// rows on the card.
void main() {
  late Store store;
  late Notes notes;
  late Area home;
  var now = DateTime.utc(2026, 11, 23, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 23, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    home = Places(store).addArea('Household');
  });
  tearDown(() => store.close());

  Future<void> search(WidgetTester tester, String query) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), query);
    await tester.pump(BrowserSearchField.pause);
    await tester.pumpAndSettle();
  }

  /// The spans drawn under [of] on the tint of a place found: the current
  /// one's or the others'.
  List<String> tinted(WidgetTester tester, Finder of, {bool current = false}) {
    final context = tester.element(of);
    final tint = foundStyle(context, null, current: current).backgroundColor;
    final words = <String>[];
    for (final text in tester.widgetList<RichText>(
      find.descendant(of: of, matching: find.byType(RichText)),
    )) {
      text.text.visitChildren((span) {
        if (span is TextSpan && span.style?.backgroundColor == tint) {
          words.add(span.text ?? '');
        }
        return true;
      });
    }
    return words;
  }

  /// The find bar's count, "3 of 12".
  String count(WidgetTester tester) =>
      tester.widget<Text>(find.byKey(const ValueKey('find-count'))).data ?? '';

  /// The places the find bar hands the Note's text field.
  FoundMarks? inField(WidgetTester tester) {
    final field = tester.widget<EditableText>(
      find
          .descendant(
            of: find.byKey(const ValueKey('note-text')),
            matching: find.byType(EditableText),
          )
          .first,
    );
    return (field.controller as TagTextController).found;
  }

  testWidgets('a card of the search lists the first three places found, '
      'each on a tint, and how many more', (tester) async {
    notes.capture(
      'Boiler check\nservice the Boiler\nboiler room key\n'
      'old boiler parts\nboiler manual',
      place: home.id,
    );
    await search(tester, 'boiler');

    for (var i = 0; i < 3; i++) {
      expect(find.byKey(ValueKey('finding-$i')), findsOneWidget);
    }
    expect(find.byKey(const ValueKey('finding-3')), findsNothing);
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('findings-more')),
        matching: find.text('+2 more'),
      ),
      findsOneWidget,
    );
    // The place on its line, whatever its case.
    expect(tinted(tester, find.byKey(const ValueKey('finding-1'))), ['Boiler']);
    // The card's own text draws every place on the tint too.
    expect(
      tinted(
        tester,
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.byType(MarkedText),
        ),
      ),
      hasLength(5),
    );
  });

  testWidgets('a tap on a place opens the Note there with the find bar, '
      'whose arrows step through every place and whose cross closes it', (
    tester,
  ) async {
    notes.capture('mint tea\nmint leaves\npeppermint', place: home.id);
    await search(tester, 'mint');

    await tester.tap(find.byKey(const ValueKey('finding-1')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('find-bar')), findsOneWidget);
    expect(count(tester), '2 of 3');
    final marks = inField(tester);
    expect(marks?.ranges, [(0, 4), (9, 13), (27, 31)]);
    expect(marks?.current, 1);

    await tester.tap(find.byKey(const ValueKey('find-next')));
    await tester.pumpAndSettle();
    expect(count(tester), '3 of 3');
    await tester.tap(find.byKey(const ValueKey('find-next')));
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 3');
    await tester.tap(find.byKey(const ValueKey('find-previous')));
    await tester.pumpAndSettle();
    expect(count(tester), '3 of 3');
    expect(inField(tester)?.current, 2);

    // The words are edited in the bar.
    await tester.enterText(find.byKey(const ValueKey('find-field')), 'tea');
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 1');
    expect(inField(tester)?.ranges, [(5, 8)]);

    await tester.tap(find.byKey(const ValueKey('find-close')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('find-bar')), findsNothing);
    expect(inField(tester), isNull);
  });

  testWidgets('a tap on the card opens the Note at its first place', (
    tester,
  ) async {
    notes.capture('a list of things\nbuy milk', place: home.id);
    await search(tester, 'milk');

    await tester.tap(
      find.textContaining('a list of things', findRichText: true).first,
    );
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 1');
  });

  testWidgets('the magnifier opens the find bar without a search, and '
      'what is typed into the Note is found anew', (tester) async {
    notes.capture('rain coat\nrain boots', place: home.id);
    await search(tester, 'coat');
    await tester.tap(find.byKey(const ValueKey('finding-0')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('find-close')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('find-in-note')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('find-bar')), findsOneWidget);
    // The words found last stay; the cursor is in the bar.
    expect(count(tester), '1 of 1');
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('find-field')))
          .focusNode
          ?.hasFocus,
      isTrue,
    );
    await tester.enterText(find.byKey(const ValueKey('find-field')), 'rain');
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 2');

    final field = find
        .descendant(
          of: find.byKey(const ValueKey('note-text')),
          matching: find.byType(EditableText),
        )
        .first;
    await tester.enterText(field, 'rain coat\nrain boots\nmore rain');
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 3');
  });

  testWidgets('accents and case do not matter to the find bar', (tester) async {
    notes.capture('Herr Müller\nStraße 4', place: home.id);
    await search(tester, 'muller');
    expect(tinted(tester, find.byKey(const ValueKey('finding-0'))), ['Müller']);
    await tester.tap(find.byKey(const ValueKey('finding-0')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('find-field')), 'strasse');
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 1');
  });

  testWidgets('a #tag found is drawn on the tint as a whole', (tester) async {
    notes.capture('ask about #garden today', place: home.id);
    await search(tester, '#garden');
    // The tag is a link to its search; the tint lies on its words.
    expect(
      tinted(
        tester,
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.byType(MarkedText),
        ),
      ),
      contains('#garden'),
    );
  });

  testWidgets('the field draws the current place on the stronger tint, '
      'and ↓ moves it', (tester) async {
    notes.capture('salt\npepper\nsalt again', place: home.id);
    await search(tester, 'salt');
    await tester.tap(find.byKey(const ValueKey('finding-0')));
    await tester.pumpAndSettle();

    /// The words the field draws on the tint of the current place.
    List<String> drawnNow() {
      final editable = tester.state<EditableTextState>(
        find
            .descendant(
              of: find.byKey(const ValueKey('note-text')),
              matching: find.byType(EditableText),
            )
            .first,
      );
      final tint = foundStyle(
        tester.element(find.byKey(const ValueKey('note-text'))),
        null,
        current: true,
      ).backgroundColor;
      final words = <String>[];
      var at = 0;
      editable.renderEditable.text?.visitChildren((span) {
        if (span is TextSpan) {
          if (span.style?.backgroundColor == tint) words.add('$at');
          at += span.text?.length ?? 0;
        }
        return true;
      });
      return words;
    }

    expect(drawnNow(), ['0']);
    await tester.tap(find.byKey(const ValueKey('find-next')));
    await tester.pumpAndSettle();
    expect(drawnNow(), ['12']);
  });

  testWidgets('a file\'s name found is drawn on the tint and shown', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(400, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    notes.capture(
      ['receipt', for (var i = 0; i < 80; i++) 'line $i'].join('\n'),
      place: home.id,
      files: [
        NewFile(name: 'receipt.pdf', bytes: Uint8List.fromList([1, 2, 3])),
      ],
    );
    await search(tester, 'receipt');
    await tester.tap(find.byKey(const ValueKey('finding-0')));
    await tester.pumpAndSettle();
    expect(count(tester), '1 of 2');

    await tester.tap(find.byKey(const ValueKey('find-next')));
    await tester.pumpAndSettle();
    expect(count(tester), '2 of 2');
    final name = find.text('receipt.pdf', findRichText: true);
    expect(
      tinted(
        tester,
        find.ancestor(of: name, matching: find.byType(AttachmentRow)),
        current: true,
      ),
      ['receipt'],
    );
    final screen = tester.getRect(find.byType(Scaffold).last);
    final rect = tester.getRect(name);
    expect(screen.top <= rect.top && rect.bottom <= screen.bottom, isTrue);
  });

  testWidgets('the find bar fits a small phone at a large text size', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(360, 740);
    tester.view.devicePixelRatio = 1;
    tester.platformDispatcher.textScaleFactorTestValue = 2;
    addTearDown(tester.view.reset);
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    notes.capture('Schraubenschlüssel im Keller', place: home.id);
    await search(tester, 'keller');
    await tester.tap(find.byKey(const ValueKey('finding-0')));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.byKey(const ValueKey('find-bar')), findsOneWidget);
    // The magnifier stays on the bar, out of ⋮.
    expect(find.byKey(const ValueKey('find-in-note')), findsOneWidget);
    for (final key in [
      'find-previous',
      'find-next',
      'find-search-all',
      'find-close',
    ]) {
      final rect = tester.getRect(find.byKey(ValueKey(key)));
      expect(rect.right, lessThanOrEqualTo(360));
    }
  });

  group('a table', () {
    String table(int rows, {required int at, required String value}) => [
      '| Item | Price |',
      '| --- | --- |',
      for (var r = 1; r <= rows; r++)
        '| ${r == at ? value : 'thing $r'} | $r |',
    ].join('\n');

    testWidgets('on the card shows all its rows when a place found lies '
        'past its fold', (tester) async {
      tester.view.physicalSize = const Size(800, 3000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      notes.capture(
        table(14, at: 13, value: 'zebra'),
        format: TextFormats.markdown,
        place: home.id,
      );
      await search(tester, 'zebra');

      expect(find.byKey(const ValueKey('table-0-row-13')), findsOneWidget);
      expect(find.byKey(const ValueKey('table-show-all-0')), findsNothing);
      expect(tinted(tester, find.byKey(const ValueKey('table-0-row-13'))), [
        'zebra',
      ]);
    });

    testWidgets('of 200 rows opened at a place deep in it shows that row in '
        'the editor', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      notes.capture(
        table(200, at: 150, value: 'walrus'),
        format: TextFormats.markdown,
        place: home.id,
      );
      await search(tester, 'walrus');
      // The card unfolds the table to it; the place found leads the card,
      // in reach.
      expect(
        find.byKey(const ValueKey('table-0-row-150'), skipOffstage: false),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('finding-0')));
      await tester.pumpAndSettle();
      expect(count(tester), '1 of 1');

      // The current place is drawn in the grid, on the screen.
      final current = find.byWidgetPredicate((widget) {
        if (widget is! RichText) return false;
        var on = false;
        widget.text.visitChildren((span) {
          if (span is TextSpan && span.text == 'walrus') {
            on = span.style?.backgroundColor != null;
          }
          return !on;
        });
        return on;
      });
      expect(current, findsOneWidget);
      final screen = tester.getRect(find.byType(Scaffold).last);
      final rect = tester.getRect(current);
      expect(screen.top <= rect.top && rect.bottom <= screen.bottom, isTrue);
    });
  });
}

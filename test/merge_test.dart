import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  String shown(DateTime utc) =>
      DateFormat.yMd('en').add_Hm().format(utc.toLocal());

  void tick([Duration by = const Duration(hours: 1)]) => now = now.add(by);

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Starts the app and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// The "merge into…" button on the card showing [text].
  Finder mergeOn(String text) => find.descendant(
    of: find.ancestor(of: find.text(text), matching: find.byType(NoteCard)),
    matching: find.byTooltip('Merge into…'),
  );

  List<String> cards() => [
    for (final card in find.byType(NoteCard).evaluate())
      (card.widget as NoteCard).note.text,
  ];

  testWidgets('the editor merges the Note, as typed, into the picked one', (
    tester,
  ) async {
    final target = notes.capture('sauce');
    tick();
    final merged = notes.capture('nees mor sauce');
    await startAtNotes(tester);
    await tester.longPress(find.text('nees mor sauce'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'need more sauce');
    await tap(tester, find.byTooltip('Merge into…'));

    expect(find.byType(MergeIntoScreen), findsOneWidget);
    expect(cards(), ['sauce']);
    expect(find.byType(BottomSheet), findsNothing);
    await tap(tester, find.text('sauce'));

    expect(find.byType(NoteEditorScreen), findsNothing);
    expect(cards(), ['sauce']);
    expect(notes.byId(merged.id)?.mergedInto, target.id);
    await tester.longPress(find.text('sauce'));
    await tester.pumpAndSettle();
    await reveal(tester, find.text('need more sauce'));
    expect(find.text('need more sauce'), findsOneWidget);
    expect(find.text(shown(DateTime.utc(2026, 10, 24, 9))), findsOneWidget);
  });

  testWidgets('a card merges its Note; the picker searches', (tester) async {
    final home = places.addArea('Household');
    notes.capture('sauce', place: home.id);
    tick();
    notes.capture('pasta');
    tick();
    final merged = notes.capture('more sauce');
    await startAtNotes(tester);
    await tap(tester, mergeOn('more sauce'));
    expect(cards(), ['pasta', 'sauce']);
    await tester.enterText(find.byType(TextField), 'sau');
    await tester.pumpAndSettle();
    expect(cards(), ['sauce']);
    await tap(tester, find.text('sauce'));

    expect(cards(), ['pasta', 'sauce']);
    expect(notes.byId(merged.id)?.isUnsorted, isFalse);
    expect(notes.sections(notes.all().last.id).single.text, 'more sauce');
  });

  testWidgets('the browser\'s cards merge too', (tester) async {
    final home = places.addArea('Household');
    notes.capture('sauce', place: home.id);
    tick();
    notes.capture('more sauce', place: home.id);
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.text('Household'));
    await tap(tester, mergeOn('more sauce'));
    await tap(tester, find.text('sauce'));
    expect(cards(), ['sauce']);
  });

  testWidgets('backing out of the picker merges nothing', (tester) async {
    notes.capture('sauce');
    await startAtNotes(tester);
    await tap(tester, mergeOn('sauce'));
    expect(find.text('There is no other Note to merge into.'), findsOneWidget);
    await tap(tester, find.byType(BackButton));
    expect(cards(), ['sauce']);
    expect(notes.all().single.isMerged, isFalse);
  });
}

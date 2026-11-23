import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A search over the Notes runs once typing pauses (obf-1068, C11): each
/// letter shows in the field at once, the results follow
/// [BrowserSearchField.pause] after the last one, and a letter typed
/// before then puts the search off, so no older query's results show.
void main() {
  late Store store;
  late Notes notes;
  late Area home;
  var now = DateTime.utc(2026, 11, 22, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 22, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    home = Places(store).addArea('Household');
    for (final text in ['milk', 'mint tea', 'bread']) {
      now = now.add(const Duration(minutes: 1));
      notes.capture(text, place: home.id);
    }
  });
  tearDown(() => store.close());

  const almost = Duration(milliseconds: 299);
  const rest = Duration(milliseconds: 1);

  Future<void> openBrowser(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
  }

  Finder field() => find.byType(TextField);

  /// The texts the field's editor shows now.
  String typed(WidgetTester tester) =>
      tester.widget<EditableText>(find.byType(EditableText)).controller.text;

  Finder card(String text) => find.descendant(
    of: find.byType(NoteCard),
    matching: find.textContaining(text, findRichText: true),
  );

  testWidgets('the Areas list shows the letter at once and the hits once '
      'typing pauses', (tester) async {
    await openBrowser(tester);

    await tester.enterText(field(), 'mil');
    await tester.pump();
    expect(typed(tester), 'mil');
    expect(find.byTooltip('Clear search'), findsOneWidget);
    // Until then the Areas stay, no "nothing found" in their place.
    expect(find.text('Household'), findsOneWidget);
    expect(find.byType(NoteCard), findsNothing);

    await tester.pump(almost);
    expect(find.byType(NoteCard), findsNothing);
    await tester.pump(rest);
    await tester.pump();
    expect(card('milk'), findsOneWidget);
    expect(find.byType(NoteCard), findsOneWidget);
  });

  testWidgets('a letter typed before the pause is over drops the older '
      'query\'s search', (tester) async {
    await openBrowser(tester);

    await tester.enterText(field(), 'mi');
    await tester.pump(const Duration(milliseconds: 200));
    await tester.enterText(field(), 'mil');
    await tester.pump(const Duration(milliseconds: 200));
    // "mi" would have found "mint tea"; it never shows.
    expect(find.byType(NoteCard), findsNothing);

    await tester.pump(const Duration(milliseconds: 100));
    await tester.pump();
    expect(card('milk'), findsOneWidget);
    expect(card('mint tea'), findsNothing);
    await tester.pump(const Duration(seconds: 1));
    expect(card('mint tea'), findsNothing);
  });

  testWidgets('clearing the field shows the Areas at once', (tester) async {
    await openBrowser(tester);
    await tester.enterText(field(), 'milk');
    await tester.pump(BrowserSearchField.pause);
    await tester.pump();
    expect(card('milk'), findsOneWidget);

    await tester.tap(find.byTooltip('Clear search'));
    await tester.pump();
    expect(find.byType(NoteCard), findsNothing);
    expect(find.text('Household'), findsOneWidget);
  });

  testWidgets('a search reads as often for many hits as for a few', (
    tester,
  ) async {
    Future<int> readsOf(String query) async {
      await tester.enterText(field(), query);
      await tester.pump(almost);
      final before = store.historyReads + store.loggedReads;
      await tester.pump(rest);
      await tester.pump();
      return store.historyReads + store.loggedReads - before;
    }

    for (var i = 0; i < 80; i++) {
      now = now.add(const Duration(minutes: 1));
      notes.capture(i < 20 ? 'few plums $i' : 'many plums $i', place: home.id);
    }
    await openBrowser(tester);
    // More hits than the screen holds either way: as many cards show;
    // the first search reads what the page then keeps.
    await readsOf('many');
    final few = await readsOf('few');
    final many = await readsOf('plums');
    expect(find.byType(NoteCard), findsWidgets);
    expect(many, few);
  });

  testWidgets('an Area\'s search keeps its Notes until typing pauses', (
    tester,
  ) async {
    await openBrowser(tester);
    await tester.tap(find.text('Household'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteCard), findsNWidgets(3));

    await tester.enterText(field(), 'bre');
    await tester.pump();
    expect(typed(tester), 'bre');
    expect(find.byType(NoteCard), findsNWidgets(3));

    await tester.pump(almost);
    expect(find.byType(NoteCard), findsNWidgets(3));
    await tester.pump(rest);
    await tester.pump();
    expect(card('bread'), findsOneWidget);
    expect(find.byType(NoteCard), findsOneWidget);
  });

  testWidgets('a Topic\'s search keeps its Notes until typing pauses', (
    tester,
  ) async {
    final kitchen = Places(store).addTopic(home.id, 'Kitchen');
    notes
      ..capture('milk jug', place: kitchen.id)
      ..capture('butter', place: kitchen.id);
    await openBrowser(tester);
    await tester.tap(find.text('Household'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Kitchen'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteCard), findsNWidgets(2));

    await tester.enterText(field(), 'jug');
    await tester.pump(almost);
    expect(typed(tester), 'jug');
    expect(find.byType(NoteCard), findsNWidgets(2));
    await tester.pump(rest);
    await tester.pump();
    expect(card('milk jug'), findsOneWidget);
    expect(find.byType(NoteCard), findsOneWidget);
  });

  testWidgets('Merge into… keeps its Notes until typing pauses', (
    tester,
  ) async {
    await openBrowser(tester);
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(
        of: find.ancestor(of: card('bread'), matching: find.byType(NoteCard)),
        matching: find.byTooltip('Merge into…'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(NoteCard), findsNWidgets(2));

    await tester.enterText(field(), 'mint');
    await tester.pump(almost);
    expect(typed(tester), 'mint');
    expect(find.byType(NoteCard), findsNWidgets(2));
    await tester.pump(rest);
    await tester.pump();
    expect(card('mint tea'), findsOneWidget);
    expect(find.byType(NoteCard), findsOneWidget);
  });
}

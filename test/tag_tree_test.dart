import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/tag_tree_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The tag tree (#267, Q623, Q639): the row of the most used tags under
/// the Areas list's search (obf-1074) searches for one and ends in "All
/// tags…", which opens every tag by its levels with their counts; a tag
/// there opens its search, and typing `#` in the search suggests the tags
/// as chips.
void main() {
  late Store store;
  late Notes notes;

  setUpAll(initializeDateFormatting);

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 12, 8))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> startAtAreas(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
  }

  /// Opens the tag tree by All tags… at the end of the row of tags.
  Future<void> openTree(WidgetTester tester) async {
    await tester.drag(key('tag-chips'), const Offset(-2000, 0));
    await tester.pumpAndSettle();
    await tap(tester, key('tag-chips-all'));
  }

  /// What the search field of the Areas list on top holds.
  String searched(WidgetTester tester) => tester
      .widget<TextField>(
        find.descendant(
          of: find.byType(AreasScreen).last,
          matching: find.byType(TextField),
        ),
      )
      .controller!
      .text;

  testWidgets('All tags… opens the tree: the top level first, counts of '
      'Notes, the levels below by the arrow, and a tag opens its search', (
    tester,
  ) async {
    Places(store).addArea('Home');
    notes.capture('Physio on Monday #life/health/back');
    notes.capture('Dentist #life/health');
    notes.capture('Plant the roses #garden');
    await startAtAreas(tester);

    // Under the search field, above the Areas (obf-1074).
    expect(
      tester.getTopLeft(key('tag-chips-all')).dy,
      greaterThan(tester.getBottomLeft(find.byType(TextField).last).dy),
    );
    expect(
      tester.getBottomLeft(key('tag-chips-all')).dy,
      lessThan(tester.getTopLeft(find.text('Home')).dy),
    );
    expect(find.text('Tags'), findsNothing);
    await openTree(tester);
    expect(find.byType(TagTreeScreen), findsOneWidget);
    expect(key('tree-row-#garden'), findsOneWidget);
    expect(key('tree-row-#life'), findsOneWidget);
    expect(
      tester.getTopLeft(key('tree-row-#garden')).dy,
      lessThan(tester.getTopLeft(key('tree-row-#life')).dy),
    );
    expect(
      find.descendant(
        of: key('tree-row-#life'),
        matching: find.text('2 Notes'),
      ),
      findsOneWidget,
    );
    expect(key('tree-row-#life/health'), findsNothing);
    // A tag without levels below has no arrow.
    expect(key('tree-open-#garden'), findsNothing);

    await tap(tester, key('tree-open-#life'));
    await tap(tester, key('tree-open-#life/health'));
    expect(
      find.descendant(
        of: key('tree-row-#life/health/back'),
        matching: find.text('back'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: key('tree-row-#life/health/back'),
        matching: find.text('1 Note'),
      ),
      findsOneWidget,
    );

    await tap(tester, key('tree-row-#life/health/back'));
    expect(find.byType(TagTreeScreen), findsNothing);
    expect(searched(tester), '#life/health/back');
    expect(
      find.descendant(
        of: find.byType(NoteCard),
        matching: find.textContaining('Physio', findRichText: true),
      ),
      findsWidgets,
    );
    expect(find.byType(NoteCard), findsOneWidget);
  });

  testWidgets('without a tag the tree says how one comes', (tester) async {
    notes.capture('No tags here');
    await startAtAreas(tester);
    await openTree(tester);
    expect(find.textContaining('No tags yet'), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(TagTreeScreen), findsNothing);
    expect(searched(tester), '');
  });

  testWidgets('typing # in the search suggests the tags as chips that fill '
      'one in; the one typed exactly and other searches show none', (
    tester,
  ) async {
    notes.capture('Physio #life/health');
    notes.capture('Plant the roses #garden');
    await startAtAreas(tester);
    final field = find.byType(TextField).last;

    await tester.enterText(field, 'li');
    await tester.pumpAndSettle();
    expect(key('search-tag-suggestions'), findsNothing);

    await tester.enterText(field, '#');
    await tester.pumpAndSettle();
    expect(key('search-tag-#garden'), findsOneWidget);
    expect(key('search-tag-#life'), findsOneWidget);
    expect(key('search-tag-#life/health'), findsOneWidget);

    await tester.enterText(field, '#LI');
    await tester.pumpAndSettle();
    expect(key('search-tag-#garden'), findsNothing);
    expect(key('search-tag-#life'), findsOneWidget);

    await tap(tester, key('search-tag-#life'));
    expect(searched(tester), '#life');
    // Typed exactly, it is no suggestion; the one below still is.
    expect(key('search-tag-#life'), findsNothing);
    expect(key('search-tag-#life/health'), findsOneWidget);
    expect(find.byType(NoteCard), findsOneWidget);
  });

  testWidgets('while a locked Area is closed the tree and the chips leave '
      'its Notes out, as every list does', (tester) async {
    final diary = Places(store).addArea('Diary');
    notes.capture('Secret #affair #life/love', place: diary.id);
    notes.capture('Physio #life/health');
    store
      ..setPin('1234', iterations: 10)
      ..setAreaLocked(diary.id, true);
    await startAtAreas(tester);
    final field = find.byType(TextField).last;

    await tester.enterText(field, '#');
    await tester.pumpAndSettle();
    expect(key('search-tag-#life'), findsOneWidget);
    expect(key('search-tag-#life/health'), findsOneWidget);
    expect(key('search-tag-#affair'), findsNothing);
    expect(key('search-tag-#life/love'), findsNothing);
    await tester.enterText(field, '');
    await tester.pumpAndSettle();

    await openTree(tester);
    expect(key('tree-row-#affair'), findsNothing);
    expect(
      find.descendant(of: key('tree-row-#life'), matching: find.text('1 Note')),
      findsOneWidget,
    );
    // Opened while the tree shows, its tags come; closed again, they go.
    final lock = LockScope.of(tester.element(find.byType(TagTreeScreen)))!;
    lock.openArea(diary.id);
    await tester.pumpAndSettle();
    expect(key('tree-row-#affair'), findsOneWidget);
    lock.closeAreas();
    await tester.pumpAndSettle();
    expect(key('tree-row-#affair'), findsNothing);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // Without the lock its tags are there again, once the search runs.
    store.setAreaLocked(diary.id, false);
    await tester.enterText(field, '#');
    await tester.pump(BrowserSearchField.pause);
    await tester.pumpAndSettle();
    expect(key('search-tag-#affair'), findsOneWidget);
    expect(key('search-tag-#life/love'), findsOneWidget);
    await tester.enterText(field, '');
    await tester.pumpAndSettle();
    await openTree(tester);
    expect(key('tree-row-#affair'), findsOneWidget);
  });

  testWidgets('the row under the search shows the most used tags first and '
      'a tap searches for one; it goes while a search is typed', (
    tester,
  ) async {
    Places(store).addArea('Home');
    notes
      ..capture('Plant the roses #garden')
      ..capture('Physio #life/health')
      ..capture('Dentist #life/health')
      ..capture('Run #life/sport');
    await startAtAreas(tester);

    double left(String tag) => tester.getTopLeft(key('tag-chip-$tag')).dx;
    // #life on three Notes, #life/health on two; of one each, the one
    // first in the tree.
    expect(left('#life'), lessThan(left('#life/health')));
    expect(left('#life/health'), lessThan(left('#garden')));
    expect(left('#garden'), lessThan(left('#life/sport')));
    expect(
      left('#life/sport'),
      lessThan(tester.getTopLeft(key('tag-chips-all')).dx),
    );

    await tap(tester, key('tag-chip-#life/health'));
    expect(searched(tester), '#life/health');
    expect(find.byType(NoteCard), findsNWidgets(2));
    expect(key('tag-chips'), findsNothing);

    // Emptied, it is back, and typing hides it at once.
    await tap(tester, find.byTooltip('Clear search'));
    expect(key('tag-chips'), findsOneWidget);
    await tester.enterText(find.byType(TextField).last, 'r');
    await tester.pump();
    expect(key('tag-chips'), findsNothing);
    await tester.pump(BrowserSearchField.pause);
    await tester.pumpAndSettle();
  });

  testWidgets('without a tag the row holds All tags… alone', (tester) async {
    notes.capture('No tags here');
    await startAtAreas(tester);
    expect(key('tag-chips-all'), findsOneWidget);
    expect(find.byType(ActionChip), findsOneWidget);
  });

  testWidgets('a row wider than the page moves sideways under the finger and '
      'turns no page; a row that fits leaves the swipe to the pages', (
    tester,
  ) async {
    notes.capture('#garden');
    await startAtAreas(tester);
    double rowLeft() => tester.getTopLeft(key('tag-chips')).dx;
    expect(rowLeft(), 0);

    // It fits: a swipe right on it goes back to the Agenda.
    await tester.flingFrom(
      tester.getCenter(key('tag-chip-#garden')),
      const Offset(600, 0),
      1000,
    );
    await tester.pumpAndSettle();
    expect(key('tag-chips'), findsNothing);
    expect(find.byType(AgendaScreen), findsOneWidget);
    await tap(tester, find.byTooltip('Search and browse'));
    expect(rowLeft(), 0);

    for (var i = 0; i < 12; i++) {
      notes.capture('#a-rather-long-tag-number-$i');
    }
    await tester.pumpAndSettle();
    expect(find.byType(ActionChip), findsNWidgets(13));
    final first = tester.getTopLeft(find.byType(ActionChip).first).dx;
    await tester.dragFrom(
      tester.getCenter(find.byType(ActionChip).first),
      const Offset(-300, 0),
    );
    await tester.pumpAndSettle();
    expect(rowLeft(), 0);
    expect(
      tester.getTopLeft(find.byType(ActionChip).first).dx,
      lessThan(first),
    );
    await tester.dragFrom(
      tester.getCenter(find.byType(ActionChip).at(2)),
      const Offset(600, 0),
    );
    await tester.pumpAndSettle();
    expect(rowLeft(), 0);
    expect(tester.getTopLeft(find.byType(ActionChip).first).dx, first);
  });

  testWidgets('beside the Agenda the tags are read once the Areas list '
      'shows', (tester) async {
    notes.capture('#garden');
    await startAtAreas(tester);
    expect(key('tag-chip-#garden'), findsOneWidget);
    await tester.flingFrom(
      tester.getCenter(key('tag-chips-all')),
      const Offset(600, 0),
      1000,
    );
    await tester.pumpAndSettle();

    // Built beside it, the page has not read the new one.
    expect(
      find.byKey(const ValueKey('tag-chip-#garden'), skipOffstage: false),
      findsOneWidget,
    );
    notes.capture('#travel');
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('tag-chip-#travel'), skipOffstage: false),
      findsNothing,
    );
    await tap(tester, find.byTooltip('Search and browse'));
    expect(key('tag-chip-#travel'), findsOneWidget);
  });

  testWidgets('the row leaves out the tags of a closed locked Area and '
      'follows it opening and closing', (tester) async {
    final diary = Places(store).addArea('Diary');
    notes.capture('Secret #affair', place: diary.id);
    notes.capture('Physio #life');
    store
      ..setPin('1234', iterations: 10)
      ..setAreaLocked(diary.id, true);
    await startAtAreas(tester);
    expect(key('tag-chip-#life'), findsOneWidget);
    expect(key('tag-chip-#affair'), findsNothing);

    final lock = LockScope.of(tester.element(find.byType(AreasScreen).last))!;
    lock.openArea(diary.id);
    await tester.pumpAndSettle();
    expect(key('tag-chip-#affair'), findsOneWidget);
    lock.closeAreas();
    await tester.pumpAndSettle();
    expect(key('tag-chip-#affair'), findsNothing);

    // Closed beside the Agenda, its tags are gone before the list shows.
    lock.openArea(diary.id);
    await tester.pumpAndSettle();
    expect(key('tag-chip-#affair'), findsOneWidget);
    await tester.flingFrom(
      tester.getCenter(key('tag-chips-all')),
      const Offset(600, 0),
      1000,
    );
    await tester.pumpAndSettle();
    expect(find.byType(AgendaScreen), findsOneWidget);
    lock.closeAreas();
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('tag-chip-#affair'), skipOffstage: false),
      findsNothing,
    );
    await tap(tester, find.byTooltip('Search and browse'));
    expect(key('tag-chip-#life'), findsOneWidget);
    expect(key('tag-chip-#affair'), findsNothing);
  });
}

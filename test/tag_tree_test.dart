import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/tag_tree_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The tag tree (#267, Q623, Q639): a row "Tags" below the Areas opens
/// every tag by its levels with their counts, a tag there opens its
/// search, and typing `#` in the search suggests the tags as chips.
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

  testWidgets('the row below the Areas opens the tree: the top level first, '
      'counts of Notes, the levels below by the arrow, and a tag opens its '
      'search', (tester) async {
    Places(store).addArea('Home');
    notes.capture('Physio on Monday #life/health/back');
    notes.capture('Dentist #life/health');
    notes.capture('Plant the roses #garden');
    await startAtAreas(tester);

    // Below the Areas, also with an Area there.
    expect(
      tester.getTopLeft(key('tags-row')).dy,
      greaterThan(tester.getTopLeft(find.text('Home')).dy),
    );
    await tap(tester, key('tags-row'));
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
    await tap(tester, key('tags-row'));
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

  testWidgets('while a locked Area is closed the row and the chips wait, as '
      'its Notes do', (tester) async {
    final diary = Places(store).addArea('Diary');
    notes.capture('Secret #affair', place: diary.id);
    store
      ..setPin('1234', iterations: 10)
      ..setAreaLocked(diary.id, true);
    await startAtAreas(tester);
    expect(key('tags-row'), findsNothing);
    await tester.enterText(find.byType(TextField).last, '#');
    await tester.pumpAndSettle();
    expect(key('search-tag-suggestions'), findsNothing);
  });
}

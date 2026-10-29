import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/trash_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  late Area home;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
    home = places.addArea('Household');
  });
  tearDown(() => store.close());

  /// [utc] as the cards and the editor show it; only valid once the app
  /// loaded its date formats.
  String shown(DateTime utc) =>
      DateFormat.yMd('en').add_Hm().format(utc.toLocal());

  void tick([Duration by = const Duration(minutes: 1)]) => now = now.add(by);

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor of the card [finder] shows, by a long press.
  Future<void> edit(WidgetTester tester, Finder finder) async {
    await tester.longPress(finder);
    await tester.pumpAndSettle();
  }

  /// Starts the app and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  NoteCard card(WidgetTester tester, String text) => tester.widget<NoteCard>(
    find.ancestor(of: find.text(text), matching: find.byType(NoteCard)),
  );

  /// The emboss of the card showing [text].
  Embossed emboss(WidgetTester tester, String text) => tester.widget<Embossed>(
    find.ancestor(of: find.text(text), matching: find.byType(Embossed)),
  );

  /// The Opacity wrapping the card showing [text], if any.
  Iterable<Opacity> dimming(WidgetTester tester, String text) =>
      tester.widgetList<Opacity>(
        find.ancestor(of: find.text(text), matching: find.byType(Opacity)),
      );

  group('editor', () {
    testWidgets('a long press on a card shows everything about the Note', (
      tester,
    ) async {
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final note = notes.capture('nees mor sauce', place: kitchen.id);
      tick(const Duration(days: 1));
      notes.amend(note.id, 'Update: sauce bought');
      await startAtNotes(tester);

      await edit(tester, find.text('nees mor sauce'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(
        tester.widget<TextField>(find.byType(TextField)).controller?.text,
        'nees mor sauce',
      );
      expect(find.text(shown(DateTime.utc(2026, 10, 24, 8))), findsOneWidget);
      expect(find.text('Household / Kitchen'), findsOneWidget);
      await reveal(tester, find.text('Update: sauce bought'));
      expect(find.text('Update: sauce bought'), findsOneWidget);
      expect(find.text(shown(DateTime.utc(2026, 10, 25, 8))), findsOneWidget);
      expect(find.text('Normal'), findsOneWidget);
    });

    testWidgets('an edit changes the text in place; the log keeps the old', (
      tester,
    ) async {
      final note = notes.capture('nees mor sauce');
      await startAtNotes(tester);
      await edit(tester, find.text('nees mor sauce'));

      tick();
      await tester.enterText(find.byType(TextField), 'need more sauce');
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(find.text('need more sauce'), findsOneWidget);
      expect(find.text('nees mor sauce'), findsNothing);
      expect(notes.byId(note.id)?.text, 'need more sauce');
      expect(
        store
            .history(note.id)
            .where((e) => e.field == NoteKeys.text)
            .map((e) => StoredText.read(e.value).text),
        ['nees mor sauce', 'need more sauce'],
      );
    });

    testWidgets('a blanked text keeps the old one', (tester) async {
      final note = notes.capture('milk');
      await startAtNotes(tester);
      await edit(tester, find.text('milk'));
      await tester.enterText(find.byType(TextField), '   ');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)?.text, 'milk');
    });

    testWidgets('Add to Note appends a dated Section', (tester) async {
      final note = notes.capture('nees mor sauce');
      await startAtNotes(tester);
      await edit(tester, find.text('nees mor sauce'));

      tick(const Duration(hours: 3));
      await reveal(tester, find.text('Add to Note'));
      await tap(tester, find.text('Add to Note'));
      await tester.enterText(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(TextField),
        ),
        'Update: sauce bought',
      );
      await tap(tester, find.text('Add'));

      expect(find.text('Update: sauce bought'), findsOneWidget);
      expect(find.text(shown(DateTime.utc(2026, 10, 24, 11))), findsOneWidget);
      final section = notes.sections(note.id).single;
      expect(section.text, 'Update: sauce bought');
      expect(section.context.captured, DateTime.utc(2026, 10, 24, 11));
      expect(notes.byId(note.id)?.text, 'nees mor sauce');
    });

    testWidgets('Importance shows on the card', (tester) async {
      notes.capture('pills');
      notes.capture('plants');
      notes.capture('socks');
      await startAtNotes(tester);
      expect(emboss(tester, 'pills').strong, isFalse);
      expect(dimming(tester, 'pills'), isEmpty);
      expect(find.byIcon(Icons.label_important), findsNothing);

      await edit(tester, find.text('pills'));
      await tap(tester, find.text('Important'));
      await tester.pageBack();
      await tester.pumpAndSettle();
      await edit(tester, find.text('socks'));
      await tap(tester, find.text('Unimportant'));
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(emboss(tester, 'pills').strong, isTrue);
      expect(
        find.descendant(
          of: find.byWidget(card(tester, 'pills')),
          matching: find.byIcon(Icons.label_important),
        ),
        findsOneWidget,
      );
      expect(
        dimming(tester, 'socks').single.opacity,
        NoteCard.unimportantOpacity,
      );
      expect(emboss(tester, 'socks').strong, isFalse);
      expect(emboss(tester, 'plants').strong, isFalse);
      expect(dimming(tester, 'plants'), isEmpty);
      expect(find.byIcon(Icons.label_important), findsOneWidget);
    });

    testWidgets('the editor has no bottom bar', (tester) async {
      notes.capture('milk');
      await startAtNotes(tester);
      await edit(tester, find.text('milk'));
      final scaffold = tester.widget<Scaffold>(
        find.descendant(
          of: find.byType(NoteEditorScreen),
          matching: find.byType(Scaffold),
        ),
      );
      expect(scaffold.bottomNavigationBar, isNull);
      expect(scaffold.bottomSheet, isNull);
      expect(find.byType(BottomAppBar), findsNothing);
    });
  });

  group('trash', () {
    Future<void> openTrash(WidgetTester tester) async {
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('Trash'));
    }

    testWidgets('a deleted Note goes to the trash and can be restored', (
      tester,
    ) async {
      final note = notes.capture('nees mor sauce', place: home.id);
      notes.capture('milk');
      await startAtNotes(tester);
      await edit(tester, find.text('nees mor sauce'));
      await tap(tester, find.byTooltip('Move to trash'));
      await tap(tester, find.text('Delete'));

      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.text('nees mor sauce'), findsNothing);
      expect(find.text('milk'), findsOneWidget);
      expect(notes.byId(note.id)?.isUnsorted, isFalse);

      tick(const Duration(days: 10));
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('Trash'));
      expect(find.byType(TrashScreen), findsOneWidget);
      expect(find.text('nees mor sauce'), findsOneWidget);
      expect(find.text('milk'), findsNothing);
      expect(find.text('Deleted for good in 20 days'), findsOneWidget);

      await tap(tester, find.text('Restore'));
      expect(find.text('nees mor sauce'), findsNothing);
      expect(find.text('The trash is empty.'), findsOneWidget);
      expect(notes.byId(note.id)?.place, home.id);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('All Notes'));
      expect(find.text('nees mor sauce'), findsOneWidget);
    });

    testWidgets('after 30 days the trash has purged the Note', (tester) async {
      final note = notes.capture('nees mor sauce');
      tick();
      notes.trash(note.id);
      tick(const Duration(days: 30));
      await start(tester);
      expect(notes.byId(note.id), isNull);
      await openTrash(tester);
      expect(find.text('The trash is empty.'), findsOneWidget);
    });
  });

  group('diary order', () {
    testWidgets('an Area orders by time or Importance and hides unimportant', (
      tester,
    ) async {
      final kitchen = places.addTopic(home.id, 'Kitchen');
      Note capture(String text, String place, Importance importance) {
        tick();
        final note = notes.capture(text, place: place);
        notes.setImportance(note.id, importance);
        return note;
      }

      capture('important, old', home.id, Importance.important);
      capture('normal', home.id, Importance.normal);
      capture('unimportant, new', home.id, Importance.unimportant);
      capture('important, in the kitchen', kitchen.id, Importance.important);
      capture(
        'unimportant, in the kitchen',
        kitchen.id,
        Importance.unimportant,
      );
      List<String> cards() => [
        for (final card in tester.widgetList<NoteCard>(find.byType(NoteCard)))
          card.note.text,
      ];

      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.text('Household'));
      expect(cards(), ['unimportant, new', 'normal', 'important, old']);

      await tap(tester, find.byTooltip('Order by Importance'));
      expect(cards(), ['important, old', 'normal', 'unimportant, new']);
      expect(find.byTooltip('Order by time'), findsOneWidget);

      await tap(tester, find.byTooltip('Hide unimportant Notes'));
      expect(cards(), ['important, old', 'normal']);

      await tap(tester, find.byTooltip('Show unimportant Notes'));
      await tap(tester, find.byTooltip('Order by time'));
      expect(cards(), ['unimportant, new', 'normal', 'important, old']);

      await tap(tester, find.text('Kitchen'));
      await tap(tester, find.byTooltip('Hide unimportant Notes'));
      expect(cards(), ['important, in the kitchen']);
    });
  });

  testWidgets('editor, diary and trash fit a phone in German', (tester) async {
    tester.view.physicalSize = const Size(1080, 2340);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final long = places.addArea('Ein ziemlich langer Bereichsname');
    final note = notes.capture('Soße kaufen', place: long.id);
    notes.amend(note.id, 'Gekauft');
    notes.trash(notes.capture('weg', place: long.id).id);

    await start(tester);
    await tap(tester, find.byTooltip('Suchen und stöbern'));
    await tap(tester, find.byTooltip('Alle Notizen'));
    await edit(tester, find.text('Soße kaufen'));
    await tap(tester, find.text('Unwichtig'));
    await reveal(tester, find.text('Zur Notiz hinzufügen'));
    expect(find.text('Zur Notiz hinzufügen'), findsOneWidget);
    await tap(tester, find.byType(BackButton));
    await tap(tester, find.byType(BackButton));
    await tap(tester, find.text('Ein ziemlich langer Bereichsname'));
    await tap(tester, find.byTooltip('Nach Wichtigkeit ordnen'));
    await tap(tester, find.byType(BackButton));
    await tap(tester, find.byTooltip('Papierkorb'));
    expect(find.text('Wird in 30 Tagen endgültig gelöscht'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

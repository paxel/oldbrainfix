import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix/src/shopping_list_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// A Note whose Lines come back a while after they are checked (#122):
/// made from the card's ⋮ and the editor, checked on full cards, counted
/// on compact ones, added to and edited in the editor, within the roles
/// of a Shared Area and picked on the Conflicts screen.
void main() {
  late Store store;
  late Notes notes;
  late ShoppingLists lists;
  var now = DateTime.utc(2026, 10, 24, 8);
  // Each write a moment later, as on a Device: the Line added last is
  // the latest and shows at the top.
  DateTime tick() => now = now.add(const Duration(microseconds: 1));

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: tick)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    lists = ShoppingLists(store);
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    if (finder.evaluate().isEmpty) await reveal(tester, finder);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Starts the app on [on] and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester, [Store? on]) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: on ?? store));
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  Future<void> cardMenu(WidgetTester tester) => tap(tester, key('note-menu'));

  Future<void> closeMenu(WidgetTester tester) async {
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
  }

  ShoppingLine lineOf(String note, String text, [Store? on]) =>
      ShoppingLists(on ?? store)
          .byNote(note, deviceWallClock)!
          .lines
          .firstWhere((l) => l.text == text);

  String idOf(String note, String text, [Store? on]) =>
      lineOf(note, text, on).id;

  /// The Lines of [note] in their order, `[x]` before a struck one.
  List<String> shown(String note, [Store? on]) => [
    for (final l
        in ShoppingLists(on ?? store).byNote(note, deviceWallClock)?.lines ??
            <ShoppingLine>[])
      '${l.struck ? '[x] ' : ''}${l.text}',
  ];

  bool struck(WidgetTester tester, String lineId) =>
      tester
          .widget<Text>(key('checklist-line-$lineId'))
          .style
          ?.decoration
          ?.contains(TextDecoration.lineThrough) ??
      false;

  double top(WidgetTester tester, String lineId) =>
      tester.getTopLeft(key('checklist-line-$lineId')).dy;

  /// "back Sat, Oct 31" as the card shows it for [line].
  String back(ShoppingLine line) =>
      'back ${DateFormat.MMMEd('en').format(line.backOn!.wall)}';

  group('on the card', () {
    testWidgets('"Make it a Shopping List" in the ⋮ makes its lines Lines; '
        'it is then neither a Checklist nor a Tally', (tester) async {
      final note = notes.capture('Shop\nMilk\nEggs').id;
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(key('note-make-checklist'), findsOneWidget);
      expect(key('note-to-plain'), findsNothing);
      await tap(tester, key('note-make-shopping'));

      expect(notes.byId(note)!.text, 'Shop');
      expect(shown(note), ['Milk', 'Eggs']);
      expect(notes.unsorted(), isEmpty);
      for (final text in ['Milk', 'Eggs']) {
        expect(key('checklist-check-${idOf(note, text)}'), findsOneWidget);
      }
      expect(lineOf(note, 'Milk').reappear, Reappear.week1);

      await cardMenu(tester);
      expect(key('note-make-shopping'), findsNothing);
      expect(key('note-make-checklist'), findsNothing);
      expect(key('note-make-tally'), findsNothing);
      expect(key('note-uncheck-all'), findsNothing);
      expect(key('note-to-plain'), findsOneWidget);
    });

    testWidgets('a full card checks a Line: struck through, at the end, with '
        'the day it comes back; on that day it is open at the top', (
      tester,
    ) async {
      final note = notes.capture('Shop\nMilk\nEggs\nTea').id;
      lists.make(note);
      final milk = idOf(note, 'Milk');
      final tea = idOf(note, 'Tea');
      await startAtNotes(tester);
      expect(key('shopping-card'), findsOneWidget);
      expect(top(tester, milk), lessThan(top(tester, tea)));

      await tap(tester, key('checklist-check-$milk'));
      expect(shown(note), ['Eggs', 'Tea', '[x] Milk']);
      expect(struck(tester, milk), isTrue);
      expect(top(tester, milk), greaterThan(top(tester, tea)));
      final line = lineOf(note, 'Milk');
      expect(line.backOn, Day.of(deviceWallClock(now)).addDays(7));
      expect(find.text(back(line)), findsOneWidget);

      // Unchecking brings it back now, at the top.
      await tap(tester, key('checklist-check-$milk'));
      expect(shown(note), ['Milk', 'Eggs', 'Tea']);
      expect(find.textContaining('back '), findsNothing);

      // Checked again, it comes back by itself when that day begins.
      await tap(tester, key('checklist-check-$milk'));
      now = now.add(const Duration(days: 7));
      await tester.pump(const Duration(days: 1));
      await tester.pumpAndSettle();
      expect(struck(tester, milk), isFalse);
      expect(top(tester, milk), lessThan(top(tester, tea)));
      expect(find.textContaining('back '), findsNothing);
    });

    testWidgets('a compact card shows how many are open', (tester) async {
      final note = notes.capture('Shop\nMilk\n✓ Eggs\nTea').id;
      lists.make(note);
      await startAtNotes(tester);
      await tap(tester, key('compactNotes'));
      expect(find.text('2 open'), findsOneWidget);
      expect(find.byType(Checkbox), findsNothing);
    });

    testWidgets('"Back to a plain Note" in the ⋮ writes every Line into the '
        'text, struck ones marked', (tester) async {
      final note = notes.capture('Shop\nMilk\n✓ Eggs').id;
      lists.make(note);
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-to-plain'));
      expect(lists.isShoppingList(note), isFalse);
      expect(notes.byId(note)!.text, 'Shop\nMilk\n✓ Eggs');
      expect(find.byType(Checkbox), findsNothing);
      await cardMenu(tester);
      expect(key('note-make-shopping'), findsOneWidget);
    });
  });

  group('in the editor', () {
    Future<void> openEditor(WidgetTester tester, String text) async {
      await startAtNotes(tester);
      await tester.longPress(find.text(text).first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    }

    Future<void> lineMenu(WidgetTester tester, String id) =>
        tap(tester, key('checklist-menu-$id'));

    TextField addField(WidgetTester tester) => tester.widget<TextField>(
      find.descendant(
        of: key('shopping-add-text'),
        matching: find.byType(TextField),
      ),
    );

    testWidgets('"Make it a Shopping List" makes the text as typed one; the '
        'field keeps the title, also after leaving', (tester) async {
      final note = notes.capture('Shop').id;
      await openEditor(tester, 'Shop');
      await tester.enterText(find.byType(TextField).first, 'Shop\nMilk\nEggs');
      await tap(tester, key('shopping-make'));
      expect(shown(note), ['Milk', 'Eggs']);
      expect(key('shopping'), findsOneWidget);
      expect(key('shopping-make'), findsNothing);
      expect(key('checklist-make'), findsNothing);
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller!.text,
        'Shop',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note)!.text, 'Shop');
      expect(shown(note), ['Milk', 'Eggs']);
    });

    testWidgets('the "Add" field at the top adds Lines on top, and a name '
        'already there brings its struck Line back instead', (tester) async {
      final note = notes.capture('Shop\nMilk\nEggs').id;
      lists.make(note);
      await openEditor(tester, 'Shop');
      final milk = idOf(note, 'Milk');
      expect(
        tester.getTopLeft(key('shopping-add-text')).dy,
        lessThan(top(tester, milk)),
      );

      await tester.enterText(key('shopping-add-text'), 'Tea\nBread');
      await tap(tester, key('shopping-add'));
      // In the order typed, above the Lines there were.
      expect(shown(note), ['Tea', 'Bread', 'Milk', 'Eggs']);
      expect(addField(tester).controller!.text, isEmpty);

      await tap(tester, key('checklist-check-$milk'));
      expect(shown(note), ['Tea', 'Bread', 'Eggs', '[x] Milk']);
      await tester.enterText(key('shopping-add-text'), '  milk ');
      await tap(tester, key('shopping-add'));
      expect(shown(note), ['Milk', 'Tea', 'Bread', 'Eggs']);
      // An open one stays where it is.
      await tester.enterText(key('shopping-add-text'), 'EGGS');
      await tap(tester, key('shopping-add'));
      expect(shown(note), ['Milk', 'Tea', 'Bread', 'Eggs']);
      expect(find.byType(Checkbox), findsNWidgets(4));
    });

    testWidgets('a Line\'s ⋮ renames it, picks its reappear time as radio '
        'buttons and deletes it for good', (tester) async {
      final note = notes.capture('Shop\nMilk\nEggs').id;
      lists.make(note);
      await openEditor(tester, 'Shop');
      final milk = idOf(note, 'Milk');

      await lineMenu(tester, milk);
      final picked = [
        for (final code in ['once', '1w', '2w', '3w', '4w'])
          tester
              .widget<Icon>(
                find.descendant(
                  of: key('shopping-reappear-$code'),
                  matching: find.byType(Icon),
                ),
              )
              .icon,
      ];
      expect(picked, [
        Icons.radio_button_unchecked,
        Icons.radio_button_checked,
        Icons.radio_button_unchecked,
        Icons.radio_button_unchecked,
        Icons.radio_button_unchecked,
      ]);
      for (final label in [
        'Reappear time',
        'Once',
        '1 week',
        '2 weeks',
        '4 weeks',
        'Delete',
      ]) {
        expect(find.text(label), findsOneWidget);
      }
      await tap(tester, key('shopping-reappear-3w'));
      expect(lineOf(note, 'Milk').reappear, Reappear.week3);
      await tap(tester, key('checklist-check-$milk'));
      expect(
        lineOf(note, 'Milk').backOn,
        Day.of(deviceWallClock(now)).addDays(21),
      );

      // Once: gone when checked.
      final eggs = idOf(note, 'Eggs');
      await lineMenu(tester, eggs);
      await tap(tester, key('shopping-reappear-once'));
      await tap(tester, key('checklist-check-$eggs'));
      expect(shown(note), ['[x] Milk']);
      expect(key('checklist-line-$eggs'), findsNothing);

      await lineMenu(tester, milk);
      await tap(tester, key('shopping-rename'));
      await tester.enterText(
        find.descendant(
          of: key('checklist-rename-text'),
          matching: find.byType(TextField),
        ),
        'Oat milk',
      );
      await tap(tester, key('checklist-rename-save'));
      expect(shown(note), ['[x] Oat milk']);

      // A long press opens the same menu where the finger is.
      await tester.longPress(key('checklist-line-$milk'));
      await tester.pumpAndSettle();
      await tap(tester, key('shopping-delete'));
      expect(find.byType(AlertDialog), findsOneWidget);
      await tap(tester, key('checklist-delete-confirm'));
      expect(shown(note), isEmpty);
      expect(find.byType(BottomSheet), findsNothing);
    });

    testWidgets('"Back to a plain Note" there too; the field then holds the '
        'whole text', (tester) async {
      final note = notes.capture('Shop\nMilk\nEggs').id;
      lists.make(note);
      await openEditor(tester, 'Shop');
      await tap(tester, key('checklist-check-${idOf(note, 'Milk')}'));
      await tap(tester, key('shopping-to-plain'));
      expect(lists.isShoppingList(note), isFalse);
      expect(key('shopping'), findsNothing);
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller!.text,
        'Shop\nEggs\n✓ Milk',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note)!.text, 'Shop\nEggs\n✓ Milk');
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String note;

    setUp(() {
      store
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ann');
      ben = Store.inMemory(clock: tick)
        ..starterSetOffered = true
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ben');
      final house = Places(store).addArea('Household');
      note = notes.capture('Shop\nMilk\nEggs', place: house.id).id;
      lists.make(note);
      final area = store.share(house.id);
      // Ann's computer invites Ben's phone as a Member.
      final done = answerSync(
        store,
        wire(
          syncRequest(
            ben,
            store.versionVector(),
            name: 'Phone',
            hostDevice: store.deviceId,
            invited: true,
          ),
        ),
        name: 'Desk',
        inviting: Invite(area, role: AreaRole.member),
      )!;
      applyAnswer(ben, wire(done.answer), shared: true);
    });
    tearDown(() => ben.close());

    testWidgets('a Member checks and adds Lines; renaming, the reappear time, '
        'deleting and the kind stay with the author', (tester) async {
      expect(shown(note, ben), ['Milk', 'Eggs']);
      final milk = idOf(note, 'Milk', ben);
      await startAtNotes(tester, ben);
      await cardMenu(tester);
      expect(key('note-to-plain'), findsNothing);
      expect(key('note-make-shopping'), findsNothing);
      await closeMenu(tester);
      await tap(tester, key('checklist-check-$milk'));
      expect(shown(note, ben), ['Eggs', '[x] Milk']);

      await tester.longPress(find.text('Shop'));
      await tester.pumpAndSettle();
      expect(key('checklist-menu-$milk'), findsNothing);
      expect(key('shopping-to-plain'), findsNothing);
      await tester.longPress(key('checklist-line-$milk'));
      await tester.pumpAndSettle();
      expect(key('shopping-rename'), findsNothing);

      await tester.enterText(key('shopping-add-text'), 'Bread');
      await tap(tester, key('shopping-add'));
      expect(shown(note, ben), ['Bread', 'Eggs', '[x] Milk']);
      // His own Line is his to change.
      final bread = idOf(note, 'Bread', ben);
      await tap(tester, key('checklist-menu-$bread'));
      expect(key('shopping-rename'), findsOneWidget);
      expect(key('shopping-reappear-2w'), findsOneWidget);
      expect(key('shopping-delete'), findsOneWidget);
    });
  });

  group('elsewhere', () {
    Widget screen(Widget child, {Locale locale = const Locale('en')}) =>
        MaterialApp(
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: child,
        );

    testWidgets('two renames of one Line apart are a Conflict on the '
        'Conflicts screen, under the Note it belongs to', (tester) async {
      final desk = Store.inMemory(clock: () => now);
      addTearDown(desk.close);
      final note = notes.capture('Shop\nMilk').id;
      lists.make(note);
      desk.applyEntries(
        store.entriesSince(const {}),
        keys: [store.ownKeyRecord],
      );
      final milk = idOf(note, 'Milk');
      lists.rename(milk, 'Oat milk');
      ShoppingLists(desk).rename(milk, 'Soy milk');
      store.applyEntries(
        desk.entriesSince(const {}),
        keys: [desk.ownKeyRecord],
      );
      expect(store.conflicts(), hasLength(1));

      await tester.pumpWidget(screen(ConflictsScreen(store: store)));
      await tester.pumpAndSettle();
      expect(find.text('Line of a Shopping List'), findsOneWidget);
      expect(find.text('Shop'), findsOneWidget);
      await tester.tap(find.text('Soy milk'));
      await tester.pumpAndSettle();
      expect(find.text('No conflicts. Every Device agrees.'), findsOneWidget);
      expect(shown(note), ['Soy milk']);
    });

    testWidgets('a Sent Item shows its Lines open with their reappear '
        'times, and an update that it becomes a Shopping List', (tester) async {
      final ann = Store.inMemory(clock: tick)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(ann.close);
      final note = Notes(ann).capture('Shop\nMilk\nEggs').id;
      final theirs = ShoppingLists(ann)..make(note);
      final eggs = theirs
          .byNote(note, deviceWallClock)!
          .lines
          .firstWhere((l) => l.text == 'Eggs')
          .id;
      theirs.setReappear(eggs, Reappear.once);
      theirs.check(
        theirs
            .byNote(note, deviceWallClock)!
            .lines
            .firstWhere((l) => l.text == 'Milk')
            .id,
        deviceWallClock,
      );
      final opened = openSentItem(store, sendNote(ann, note));
      await tester.pumpWidget(
        screen(SentItemScreen(store: store, opened: opened)),
      );
      await tester.pumpAndSettle();
      for (final text in ['Milk', 'Reappear time: 1 week']) {
        expect(
          find.descendant(of: key('sent-lines'), matching: find.text(text)),
          findsOneWidget,
        );
      }
      // Eggs is Once and not checked, so it goes along; Milk arrives open.
      expect(find.text('Reappear time: Once'), findsOneWidget);
      final checks = tester.widgetList<Checkbox>(
        find.descendant(of: key('sent-lines'), matching: find.byType(Checkbox)),
      );
      expect([for (final c in checks) c.value], [false, false]);

      final plain = Notes(ann).capture('Hardware\nNails').id;
      acceptSentItem(store, openSentItem(store, sendNote(ann, plain)).item);
      ShoppingLists(ann).make(plain);
      final update = openSentItem(store, sendNote(ann, plain));
      await tester.pumpWidget(
        screen(
          SentItemScreen(
            key: const ValueKey('update'),
            store: store,
            opened: update,
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('It becomes a Shopping List.'), findsOneWidget);
      expect(find.text('New Line: Nails'), findsOneWidget);
    });

    testWidgets('in German the day it comes back reads "wieder" with the '
        'short weekday and date', (tester) async {
      final note = notes.capture('Einkauf\nMilch').id;
      lists.make(note);
      lists.check(idOf(note, 'Milch'), deviceWallClock);
      await tester.pumpWidget(
        screen(
          Scaffold(
            body: SingleChildScrollView(
              child: ShoppingSection(notes: notes, noteId: note),
            ),
          ),
          locale: const Locale('de'),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Einkaufsliste'), findsOneWidget);
      expect(find.text('wieder Sa., 31. Okt.'), findsOneWidget);
    });
  });
}

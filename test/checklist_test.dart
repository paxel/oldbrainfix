import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// A Note whose Lines are checked off (#120): made from the card's ⋮ and
/// the editor, checked on full cards, counted on compact ones, edited in
/// the editor, within the roles of a Shared Area, picked on the
/// Conflicts screen and shown in a Sent Item's preview.
void main() {
  late Store store;
  late Notes notes;
  late Checklists lists;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    lists = Checklists(store);
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

  String idOf(String note, String text, [Store? on]) =>
      Checklists(on ?? store)
          .byNote(note)!
          .lines
          .firstWhere((l) => l.text == text)
          .id;

  /// The Lines of [note] as `text`, `  text` for one of a group, `[x]`
  /// for one shown checked.
  List<String> shown(String note, [Store? on]) => [
    for (final l in Checklists(on ?? store).byNote(note)?.lines ?? <Line>[])
      '${l.inGroup ? '  ' : ''}${l.checked ? '[x] ' : ''}${l.text}',
  ];

  bool struck(WidgetTester tester, String lineId) =>
      tester
          .widget<Text>(key('checklist-line-$lineId'))
          .style
          ?.decoration
          ?.contains(TextDecoration.lineThrough) ??
      false;

  bool? checkbox(WidgetTester tester, String lineId) =>
      tester.widget<Checkbox>(key('checklist-check-$lineId')).value;

  group('on the card', () {
    testWidgets('"Make it a Checklist" in the ⋮ makes its lines Lines, an '
        'indented one under the line above, the group in a frame under '
        'its parent', (tester) async {
      final note = notes.capture('Trip\nBath\n  Soap\nTent').id;
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(key('note-to-plain'), findsNothing);
      await tap(tester, key('note-make-checklist'));

      expect(notes.byId(note)!.text, 'Trip');
      expect(shown(note), ['Bath', '  Soap', 'Tent']);
      expect(notes.unsorted(), isEmpty);
      final bath = idOf(note, 'Bath');
      final soap = idOf(note, 'Soap');
      final tent = idOf(note, 'Tent');
      for (final id in [bath, soap, tent]) {
        expect(key('checklist-check-$id'), findsOneWidget);
      }
      final frame = key('checklist-group-$bath');
      expect(
        find.descendant(of: frame, matching: key('checklist-line-$soap')),
        findsOneWidget,
      );
      expect(
        find.descendant(of: frame, matching: key('checklist-line-$bath')),
        findsNothing,
      );
      expect(
        tester.getTopLeft(frame).dy,
        greaterThan(tester.getTopLeft(key('checklist-line-$bath')).dy),
      );
      expect(
        tester.getTopLeft(key('checklist-check-$soap')).dx,
        greaterThan(tester.getTopLeft(key('checklist-check-$bath')).dx),
      );

      // A Checklist is no Tally, and is made only once.
      await cardMenu(tester);
      expect(key('note-make-checklist'), findsNothing);
      expect(key('note-make-tally'), findsNothing);
      expect(key('note-to-plain'), findsOneWidget);
    });

    testWidgets('a full card checks its Lines in place, struck through; the '
        'last of a group checks its parent', (tester) async {
      final note = notes.capture('Trip\nBath\n  Soap\nTent').id;
      lists.make(note);
      final bath = idOf(note, 'Bath');
      final soap = idOf(note, 'Soap');
      final tent = idOf(note, 'Tent');
      await startAtNotes(tester);
      final before = tester.getTopLeft(key('checklist-line-$tent')).dy;

      await tap(tester, key('checklist-check-$soap'));
      expect(shown(note), ['[x] Bath', '  [x] Soap', 'Tent']);
      expect(checkbox(tester, bath), isTrue);
      expect(struck(tester, soap), isTrue);
      expect(struck(tester, bath), isTrue);
      expect(struck(tester, tent), isFalse);
      // It stays where it is.
      expect(tester.getTopLeft(key('checklist-line-$tent')).dy, before);
      expect(notes.byId(note)!.text, 'Trip');

      await tap(tester, key('checklist-check-$bath'));
      expect(shown(note), ['Bath', '  Soap', 'Tent']);
      expect(checkbox(tester, soap), isFalse);
    });

    testWidgets('a compact card shows how many are checked; "Uncheck all" '
        'in the ⋮ unchecks them', (tester) async {
      final note = notes.capture('Shop\n✓ Milk\nEggs\n✓ Tea').id;
      lists.make(note);
      await startAtNotes(tester);
      await tap(tester, key('compactNotes'));
      expect(find.text('2 of 3 checked'), findsOneWidget);
      expect(find.byType(Checkbox), findsNothing);

      await cardMenu(tester);
      await tap(tester, key('note-uncheck-all'));
      expect(shown(note), ['Milk', 'Eggs', 'Tea']);
      expect(find.text('0 of 3 checked'), findsOneWidget);
      await cardMenu(tester);
      // Nothing left to uncheck.
      expect(key('note-uncheck-all'), findsNothing);
    });

    testWidgets('"Back to a plain Note" in the ⋮ writes every Line into the '
        'text, checked ones marked, group Lines indented', (tester) async {
      final note = notes.capture('Trip\nBath\n  Soap\n✓ Tent').id;
      lists.make(note);
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-to-plain'));
      expect(lists.isChecklist(note), isFalse);
      expect(notes.byId(note)!.text, 'Trip\nBath\n  Soap\n✓ Tent');
      expect(find.byType(Checkbox), findsNothing);
      await cardMenu(tester);
      expect(key('note-make-checklist'), findsOneWidget);
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

    testWidgets('"Make it a Checklist" makes the text as typed one; the '
        'field keeps the title, also after leaving', (tester) async {
      final note = notes.capture('Shop').id;
      await openEditor(tester, 'Shop');
      await tester.enterText(find.byType(TextField).first, 'Shop\nMilk\nEggs');
      await tap(tester, key('checklist-make'));
      expect(shown(note), ['Milk', 'Eggs']);
      expect(key('checklist'), findsOneWidget);
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

    testWidgets('Lines are added, renamed, moved, indented and deleted', (
      tester,
    ) async {
      final note = notes.capture('Shop\nMilk\nEggs').id;
      lists.make(note);
      await openEditor(tester, 'Shop');

      // Each line typed becomes a Line; one starting with a space goes
      // under the Line above.
      await tester.enterText(key('checklist-add-text'), 'Tea\n  Green');
      await tap(tester, key('checklist-add'));
      expect(shown(note), ['Milk', 'Eggs', 'Tea', '  Green']);
      expect(
        tester
            .widget<TextField>(
              find.descendant(
                of: key('checklist-add-text'),
                matching: find.byType(TextField),
              ),
            )
            .controller!
            .text,
        isEmpty,
      );

      final milk = idOf(note, 'Milk');
      await lineMenu(tester, milk);
      // The first Line has nothing above to move past or go under.
      expect(key('checklist-up'), findsNothing);
      expect(key('checklist-indent'), findsNothing);
      await tap(tester, key('checklist-rename'));
      await tester.enterText(
        find.descendant(
          of: key('checklist-rename-text'),
          matching: find.byType(TextField),
        ),
        'Oat milk',
      );
      await tap(tester, key('checklist-rename-save'));
      expect(shown(note), ['Oat milk', 'Eggs', 'Tea', '  Green']);

      await lineMenu(tester, milk);
      await tap(tester, key('checklist-down'));
      expect(shown(note), ['Eggs', 'Oat milk', 'Tea', '  Green']);
      await lineMenu(tester, milk);
      await tap(tester, key('checklist-down'));
      // Past the next Line with its group, never into it.
      expect(shown(note), ['Eggs', 'Tea', '  Green', 'Oat milk']);
      await lineMenu(tester, milk);
      expect(key('checklist-down'), findsNothing);
      await tap(tester, key('checklist-up'));
      expect(shown(note), ['Eggs', 'Oat milk', 'Tea', '  Green']);

      final eggs = idOf(note, 'Eggs');
      await lineMenu(tester, milk);
      await tap(tester, key('checklist-indent'));
      expect(shown(note), ['Eggs', '  Oat milk', 'Tea', '  Green']);
      expect(key('checklist-group-$eggs'), findsOneWidget);

      // A Line of a group moves within it.
      final tea = idOf(note, 'Tea');
      await tester.enterText(key('checklist-add-text'), ' Black');
      await tap(tester, key('checklist-add'));
      final green = idOf(note, 'Green');
      await lineMenu(tester, green);
      expect(key('checklist-up'), findsNothing);
      await tap(tester, key('checklist-down'));
      expect(shown(note), ['Eggs', '  Oat milk', 'Tea', '  Black', '  Green']);
      await lineMenu(tester, green);
      await tap(tester, key('checklist-outdent'));
      expect(shown(note), ['Eggs', '  Oat milk', 'Tea', '  Black', 'Green']);
      expect(key('checklist-group-$tea'), findsOneWidget);

      // A long press opens the same menu where the finger is.
      await tester.longPress(key('checklist-line-$green'));
      await tester.pumpAndSettle();
      await tap(tester, key('checklist-delete'));
      expect(find.byType(AlertDialog), findsOneWidget);
      await tap(tester, key('checklist-delete-confirm'));
      expect(shown(note), ['Eggs', '  Oat milk', 'Tea', '  Black']);
      expect(find.byType(BottomSheet), findsNothing);
    });

    testWidgets('checks, "Uncheck all" and "Back to a plain Note" there too; '
        'the field then holds the whole text', (tester) async {
      final note = notes.capture('Shop\nMilk\nEggs').id;
      lists.make(note);
      await openEditor(tester, 'Shop');
      final milk = idOf(note, 'Milk');
      expect(key('checklist-uncheck-all'), findsNothing);
      await tap(tester, key('checklist-check-$milk'));
      expect(shown(note), ['[x] Milk', 'Eggs']);
      expect(struck(tester, milk), isTrue);
      await tap(tester, key('checklist-uncheck-all'));
      expect(shown(note), ['Milk', 'Eggs']);

      await tap(tester, key('checklist-check-$milk'));
      await tap(tester, key('checklist-to-plain'));
      expect(lists.isChecklist(note), isFalse);
      expect(key('checklist'), findsNothing);
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller!.text,
        'Shop\n✓ Milk\nEggs',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note)!.text, 'Shop\n✓ Milk\nEggs');
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String note;

    setUp(() {
      store
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ann');
      ben = Store.inMemory()
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

    testWidgets('a Member checks and adds Lines; renaming, deleting, moving '
        'and the kind stay with the author', (tester) async {
      expect(shown(note, ben), ['Milk', 'Eggs']);
      final milk = idOf(note, 'Milk', ben);
      await startAtNotes(tester, ben);
      await cardMenu(tester);
      expect(key('note-to-plain'), findsNothing);
      expect(key('note-make-checklist'), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();
      await tap(tester, key('checklist-check-$milk'));
      expect(shown(note, ben), ['[x] Milk', 'Eggs']);
      await cardMenu(tester);
      expect(key('note-uncheck-all'), findsOneWidget);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tester.longPress(find.text('Shop'));
      await tester.pumpAndSettle();
      expect(key('checklist-menu-$milk'), findsNothing);
      expect(key('checklist-to-plain'), findsNothing);
      await tester.longPress(key('checklist-line-$milk'));
      await tester.pumpAndSettle();
      expect(key('checklist-rename'), findsNothing);

      await tester.enterText(key('checklist-add-text'), 'Bread');
      await tap(tester, key('checklist-add'));
      expect(shown(note, ben), ['[x] Milk', 'Eggs', 'Bread']);
      // His own Line is his to change.
      final bread = idOf(note, 'Bread', ben);
      await tap(tester, key('checklist-menu-$bread'));
      expect(key('checklist-rename'), findsOneWidget);
      expect(key('checklist-delete'), findsOneWidget);
    });
  });

  group('elsewhere', () {
    Widget screen(Widget child) => MaterialApp(
      locale: const Locale('en'),
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
      Checklists(desk).rename(milk, 'Soy milk');
      store.applyEntries(
        desk.entriesSince(const {}),
        keys: [desk.ownKeyRecord],
      );
      expect(store.conflicts(), hasLength(1));

      await tester.pumpWidget(screen(ConflictsScreen(store: store)));
      await tester.pumpAndSettle();
      expect(find.text('Line of a Checklist'), findsOneWidget);
      expect(find.text('Shop'), findsOneWidget);
      expect(find.text('Oat milk'), findsOneWidget);
      expect(find.text('Soy milk'), findsOneWidget);
      await tester.tap(find.text('Soy milk'));
      await tester.pumpAndSettle();
      expect(find.text('No conflicts. Every Device agrees.'), findsOneWidget);
      expect(shown(note), ['Soy milk']);
    });

    testWidgets('a Line Conflict of a closed locked Area stays out', (
      tester,
    ) async {
      final desk = Store.inMemory(clock: () => now);
      addTearDown(desk.close);
      final area = Places(store).addArea('Doctor');
      final note = notes.capture('Ask\nBlood test', place: area.id).id;
      lists.make(note);
      desk.applyEntries(
        store.entriesSince(const {}),
        keys: [store.ownKeyRecord],
      );
      final line = idOf(note, 'Blood test');
      lists.rename(line, 'Blood test on Monday');
      Checklists(desk).rename(line, 'Blood test on Tuesday');
      store.applyEntries(
        desk.entriesSince(const {}),
        keys: [desk.ownKeyRecord],
      );
      expect(store.conflicts(), hasLength(1));
      store.setPin('1234', iterations: 1);
      store.setAreaLocked(area.id, true);
      final lock = AppLock(store: store);
      addTearDown(lock.dispose);

      Future<void> show(Key key) async {
        await tester.pumpWidget(
          screen(
            LockScope(
              lock: lock,
              child: ConflictsScreen(key: key, store: store),
            ),
          ),
        );
        await tester.pumpAndSettle();
      }

      await show(const ValueKey('closed'));
      expect(find.text('Line of a Checklist'), findsNothing);
      lock.openArea(area.id);
      await show(const ValueKey('open'));
      expect(find.text('Line of a Checklist'), findsOneWidget);
    });

    testWidgets('a Sent Item shows its Lines, and an update what it does '
        'to them', (tester) async {
      final ann = Store.inMemory(clock: () => now)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(ann.close);
      final note = Notes(ann).capture('Trip\nBath\n  Soap\n✓ Tent').id;
      Checklists(ann).make(note);
      final first = openSentItem(store, sendNote(ann, note));
      await tester.pumpWidget(
        screen(SentItemScreen(store: store, opened: first)),
      );
      await tester.pumpAndSettle();
      expect(key('sent-lines'), findsOneWidget);
      for (final text in ['Bath', 'Soap', 'Tent']) {
        expect(
          find.descendant(of: key('sent-lines'), matching: find.text(text)),
          findsOneWidget,
        );
      }
      final checks = tester
          .widgetList<Checkbox>(
            find.descendant(
              of: key('sent-lines'),
              matching: find.byType(Checkbox),
            ),
          )
          .toList();
      expect([for (final c in checks) c.value], [false, false, true]);
      // To look at, not to check.
      expect(checks.every((c) => c.onChanged == null), isTrue);
      final bath = first.item.lines!.first.id;
      expect(key('checklist-group-$bath'), findsOneWidget);

      acceptSentItem(store, first.item);
      final lines = Checklists(ann);
      lines.rename(idOf(note, 'Bath', ann), 'Shower');
      lines.delete(idOf(note, 'Tent', ann));
      lines.add(note, 'Torch');
      final update = openSentItem(store, sendNote(ann, note));
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
      expect(find.text('New Line: Torch'), findsOneWidget);
      expect(find.text('Line: Bath → Shower'), findsOneWidget);
      expect(find.text('Line removed: Tent'), findsOneWidget);
    });
  });
}

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  var now = DateTime.utc(2026, 11, 10, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 10, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    places = Places(store);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  void tick() => now = now.add(const Duration(minutes: 1));

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// A tall phone, so every card of the list is built.
  Future<void> start(WidgetTester tester, [Store? on]) async {
    tester.view.physicalSize = const Size(1080, 4800);
    tester.view.devicePixelRatio = 2.625;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: on ?? store));
    await tester.pumpAndSettle();
  }

  Future<void> openAllNotes(WidgetTester tester) async {
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// The box on the card of the Note [id].
  Finder box(String id) => key('select-$id');

  bool ticked(WidgetTester tester, String id) =>
      tester.widget<Checkbox>(box(id)).value ?? false;

  /// Ticks the cards of [ids] after Select.
  Future<void> select(WidgetTester tester, List<String> ids) async {
    await tap(tester, key('selectNotes'));
    for (final id in ids) {
      await tap(tester, box(id));
    }
  }

  testWidgets('Select shows a box on every card; only the box ticks, a tap '
      'still switches the view and back drops the selection', (tester) async {
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    await start(tester);
    await openAllNotes(tester);
    expect(box(a.id), findsNothing);
    await tap(tester, key('selectNotes'));
    expect(box(a.id), findsOneWidget);
    expect(box(b.id), findsOneWidget);
    expect(find.text('0 selected'), findsOneWidget);
    // Nothing ticked: the selection's actions are greyed.
    final file = tester.widget<IconButton>(
      find.descendant(
        of: key('selection-file'),
        matching: find.byType(IconButton),
      ),
    );
    expect(file.onPressed, isNull);

    await tap(tester, box(a.id));
    expect(ticked(tester, a.id), isTrue);
    expect(find.text('1 selected'), findsOneWidget);
    final before = (tester.widget<NoteCard>(
      find.ancestor(of: box(b.id), matching: find.byType(NoteCard)),
    )).full;
    await tap(tester, find.text('bread'));
    expect(ticked(tester, b.id), isFalse);
    expect(
      tester
          .widget<NoteCard>(
            find.ancestor(of: box(b.id), matching: find.byType(NoteCard)),
          )
          .full,
      isNot(before),
    );
    // A long press still opens the editor.
    await tester.longPress(find.text('apples'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // Back ends selecting, the page stays.
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(box(a.id), findsNothing);
    expect(find.text('All Notes'), findsOneWidget);
    expect(key('selectNotes'), findsOneWidget);
  });

  testWidgets('swipe right drops the selection and stays; leaving the '
      'Areas list for the Agenda drops its selection too', (tester) async {
    final home = places.addArea('Household');
    final a = notes.capture('apples', place: home.id);
    await start(tester);
    await openAllNotes(tester);
    await select(tester, [a.id]);
    await tester.flingFrom(
      tester.getCenter(find.byType(MaterialApp)),
      const Offset(400, 0),
      1000,
    );
    await tester.pumpAndSettle();
    expect(box(a.id), findsNothing);
    expect(find.text('All Notes'), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // The Areas list beside the Agenda: its search's hits select.
    await tester.enterText(find.byType(TextField), 'apples');
    await tester.pumpAndSettle();
    await select(tester, [a.id]);
    expect(find.text('1 selected'), findsOneWidget);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.byTooltip('Search and browse'), findsOneWidget);
    await tap(tester, find.byTooltip('Search and browse'));
    expect(find.text('1 selected'), findsNothing);
    expect(box(a.id), findsNothing);
    expect(key('selectNotes'), findsOneWidget);
  });

  testWidgets('with Fat fingers on a phone the bars keep Select within '
      'reach', (tester) async {
    store.fatFingers = true;
    final home = places.addArea('Household');
    final a = notes.capture('apples', place: home.id);
    tester.view.physicalSize = const Size(1080, 2400);
    tester.view.devicePixelRatio = 2.625;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();

    /// Taps Select, out of ⋮ when it went there; either lies on the
    /// screen.
    Future<void> reachSelect() async {
      final inMore = key('selectNotes').evaluate().isEmpty;
      final reach = inMore ? key('bar-more') : key('selectNotes');
      expect(tester.getRect(reach).right, lessThanOrEqualTo(1080 / 2.625));
      if (inMore) await tap(tester, key('bar-more'));
      await tap(tester, key('selectNotes'));
    }

    await tap(tester, find.byTooltip('Search and browse'));
    await tester.enterText(find.byType(TextField), 'apples');
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    await reachSelect();
    expect(box(a.id), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.text('Household'));
    expect(tester.takeException(), isNull);
    await reachSelect();
    expect(box(a.id), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Unsorted files the selection into the place picked, in one '
      'go', (tester) async {
    final home = places.addArea('Household');
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    tick();
    final c = notes.capture('coffee');
    await start(tester);
    await tap(tester, key('unsortedBanner'));
    await select(tester, [a.id, c.id]);
    await tap(tester, key('selection-file'));
    expect(find.byType(FileIntoScreen), findsOneWidget);
    var heard = 0;
    void hear() => heard++;
    store.addListener(hear);
    addTearDown(() => store.removeListener(hear));
    await tap(tester, find.text('Household'));
    expect(heard, 1);
    expect(notes.byId(a.id)?.place, home.id);
    expect(notes.byId(c.id)?.place, home.id);
    expect(notes.byId(b.id)?.isUnsorted, isTrue);
    expect(store.recentlyFiledInto, [home.id]);
    // The selection ended; the list shows what is left.
    expect(box(b.id), findsNothing);
    expect(find.text('bread'), findsOneWidget);
    expect(find.text('apples'), findsNothing);
  });

  testWidgets('Move to trash takes the selection without asking; one undo '
      'at the top brings all of them back (#268)', (tester) async {
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    tick();
    final c = notes.capture('coffee');
    await start(tester);
    await openAllNotes(tester);
    await select(tester, [a.id, b.id]);
    await tap(tester, key('selection-trash'));
    expect(find.byType(AlertDialog), findsNothing);
    expect(notes.byId(a.id)?.isTrashed, isTrue);
    expect(notes.byId(b.id)?.isTrashed, isTrue);
    expect(notes.byId(c.id)?.isTrashed, isFalse);
    expect(find.text('Moved to the Trash'), findsOneWidget);
    await tap(tester, find.text('Moved to the Trash'));
    expect(notes.byId(a.id)?.isTrashed, isFalse);
    expect(notes.byId(b.id)?.isTrashed, isFalse);
    expect(find.text('apples'), findsOneWidget);
    expect(find.text('bread'), findsOneWidget);
  });

  testWidgets('Merge into… merges the selection into the Note picked, or '
      'into the ticked one picked', (tester) async {
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    tick();
    final c = notes.capture('coffee');
    tick();
    final d = notes.capture('dates');
    await start(tester);
    await openAllNotes(tester);
    await select(tester, [a.id, b.id]);
    await tap(tester, key('selection-merge'));
    expect(find.byType(MergeIntoScreen), findsOneWidget);
    await tap(
      tester,
      find.descendant(
        of: find.byType(MergeIntoScreen),
        matching: find.text('coffee'),
      ),
    );
    expect(notes.byId(a.id)?.mergedInto, c.id);
    expect(notes.byId(b.id)?.mergedInto, c.id);
    expect(notes.sections(c.id).map((s) => s.id), [a.id, b.id]);

    await select(tester, [c.id, d.id]);
    await tap(tester, key('selection-merge'));
    await tap(
      tester,
      find.descendant(
        of: find.byType(MergeIntoScreen),
        matching: find.text('dates'),
      ),
    );
    expect(notes.byId(c.id)?.mergedInto, d.id);
    expect(notes.byId(d.id)?.isMerged, isFalse);
    expect(notes.all().map((n) => n.id), [d.id]);
  });

  testWidgets('Importance opens its menu at the button and sets the '
      'selection\'s', (tester) async {
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    await start(tester);
    await openAllNotes(tester);
    await select(tester, [a.id, b.id]);
    await tap(tester, key('selection-importance'));
    await tap(tester, key('selection-importance-important'));
    expect(notes.byId(a.id)?.importance, Importance.important);
    expect(notes.byId(b.id)?.importance, Importance.important);
    expect(box(a.id), findsNothing);
  });

  testWidgets('the Trash restores the selection', (tester) async {
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    tick();
    final c = notes.capture('coffee');
    final routine = Routines(store).spawn(
      a.id,
      Schedule(start: Day(2026, 11, 10), times: [ClockTime(20, 0)]),
    );
    for (final id in [a.id, b.id, c.id]) {
      Routines(store).trashNote(id);
    }
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('Trash'));
    await select(tester, [a.id, c.id]);
    expect(key('selection-trash'), findsNothing);
    expect(key('selection-file'), findsNothing);
    await tap(tester, key('selection-restore'));
    expect(notes.byId(a.id)?.isTrashed, isFalse);
    expect(notes.byId(c.id)?.isTrashed, isFalse);
    expect(notes.byId(b.id)?.isTrashed, isTrue);
    expect(Routines(store).byId(routine.id)?.isTrashed, isFalse);
    expect(box(b.id), findsNothing);
  });

  testWidgets('an Area and a Topic select their cards, and a search too', (
    tester,
  ) async {
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    final a = notes.capture('apples', place: home.id);
    final b = notes.capture('bread', place: kitchen.id);
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.text('Household'));
    await tap(tester, key('selectNotes'));
    expect(box(a.id), findsOneWidget);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    await tap(tester, find.text('Kitchen'));
    await tap(tester, key('selectNotes'));
    expect(box(b.id), findsOneWidget);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    await tester.pageBack();
    await tester.pumpAndSettle();
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(key('selectNotes'), findsNothing);
    await tester.enterText(find.byType(TextField), 'bread');
    await tester.pumpAndSettle();
    await tap(tester, key('selectNotes'));
    await tap(tester, box(b.id));
    expect(find.text('1 selected'), findsOneWidget);
    // Clearing the search drops the selection with the hits.
    await tester.enterText(find.byType(TextField), '');
    await tester.pumpAndSettle();
    expect(find.text('1 selected'), findsNothing);
    expect(box(b.id), findsNothing);
  });

  testWidgets('a Member gets no box on another person\'s Note', (tester) async {
    store
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    final home = places.addArea('Household');
    final area = store.share(home.id);
    final anns = notes.capture('Ann\'s toner', place: area);
    final ben = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Pixel')
      ..renameOwnProfile('Ben');
    addTearDown(ben.close);
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
    expect(Notes(ben).byId(anns.id), isNotNull);

    await start(tester, ben);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.text('Household'));
    // Nothing here is Ben's to change: nothing to tick, no Select.
    expect(find.text('Ann\'s toner'), findsOneWidget);
    expect(key('selectNotes'), findsNothing);
    final bens = Notes(ben).capture('Ben\'s paper', place: area);
    await tester.pumpAndSettle();
    await tap(tester, key('selectNotes'));
    expect(box(bens.id), findsOneWidget);
    expect(box(anns.id), findsNothing);
    expect(find.text('Ann\'s toner'), findsOneWidget);
  });
}

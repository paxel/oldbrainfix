import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_test.dart' show FakeFolders;

/// A list of many Notes reads what its cards show once for the list
/// (obf-1069): a tap that only switches a card's view reads nothing
/// again, and the Note editor shows before it works out the Notes
/// linking to its Note.
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 11, 20, 8);

  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime.utc(2026, 11, 20, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  /// The app on All Notes, a tall screen so every card shows.
  Future<void> openAllNotes(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 4000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(store: store, folders: FakeFolders()),
    );
    await tester.pumpAndSettle();
    await tester.tap(key('browse'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
  }

  testWidgets('a tap that switches a card to compact and back reads '
      'nothing again', (tester) async {
    for (var i = 0; i < 8; i++) {
      final note = notes.capture('Note $i\nline two of $i');
      if (i.isEven) Checklists(store).make(note.id);
      if (i % 3 == 0) {
        Routines(store).spawn(
          note.id,
          Schedule(
            start: Day(2026, 11, 20),
            times: [ClockTime(18, 0)],
            rule: Recurrence(frequency: Frequency.daily),
          ),
        );
      }
    }
    await openAllNotes(tester);
    final card = find.textContaining('Note 6', findRichText: true).first;

    final before = store.historyReads;
    await tester.tap(card);
    await tester.pumpAndSettle();
    await tester.tap(card);
    await tester.pumpAndSettle();

    expect(store.historyReads, before);
  });

  test('a list places its Notes as each is placed on its own, in as many '
      'reads for many Notes as for a few', () {
    final places = Places(store);
    final home = places.addArea('Home').id;
    final desk = places.addTopic(home, 'Desk').id;
    void seed(int count) {
      for (var i = 0; i < count; i++) {
        notes.capture('Home $i', place: home);
        notes.capture('Desk $i', place: desk);
        notes.capture('Unsorted $i');
      }
    }

    String shown(PlacedNote p) => [
      p.note.id,
      p.area?.id,
      p.topic?.id,
      p.changes,
      p.schedules,
      p.amends,
      p.author?.id,
    ].join(' ');
    int reads() {
      final all = notes.all();
      final before = store.historyReads;
      placeNotes(places, all);
      return store.historyReads - before;
    }

    seed(2);
    final all = notes.all();
    expect(
      placeNotes(places, all).map(shown),
      all.map((note) => shown(placeNote(places, note))),
    );
    expect(placeNotes(places, all).where((p) => p.topic != null), hasLength(2));
    final few = reads();
    seed(20);
    expect(reads(), few);
  });

  testWidgets('the Note editor shows before the Notes linking to its Note '
      'are read, and lists them once it has shown', (tester) async {
    final ann = notes.capture("Ann's number");
    final call = notes.capture(
      'Call Ann\nabout [her number](${AppLink(AppLinkKind.note, ann.id).url})',
      format: TextFormats.markdown,
    );
    await openAllNotes(tester);
    // Compact, so no card lists them.
    await tester.tap(key('compactNotes'));
    await tester.pumpAndSettle();
    expect(key('linked-from:${call.id}'), findsNothing);

    await tester.longPress(find.text("Ann's number", findRichText: true));
    await tester.pump();
    final editor = find.byType(NoteEditorScreen);
    expect(editor, findsOneWidget);
    expect(key('linked-from:${call.id}'), findsNothing);

    await tester.pumpAndSettle();
    expect(
      find.descendant(of: editor, matching: key('linked-from:${call.id}')),
      findsOneWidget,
    );
  });
}

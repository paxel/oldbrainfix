import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_test.dart' show FakeFolders;

/// A Note shows the Notes that link to it (obf-1035): "Linked from" at
/// the end of its detailed card and of its editor, one row per Note by
/// its first line, a tap opening it; nothing on the compact card or on
/// a Note nothing links to.
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 11, 5, 8);

  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime.utc(2026, 11, 5, 8);
    store =
        Store.inMemory(clock: () => now = now.add(const Duration(seconds: 1)))
          ..starterSetOffered = true
          ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  String link(Note note) => AppLink(AppLinkKind.note, note.id).url;

  /// The app on All Notes, a tall screen so every card shows.
  Future<void> pumpAllNotes(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(store: store, folders: FakeFolders()),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
  }

  testWidgets('the detailed card lists the Notes linking to it, a tap opens '
      'one, whose editor lists its own', (tester) async {
    final ann = notes.capture("Ann's number");
    final call = notes.capture(
      'Call Ann\nabout [her number](${link(ann)})',
      format: TextFormats.markdown,
    );
    final plan = notes.capture(
      'Weekly plan\n**[the call](${link(call)})**',
      format: TextFormats.markdown,
    );
    await pumpAllNotes(tester);

    expect(find.text('Linked from'), findsNWidgets(2));
    expect(key('linked-from:${call.id}'), findsOneWidget);
    expect(key('linked-from:${plan.id}'), findsOneWidget);
    expect(
      find.descendant(
        of: key('linked-from:${call.id}'),
        matching: find.text('Call Ann'),
      ),
      findsOneWidget,
    );

    await tester.tap(key('linked-from:${call.id}'));
    await tester.pumpAndSettle();
    final editor = find.byType(NoteEditorScreen);
    expect(tester.widget<NoteEditorScreen>(editor).noteId, call.id);
    expect(
      find.descendant(of: editor, matching: key('linked-from:${plan.id}')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: editor, matching: find.text('Weekly plan')),
      findsOneWidget,
    );
  });

  testWidgets('a link from the trash shows none', (tester) async {
    final ann = notes.capture("Ann's number");
    final gone = notes.capture(
      'Old [link](${link(ann)})',
      format: TextFormats.markdown,
    );
    notes.trash(gone.id);
    await pumpAllNotes(tester);
    expect(find.text('Linked from'), findsNothing);
  });

  testWidgets('the compact card shows none', (tester) async {
    final ann = notes.capture("Ann's number");
    notes.capture('Call [Ann](${link(ann)})', format: TextFormats.markdown);
    await pumpAllNotes(tester);
    expect(find.text('Linked from'), findsOneWidget);
    await tester.tap(key('compactNotes'));
    await tester.pumpAndSettle();
    expect(find.text('Linked from'), findsNothing);
  });
}

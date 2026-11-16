import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/tally_field_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'open_capture.dart';

/// Capture's Kind choice (#310, Q715): Text, Checklist or Shopping List
/// above the field, the Note made as that Kind at once, each typed line a
/// Line unless the first is a heading (#309).
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 11, 15, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 15, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));
  final kinds = find.descendant(
    of: find.byType(CaptureScreen),
    matching: find.byWidgetPredicate((w) => w is ChoiceChips<NoteKind>),
  );
  final save = key('captureSave');

  Future<void> start(WidgetTester tester, {PhotoPlatform? photos}) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, photos: photos ?? const NoPhotos()),
    );
    await tester.pumpAndSettle();
    await openTextCapture(tester);
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> choose(WidgetTester tester, NoteKind kind) =>
      tap(tester, key('capture-kind-${kind.name}'));

  List<String> checklistLines(String id) => [
    for (final line in Checklists(store).byNote(id)!.lines) line.text,
  ];

  List<String> shoppingLines(String id) => [
    for (final line in ShoppingLists(store).byNote(id, deviceWallClock)!.open)
      line.text,
  ];

  testWidgets('offers Text, Checklist and Shopping List above the field, '
      'Text chosen', (tester) async {
    await start(tester);
    expect(kinds, findsOneWidget);
    final chips = tester.widget<ChoiceChips<NoteKind>>(kinds);
    expect(chips.value, NoteKind.plain);
    expect(
      [for (final option in chips.options) option.value],
      [NoteKind.plain, NoteKind.checklist, NoteKind.shoppingList],
    );
    for (final word in ['Text', 'Checklist', 'Shopping List']) {
      expect(
        find.descendant(of: kinds, matching: find.text(word)),
        findsOneWidget,
        reason: word,
      );
    }
    // Above the field.
    expect(
      tester.getBottomLeft(kinds).dy,
      lessThanOrEqualTo(tester.getTopLeft(find.byType(TextField)).dy),
    );
  });

  testWidgets('Checklist chosen, Save makes a Checklist with each line a '
      'Line and no Name', (tester) async {
    await start(tester);
    await tester.enterText(
      find.byType(TextField),
      'Passport\nTickets\nCharger',
    );
    await choose(tester, NoteKind.checklist);
    // Made a list it leaves Unsorted: one Save, no Nudge, no Add to Note….
    expect(find.text('10 min'), findsNothing);
    expect(key('captureAddToNote'), findsNothing);
    await tap(tester, save);

    final note = notes.all().single;
    expect(Checklists(store).isChecklist(note.id), isTrue);
    expect(checklistLines(note.id), ['Passport', 'Tickets', 'Charger']);
    expect(note.text, '');
    expect(note.isUnsorted, isFalse);
    expect(store.current(note.id, 'nudge'), isNull);
  });

  testWidgets('a "# heading" first line becomes the Name of the Shopping '
      'List', (tester) async {
    await start(tester);
    await choose(tester, NoteKind.shoppingList);
    await tester.enterText(find.byType(TextField), '# Weekend\nMilk\nEggs');
    await tap(tester, save);

    final note = notes.all().single;
    expect(ShoppingLists(store).isShoppingList(note.id), isTrue);
    expect(note.text, 'Weekend');
    expect(shoppingLines(note.id), ['Milk', 'Eggs']);
  });

  testWidgets('Text chosen again keeps it a Plain Note with its Nudge', (
    tester,
  ) async {
    await start(tester);
    await tester.enterText(find.byType(TextField), 'Milk\nEggs');
    await choose(tester, NoteKind.shoppingList);
    await choose(tester, NoteKind.plain);
    expect(find.text('10 min'), findsOneWidget);
    expect(key('captureAddToNote'), findsOneWidget);
    await tap(tester, find.text('10 min'));

    final note = notes.all().single;
    expect(note.text, 'Milk\nEggs');
    expect(ShoppingLists(store).isShoppingList(note.id), isFalse);
    expect(Checklists(store).isChecklist(note.id), isFalse);
    expect(note.isUnsorted, isTrue);
  });

  testWidgets('To full editor makes the Note that Kind and opens it', (
    tester,
  ) async {
    await start(tester);
    await choose(tester, NoteKind.checklist);
    await tester.enterText(find.byType(TextField), 'Oil\nTyres');
    await tap(tester, key('captureToEditor'));

    expect(find.byType(NoteEditorScreen), findsOneWidget);
    final note = notes.all().single;
    expect(checklistLines(note.id), ['Oil', 'Tyres']);
  });

  testWidgets('the app going to the background keeps the text as a Plain '
      'Note; leaving makes it the Kind with what was typed since', (
    tester,
  ) async {
    await start(tester);
    await choose(tester, NoteKind.checklist);
    await tester.enterText(find.byType(TextField), 'Oil');
    for (final state in [
      AppLifecycleState.inactive,
      AppLifecycleState.hidden,
      AppLifecycleState.paused,
      AppLifecycleState.hidden,
      AppLifecycleState.inactive,
      AppLifecycleState.resumed,
    ]) {
      tester.binding.handleAppLifecycleStateChanged(state);
    }
    await tester.pumpAndSettle();
    final written = notes.all().single;
    expect(written.text, 'Oil');
    expect(Checklists(store).isChecklist(written.id), isFalse);
    // Unsorted until it is a list, so it still nudges if the app ends.
    expect(store.current(written.id, 'nudge'), isNotNull);

    await tester.enterText(find.byType(TextField), 'Oil\nTyres');
    await tap(tester, save);
    expect(notes.all(), hasLength(1));
    expect(checklistLines(written.id), ['Oil', 'Tyres']);
  });

  testWidgets('with nothing typed a photo stays a Plain Note', (tester) async {
    await start(tester, photos: FakePhotos(nextPhoto: onePixelPng));
    await choose(tester, NoteKind.checklist);
    await tap(tester, key('capturePhoto'));
    await tap(tester, save);

    final note = notes.all().single;
    expect(note.photos, hasLength(1));
    expect(Checklists(store).isChecklist(note.id), isFalse);
  });

  group('beside the other choices', () {
    setUp(() {
      store.setQuickNoteShows(QuickNoteControl.schedule, true);
      store.setQuickNoteShows(QuickNoteControl.shopping, true);
    });

    testWidgets('a list Kind offers no Tally and no Session, and a Tally set '
        'up hides the Kind choice', (tester) async {
      await start(tester);
      await tester.enterText(find.byType(TextField), 'Push-ups');
      await choose(tester, NoteKind.checklist);
      await tap(tester, find.byTooltip('More'));
      expect(find.text('Make it a Tally'), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('Routine, Reminder, Frog or Session'));
      expect(find.text('Routine'), findsOneWidget);
      expect(find.text('Session'), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      // Text again: a Tally set up makes the Note a Tally, so the Kind
      // choice goes.
      await choose(tester, NoteKind.plain);
      await tap(tester, find.byTooltip('More'));
      await tap(tester, find.byKey(const ValueKey('note-make-tally')));
      final field = find.byType(TallyFieldScreen);
      await tester.enterText(
        find.descendant(of: field, matching: find.byType(TextField)).first,
        'Push-ups',
      );
      await tap(
        tester,
        find.descendant(
          of: field,
          matching: find.widgetWithText(TextButton, 'Save'),
        ),
      );
      expect(find.text('Tally: Push-ups'), findsOneWidget);
      expect(kinds, findsNothing);
    });

    testWidgets('Add to a Shopping List adds to the list picked: the Kind '
        'choice waits unshown and no new list is made', (tester) async {
      final weekly = notes.capture('Weekly').id;
      ShoppingLists(store).make(weekly);
      await start(tester);
      await choose(tester, NoteKind.shoppingList);
      await tester.enterText(find.byType(TextField), 'Bread');
      await tap(tester, find.byTooltip('More'));
      await tap(tester, find.text('Add to Weekly'));
      expect(kinds, findsNothing);
      await tap(tester, save);

      expect(notes.all().map((n) => n.id), [weekly]);
      expect(shoppingLines(weekly), ['Bread']);
    });
  });
}

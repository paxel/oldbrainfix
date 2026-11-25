import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix/src/shopping_pick_screen.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_speech.dart';
import 'fake_voice.dart';

/// A recording the fake speech platform transcribes by its last byte.
final recorded = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);

/// "Add to a Shopping List" in the Note editor (#124, #352), where it
/// went from the quick note: each line of the Note becomes a Line of the
/// list picked, and the Note goes.
void main() {
  late Store store;
  late Notes notes;
  late ShoppingLists lists;
  late FakeSpeech speech;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    // Each read a moment on, as on a Device: the Lines added one after
    // the other show in the order they came.
    store =
        Store.inMemory(
            clock: () => now = now.add(const Duration(milliseconds: 1)),
          )
          ..starterSetOffered = true
          ..alarmSetupDone = true;
    notes = Notes(store);
    lists = ShoppingLists(store);
    speech = FakeSpeech(installed: WhisperModel.base.sha256);
  });
  tearDown(() => store.close());

  /// A Shopping List made from a new Note with [text]; its Note's id.
  String shopping(String text) {
    final id = notes.capture(text).id;
    lists.make(id);
    return id;
  }

  /// The Lines of the Shopping List [id] as they show: `~text` for a
  /// struck one.
  List<String> shown(String id) => [
    for (final line in [...?lists.byNote(id, deviceWallClock)?.lines])
      '${line.struck ? '~' : ''}${line.text}',
  ];

  Future<void> start(WidgetTester tester) async {
    // Tall, so the editor's groups at the page's end show without
    // scrolling (obf-1073).
    tester.view
      ..physicalSize = const Size(1280, 1600)
      ..devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        voice: FakeVoice(),
        photos: FakePhotos(),
        speech: speech,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor of the Note [id] from the Agenda.
  Future<void> edit(WidgetTester tester, String id) async {
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, Places(store), notes.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  final action = find.byKey(const ValueKey('add-to-shopping'));

  /// The words of the editor's "Add to a Shopping List".
  String label(WidgetTester tester) => tester
      .widget<Text>(find.descendant(of: action, matching: find.byType(Text)))
      .data!;

  testWidgets('each line becomes a Line of the list picked, a name the list '
      'has comes back instead, and the Note goes with the editor', (
    tester,
  ) async {
    final groceries = shopping('Groceries\nMilk\nTea');
    final hardware = shopping('Hardware\nNails');
    final milk = lists.byNote(groceries, deviceWallClock)!.lines.first.id;
    lists.check(milk, deviceWallClock);
    final note = notes.capture('Eggs\nmilk\n\nTea\nbread');
    await start(tester);
    await edit(tester, note.id);
    expect(label(tester), 'Add to a Shopping List');
    await tap(tester, action);

    // Every list to add to, by its title.
    final picker = find.byType(ShoppingPickScreen);
    expect(
      find.descendant(of: picker, matching: find.text('Groceries')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: picker, matching: find.text('Hardware')),
      findsOneWidget,
    );
    await tap(tester, find.byKey(ValueKey('shopping-pick-$groceries')));

    expect(find.byType(NoteEditorScreen), findsNothing);
    expect(shown(groceries), ['Milk', 'Tea', 'Eggs', 'bread']);
    expect(shown(hardware), ['Nails']);
    expect(notes.all().map((n) => n.id).toSet(), {groceries, hardware});
    expect(notes.trashed(), isEmpty);
    expect(store.lastShoppingList, groceries);
  });

  testWidgets('with one list to add to it names it and adds at once (#276)', (
    tester,
  ) async {
    final groceries = shopping('Groceries');
    final note = notes.capture('Coffee');
    await start(tester);
    await edit(tester, note.id);
    expect(label(tester), 'Add to Groceries');
    await tap(tester, action);

    expect(find.byType(ShoppingPickScreen), findsNothing);
    expect(find.byType(NoteEditorScreen), findsNothing);
    expect(shown(groceries), ['Coffee']);
    expect(notes.byId(note.id), isNull);
  });

  testWidgets('back from the page without a pick changes nothing', (
    tester,
  ) async {
    shopping('Groceries');
    shopping('Hardware');
    final note = notes.capture('Coffee');
    await start(tester);
    await edit(tester, note.id);
    await tap(tester, action);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(notes.byId(note.id)?.text, 'Coffee');
  });

  testWidgets('not offered without a list, for a photo, for a list itself, '
      'nor with a Routine spawned from it', (tester) async {
    final alone = notes.capture('Milk');
    final photo = notes.capture('Milk', photos: [onePixelPng]);
    final spawned = notes.capture('water the plants');
    await start(tester);
    await edit(tester, alone.id);
    expect(action, findsNothing, reason: 'no Shopping List');
    await tester.pageBack();
    await tester.pumpAndSettle();

    final groceries = shopping('Groceries');
    Routines(store).spawn(
      spawned.id,
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(9, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    for (final (id, reason) in [
      (photo.id, 'a photo is on it'),
      (groceries, 'a list itself'),
      (spawned.id, 'a Routine hangs on it'),
    ]) {
      await edit(tester, id);
      expect(action, findsNothing, reason: reason);
      await tester.pageBack();
      await tester.pumpAndSettle();
    }
    await edit(tester, alone.id);
    expect(action, findsOneWidget);
  });

  testWidgets('a list made a plain Note meanwhile keeps the Note, announced '
      'at the top, whose tap says why and what to do', (tester) async {
    final groceries = shopping('Groceries');
    final note = notes.capture('Rice');
    await start(tester);
    await edit(tester, note.id);
    // Made a plain Note again meanwhile, on another Device say.
    lists.toPlain(groceries, deviceWallClock);
    await tap(tester, action);

    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(notes.byId(note.id)?.text, 'Rice');
    const announced =
        'Kept as a Note: that Shopping List can no longer be added to.';
    expect(find.text(announced), findsOneWidget);
    await tester.tap(find.text(announced));
    await tester.pumpAndSettle();
    expect(find.text('Not added to the Shopping List'), findsOneWidget);
  });

  testWidgets('a voice Note stays on the list as a Section, and the items '
      'said become Lines (#311)', (tester) async {
    speech.transcripts[1] = 'Butter, flour und Eier.';
    final groceries = shopping('Groceries\nFlour');
    final note = notes.capture('Sugar', recordings: [recorded]);
    await start(tester);
    await edit(tester, note.id);
    await tap(tester, action);

    expect(find.byType(NoteEditorScreen), findsNothing);
    // The typed line, then the items said; Flour once.
    expect(shown(groceries), ['Flour', 'Sugar', 'Butter', 'Eier']);
    final voice = notes.byId(note.id)!;
    expect(voice.mergedInto, groceries);
    expect(notes.sections(groceries).single.recordings, voice.recordings);
    expect(notes.all().map((n) => n.id), [groceries]);
    expect(notes.trashed(), isEmpty);
  });
}

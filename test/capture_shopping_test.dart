import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/shopping_pick_screen.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_speech.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

/// A recording the fake speech platform transcribes by its last byte.
final recorded = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);

/// Sorting a capture into a Shopping List (#124): each line becomes a
/// Line of the list, and no Note is kept.
void main() {
  late Store store;
  late Notes notes;
  late ShoppingLists lists;
  late FakeVoice voice;
  late FakePhotos photos;
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
    voice = FakeVoice(nextRecording: recorded);
    photos = FakePhotos(nextPhoto: onePixelPng);
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
    for (final line in lists.byNote(id, deviceWallClock)?.lines ?? const [])
      '${line.struck ? '~' : ''}${line.text}',
  ];

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        voice: voice,
        photos: photos,
        speech: speech,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  final row = find.descendant(
    of: find.byType(CaptureScreen),
    matching: find.byKey(const ValueKey('note-actions')),
  );
  final more = find.descendant(
    of: row,
    matching: find.byKey(const ValueKey('note-menu')),
  );
  final addToShopping = find.byKey(const ValueKey('note-add-to-shopping'));
  final save = find.byKey(const ValueKey('captureSave'));
  final nudge10 = find.byKey(const ValueKey('nudge10'));
  final chip = find.byKey(const ValueKey('choice-shopping'));

  /// Picks the Shopping List [id] with "Add to a Shopping List" in ⋮.
  Future<void> pick(WidgetTester tester, String id) async {
    await tap(tester, more);
    await tap(tester, addToShopping);
    expect(find.byType(ShoppingPickScreen), findsOneWidget);
    await tap(tester, find.byKey(ValueKey('shopping-pick-$id')));
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  /// Whether ⋮ offers "Add to a Shopping List"; closes it again.
  Future<bool> offered(WidgetTester tester) async {
    await tap(tester, more);
    final found = addToShopping.evaluate().isNotEmpty;
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
    return found;
  }

  testWidgets('each typed line becomes a Line, a name the list has comes '
      'back instead, and no Note is kept', (tester) async {
    final groceries = shopping('Groceries\nMilk\nTea');
    final hardware = shopping('Hardware\nNails');
    final milk = lists.byNote(groceries, deviceWallClock)!.lines.first.id;
    lists.check(milk, deviceWallClock);
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'Eggs\nmilk\n\nTea\nbread');
    await tap(tester, more);
    await tap(tester, addToShopping);

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

    expect(find.text('Shopping List: Groceries'), findsOneWidget);
    // A sorted capture needs no Nudge; nothing is written yet.
    expect(nudge10, findsNothing);
    expect(save, findsOneWidget);
    expect(shown(groceries), ['Tea', '~Milk']);
    await tap(tester, save);

    expect(find.byType(CaptureScreen), findsNothing);
    expect(shown(groceries), ['Eggs', 'Milk', 'bread', 'Tea']);
    expect(shown(hardware), ['Nails']);
    expect(store.entitiesOfType(ChecklistKeys.typeLine), hasLength(5));
    expect(notes.all().map((n) => n.id).toSet(), {groceries, hardware});
    expect(notes.trashed(), isEmpty);
  });

  testWidgets('the other choices wait while a list is picked, and come '
      'back when it is dropped', (tester) async {
    final household = Places(store).addArea('Household');
    final groceries = shopping('Groceries');
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'descale the kettle');
    await tap(tester, find.byTooltip('File into…'));
    await tap(
      tester,
      find.descendant(
        of: find.byType(FileIntoScreen),
        matching: find.text('Household'),
      ),
    );
    await pick(tester, groceries);

    expect(find.text('→ Household'), findsNothing);
    expect(chip, findsOneWidget);
    // Lines go into no Area and spawn nothing.
    expect(find.byTooltip('File into…'), findsNothing);
    expect(find.byIcon(Icons.alarm_add_outlined), findsNothing);

    await tap(
      tester,
      find.descendant(of: chip, matching: find.byTooltip('Remove')),
    );
    expect(chip, findsNothing);
    expect(find.text('→ Household'), findsOneWidget);
    await tap(tester, save);

    expect(shown(groceries), isEmpty);
    final note = notes.byId(notes.all().first.id);
    expect(note?.text, 'descale the kettle');
    expect(note?.place, household.id);
  });

  testWidgets('Back saves into the list as well', (tester) async {
    final groceries = shopping('Groceries');
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'Coffee');
    await pick(tester, groceries);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    expect(find.byType(CaptureScreen), findsNothing);
    expect(shown(groceries), ['Coffee']);
    expect(notes.all().map((n) => n.id), [groceries]);
  });

  testWidgets('not offered without a list, nor with a photo, which a photo '
      'taken after picking drops', (tester) async {
    await start(tester);
    await openTextCapture(tester);
    expect(await offered(tester), isFalse, reason: 'no Shopping List');
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    final groceries = shopping('Groceries');
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'Milk');
    await pick(tester, groceries);
    // A photo needs the Note: the list choice goes.
    await tap(tester, find.byKey(const ValueKey('capturePhoto')));
    expect(chip, findsNothing);
    expect(await offered(tester), isFalse, reason: 'a photo is on it');
    await tap(tester, find.byKey(const ValueKey('nudge10')));

    expect(shown(groceries), isEmpty);
    final note = notes.all().firstWhere((n) => n.id != groceries);
    expect(note.text, 'Milk');
    expect(note.photos, hasLength(1));
  });

  testWidgets('not offered behind the App Lock, which keeps the lists out '
      'of reach', (tester) async {
    store
      ..setPin('2468', iterations: 10)
      ..appLockOn = true;
    shopping('Groceries');
    await start(tester);
    await openTextCapture(tester);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(await offered(tester), isFalse);
  });

  testWidgets('not offered once a Routine was written with the Note', (
    tester,
  ) async {
    shopping('Groceries');
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'water the plants');
    await tap(tester, find.byIcon(Icons.alarm_add_outlined));
    await tap(tester, find.text('Routine').last);
    await tap(
      tester,
      find.descendant(
        of: find.byType(RoutineEditorScreen),
        matching: find.widgetWithText(TextButton, 'Save'),
      ),
    );
    // The app goes to the background: the Note and its Routine are
    // written.
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(await offered(tester), isFalse);
  });

  testWidgets('a Note written meanwhile, as the app went to the '
      'background, goes when the lines go into the list', (tester) async {
    final groceries = shopping('Groceries');
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'Rice');
    await pick(tester, groceries);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    // Kept in case the app ends now; Unsorted, so nothing is lost.
    final waiting = notes.all().firstWhere((n) => n.id != groceries);
    expect(waiting.text, 'Rice');
    expect(waiting.isUnsorted, isTrue);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Rice\nBeans');
    await tap(tester, save);

    expect(shown(groceries), ['Rice', 'Beans']);
    expect(notes.byId(waiting.id), isNull);
    expect(notes.trashed(), isEmpty);
  });

  group('A voice capture', () {
    Future<void> record(WidgetTester tester) async {
      await tap(tester, find.byKey(const ValueKey('captureVoice')));
      await tap(tester, find.byKey(const ValueKey('stopRecording')));
    }

    testWidgets('puts the lines of its Transcript into the list', (
      tester,
    ) async {
      speech.transcripts[1] = 'Butter\nflour';
      final groceries = shopping('Groceries\nFlour');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'Sugar');
      await record(tester);
      // Transcribed meanwhile.
      expect(
        notes.all().firstWhere((n) => n.id != groceries).transcripts.values,
        ['Butter\nflour'],
      );
      await pick(tester, groceries);
      await tap(tester, save);

      expect(shown(groceries), ['Sugar', 'Butter', 'Flour']);
      expect(notes.all().map((n) => n.id), [groceries]);
      expect(notes.trashed(), isEmpty);
    });

    testWidgets('whose Transcript is not ready yet leaves at once and goes '
        'into the list once it is', (tester) async {
      speech
        ..transcripts[1] = 'Butter'
        ..transcribeGate = Completer<void>();
      final groceries = shopping('Groceries');
      await start(tester);
      await openTextCapture(tester);
      await pick(tester, groceries);
      await record(tester);
      await tap(tester, save);

      // Back on the Agenda; the Note holds it all while Whisper works.
      expect(find.byType(CaptureScreen), findsNothing);
      expect(shown(groceries), isEmpty);
      final waiting = notes.all().firstWhere((n) => n.id != groceries);
      expect(waiting.recordings, hasLength(1));

      speech.transcribeGate?.complete();
      await tester.pumpAndSettle();
      expect(shown(groceries), ['Butter']);
      expect(notes.byId(waiting.id), isNull);
      expect(notes.trashed(), isEmpty);
    });

    testWidgets('whose recording cannot be transcribed stays a Note, '
        'announced at the top', (tester) async {
      // Nothing is heard in the recording: it cannot be transcribed.
      final groceries = shopping('Groceries');
      speech.transcribeGate = Completer<void>();
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'Sugar');
      await record(tester);
      await pick(tester, groceries);
      await tap(tester, save);
      speech.transcribeGate?.complete();
      await tester.pumpAndSettle();

      expect(shown(groceries), isEmpty);
      final kept = notes.all().firstWhere((n) => n.id != groceries);
      expect(kept.text, 'Sugar');
      expect(kept.recordings, hasLength(1));
      expect(
        find.text(
          'Kept as a Note: its recording could not be transcribed for the '
          'Shopping List.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('is not offered a list without a speech model', (tester) async {
      speech.installed = null;
      shopping('Groceries');
      await start(tester);
      await openTextCapture(tester);
      await record(tester);
      expect(await offered(tester), isFalse);
    });
  });
}

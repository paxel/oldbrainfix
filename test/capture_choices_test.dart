import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_requests.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/tally_field_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_capture_requests.dart';
import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

final recorded = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70]);

/// Finishing a Note while capturing it (#113): the card's action row on
/// every capture screen, its choices shown and written with the Note.
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  late FakeCaptureRequests requests;
  late FakeVoice voice;
  late FakePhotos photos;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
    requests = FakeCaptureRequests();
    voice = FakeVoice(nextRecording: recorded);
    photos = FakePhotos(nextPhoto: onePixelPng);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        requests: requests,
        voice: voice,
        photos: photos,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Leaves Capture by the system's Back.
  Future<void> back(WidgetTester tester) async {
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
  }

  final row = find.descendant(
    of: find.byType(CaptureScreen),
    matching: find.byKey(const ValueKey('note-actions')),
  );
  final fileInto = find.descendant(
    of: row,
    matching: find.byTooltip('File into…'),
  );
  final spawnMenu = find.descendant(
    of: row,
    matching: find.byIcon(Icons.alarm_add_outlined),
  );
  final more = find.descendant(
    of: row,
    matching: find.byKey(const ValueKey('note-menu')),
  );
  final save = find.byKey(const ValueKey('captureSave'));
  final nudge10 = find.byKey(const ValueKey('nudge10'));

  /// Files the Note being captured into [name] with "File into…".
  Future<void> fileNoteInto(WidgetTester tester, String name) async {
    await tap(tester, fileInto);
    expect(find.byType(FileIntoScreen), findsOneWidget);
    await tap(
      tester,
      find.descendant(
        of: find.byType(FileIntoScreen),
        matching: find.text(name),
      ),
    );
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  /// Sets up a Routine as its editor opens, with what it proposes.
  Future<void> addRoutine(
    WidgetTester tester, {
    String kind = 'Routine',
  }) async {
    await tap(tester, spawnMenu);
    await tap(tester, find.text(kind).last);
    expect(find.byType(RoutineEditorScreen), findsOneWidget);
    await tap(
      tester,
      find.descendant(
        of: find.byType(RoutineEditorScreen),
        matching: find.widgetWithText(TextButton, 'Save'),
      ),
    );
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  group('The text capture screen', () {
    testWidgets('shows the card\'s action row above the Nudge buttons', (
      tester,
    ) async {
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);

      expect(row, findsOneWidget);
      expect(fileInto, findsOneWidget);
      expect(spawnMenu, findsOneWidget);
      expect(more, findsOneWidget);
      expect(
        tester.getRect(row).bottom,
        lessThanOrEqualTo(tester.getRect(nudge10).top),
      );
      // Merging needs a written Note: it waits for the card.
      expect(
        find.descendant(of: row, matching: find.byIcon(Icons.call_merge)),
        findsNothing,
      );
    });

    testWidgets('files the Note where it was put, shown and written only on '
        'Save, with one Save in place of the Nudges and no Nudge', (
      tester,
    ) async {
      final household = places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'descale the kettle');
      await fileNoteInto(tester, 'Household');

      expect(find.text('→ Household'), findsOneWidget);
      // Still typing: nothing is written yet.
      expect(notes.all(), isEmpty);
      expect(nudge10, findsNothing);
      expect(save, findsOneWidget);
      expect(
        tester.getRect(row).bottom,
        lessThanOrEqualTo(tester.getRect(save).top),
      );

      await tester.enterText(find.byType(TextField), 'descale the kettle now');
      await tap(tester, save);

      expect(find.byType(CaptureScreen), findsNothing);
      final note = notes.all().single;
      expect(note.text, 'descale the kettle now');
      expect(note.place, household.id);
      expect(note.nudge, isNull);
      expect(notes.unsorted(), isEmpty);
    });

    testWidgets('a Topic shows with its Area', (tester) async {
      final household = places.addArea('Household');
      final kitchen = places.addTopic(household.id, 'Kitchen');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'descale the kettle');
      await fileNoteInto(tester, 'Kitchen');

      expect(find.text('→ Household / Kitchen'), findsOneWidget);
      await back(tester);
      expect(notes.all().single.place, kitchen.id);
    });

    testWidgets('a Routine set up comes back as a choice, nothing written '
        'until leaving, then both together', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'water the plants');
      await tap(tester, spawnMenu);
      await tap(tester, find.text('Routine').last);

      // The editor shows the Note being captured.
      expect(
        find.descendant(
          of: find.byType(RoutineEditorScreen),
          matching: find.text('water the plants'),
        ),
        findsOneWidget,
      );
      await tap(
        tester,
        find.descendant(
          of: find.byType(RoutineEditorScreen),
          matching: find.widgetWithText(TextButton, 'Save'),
        ),
      );

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(find.textContaining('Routine: '), findsOneWidget);
      expect(notes.all(), isEmpty);
      expect(Routines(store).all(), isEmpty);
      // Spawned from, it leaves Unsorted: no Nudge.
      expect(save, findsOneWidget);
      await tester.enterText(find.byType(TextField), 'water the plants, all');

      await back(tester);
      final note = notes.all().single;
      expect(note.text, 'water the plants, all');
      expect(note.nudge, isNull);
      final routine = Routines(store).all().single;
      expect(Routines(store).note(routine)?.id, note.id);
      expect(routine.isReminder, isFalse);
    });

    testWidgets('a Reminder set up is written as one', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'call the vet');
      await addRoutine(tester, kind: 'Reminder');

      expect(find.textContaining('Reminder: '), findsOneWidget);
      await tap(tester, save);
      expect(Routines(store).all().single.isReminder, isTrue);
    });

    testWidgets('nothing is written for an empty Note, not even what was set '
        'up for it', (tester) async {
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);
      await addRoutine(tester);
      await fileNoteInto(tester, 'Household');
      await tap(tester, spawnMenu);
      await tap(tester, find.text('Frog').last);

      await back(tester);
      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all(), isEmpty);
      expect(Routines(store).all(), isEmpty);
      expect(Frogs(store).open(), isEmpty);
    });

    testWidgets('a choice dropped again is not written', (tester) async {
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'buy stamps');
      await addRoutine(tester);
      await fileNoteInto(tester, 'Household');

      for (final key in ['choice-schedule-0', 'choice-place']) {
        await tap(
          tester,
          find.descendant(
            of: find.byKey(ValueKey(key)),
            matching: find.byTooltip('Remove'),
          ),
        );
      }
      expect(find.textContaining('Routine: '), findsNothing);
      expect(find.text('→ Household'), findsNothing);
      // Unsorted again: the Nudges are back.
      expect(nudge10, findsOneWidget);

      await tap(tester, nudge10);
      final note = notes.all().single;
      expect(note.place, isNull);
      expect(note.nudge, now.add(const Duration(minutes: 10)));
      expect(Routines(store).all(), isEmpty);
    });

    testWidgets('a Frog, the Importance and a Tally are written with the '
        'Note', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'push-ups');
      await tap(tester, spawnMenu);
      await tap(tester, find.text('Frog').last);
      expect(find.text('Frog'), findsOneWidget);

      await tap(tester, more);
      await tap(tester, find.byKey(const ValueKey('importance-important')));
      expect(find.text('Important'), findsOneWidget);

      await tap(tester, more);
      await tap(tester, find.byKey(const ValueKey('note-make-tally')));
      expect(find.byType(TallyFieldScreen), findsOneWidget);
      await tester.enterText(
        find
            .descendant(
              of: find.byType(TallyFieldScreen),
              matching: find.byType(TextField),
            )
            .first,
        'Push-ups',
      );
      await tap(
        tester,
        find.descendant(
          of: find.byType(TallyFieldScreen),
          matching: find.widgetWithText(TextButton, 'Save'),
        ),
      );
      expect(find.text('Tally: Push-ups'), findsOneWidget);
      expect(notes.all(), isEmpty);
      // A Tally takes no second first field.
      await tap(tester, more);
      expect(find.byKey(const ValueKey('note-make-tally')), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tap(tester, save);
      final note = notes.all().single;
      expect(note.importance, Importance.important);
      expect(Frogs(store).ofNote(note.id), hasLength(1));
      expect(Tallies(store).byNote(note.id)?.fields.single.name, 'Push-ups');
    });

    testWidgets('a Session set up picks from the Tallies and is written with '
        'the Note', (tester) async {
      final pushUps = notes.capture('Push-ups');
      Tallies(store).addField(pushUps.id, name: 'Push-ups');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'Sport');
      await tap(tester, spawnMenu);
      await tap(tester, find.text('Session').last);
      await tap(tester, find.byKey(ValueKey('session-pool-${pushUps.id}')));
      await tap(
        tester,
        find.descendant(
          of: find.byType(RoutineEditorScreen),
          matching: find.widgetWithText(TextButton, 'Save'),
        ),
      );

      expect(find.textContaining('Session: '), findsOneWidget);
      expect(Sessions(store).all(), isEmpty);
      await back(tester);
      final session = Sessions(store).all().single;
      expect(session.pool, [pushUps.id]);
    });

    /// Sets up a Note filed into Household, Important, with a Routine and
    /// then a Session whose Tally goes meanwhile, as from another Device:
    /// the Session, written after the Note, its place and the Routine,
    /// fails.
    Future<void> setUpFailingSave(WidgetTester tester) async {
      places.addArea('Household');
      final pushUps = notes.capture('Push-ups');
      Tallies(store).addField(pushUps.id, name: 'Push-ups');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'Sport');
      await fileNoteInto(tester, 'Household');
      await tap(tester, more);
      await tap(tester, find.byKey(const ValueKey('importance-important')));
      await addRoutine(tester);
      await tap(tester, spawnMenu);
      await tap(tester, find.text('Session').last);
      await tap(tester, find.byKey(ValueKey('session-pool-${pushUps.id}')));
      await tap(
        tester,
        find.descendant(
          of: find.byType(RoutineEditorScreen),
          matching: find.widgetWithText(TextButton, 'Save'),
        ),
      );
      notes.trash(pushUps.id);
    }

    /// Checks that nothing of the failed save is written and the screen
    /// stays; drops the Session and leaves by Save, which writes the rest
    /// once.
    Future<void> expectNothingThenTheRest(WidgetTester tester) async {
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(notes.all(), isEmpty);
      expect(Routines(store).all(), isEmpty);
      expect(Sessions(store).all(), isEmpty);

      await tap(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('choice-schedule-1')),
          matching: find.byTooltip('Remove'),
        ),
      );
      await tap(tester, save);
      expect(find.byType(CaptureScreen), findsNothing);
      final note = notes.all().single;
      expect(note.text, 'Sport');
      expect(note.importance, Importance.important);
      expect(places.areaById(note.place ?? '')?.name, 'Household');
      expect(Routines(store).all(), hasLength(1));
      expect(Sessions(store).all(), isEmpty);
    }

    testWidgets('a write failing on the way to the background leaves nothing '
        'of the save; the screen keeps all of it', (tester) async {
      await setUpFailingSave(tester);

      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      expect(tester.takeException(), isArgumentError);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pumpAndSettle();
      await expectNothingThenTheRest(tester);
    });

    testWidgets('a write failing on Save leaves nothing of the save; the '
        'screen stays, to change and leave again', (tester) async {
      await setUpFailingSave(tester);

      Object? failed;
      await runZonedGuarded(
        () => tap(tester, save),
        (error, _) => failed = error,
      );
      expect(failed, isArgumentError);
      await expectNothingThenTheRest(tester);
    });

    testWidgets('the app going to the background writes the Note and its '
        'choices once; leaving later adds nothing twice', (tester) async {
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'renew the passport');
      await addRoutine(tester);
      await fileNoteInto(tester, 'Household');

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

      final note = notes.all().single;
      expect(note.place, isNotNull);
      expect(Routines(store).ofNote(note.id), hasLength(1));
      // Written, they show without a button to drop them.
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('choice-written-0')),
          matching: find.byTooltip('Remove'),
        ),
        findsNothing,
      );

      await addRoutine(tester, kind: 'Reminder');
      await tap(tester, save);
      expect(notes.all(), hasLength(1));
      expect(Routines(store).ofNote(note.id), hasLength(2));
    });
  });

  group('Every capture path shows the row', () {
    testWidgets('Voice: Stop comes back to the screen; leaving saves the '
        'recording where it was put', (tester) async {
      final household = places.addArea('Household');
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      await tap(tester, fanEntry('voice'));
      expect(voice.recording, isTrue);
      // Recording, the stop button comes first.
      expect(row, findsNothing);

      await tap(tester, find.byKey(const ValueKey('stopRecording')));
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(row, findsOneWidget);
      expect(find.byType(TextField), findsOneWidget);
      await fileNoteInto(tester, 'Household');

      await back(tester);
      final note = notes.all().single;
      expect(note.recordings, hasLength(1));
      expect(note.place, household.id);
    });

    testWidgets('Photo: back from the camera, the row is there; leaving '
        'saves', (tester) async {
      final household = places.addArea('Household');
      requests.waiting = CaptureMode.photo;
      await start(tester);

      expect(photos.taken, 1);
      expect(row, findsOneWidget);
      await fileNoteInto(tester, 'Household');
      await back(tester);

      final note = notes.all().single;
      expect(note.photos, hasLength(1));
      expect(note.place, household.id);
    });

    testWidgets('Gallery: back from the picker, the row is there', (
      tester,
    ) async {
      photos.nextPicked = [onePixelPng];
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      await tap(tester, fanEntry('gallery'));

      expect(row, findsOneWidget);
      await tap(tester, spawnMenu);
      await tap(tester, find.text('Frog').last);
      await back(tester);

      final note = notes.all().single;
      expect(note.photos, hasLength(1));
      expect(Frogs(store).ofNote(note.id), hasLength(1));
    });

    testWidgets('a share: the row is there, its choices go with it', (
      tester,
    ) async {
      await start(tester);
      photos.share(const SharedItem(text: 'https://example.org/recipe'));
      await tester.pumpAndSettle();

      expect(row, findsOneWidget);
      await addRoutine(tester);
      await back(tester);

      final note = notes.all().single;
      expect(note.text, 'https://example.org/recipe');
      expect(Routines(store).ofNote(note.id), hasLength(1));
    });

    testWidgets('a widget, tile, shortcut or button asking for text: the '
        'row is there', (tester) async {
      final household = places.addArea('Household');
      requests.waiting = CaptureMode.text;
      await start(tester);

      expect(row, findsOneWidget);
      await tester.enterText(find.byType(TextField), 'water plants');
      await fileNoteInto(tester, 'Household');
      await tap(tester, save);
      expect(notes.all().single.place, household.id);
    });

    testWidgets('one asking for voice: Stop comes back to the row', (
      tester,
    ) async {
      requests.waiting = CaptureMode.voice;
      await start(tester);
      await tap(tester, find.byKey(const ValueKey('stopRecording')));

      expect(row, findsOneWidget);
      await back(tester);
      expect(notes.all().single.recordings, hasLength(1));
    });
  });

  group('Behind the App Lock', () {
    testWidgets('locking again meanwhile closes "File into…" over Capture; '
        'what was set up stays', (tester) async {
      places.addArea('Household');
      await start(tester);
      await tester.enterText(find.byKey(const ValueKey('lockPin')), '2468');
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      await openTextCapture(tester);
      await addRoutine(tester);
      await tap(tester, fileInto);
      expect(find.byType(FileIntoScreen), findsOneWidget);

      final binding = tester.binding;
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
      ]) {
        binding.handleAppLifecycleStateChanged(state);
      }
      now = now.add(const Duration(minutes: 5));
      for (final state in [
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();

      expect(find.byType(FileIntoScreen, skipOffstage: false), findsNothing);
      expect(find.text('Household'), findsNothing);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(fileInto, findsNothing);
      expect(find.textContaining('Routine: '), findsOneWidget);

      await tester.enterText(find.byType(TextField), 'renew the passport');
      await back(tester);
      expect(Routines(store).all(), hasLength(1));
    });

    setUp(() {
      store
        ..setPin('2468', iterations: 10)
        ..appLockOn = true;
    });

    testWidgets('capture keeps the row but offers neither "File into…" nor '
        'the Session, which show Areas and Tallies', (tester) async {
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);

      expect(row, findsOneWidget);
      expect(fileInto, findsNothing);
      await tap(tester, spawnMenu);
      expect(find.text('Routine'), findsWidgets);
      expect(find.text('Session'), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tester.enterText(find.byType(TextField), 'buy stamps');
      await addRoutine(tester, kind: 'Reminder');
      await back(tester);
      expect(Routines(store).all().single.isReminder, isTrue);
    });
  });
}

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';
import 'open_settings.dart';

/// The quick note (#352): the text at the top, under it the three Nudge
/// buttons and More, nothing else; Back saves as the 1 h button does, and
/// More saves and opens the full editor in its place.
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        voice: FakeVoice(nextRecording: Uint8List.fromList(const [1, 2, 3])),
        photos: FakePhotos(nextPhoto: onePixelPng, nextPicked: [onePixelPng]),
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

  Finder inCapture(Finder finder) =>
      find.descendant(of: find.byType(CaptureScreen), matching: finder);
  final more = find.byKey(const ValueKey('captureToEditor'));
  final save = find.byKey(const ValueKey('captureSave'));
  final nudge10 = find.byKey(const ValueKey('nudge10'));

  /// Every button of the quick note.
  List<ButtonStyleButton> buttons(WidgetTester tester) => tester
      .widgetList<ButtonStyleButton>(
        inCapture(find.bySubtype<ButtonStyleButton>()),
      )
      .toList();

  for (final simple in [false, true]) {
    testWidgets('shows only the text, the three Nudge buttons and More, '
        '${simple ? 'in Simple' : 'in Advanced'} alike', (tester) async {
      store.advanced = !simple;
      // What the quick note's switches showed before stays stored and
      // shows no more.
      for (final control in QuickNoteControl.values) {
        store.setQuickNoteShows(control, true);
      }
      ShoppingLists(store).make(notes.capture('Groceries').id);
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);

      expect(inCapture(find.byType(TextField)), findsOneWidget);
      expect(
        buttons(tester).map((button) => button.key),
        unorderedEquals(const [
          ValueKey('nudge10'),
          ValueKey('nudge30'),
          ValueKey('nudge60'),
          ValueKey('captureToEditor'),
        ]),
      );
      expect(inCapture(find.byType(IconButton)), findsNothing);
      expect(inCapture(find.byType(InputChip)), findsNothing);
      expect(inCapture(find.byType(ChoiceChip)), findsNothing);
      expect(
        inCapture(find.byKey(const ValueKey('note-actions'))),
        findsNothing,
      );
      expect(find.text('More'), findsOneWidget);
      // The text first, at the top; the buttons under it, More last.
      final field = tester.getRect(inCapture(find.byType(TextField)));
      expect(field.top, lessThan(tester.getRect(nudge10).top));
      expect(
        tester.getRect(nudge10).bottom,
        lessThanOrEqualTo(tester.getRect(more).top),
      );
    });
  }

  testWidgets('on Android the bar holds no Save and no back arrow', (
    tester,
  ) async {
    await start(tester);
    await openTextCapture(tester);
    expect(inCapture(find.byType(BackButton)), findsNothing);
    expect(inCapture(find.widgetWithText(TextButton, 'Save')), findsNothing);
  });

  testWidgets('Back saves the Note Unsorted, nudged in 1 h', (tester) async {
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'ring the plumber');
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    expect(find.byType(CaptureScreen), findsNothing);
    final note = notes.all().single;
    expect(note.text, 'ring the plumber');
    expect(note.isUnsorted, isTrue);
    expect(note.nudge, now.add(Notes.defaultNudge));
  });

  testWidgets('an empty text saves nothing on Back', (tester) async {
    await start(tester);
    await openTextCapture(tester);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    expect(find.byType(CaptureScreen), findsNothing);
    expect(notes.all(), isEmpty);
  });

  testWidgets('a Nudge button saves with its time', (tester) async {
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'water the plants');
    await tap(tester, nudge10);

    expect(find.byType(CaptureScreen), findsNothing);
    expect(notes.all().single.nudge, now.add(const Duration(minutes: 10)));
  });

  testWidgets('the iPhone keeps a back arrow top left that saves as Back '
      'does', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
    try {
      await start(tester);
      await openTextCapture(tester);
      final back = inCapture(find.byType(BackButton));
      expect(back, findsOneWidget);
      expect(inCapture(find.widgetWithText(TextButton, 'Save')), findsNothing);
      expect(
        tester.getRect(back).left,
        lessThan(tester.getSize(find.byType(CaptureScreen)).width / 2),
      );

      await tester.enterText(find.byType(TextField), 'descale the kettle');
      await tap(tester, back);
      expect(find.byType(CaptureScreen), findsNothing);
      final note = notes.all().single;
      expect(note.text, 'descale the kettle');
      expect(note.nudge, now.add(Notes.defaultNudge));
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  group('More', () {
    testWidgets('waits for something to save', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      expect(tester.widget<TextButton>(more).onPressed, isNull);

      await tester.enterText(find.byType(TextField), 'ring the plumber');
      await tester.pump();
      expect(tester.widget<TextButton>(more).onPressed, isNotNull);
    });

    testWidgets('saves the Note Unsorted with the default Nudge and opens '
        'the editor on it; filing there drops the Nudge, and leaving comes '
        'back to the Agenda', (tester) async {
      final household = places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'descale the kettle');
      await tap(tester, more);

      expect(find.byType(CaptureScreen), findsNothing);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      final note = notes.all().single;
      expect(note.text, 'descale the kettle');
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
      expect(notes.unsorted().map((n) => n.id), [note.id]);

      await tap(tester, find.byKey(const ValueKey('badge-place')));
      await tap(
        tester,
        find.descendant(
          of: find.byType(FileIntoScreen),
          matching: find.text('Household'),
        ),
      );
      expect(notes.byId(note.id)!.place, household.id);
      expect(notes.unsorted(), isEmpty);

      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.byType(CaptureScreen), findsNothing);
      expect(find.byIcon(Icons.add), findsOneWidget);
      expect(notes.all(), hasLength(1));
    });

    testWidgets('is not offered behind the App Lock', (tester) async {
      store
        ..setPin('2468', iterations: 10)
        ..appLockOn = true;
      await start(tester);
      await openTextCapture(tester);

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(more, findsNothing);
      expect(nudge10, findsOneWidget);
    });
  });

  group('Every way in that brings something', () {
    testWidgets('a photo shows above the text, and More takes it on', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      await tap(tester, fanEntry('photo'));

      expect(find.byType(CaptureScreen), findsOneWidget);
      final note = notes.all().single;
      expect(note.photos, hasLength(1));
      expect(tester.widget<TextButton>(more).onPressed, isNotNull);
      await tap(tester, more);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(notes.all().single.id, note.id);
    });

    testWidgets('pictures picked show above the text', (tester) async {
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      await tap(tester, fanEntry('gallery'));

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(notes.all().single.photos, hasLength(1));
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(notes.all().single.nudge, now.add(Notes.defaultNudge));
    });

    testWidgets('voice records at once with the big stop button; Stop comes '
        'back to the text with the recording, and Back saves', (tester) async {
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      await tap(tester, fanEntry('voice'));

      expect(find.byKey(const ValueKey('stopRecording')), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('stopRecording')));
      expect(find.byKey(const ValueKey('stopRecording')), findsNothing);
      expect(inCapture(find.byType(TextField)), findsOneWidget);
      expect(notes.all().single.recordings, hasLength(1));

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all().single.recordings, hasLength(1));
    });
  });

  group('Settings ▸ Quick note', () {
    testWidgets('shows only the Nudge buttons switch, the others of before '
        'hidden with their stored values untouched', (tester) async {
      store.setQuickNoteShows(QuickNoteControl.fileInto, true);
      await start(tester);
      await openDeviceSettings(tester);

      for (final control in QuickNoteControl.values) {
        expect(
          find.byKey(ValueKey('quickNote-${control.name}')),
          findsNothing,
          reason: control.name,
        );
      }
      expect(store.quickNoteShows(QuickNoteControl.fileInto), isTrue);
      final nudges = find.byKey(const ValueKey('quickNote-nudgeButtons'));
      expect(tester.widget<Switch>(nudges).value, isTrue);
      await tap(tester, nudges);
      expect(store.quickNoteNudgeButtonsOn, isFalse);
      await tap(tester, nudges);
      expect(store.quickNoteNudgeButtonsOn, isTrue);
      expect(store.allEntries(), hasLength(0), reason: 'never synced');
    });

    testWidgets('the Nudge buttons off give a single Save, nudged in 1 h', (
      tester,
    ) async {
      store.quickNoteNudgeButtonsOn = false;
      await start(tester);
      await openTextCapture(tester);
      expect(nudge10, findsNothing);
      expect(save, findsOneWidget);
      expect(more, findsOneWidget);

      await tester.enterText(find.byType(TextField), 'water the plants');
      await tap(tester, save);
      final note = notes.all().single;
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
    });
  });

  testWidgets('fits a small German phone, the keyboard up', (tester) async {
    const height = 640.0;
    const keyboard = 300.0;
    final ratio = tester.view.devicePixelRatio;
    tester.view.physicalSize = Size(360 * ratio, height * ratio);
    addTearDown(tester.view.reset);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    await start(tester);
    await openTextCapture(tester);
    await tester.showKeyboard(find.byType(TextField));
    tester.view.viewInsets = FakeViewPadding(bottom: keyboard * ratio);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Mehr'), findsOneWidget);
    expect(tester.getRect(more).bottom, lessThanOrEqualTo(height - keyboard));
    await tester.tap(nudge10);
    await tester.pumpAndSettle();
    expect(find.byType(CaptureScreen), findsNothing);
  });
}

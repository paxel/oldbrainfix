import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';
import 'open_settings.dart';

/// The quick note uncluttered (#142): the text, Photo, Voice, Gallery and
/// Paste, the Nudge buttons and "To full editor" by default, the rest
/// switched on per Device in Settings ▸ Quick note.
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
        voice: FakeVoice(),
        photos: FakePhotos(nextPhoto: onePixelPng),
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
  final row = inCapture(find.byKey(const ValueKey('note-actions')));
  final fileInto = inCapture(find.byTooltip('File into…'));
  final spawnMenu = inCapture(find.byIcon(Icons.alarm_add_outlined));
  final more = inCapture(find.byKey(const ValueKey('note-menu')));
  final toEditor = find.byKey(const ValueKey('captureToEditor'));
  final save = find.byKey(const ValueKey('captureSave'));
  final nudge10 = find.byKey(const ValueKey('nudge10'));

  group('The quick note by default', () {
    testWidgets('shows the text, Photo, Voice, Gallery, Paste, the Nudge '
        'buttons and "To full editor", and nothing that sorts', (tester) async {
      places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);

      expect(inCapture(find.byType(TextField)), findsOneWidget);
      for (final key in [
        'capturePhoto',
        'captureVoice',
        'captureGallery',
        'capturePaste',
        'nudge10',
        'nudge30',
        'nudge60',
        'captureToEditor',
      ]) {
        expect(find.byKey(ValueKey(key)), findsOneWidget, reason: key);
      }
      expect(row, findsNothing);
      expect(fileInto, findsNothing);
      expect(spawnMenu, findsNothing);
      expect(more, findsNothing);
      expect(save, findsNothing);
      expect(inCapture(find.byType(InputChip)), findsNothing);
      expect(find.byType(TwoRowTextField), findsNothing);
    });

    testWidgets('"To full editor" waits for something to save', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      expect(tester.widget<TextButton>(toEditor).onPressed, isNull);

      await tester.enterText(find.byType(TextField), 'ring the plumber');
      await tester.pump();
      expect(tester.widget<TextButton>(toEditor).onPressed, isNotNull);
    });
  });

  group('"To full editor"', () {
    testWidgets('saves the Note Unsorted with the default Nudge and opens '
        'the editor on it; filing there drops the Nudge, and leaving comes '
        'back to the Agenda', (tester) async {
      final household = places.addArea('Household');
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'descale the kettle');
      await tap(tester, toEditor);

      expect(find.byType(CaptureScreen), findsNothing);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      final note = notes.all().single;
      expect(note.text, 'descale the kettle');
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
      expect(notes.unsorted().map((n) => n.id), [note.id]);

      await tap(
        tester,
        find.descendant(
          of: find.byType(NoteEditorScreen),
          matching: find.byTooltip('File into…'),
        ),
      );
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
      expect(toEditor, findsNothing);
    });
  });

  group('Settings ▸ Quick note', () {
    testWidgets('switches each control on and off again, on this Device', (
      tester,
    ) async {
      await start(tester);
      await openDeviceSettings(tester);

      for (final control in QuickNoteControl.values) {
        final key = find.byKey(ValueKey('quickNote-${control.name}'));
        expect(tester.widget<Switch>(key).value, isFalse);
        await tap(tester, key);
        expect(store.quickNoteShows(control), isTrue, reason: control.name);
        await tap(tester, key);
        expect(store.quickNoteShows(control), isFalse, reason: control.name);
      }
      // The switch names "Make it a Tally" with the Routine (Q481).
      expect(
        find.text('Routine, Reminder, Frog, Session or Tally'),
        findsOneWidget,
      );
      final nudges = find.byKey(const ValueKey('quickNote-nudgeButtons'));
      expect(tester.widget<Switch>(nudges).value, isTrue);
      await tap(tester, nudges);
      expect(store.quickNoteNudgeButtonsOn, isFalse);
      await tap(tester, nudges);
      expect(store.quickNoteNudgeButtonsOn, isTrue);
      expect(store.allEntries(), isEmpty, reason: 'settings are never synced');
    });

    testWidgets('"File into…" brings back its button', (tester) async {
      store.setQuickNoteShows(QuickNoteControl.fileInto, true);
      await start(tester);
      await openTextCapture(tester);
      expect(fileInto, findsOneWidget);
      expect(spawnMenu, findsNothing);
      expect(more, findsNothing);
    });

    testWidgets('"Routine, Reminder, Frog, Session or Tally" brings back its '
        'menu and ⋮ with "Make it a Tally" alone (Q481)', (tester) async {
      store.setQuickNoteShows(QuickNoteControl.schedule, true);
      await start(tester);
      await openTextCapture(tester);
      expect(spawnMenu, findsOneWidget);
      expect(fileInto, findsNothing);
      await tap(tester, more);
      expect(find.byKey(const ValueKey('note-make-tally')), findsOneWidget);
      expect(find.byKey(const ValueKey('importance-important')), findsNothing);
      expect(find.byKey(const ValueKey('note-add-to-shopping')), findsNothing);
    });

    testWidgets('the Importance brings back ⋮ with it alone', (tester) async {
      store.setQuickNoteShows(QuickNoteControl.importance, true);
      await start(tester);
      await openTextCapture(tester);
      expect(fileInto, findsNothing);
      expect(spawnMenu, findsNothing);
      await tap(tester, more);
      expect(find.byKey(const ValueKey('importance-important')), findsWidgets);
      expect(find.byKey(const ValueKey('note-add-to-shopping')), findsNothing);
      expect(find.byKey(const ValueKey('note-make-tally')), findsNothing);
    });

    testWidgets('"Add to a Shopping List" brings back ⋮ with it alone', (
      tester,
    ) async {
      ShoppingLists(store).make(notes.capture('Groceries').id);
      store.setQuickNoteShows(QuickNoteControl.shopping, true);
      await start(tester);
      await openTextCapture(tester);
      await tap(tester, more);
      expect(find.byKey(const ValueKey('note-add-to-shopping')), findsWidgets);
      expect(find.byKey(const ValueKey('importance-important')), findsNothing);
      expect(find.byKey(const ValueKey('note-make-tally')), findsNothing);
    });

    testWidgets('the format bar formats the quick note, saved formatted', (
      tester,
    ) async {
      store.setQuickNoteShows(QuickNoteControl.formatBar, true);
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await start(tester);
      await openTextCapture(tester);
      expect(find.byType(TwoRowTextField), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'buy milk');
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
      tester.widget<TextField>(find.byType(TextField)).controller!.selection =
          const TextSelection.collapsed(offset: 5);
      await tester.pump();
      // Bold, and bold off again: the text stays formatted (ADR 0033).
      await tester.tap(find.byKey(const ValueKey('format-bold')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('format-bold')));
      await tester.pumpAndSettle();
      tester.view.resetViewInsets();
      await tester.pumpAndSettle();
      await tap(tester, nudge10);

      final note = notes.all().single;
      expect(note.text, 'buy milk');
      expect(note.textFormat, TextFormats.markdown);
    });

    testWidgets('the Nudge buttons off give a single Save, nudged in 1 h', (
      tester,
    ) async {
      store.quickNoteNudgeButtonsOn = false;
      await start(tester);
      await openTextCapture(tester);
      expect(nudge10, findsNothing);
      expect(save, findsOneWidget);

      await tester.enterText(find.byType(TextField), 'water the plants');
      await tap(tester, save);
      final note = notes.all().single;
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
    });

    testWidgets('all of it on fits a small German phone, the keyboard up', (
      tester,
    ) async {
      for (final control in QuickNoteControl.values) {
        store.setQuickNoteShows(control, true);
      }
      ShoppingLists(store).make(notes.capture('Einkauf').id);
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
      final room = height - keyboard - TwoRowTextField.barHeight;
      for (final key in ['capturePhoto', 'captureVoice', 'captureGallery']) {
        expect(
          tester.getRect(find.byKey(ValueKey(key))).bottom,
          lessThan(room),
        );
      }
      // The text keeps two lines; the buttons under it scroll.
      expect(
        tester.getRect(find.byType(TextField)).height,
        greaterThanOrEqualTo(48),
      );
      expect(find.text('Zum vollen Editor'), findsOneWidget);
      final editor = tester.getRect(toEditor);
      expect(
        editor.right,
        lessThanOrEqualTo(
          tester
              .getRect(inCapture(find.byIcon(Icons.drive_file_move_outline)))
              .left,
        ),
      );
      final words = tester.getRect(find.text('Zum vollen Editor'));
      expect(words.right, lessThanOrEqualTo(editor.right));
      await tester.ensureVisible(nudge10);
      await tester.pumpAndSettle();
      expect(tester.getRect(nudge10).bottom, lessThanOrEqualTo(room));
      await tester.tap(nudge10);
      await tester.pumpAndSettle();
      expect(find.byType(CaptureScreen), findsNothing);
    });

    testWidgets('off again hides the control once more', (tester) async {
      store
        ..setQuickNoteShows(QuickNoteControl.fileInto, true)
        ..setQuickNoteShows(QuickNoteControl.fileInto, false);
      await start(tester);
      await openTextCapture(tester);
      expect(row, findsNothing);
    });
  });
}

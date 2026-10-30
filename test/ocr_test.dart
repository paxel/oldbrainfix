import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/ocr_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_ocr.dart';
import 'fake_photos.dart';
import 'open_capture.dart';

void main() {
  late Store store;
  late Notes notes;
  late FakeOcr ocr;
  late FakePhotos photos;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    ocr = FakeOcr();
    photos = FakePhotos(nextPhoto: onePixelPng);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester, {OcrPlatform? platform}) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, photos: photos, ocr: platform ?? ocr),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
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

  group('Photo Texts', () {
    testWidgets('photos from before are read at start', (tester) async {
      final note = notes.capture('', photos: [onePixelPng]);
      ocr.texts[onePixelPng.length] = 'Dr. Weber\nMon 9-12';
      await start(tester);

      expect(ocr.readPhotos, [onePixelPng]);
      final read = notes.byId(note.id);
      expect(read?.photoTexts, {read?.photos.single: 'Dr. Weber\nMon 9-12'});
      // The photo stays.
      expect(notes.photoBytes(read?.photos.single ?? ''), onePixelPng);
      expect(ocr.idles, 1);
    });

    testWidgets('nothing is read where RustO does not run', (tester) async {
      notes.capture('', photos: [onePixelPng]);
      ocr
        ..runs = false
        ..texts[onePixelPng.length] = 'text';
      await start(tester);
      notes.capture('more', photos: [otherPng]);
      await tester.pumpAndSettle();

      expect(ocr.readPhotos, isEmpty);
      expect(notes.unreadPhotos(), hasLength(2));
    });

    testWidgets('a new photo Note is read after it is saved, without '
        'holding Capture up', (tester) async {
      ocr
        ..texts[onePixelPng.length] = 'Level 3, row F'
        ..readGate = Completer<void>();
      await start(tester);
      await openTextCapture(tester);
      await tap(tester, find.byIcon(Icons.photo_camera_outlined));
      await tap(tester, find.byKey(const ValueKey('nudge10')));

      // Saved and back on the Agenda while RustO still reads.
      expect(find.byType(CaptureScreen), findsNothing);
      expect(find.byType(NoteCard), findsOneWidget);
      expect(ocr.readPhotos, [onePixelPng]);
      final note = notes.all().single;
      expect(note.photoTexts, isEmpty);

      ocr.readGate?.complete();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)?.photoTexts.values, ['Level 3, row F']);
    });

    testWidgets('a shared photo is read too', (tester) async {
      ocr.texts[otherPng.length] = 'Invoice 42';
      await start(tester);
      photos.share(SharedItem(photos: [otherPng]));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Save'));

      expect(notes.all().single.photoTexts.values, ['Invoice 42']);
    });

    testWidgets('a photo picked from the gallery is read too', (tester) async {
      ocr.texts[otherPng.length] = 'Parking P4';
      photos.nextPicked = [otherPng];
      await start(tester);
      await openTextCapture(tester);
      await tap(tester, find.byKey(const ValueKey('captureGallery')));
      await tap(tester, find.text('Save'));

      expect(notes.all().single.photoTexts.values, ['Parking P4']);
    });

    testWidgets('a photo that cannot be read is not tried again until the '
        'next start', (tester) async {
      final note = notes.capture('', photos: [onePixelPng]);
      await start(tester);
      expect(ocr.readPhotos, [onePixelPng]);
      expect(notes.byId(note.id)?.photoTexts, isEmpty);

      notes.edit(note.id, 'something else changes the log');
      await tester.pumpAndSettle();
      expect(ocr.readPhotos, [onePixelPng]);
      expect(notes.byId(note.id)?.photos, hasLength(1));
    });

    testWidgets('a photo without text is kept as such and not read again', (
      tester,
    ) async {
      ocr.texts[onePixelPng.length] = '';
      final note = notes.capture('', photos: [onePixelPng]);
      await start(tester);
      expect(notes.byId(note.id)?.photoTexts.values, ['']);

      notes.edit(note.id, 'something else changes the log');
      await tester.pumpAndSettle();
      expect(ocr.readPhotos, [onePixelPng]);
    });

    testWidgets('the editor shows each photo\'s text, a merged Note\'s '
        'too', (tester) async {
      ocr
        ..texts[onePixelPng.length] = 'Room 12'
        ..texts[otherPng.length] = 'Bring your insurance card';
      final into = notes.capture('doctor', photos: [onePixelPng]);
      now = now.add(const Duration(minutes: 5));
      final merged = notes.capture('', photos: [otherPng]);
      notes.merge(merged.id, into.id);
      await start(tester);
      await edit(tester, into.id);

      final editor = find.byType(NoteEditorScreen);
      expect(
        find.descendant(of: editor, matching: find.text('Room 12')),
        findsOneWidget,
      );
      await tester.scrollUntilVisible(
        find.text('Bring your insurance card'),
        100,
        scrollable: find
            .descendant(of: editor, matching: find.byType(Scrollable))
            .first,
      );
      expect(find.text('Bring your insurance card'), findsOneWidget);
    });

    testWidgets('the editor shows text found while it is open', (tester) async {
      ocr
        ..texts[onePixelPng.length] = 'Gate B7'
        ..readGate = Completer<void>();
      final note = notes.capture('flight', photos: [onePixelPng]);
      await start(tester);
      await edit(tester, note.id);
      expect(find.text('Gate B7', skipOffstage: false), findsNothing);

      ocr.readGate?.complete();
      await tester.pumpAndSettle();
      // Under the photo, which may fill the screen.
      await tester.scrollUntilVisible(
        find.text('Gate B7'),
        100,
        scrollable: find
            .descendant(
              of: find.byType(NoteEditorScreen),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      expect(find.text('Gate B7'), findsOneWidget);
    });

    testWidgets('the notes browser finds a Note by its Photo Text', (
      tester,
    ) async {
      ocr.texts[onePixelPng.length] = 'Ibuprofen 400';
      notes.capture('', photos: [onePixelPng]);
      notes.capture('call the garage');
      await start(tester);
      await tap(tester, find.byIcon(Icons.search));
      await tester.enterText(
        find.descendant(
          of: find.byType(AreasScreen),
          matching: find.byType(TextField),
        ),
        'IBUPROFEN',
      );
      await tester.pumpAndSettle();

      expect(find.byType(NoteCard), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.text('call the garage'),
        ),
        findsNothing,
      );
    });
  });
}

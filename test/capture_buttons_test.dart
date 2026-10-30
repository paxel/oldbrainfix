import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/photo_views.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

void main() {
  late Store store;
  late FakePhotos photos;
  late FakeVoice voice;

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
    photos = FakePhotos(nextPhoto: onePixelPng);
    voice = FakeVoice(nextRecording: Uint8List.fromList([1, 2, 3]));
  });
  tearDown(() => store.close());

  final photo = find.byKey(const ValueKey('capturePhoto'));
  final mic = find.byKey(const ValueKey('captureVoice'));
  final gallery = find.byKey(const ValueKey('captureGallery'));
  final paste = find.byKey(const ValueKey('capturePaste'));
  final stop = find.byKey(const ValueKey('stopRecording'));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens text Capture from the Agenda's +.
  Future<void> openCapture(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, photos: photos, voice: voice),
    );
    await tester.pumpAndSettle();
    await openTextCapture(tester);
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  /// The top of the remind me line.
  double nudgeTop(WidgetTester tester) =>
      tester.getRect(find.byType(NudgeButtons)).top;

  /// Whether the label [text] shows on one line, as high as the same
  /// words laid out without a width limit. The test font's glyphs are
  /// wider than the phone's, so whether the words fit across is not
  /// measured.
  void expectOneLine(WidgetTester tester, String text) {
    final label = tester.renderObject<RenderParagraph>(find.text(text));
    final line = TextPainter(
      text: label.text,
      textDirection: TextDirection.ltr,
      textScaler: label.textScaler,
    )..layout();
    addTearDown(line.dispose);
    expect(label.didExceedMaxLines, isFalse);
    expect(label.size.height, line.height);
  }

  testWidgets('Photo, Voice, Gallery and Paste sit side by side above the '
      'remind me line', (tester) async {
    await openCapture(tester);

    expect(find.text('Photo'), findsOneWidget);
    expect(find.text('Voice'), findsOneWidget);
    expect(find.text('Gallery'), findsOneWidget);
    expect(find.text('Paste'), findsOneWidget);
    final rects = [
      for (final button in [photo, mic, gallery, paste]) tester.getRect(button),
    ];
    for (var i = 1; i < rects.length; i++) {
      expect(rects[i].top, rects[0].top);
      expect(rects[i - 1].right, lessThan(rects[i].left));
    }
    for (final rect in rects) {
      expect(rect.bottom, lessThanOrEqualTo(nudgeTop(tester)));
    }
    for (final label in ['Photo', 'Voice', 'Gallery', 'Paste']) {
      expectOneLine(tester, label);
    }
    expect(
      find.descendant(of: paste, matching: find.byIcon(Icons.content_paste)),
      findsOneWidget,
    );
    // The clipboard is read only once Paste is tapped.
    expect(photos.pastes, 0);
    expect(
      find.descendant(
        of: gallery,
        matching: find.byIcon(Icons.photo_library_outlined),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: photo,
        matching: find.byIcon(Icons.photo_camera_outlined),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(of: mic, matching: find.byIcon(Icons.mic_none)),
      findsOneWidget,
    );
  });

  testWidgets('the top bar holds neither camera nor microphone', (
    tester,
  ) async {
    await openCapture(tester);

    final bar = find.byType(AppBar);
    expect(
      find.descendant(
        of: bar,
        matching: find.byIcon(Icons.photo_camera_outlined),
      ),
      findsNothing,
    );
    expect(
      find.descendant(of: bar, matching: find.byIcon(Icons.mic_none)),
      findsNothing,
    );
    expect(
      find.descendant(of: bar, matching: find.text('Save')),
      findsOneWidget,
    );
  });

  testWidgets('Photo takes a photo', (tester) async {
    await openCapture(tester);
    await tap(tester, photo);

    expect(photos.taken, 1);
    expect(Notes(store).all().single.photos, hasLength(1));
  });

  testWidgets('Gallery puts the pictures picked into one Note', (tester) async {
    photos.nextPicked = [onePixelPng, otherPng];
    await openCapture(tester);
    await tester.enterText(find.byType(TextField), 'Receipts');
    await tap(tester, gallery);

    expect(photos.picks, 1);
    expect(photos.taken, 0);
    final note = Notes(store).all().single;
    expect(note.photos, hasLength(2));
    expect(note.text, 'Receipts');
    expect(find.byType(CaptureScreen), findsOneWidget);
  });

  testWidgets('Paste puts the text at the cursor and the picture into the '
      'Note', (tester) async {
    photos.clipboard = SharedItem(text: 'old flat', photos: [onePixelPng]);
    await openCapture(tester);
    await tester.enterText(find.byType(TextField), 'Receipts of the ');
    await tap(tester, paste);

    expect(photos.pastes, 1);
    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.controller!.text, 'Receipts of the old flat');
    expect(
      field.controller!.selection.baseOffset,
      field.controller!.text.length,
    );
    final note = Notes(store).all().single;
    expect(note.photos, hasLength(1));
    expect(note.text, 'Receipts of the old flat');
    expect(find.byType(CaptureScreen), findsOneWidget);
  });

  testWidgets('Paste replaces the selected words', (tester) async {
    photos.clipboard = const SharedItem(text: 'Tuesday');
    await openCapture(tester);
    await tester.enterText(find.byType(TextField), 'dentist Monday 9:00');
    final field = tester.widget<TextField>(find.byType(TextField));
    field.controller!.selection = const TextSelection(
      baseOffset: 8,
      extentOffset: 14,
    );
    await tap(tester, paste);

    expect(field.controller!.text, 'dentist Tuesday 9:00');
    await tap(tester, find.text('Save'));
    expect(Notes(store).all().single.text, 'dentist Tuesday 9:00');
  });

  testWidgets('Paste with nothing usable says so above the field and '
      'changes nothing', (tester) async {
    await openCapture(tester);
    await tester.enterText(find.byType(TextField), 'buy stamps');
    await tap(tester, paste);

    expect(photos.pastes, 1);
    expect(find.byKey(const ValueKey('paste-empty')), findsOneWidget);
    expect(
      find.text(
        'Nothing to paste: the clipboard holds no text and no picture.',
      ),
      findsOneWidget,
    );
    expect(Notes(store).all(), isEmpty);

    // A Paste that finds something answers it.
    photos.clipboard = const SharedItem(text: ' at noon');
    await tap(tester, paste);
    expect(find.byKey(const ValueKey('paste-empty')), findsNothing);
    await tap(tester, find.text('Save'));
    expect(Notes(store).all().single.text, 'buy stamps at noon');
  });

  testWidgets('leaving while the pictures are read still keeps them', (
    tester,
  ) async {
    photos.picker = Completer();
    await openCapture(tester);
    await tester.tap(gallery);
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('nudge10')));
    await tester.pump();
    photos.picker?.complete([onePixelPng, otherPng]);
    await tester.pumpAndSettle();

    expect(find.byType(CaptureScreen), findsNothing);
    final note = Notes(store).all().single;
    expect(note.photos, hasLength(2));
    // The Nudge picked holds for the pictures: they were written first.
    expect(
      note.nudge?.isBefore(DateTime.now().add(const Duration(minutes: 20))),
      isTrue,
    );
  });

  testWidgets('closing the picker without picking changes nothing', (
    tester,
  ) async {
    await openCapture(tester);
    await tester.enterText(find.byType(TextField), 'Half a thought');
    await tap(tester, gallery);

    expect(photos.picks, 1);
    expect(Notes(store).all(), isEmpty);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('Half a thought'), findsOneWidget);
    expect(find.byType(PhotoStrip), findsNothing);
  });

  testWidgets('Voice records at once; the buttons wait meanwhile', (
    tester,
  ) async {
    await openCapture(tester);
    await tap(tester, mic);

    expect(voice.starts, 1);
    expect(voice.recording, isTrue);
    expect(find.text('Recording…'), findsOneWidget);
    expect(photo, findsNothing);
    expect(mic, findsNothing);
    expect(find.byType(NudgeButtons), findsOneWidget);

    await tap(tester, stop);
    expect(Notes(store).all().single.recordings, hasLength(1));
  });

  testWidgets('they stay above the keyboard on a small German phone', (
    tester,
  ) async {
    // 360 × 640 with a 48 high navigation bar, which the keyboard covers.
    const height = 640.0;
    const keyboard = 300.0;
    final ratio = tester.view.devicePixelRatio;
    tester.view.physicalSize = Size(360 * ratio, height * ratio);
    tester.view.viewPadding = FakeViewPadding(bottom: 48 * ratio);
    addTearDown(tester.view.reset);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    await openCapture(tester);
    // A photo taken first: its strip takes room too.
    await tap(tester, photo);
    expect(photos.taken, 1);

    tester.view.padding = FakeViewPadding.zero;
    tester.view.viewInsets = FakeViewPadding(bottom: keyboard * ratio);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Foto'), findsOneWidget);
    expect(find.text('Sprache'), findsOneWidget);
    expect(find.text('Galerie'), findsOneWidget);
    expect(find.text('Einfügen'), findsOneWidget);
    for (final label in ['Foto', 'Sprache', 'Galerie', 'Einfügen']) {
      expectOneLine(tester, label);
    }
    // Four in a row: a long word shrinks to fit its button, not cut.
    for (final (button, label) in [
      (photo, 'Foto'),
      (mic, 'Sprache'),
      (gallery, 'Galerie'),
      (paste, 'Einfügen'),
    ]) {
      final words = tester.renderObject<RenderParagraph>(find.text(label));
      expect(
        words.size.width,
        greaterThanOrEqualTo(words.getMaxIntrinsicWidth(double.infinity)),
      );
      final shown = tester.getRect(find.text(label));
      final box = tester.getRect(button);
      expect(shown.left, greaterThanOrEqualTo(box.left));
      expect(shown.right, lessThanOrEqualTo(box.right));
    }
    for (final button in [photo, mic, gallery, paste]) {
      expect(
        tester.getRect(button).bottom,
        lessThanOrEqualTo(height - keyboard),
      );
    }
    await tap(tester, photo);
    expect(photos.taken, 2);
    photos.nextPicked = [otherPng];
    await tap(tester, gallery);
    expect(photos.picks, 1);
    // The camera's two photos are the same picture, kept once.
    expect(Notes(store).all().single.photos, hasLength(2));
    await tap(tester, mic);
    expect(voice.starts, 1);
    // Voice takes the focus from the text: the keyboard goes down.
    tester.view.resetViewInsets();
    await tester.pumpAndSettle();
    await tap(tester, stop);
    expect(Notes(store).all().single.recordings, hasLength(1));
  });
}

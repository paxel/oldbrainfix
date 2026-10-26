import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';

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
    await tap(tester, find.byIcon(Icons.add));
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  /// The top of the remind me line.
  double nudgeTop(WidgetTester tester) =>
      tester.getRect(find.byType(NudgeButtons)).top;

  testWidgets('Photo and Voice sit side by side above the remind me line', (
    tester,
  ) async {
    await openCapture(tester);

    expect(find.text('Photo'), findsOneWidget);
    expect(find.text('Voice'), findsOneWidget);
    final left = tester.getRect(photo);
    final right = tester.getRect(mic);
    expect(left.top, right.top);
    expect(left.right, lessThan(right.left));
    expect(left.bottom, lessThanOrEqualTo(nudgeTop(tester)));
    expect(right.bottom, lessThanOrEqualTo(nudgeTop(tester)));
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
    for (final button in [photo, mic]) {
      expect(
        tester.getRect(button).bottom,
        lessThanOrEqualTo(height - keyboard),
      );
    }
    await tap(tester, photo);
    expect(photos.taken, 2);
    await tap(tester, mic);
    expect(voice.starts, 1);
    // Voice takes the focus from the text: the keyboard goes down.
    tester.view.resetViewInsets();
    await tester.pumpAndSettle();
    await tap(tester, stop);
    expect(Notes(store).all().single.recordings, hasLength(1));
  });
}

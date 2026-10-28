import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_requests.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_capture_requests.dart';
import 'fake_photos.dart';
import 'fake_voice.dart';

final recorded = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70]);

void main() {
  late Store store;
  late Notes notes;
  late FakeCaptureRequests requests;
  late FakeVoice voice;
  late FakePhotos photos;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
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

  final stop = find.byKey(const ValueKey('stopRecording'));

  group('An entry point asking at start', () {
    testWidgets('for text opens text Capture with the keyboard up', (
      tester,
    ) async {
      requests.waiting = CaptureMode.text;
      await start(tester);

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(find.byType(NudgeButtons), findsOneWidget);
      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.autofocus, isTrue);
      expect(voice.starts, 0);
      expect(photos.taken, 0);

      await tester.enterText(find.byType(TextField), 'call the vet');
      await back(tester);

      // Back saves, Unsorted and nudged in 1 h, as from "+".
      final note = notes.all().single;
      expect(note.text, 'call the vet');
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
      expect(find.byType(CaptureScreen), findsNothing);
      expect(find.byType(NoteCard), findsOneWidget);
    });

    testWidgets('for voice records at once', (tester) async {
      requests.waiting = CaptureMode.voice;
      await start(tester);

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(voice.starts, 1);
      expect(voice.recording, isTrue);
      expect(find.byType(NudgeButtons), findsOneWidget);

      await tap(tester, stop);

      final note = notes.all().single;
      expect(note.recordings, [store.putAudio(recorded)]);
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
    });

    testWidgets('for a photo opens the camera at once', (tester) async {
      requests.waiting = CaptureMode.photo;
      await start(tester);

      expect(photos.taken, 1);
      expect(voice.starts, 0);
      // Back from the camera, the photo is in the Note already.
      final note = notes.all().single;
      expect(note.photos, hasLength(1));

      await tap(tester, find.byKey(const ValueKey('nudge10')));

      final saved = notes.byId(note.id);
      expect(saved?.isUnsorted, isTrue);
      expect(saved?.nudge, now.add(const Duration(minutes: 10)));
      expect(find.byType(CaptureScreen), findsNothing);
    });

    testWidgets('for a photo leaves nothing when the camera is cancelled', (
      tester,
    ) async {
      photos.nextPhoto = null;
      requests.waiting = CaptureMode.photo;
      await start(tester);

      expect(photos.taken, 1);
      expect(find.byType(CaptureScreen), findsOneWidget);
      await back(tester);

      expect(notes.all(), isEmpty);
    });
  });

  testWidgets('an entry point asking while the app runs opens Capture', (
    tester,
  ) async {
    await start(tester);
    expect(find.byType(CaptureScreen), findsNothing);

    requests.request(CaptureMode.voice);
    await tester.pumpAndSettle();

    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(voice.starts, 1);
    await tap(tester, stop);
    expect(notes.all(), hasLength(1));
  });

  testWidgets('a request opens once: coming back does not open it again', (
    tester,
  ) async {
    requests.waiting = CaptureMode.text;
    await start(tester);
    await tester.enterText(find.byType(TextField), 'milk');
    await back(tester);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(find.byType(CaptureScreen), findsNothing);
    expect(notes.all(), hasLength(1));
  });

  testWidgets('a request on the first start waits for the Starter Set', (
    tester,
  ) async {
    store.starterSetOffered = false;
    requests.waiting = CaptureMode.text;
    await start(tester);
    await tap(tester, find.byKey(const ValueKey('first-new')));

    expect(find.byType(StarterSetScreen), findsOneWidget);
    expect(find.byType(CaptureScreen), findsNothing);

    await tap(tester, find.text('Start'));

    expect(find.byType(CaptureScreen), findsOneWidget);
  });

  testWidgets('a share and a request open one after the other', (tester) async {
    photos.waiting.add(const SharedItem(text: 'shared text'));
    requests.waiting = CaptureMode.text;
    await start(tester);

    // The share first, filled in.
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('shared text'), findsOneWidget);
    await back(tester);

    // Then the capture asked for, empty; never both at once.
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('shared text'), findsNothing);
    await tester.enterText(find.byType(TextField), 'own words');
    await back(tester);

    expect(
      notes.all().map((note) => note.text),
      containsAll(['shared text', 'own words']),
    );
  });
}

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations_de.dart';
import 'package:oldbrainfix/l10n/app_localizations_en.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_requests.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_capture_requests.dart';
import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

final recorded = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70]);

final en = AppLocalizationsEn();

void main() {
  late Store store;
  late Notes notes;
  late FakeCaptureRequests requests;
  late FakeVoice voice;
  late FakePhotos photos;

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
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

  final plus = find.byIcon(Icons.add);
  final close = find.byKey(const ValueKey('captureFanClose'));
  const ways = ['text', 'voice', 'photo', 'gallery'];

  void expectFanOpen(bool open) {
    for (final way in ways) {
      expect(fanEntry(way), open ? findsOneWidget : findsNothing);
    }
    expect(close, open ? findsOneWidget : findsNothing);
  }

  group('"+" on the Agenda', () {
    testWidgets('fans out Text, Voice, Photo and Gallery, each with its '
        'words, and opens no capture by itself', (tester) async {
      await start(tester);
      await tap(tester, plus);

      expectFanOpen(true);
      for (final label in ['Text', 'Voice', 'Photo', 'Gallery']) {
        expect(find.text(label), findsOneWidget);
      }
      expect(find.byType(CaptureScreen), findsNothing);
      // The fan opens upward from "+", Text nearest it.
      final closeTop = tester.getRect(close).top;
      final tops = [for (final way in ways) tester.getRect(fanEntry(way)).top];
      expect(tops.first, lessThan(closeTop));
      for (var i = 1; i < tops.length; i++) {
        expect(tops[i], lessThan(tops[i - 1]));
      }
      expect(voice.starts, 0);
      expect(photos.taken, 0);
      expect(photos.picks, 0);
    });

    testWidgets('shows only the ways the Device has', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tap(tester, plus);

      expect(fanEntry('text'), findsOneWidget);
      expect(fanEntry('voice'), findsNothing);
      expect(fanEntry('photo'), findsNothing);
      expect(fanEntry('gallery'), findsNothing);
    });

    testWidgets('speaks German', (tester) async {
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await start(tester);
      await tap(tester, plus);

      final de = AppLocalizationsDe();
      for (final label in [de.text, de.voice, de.photo, de.gallery]) {
        expect(find.text(label), findsOneWidget);
      }
      expect(find.byTooltip(de.captureFanClose), findsOneWidget);
    });
  });

  group('The fan closes', () {
    testWidgets('on a second tap of "+", which reads as close', (tester) async {
      await start(tester);
      await tap(tester, plus);
      expect(
        find.descendant(of: close, matching: find.byIcon(Icons.close)),
        findsOneWidget,
      );

      await tap(tester, close);

      expectFanOpen(false);
      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all(), isEmpty);
    });

    testWidgets('on a tap outside, which opens nothing under it', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, plus);

      // Where the button for a new Routine is, under the fan's shade.
      final newRoutine = find.byTooltip(en.newRoutineTooltip);
      await tester.tapAt(tester.getCenter(newRoutine));
      await tester.pumpAndSettle();

      expectFanOpen(false);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
      // With the fan closed, the same tap opens it.
      await tap(tester, newRoutine);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
    });

    testWidgets('on Back, staying on the Agenda', (tester) async {
      await start(tester);
      await tap(tester, plus);

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();

      expectFanOpen(false);
      expect(find.byType(AgendaScreen), findsOneWidget);
      // "+" opens it again.
      await tap(tester, plus);
      expectFanOpen(true);
    });

    testWidgets('when an entry point asks for Capture meanwhile, which '
        'opens directly', (tester) async {
      await start(tester);
      await tap(tester, plus);

      requests.request(CaptureMode.voice);
      await tester.pumpAndSettle();

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(voice.starts, 1);
      expect(fanEntry('text'), findsNothing);
      await tap(tester, find.byKey(const ValueKey('stopRecording')));
      await tap(tester, find.text('Save'));
      expectFanOpen(false);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });
  });

  group('Each way opens its capture', () {
    testWidgets('Text opens text Capture with the keyboard up', (tester) async {
      await start(tester);
      await openTextCapture(tester);

      expect(find.byType(CaptureScreen), findsOneWidget);
      expectFanOpen(false);
      expect(tester.widget<TextField>(find.byType(TextField)).autofocus, true);
      expect(voice.starts, 0);
      expect(photos.taken, 0);
      expect(photos.picks, 0);

      await tester.enterText(find.byType(TextField), 'call the vet');
      await tap(tester, find.text('Save'));
      expect(notes.all().single.text, 'call the vet');
    });

    testWidgets('Voice starts recording at once', (tester) async {
      await start(tester);
      await tap(tester, plus);
      await tap(tester, fanEntry('voice'));

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(voice.starts, 1);
      expect(voice.recording, isTrue);

      await tap(tester, find.byKey(const ValueKey('stopRecording')));
      expect(notes.all().single.recordings, hasLength(1));
    });

    testWidgets('Photo opens the camera, then text Capture with the photo', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, plus);
      await tap(tester, fanEntry('photo'));

      expect(photos.taken, 1);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(tester.widget<TextField>(find.byType(TextField)).autofocus, false);
      expect(notes.all().single.photos, hasLength(1));
    });

    testWidgets('Gallery opens the picker and continues as a photo Note', (
      tester,
    ) async {
      photos.nextPicked = [onePixelPng, otherPng];
      await start(tester);
      await tap(tester, plus);
      await tap(tester, fanEntry('gallery'));

      expect(photos.picks, 1);
      expect(photos.taken, 0);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(tester.widget<TextField>(find.byType(TextField)).autofocus, false);
      expect(notes.all().single.photos, hasLength(2));

      await tester.enterText(find.byType(TextField), 'the old flat');
      await tap(tester, find.text('Save'));
      expect(notes.all().single.text, 'the old flat');
    });

    testWidgets('Gallery closed without picking leaves plain text Capture', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, plus);
      await tap(tester, fanEntry('gallery'));

      expect(photos.picks, 1);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(notes.all(), isEmpty);
      await tap(tester, find.text('Save'));
      expect(notes.all(), isEmpty);
    });
  });
}

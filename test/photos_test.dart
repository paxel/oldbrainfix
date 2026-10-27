import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/photo_views.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';
import 'reveal.dart';
import 'open_capture.dart';

void main() {
  late Store store;
  late Notes notes;
  late FakePhotos photos;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    photos = FakePhotos(nextPhoto: onePixelPng);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, photos: photos));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor of [note] from the Agenda.
  Future<void> edit(WidgetTester tester, Note note) async {
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, Places(store), note));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  final camera = find.byIcon(Icons.photo_camera_outlined);

  group('the camera', () {
    testWidgets('is not offered where there is none', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openTextCapture(tester);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(camera, findsNothing);
    });

    testWidgets('takes a photo Note with the Nudge picked', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tap(tester, camera);

      expect(photos.taken, 1);
      // The photo shows on the capture screen; words are optional.
      expect(find.byType(NotePhoto), findsOneWidget);
      expect(find.text('A few words about it (optional)'), findsOneWidget);
      expect(find.byType(NudgeButtons), findsOneWidget);

      await tap(tester, find.byKey(const ValueKey('nudge10')));
      final note = notes.all().single;
      expect(note.text, '');
      expect(note.photos, [store.putPhoto(onePixelPng)]);
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(const Duration(minutes: 10)));
      // Its card shows on the Agenda with a thumbnail.
      expect(
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.byType(NotePhoto),
        ),
        findsOneWidget,
      );
    });

    testWidgets('a few words go with the photo into one Note', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'this shelf');
      await tap(tester, camera);
      photos.nextPhoto = otherPng;
      await tap(tester, camera);
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.text, 'this shelf');
      expect(note.photos, hasLength(2));
    });

    testWidgets('backing out of the camera captures nothing', (tester) async {
      photos.nextPhoto = null;
      await start(tester);
      await openTextCapture(tester);
      await tap(tester, camera);
      await tap(tester, find.text('Save'));

      expect(notes.all(), isEmpty);
    });

    testWidgets('the app going away for the camera makes one Note', (
      tester,
    ) async {
      final shot = Completer<Uint8List?>();
      photos.camera = shot;
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'parking spot');
      await tester.tap(camera);
      await tester.pump();
      // The camera app covers ours: the words are saved already.
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
      expect(notes.all().single.photos, isEmpty);
      shot.complete(onePixelPng);
      await tester.pumpAndSettle();
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.text, 'parking spot');
      expect(note.photos, hasLength(1));
    });
  });

  group('sharing in', () {
    testWidgets('a share at start opens Capture filled in, Unsorted', (
      tester,
    ) async {
      final places = Places(store);
      places.addArea('Household');
      photos.waiting.add(
        SharedItem(
          text: 'Sauce recipe\nhttps://example.org/sauce',
          photos: [onePixelPng],
        ),
      );
      await start(tester);

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(find.text('Sauce recipe\nhttps://example.org/sauce'), findsOne);
      expect(find.byType(NotePhoto), findsOneWidget);
      expect(find.byType(NudgeButtons), findsOneWidget);

      await tester.enterText(find.byType(TextField), 'Sauce recipe for Sunday');
      await tap(tester, find.byKey(const ValueKey('nudge30')));

      final note = notes.all().single;
      expect(note.text, 'Sauce recipe for Sunday');
      expect(note.photos, hasLength(1));
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(const Duration(minutes: 30)));
      expect(find.byType(NoteCard), findsOneWidget);
    });

    testWidgets('a share while the app runs opens Capture', (tester) async {
      await start(tester);
      photos.share(const SharedItem(text: 'https://example.org/pills'));
      await tester.pumpAndSettle();

      expect(find.byType(CaptureScreen), findsOneWidget);
      // It is kept even when the user just goes back.
      expect(notes.all().single.text, 'https://example.org/pills');
      final dynamic navigator = tester.state(find.byType(Navigator).first);
      await (navigator as NavigatorState).maybePop();
      await tester.pumpAndSettle();

      final note = notes.all().single;
      expect(note.text, 'https://example.org/pills');
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
    });

    testWidgets('two shares open one after the other', (tester) async {
      await start(tester);
      photos.waiting.add(const SharedItem(text: 'first'));
      photos.share(SharedItem(photos: [onePixelPng, otherPng]));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Save'));
      await tap(tester, find.text('Save'));

      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all().map((n) => n.photos.length), unorderedEquals([0, 2]));
    });

    test('a link\'s title goes on top of the link', () {
      expect(
        sharedText('Sauce', 'https://example.org/sauce'),
        'Sauce\nhttps://example.org/sauce',
      );
      expect(sharedText('Sauce', 'Sauce https://x.org'), 'Sauce https://x.org');
      expect(sharedText(null, ' hello '), 'hello');
      expect(sharedText('Only a subject', null), 'Only a subject');
      expect(sharedText('', ''), '');
    });
  });

  group('photos on Notes', () {
    testWidgets('the editor shows them in full; a tap opens one', (
      tester,
    ) async {
      final note = notes.capture('receipt', photos: [onePixelPng]);
      await start(tester);
      await edit(tester, note);

      expect(find.byType(NotePhoto), findsOneWidget);
      expect(tester.widget<NotePhoto>(find.byType(NotePhoto)).size, isNull);
      // Decoding runs outside the test's fake time.
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 200)),
      );
      await tester.pumpAndSettle();
      await tap(tester, find.byType(NotePhoto));
      expect(find.byType(PhotoViewerScreen), findsOneWidget);
      expect(find.byType(InteractiveViewer), findsOneWidget);
    });

    testWidgets('a merged photo Note shows its photo in the Section', (
      tester,
    ) async {
      final into = notes.capture('sauce');
      now = now.add(const Duration(minutes: 1));
      final merged = notes.capture('', photos: [onePixelPng]);
      notes.merge(merged.id, into.id);
      await start(tester);
      await edit(tester, into);

      await reveal(tester, find.byType(NotePhoto));
      expect(find.byType(NotePhoto), findsOneWidget);
    });

    testWidgets('a card shows three thumbnails and counts the rest', (
      tester,
    ) async {
      final more = [
        for (var i = 0; i < 5; i++) Uint8List.fromList([...onePixelPng, i]),
      ];
      final note = notes.capture('lots', photos: more);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: PhotoScope(
            photos: photos,
            bytes: notes.photoBytes,
            child: Scaffold(
              body: NoteCard(note: note, when: DateFormat.yMd('en').add_Hm()),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(NotePhoto), findsNWidgets(3));
      expect(find.text('+2'), findsOneWidget);
    });

    testWidgets('a photo Note without words shows its photo when Unsorted', (
      tester,
    ) async {
      notes.capture('', photos: [onePixelPng]);
      await start(tester);
      await tap(tester, find.text('1 Unsorted'));

      expect(
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.byType(NotePhoto),
        ),
        findsOneWidget,
      );
      final texts = tester
          .widgetList<Text>(
            find.descendant(
              of: find.byType(NoteCard),
              matching: find.byType(Text),
            ),
          )
          .map((w) => w.data);
      expect(texts, isNot(contains('')));
    });

    testWidgets('a Routine from a photo Note without words is "Photo"', (
      tester,
    ) async {
      final alarms = FakeAlarms();
      store.alarmSetupDone = true;
      final note = notes.capture('', photos: [onePixelPng]);
      final local = now.toLocal();
      Routines(store).spawn(
        note.id,
        Schedule(
          start: Day(local.year, local.month, local.day),
          times: [ClockTime(23, 59)],
        ),
      );
      await tester.pumpWidget(
        OldBrainFixApp(store: store, photos: photos, alarms: alarms),
      );
      await tester.pumpAndSettle();

      expect(find.text('Photo'), findsOneWidget);
      expect(alarms.armed.single.title, 'Photo');
    });

    testWidgets('a photo not on this Device shows as broken', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: PhotoScope(
            photos: photos,
            bytes: notes.photoBytes,
            child: const Scaffold(body: NotePhoto(hash: 'missing', size: 64)),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
    });
  });
}

import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/photo_views.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'open_capture.dart';

/// The Note lists show every Note in full, with a full/compact switch
/// per list; the editor opens by a long press or Edit in ⋮ (#114).
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  /// Taps [finder] after scrolling it on screen: the card's ⋮ is longer
  /// than the test surface, and its menu scrolls.
  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store, photos: FakePhotos()));
    await tester.pumpAndSettle();
  }

  Future<void> openAllNotes(WidgetTester tester) async {
    await tap(tester, find.byIcon(Icons.search));
    await tap(tester, find.byTooltip('All Notes'));
  }

  double top(WidgetTester tester, String text) =>
      tester.getTopLeft(find.text(text)).dy;

  /// A Note with a recording and a photo and their texts, a Note merged
  /// into it later and an Amend after that.
  Note amendedNote() {
    final note = notes.capture(
      'Original first line\nOriginal second line',
      recordings: [
        Uint8List.fromList(const [1, 2, 3]),
      ],
      photos: [onePixelPng],
    );
    notes.setTranscript(note.id, note.recordings.single, 'Spoken words');
    notes.setPhotoText(note.id, note.photos.single, 'Words on the photo');
    now = now.add(const Duration(hours: 1));
    final merged = notes.capture('Merged in');
    notes.merge(merged.id, note.id);
    now = now.add(const Duration(hours: 1));
    notes.amend(note.id, 'Amended later');
    return note;
  }

  group('a full card', () {
    testWidgets('shows every Section oldest first, each under its date, '
        'and the Transcripts and Photo Texts labelled', (tester) async {
      amendedNote();
      await start(tester);
      await openAllNotes(tester);

      final when = DateFormat.yMd('en').add_Hm();
      String at(int hours) =>
          when.format(DateTime.utc(2026, 10, 24, 8 + hours).toLocal());
      final order = [
        top(tester, at(0)),
        top(tester, 'Original first line\nOriginal second line'),
        top(tester, 'Words on the photo'),
        top(tester, 'Spoken words'),
        top(tester, at(1)),
        top(tester, 'Merged in'),
        top(tester, at(2)),
        top(tester, 'Amended later'),
      ];
      for (var i = 1; i < order.length; i++) {
        expect(order[i], greaterThan(order[i - 1]), reason: 'step $i');
      }
      expect(find.text('Photo Text'), findsOneWidget);
      expect(find.text('Transcript'), findsOneWidget);
      expect(top(tester, 'Transcript'), lessThan(top(tester, 'Spoken words')));
      // The photo at full width, not as a thumbnail.
      final photo = find.descendant(
        of: find.byType(NoteCard),
        matching: find.byType(NotePhoto),
      );
      expect(tester.widget<NotePhoto>(photo).size, isNull);
    });

    testWidgets('a list reads the Sections of all its Notes in one pass, '
        'once per reload', (tester) async {
      final counted = _CountingNotes(store);
      final first = notes.capture('First Note');
      now = now.add(const Duration(hours: 1));
      notes.amend(first.id, 'First amended');
      final second = notes.capture('Second Note');
      final merged = notes.capture('Merged into the second');
      notes.merge(merged.id, second.id);
      final places = Places(store);
      List<PlacedNote> load() => [
        for (final note in counted.all()) placeNote(places, note),
      ];
      var shown = load();
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return ListView(
                  children: diaryCards(
                    context,
                    shown,
                    (_) {},
                    (_) {},
                    (_) {},
                    notes: counted,
                    onChanged: () {},
                    showPlace: (_) => false,
                  ),
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('First amended'), findsOneWidget);
      expect(find.text('Merged into the second'), findsOneWidget);
      expect((counted.atOnce, counted.each), (1, 0));

      // A reload reads them again, once; a rebuild without one does not.
      rebuild(() => shown = load());
      await tester.pumpAndSettle();
      expect((counted.atOnce, counted.each), (2, 0));
      rebuild(() {});
      await tester.pumpAndSettle();
      expect((counted.atOnce, counted.each), (2, 0));
      expect(find.text('First amended'), findsOneWidget);
    });

    testWidgets('a tap does nothing; a long press opens the editor', (
      tester,
    ) async {
      amendedNote();
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, find.text('Amended later'));
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.text('Merged in'), findsOneWidget);

      await tester.longPress(find.text('Amended later'));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('Edit in ⋮ opens the editor', (tester) async {
      notes.capture('Buy milk');
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await tap(tester, find.byKey(const ValueKey('note-edit')));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('a tapped photo opens full screen', (tester) async {
      notes.capture('A picture', photos: [onePixelPng]);
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, find.byType(NotePhoto));
      expect(find.byType(PhotoViewerScreen), findsOneWidget);
      expect(find.byType(InteractiveViewer), findsOneWidget);
    });
  });

  testWidgets('the Agenda\'s just-captured card is compact and unfolds on '
      'a tap', (tester) async {
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'milk\nand bread');
    await tap(tester, find.text('Save'));

    expect(find.text('Just captured'), findsOneWidget);
    expect(find.byKey(const ValueKey('note-first-line')), findsOneWidget);
    await tap(tester, find.byKey(const ValueKey('note-first-line')));
    expect(find.byType(NoteEditorScreen), findsNothing);
    expect(find.text('milk\nand bread'), findsOneWidget);
    expect(find.byKey(const ValueKey('note-first-line')), findsNothing);
  });

  group('the full/compact switch', () {
    testWidgets('shows the first line only; a tap unfolds the Note in '
        'place', (tester) async {
      amendedNote();
      notes.capture('Another Note');
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(find.text('Original first line'), findsOneWidget);
      expect(find.text('Merged in'), findsNothing);
      expect(find.text('Amended later'), findsNothing);
      expect(find.text('Transcript'), findsNothing);
      expect(find.byTooltip('Show Notes in full'), findsOneWidget);

      await tap(tester, find.text('Original first line'));
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.text('Merged in'), findsOneWidget);
      expect(find.text('Amended later'), findsOneWidget);
      // Only the Note tapped unfolds.
      expect(find.byKey(const ValueKey('note-first-line')), findsOneWidget);

      await tester.longPress(find.text('Another Note'));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('is remembered per list on this Device', (tester) async {
      final places = Places(store);
      final area = places.addArea('Household');
      notes.capture('Fix the tap\nwith the washer', place: area.id);
      await start(tester);
      await openAllNotes(tester);

      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(store.localSetting('compactNotes:allNotes'), 'true');
      expect(find.text('Fix the tap'), findsOneWidget);

      // Another list keeps its own choice: full.
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.text('Household'));
      expect(find.text('Fix the tap\nwith the washer'), findsOneWidget);
      expect(find.byTooltip('Show Notes compact'), findsOneWidget);

      // Back to All Notes, still compact; full again is not kept.
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('All Notes'));
      expect(find.text('Fix the tap'), findsOneWidget);
      await tap(tester, find.byTooltip('Show Notes in full'));
      expect(find.text('Fix the tap\nwith the washer'), findsOneWidget);
      expect(store.localSetting('compactNotes:allNotes'), isNull);
    });

    testWidgets('is on the Unsorted list and the search results', (
      tester,
    ) async {
      notes.capture('Call the plumber\ntomorrow');
      await start(tester);

      await tap(tester, find.text('1 Unsorted'));
      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(store.localSetting('compactNotes:unsorted'), 'true');
      expect(find.text('Call the plumber'), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();

      await tap(tester, find.byIcon(Icons.search));
      await tester.enterText(find.byType(TextField), 'plumber');
      await tester.pumpAndSettle();
      expect(find.text('Call the plumber\ntomorrow'), findsOneWidget);
      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(store.localSetting('compactNotes:search'), 'true');
    });
  });

  group('an extreme photo', () {
    /// A PNG of [width] × [height] pixels.
    Future<Uint8List> png(WidgetTester tester, int width, int height) async {
      final bytes = await tester.runAsync(() async {
        final recorder = ui.PictureRecorder();
        Canvas(recorder).drawRect(
          Rect.fromLTWH(0, 0, width.toDouble(), height.toDouble()),
          Paint()..color = const Color(0xFF008080),
        );
        final image = await recorder.endRecording().toImage(width, height);
        final data = await image.toByteData(format: ui.ImageByteFormat.png);
        image.dispose();
        return data!.buffer.asUint8List();
      });
      return bytes!;
    }

    test('decodes within its bounds, at least a pixel each way', () {
      final tall = BoundedPhoto.fit(12, 34000, maxWidth: 1080, maxHeight: 1440);
      expect((tall.width, tall.height), (1, 1440));
      final wide = BoundedPhoto.fit(34000, 12, maxWidth: 1080, maxHeight: 1440);
      expect((wide.width, wide.height), (1080, 1));
      final small = BoundedPhoto.fit(200, 100, maxWidth: 1080, maxHeight: 1440);
      expect((small.width, small.height), (200, 100));
      final photo = BoundedPhoto.fit(
        4000,
        3000,
        maxWidth: 1080,
        maxHeight: 1440,
      );
      expect((photo.width, photo.height), (1080, 810));
    });

    testWidgets('of 12 × 34,000 pixels fills the width, its height capped, '
        'decoded small', (tester) async {
      tester.view.physicalSize = const Size(400, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final note = notes.capture(
        'Receipt',
        photos: [await png(tester, 12, 34000)],
      );
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          // Over every page, the full-screen one included.
          builder: (_, child) => PhotoScope(
            photos: FakePhotos(),
            bytes: notes.photoBytes,
            child: child!,
          ),
          home: Scaffold(
            body: ListView(
              children: [
                NoteCard(
                  note: note,
                  when: DateFormat.yMd('en').add_Hm(),
                  full: true,
                ),
              ],
            ),
          ),
        ),
      );
      RawImage raw() => tester.widget<RawImage>(find.byType(RawImage));
      for (var i = 0; i < 100 && raw().image == null; i++) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 20)),
        );
        await tester.pump();
      }

      expect(tester.takeException(), isNull);
      final image = raw().image;
      expect(image, isNotNull);
      expect(image!.height, lessThanOrEqualTo(NoteCard.photoHeight));
      expect(image.width, greaterThanOrEqualTo(1));
      final shown = tester.getSize(find.byType(RawImage));
      expect(shown.height, NoteCard.photoHeight);
      final card = tester.getSize(find.byType(Divider).first);
      expect(shown.width, closeTo(card.width, 1));

      // Full screen it is decoded no larger than a texture holds.
      await tap(tester, find.byType(NotePhoto));
      expect(find.byType(PhotoViewerScreen), findsOneWidget);
      RawImage viewed() => tester.widget<RawImage>(
        find.descendant(
          of: find.byType(PhotoViewerScreen),
          matching: find.byType(RawImage),
        ),
      );
      for (var i = 0; i < 100 && viewed().image == null; i++) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 20)),
        );
        await tester.pump();
      }
      expect(tester.takeException(), isNull);
      expect(viewed().image!.height, lessThanOrEqualTo(4096));
    });
  });
}

/// Counts how the Sections are read.
class _CountingNotes extends Notes {
  int atOnce = 0;
  int each = 0;

  _CountingNotes(super.store);

  @override
  Map<String, List<Section>> sectionsOf(Iterable<String> noteIds) {
    atOnce++;
    return super.sectionsOf(noteIds);
  }

  @override
  List<Section> sections(String noteId) {
    each++;
    return super.sections(noteId);
  }
}

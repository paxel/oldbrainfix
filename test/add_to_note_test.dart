import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

/// "Add to Note…" (#308): an Unsorted Note's card adds to another Note
/// as a Section, picked with a filter and the Notes added to last at the
/// top; a capture does so in the full editor (#352).
void main() {
  late Store store;
  late Places places;
  late Notes notes;
  late FakePhotos photos;
  late FakeVoice voice;
  var now = DateTime.utc(2026, 11, 15, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 15, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
    photos = FakePhotos(nextPhoto: onePixelPng);
    voice = FakeVoice(nextRecording: Uint8List.fromList([1, 2, 3]));
  });
  tearDown(() => store.close());

  void tick() => now = now.add(const Duration(hours: 1));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    // Tall enough for every card of Add to Note….
    tester.view.physicalSize = const Size(800, 3200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(store: store, photos: photos, voice: voice),
    );
    await tester.pumpAndSettle();
  }

  /// The card of the Note [text] in Add to Note…'s list, below the recent
  /// ones.
  Finder listed(String text) =>
      find.ancestor(of: find.text(text), matching: find.byType(NoteCard)).last;

  /// The Notes left Unsorted.
  List<String> unsorted() => [
    for (final note in notes.all())
      if (note.isUnsorted) note.text,
  ];

  testWidgets('the quick note has none: More opens the full editor, whose '
      'Merge into… merges the capture into the Note picked (#352)', (
    tester,
  ) async {
    final sauce = notes.capture('Sauce', place: places.addArea('Home').id);
    tick();
    await start(tester);
    await openTextCapture(tester);
    expect(find.byKey(const ValueKey('captureAddToNote')), findsNothing);
    await tester.enterText(find.byType(TextField), 'more garlic');
    // More waits for the text: built anew with it first.
    await tester.pump();
    await tap(tester, find.byKey(const ValueKey('captureToEditor')));
    final garlic = notes.all().singleWhere((n) => n.text == 'more garlic').id;
    await tap(tester, find.byTooltip('Merge into…'));
    await tap(tester, listed('Sauce'));

    expect(notes.byId(garlic)?.mergedInto, sauce.id);
    expect(unsorted(), isEmpty);
    expect(find.byType(CaptureScreen), findsNothing);
  });

  group('An Unsorted Note\'s card', () {
    Future<void> allNotes(WidgetTester tester) async {
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byTooltip('All Notes'));
    }

    Finder on(String text, String tooltip) => find.descendant(
      of: find.ancestor(of: find.text(text), matching: find.byType(NoteCard)),
      matching: find.byTooltip(tooltip),
    );

    testWidgets('offers Add to Note… in place of Merge into…, which merges '
        'it into the Note picked', (tester) async {
      final sauce = notes.capture('Sauce', place: places.addArea('Home').id);
      tick();
      final garlic = notes.capture('more garlic');
      await allNotes(tester);

      expect(on('more garlic', 'Add to Note…'), findsOneWidget);
      expect(on('more garlic', 'Merge into…'), findsNothing);
      // A filed Note's card keeps Merge into….
      expect(on('Sauce', 'Merge into…'), findsOneWidget);
      expect(on('Sauce', 'Add to Note…'), findsNothing);
      await tap(tester, on('more garlic', 'Add to Note…'));
      expect(find.text('Add to Note…'), findsOneWidget);
      await tap(tester, listed('Sauce'));

      // The core's Merge, as ever.
      expect(notes.byId(garlic.id)?.mergedInto, sauce.id);
      expect(unsorted(), isEmpty);
      expect(store.recentlyAddedTo, [sauce.id]);
    });

    testWidgets('names it so in its compact view\'s ⋮', (tester) async {
      notes.capture('Sauce', place: places.addArea('Home').id);
      tick();
      notes.capture('more garlic');
      // The list shows every card compact (#189).
      store.setLocalSetting('compactNotes:allNotes', 'true');
      await allNotes(tester);
      await tap(
        tester,
        find.descendant(
          of: find.ancestor(
            of: find.text('more garlic'),
            matching: find.byType(NoteCard),
          ),
          matching: find.byKey(const ValueKey('note-menu')),
        ),
      );

      expect(
        find.descendant(
          of: find.byKey(const ValueKey('note-merge')),
          matching: find.text('Add to Note…'),
        ),
        findsOneWidget,
      );
    });
  });
}

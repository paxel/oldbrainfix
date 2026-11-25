import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'card_menu.dart';
import 'editor_actions.dart';
import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

/// An Unsorted Note's card merges into another Note as a Section with
/// Merge into…, as every card does (obf-1014), picked with a filter and
/// the Notes added to last at the top (#308); a capture does so in the
/// full editor (#352).
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
    await tapEditorAction(tester, find.byKey(const ValueKey('editor-merge')));
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

    testWidgets('offers Merge into… as a filed Note\'s does, which merges '
        'it into the Note picked, the Notes added to last first', (
      tester,
    ) async {
      final sauce = notes.capture('Sauce', place: places.addArea('Home').id);
      tick();
      final garlic = notes.capture('more garlic');
      await allNotes(tester);

      expect(on('more garlic', 'Merge into…'), findsOneWidget);
      expect(on('more garlic', 'Add to Note…'), findsNothing);
      expect(on('Sauce', 'Merge into…'), findsOneWidget);
      await tap(tester, on('more garlic', 'Merge into…'));
      // One name for one action (obf-1014): the page is Merge into… too.
      expect(find.text('Merge into…'), findsOneWidget);
      expect(find.text('Add to Note…'), findsNothing);
      await tap(tester, listed('Sauce'));

      // The core's Merge, as ever.
      expect(notes.byId(garlic.id)?.mergedInto, sauce.id);
      expect(unsorted(), isEmpty);
      expect(store.recentlyAddedTo, [sauce.id]);
    });

    testWidgets('names it Merge into… in its compact view\'s ⋮ ▸ Tidy', (
      tester,
    ) async {
      notes.capture('Sauce', place: places.addArea('Home').id);
      tick();
      notes.capture('more garlic');
      // The list shows every card compact (#189).
      store.setLocalSetting('compactNotes:allNotes', 'true');
      await allNotes(tester);
      await openCardMenu(
        tester,
        'note-merge',
        menu: find.descendant(
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
          matching: find.text('Merge into…'),
        ),
        findsOneWidget,
      );
    });
  });
}

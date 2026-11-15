import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';

/// "Add to Note…" (#308): a capture, or an Unsorted Note's card, adds
/// to another Note as a Section, picked with a filter and the Notes
/// added to last at the top.
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

  final addToNote = find.byKey(const ValueKey('captureAddToNote'));

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

  /// The texts of the cards Add to Note… shows, recent ones included.
  List<String> cards() => [
    for (final card
        in find
            .descendant(
              of: find.byType(MergeIntoScreen),
              matching: find.byType(NoteCard),
            )
            .evaluate())
      (card.widget as NoteCard).note.text,
  ];

  /// The card of the Note [text] in Add to Note…'s list, below the recent
  /// ones.
  Finder listed(String text) =>
      find.ancestor(of: find.text(text), matching: find.byType(NoteCard)).last;

  /// The Notes left Unsorted.
  List<String> unsorted() => [
    for (final note in notes.all())
      if (note.isUnsorted) note.text,
  ];

  testWidgets('Capture adds the text to the Note picked as a Section, and '
      'nothing is left in Unsorted', (tester) async {
    final home = places.addArea('Household');
    final sauce = notes.capture('Sauce', place: home.id);
    tick();
    await start(tester);
    await openTextCapture(tester);
    // It waits while there is nothing to add.
    expect(tester.widget<TextButton>(addToNote).onPressed, isNull);
    await tester.enterText(find.byType(TextField), 'more garlic');
    await tester.pump();
    await tap(tester, addToNote);

    expect(find.byType(MergeIntoScreen), findsOneWidget);
    expect(find.text('Add to Note…'), findsOneWidget);
    expect(cards(), ['Sauce']);
    await tap(tester, listed('Sauce'));

    // At once: no Save, and Capture is gone.
    expect(find.byType(CaptureScreen), findsNothing);
    final section = notes.sections(sauce.id).single;
    expect(section.text, 'more garlic');
    expect(notes.all().map((n) => n.text), ['Sauce']);
    expect(unsorted(), isEmpty);
    // Where the Note is, so it is sealed for the same people.
    expect(notes.byId(section.id)?.place, home.id);
    expect(store.recentlyAddedTo, [sauce.id]);
  });

  testWidgets('the Notes added to last are at the top; the filter finds the '
      'rest and hides them', (tester) async {
    final pasta = notes.capture('Pasta');
    tick();
    final sauce = notes.capture('Sauce');
    tick();
    notes.capture('Salad');
    tick();
    store
      ..addedTo(sauce.id)
      ..addedTo(pasta.id);
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'basil');
    await tester.pump();
    await tap(tester, addToNote);

    expect(find.text('Added to recently'), findsOneWidget);
    // The latest first, then every Note, newest first.
    expect(cards(), ['Pasta', 'Sauce', 'Salad', 'Sauce', 'Pasta']);
    final recent = find.byKey(ValueKey('add-to-note-recent-${pasta.id}'));
    expect(
      tester.getTopLeft(recent).dy,
      lessThan(tester.getTopLeft(listed('Salad')).dy),
    );
    await tester.enterText(
      find.descendant(
        of: find.byType(MergeIntoScreen),
        matching: find.byType(TextField),
      ),
      'sau',
    );
    await tester.pumpAndSettle();
    expect(find.text('Added to recently'), findsNothing);
    expect(cards(), ['Sauce']);
    await tap(tester, listed('Sauce'));

    expect(notes.sections(sauce.id).single.text, 'basil');
    expect(store.recentlyAddedTo, [sauce.id, pasta.id]);
  });

  testWidgets('Back in Add to Note… leaves the capture as it was', (
    tester,
  ) async {
    notes.capture('Sauce');
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'more garlic');
    await tester.pump();
    await tap(tester, addToNote);
    await tap(tester, find.byType(BackButton));

    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('more garlic'), findsOneWidget);
    expect(notes.all().map((n) => n.text), ['Sauce']);
    expect(store.recentlyAddedTo, isEmpty);
  });

  testWidgets('a voice recording becomes a Section of the Note picked', (
    tester,
  ) async {
    final sauce = notes.capture('Sauce', place: places.addArea('Home').id);
    tick();
    await start(tester);
    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();
    await tap(tester, fanEntry('voice'));
    expect(voice.recording, isTrue);
    await tap(tester, find.byKey(const ValueKey('stopRecording')));
    await tap(tester, addToNote);
    await tap(tester, listed('Sauce'));

    final section = notes.sections(sauce.id).single;
    expect(section.recordings, hasLength(1));
    expect(notes.all().map((n) => n.id), [sauce.id]);
    expect(unsorted(), isEmpty);
  });

  testWidgets('a photo becomes a Section of the Note picked, its Nudge gone', (
    tester,
  ) async {
    final sauce = notes.capture('Sauce', place: places.addArea('Home').id);
    tick();
    await start(tester);
    await openTextCapture(tester);
    await tap(tester, find.byKey(const ValueKey('capturePhoto')));
    // The photo writes the Note at once, Unsorted until it is added.
    expect(unsorted(), ['']);
    await tester.enterText(find.byType(TextField), 'the label');
    await tester.pump();
    await tap(tester, addToNote);
    await tap(tester, listed('Sauce'));

    final section = notes.sections(sauce.id).single;
    expect(section.photos, hasLength(1));
    expect(section.text, 'the label');
    expect(notes.byId(section.id)?.place, sauce.place);
    expect(unsorted(), isEmpty);
  });

  testWidgets('a Note in a Shared Area gets the Section in that Area', (
    tester,
  ) async {
    final family = store.share(places.addArea('Family').id);
    final milk = notes.capture('Milk for the family', place: family);
    tick();
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'oat milk');
    await tester.pump();
    await tap(tester, addToNote);
    await tap(tester, listed('Milk for the family'));

    final section = notes.sections(milk.id).single;
    // Sealed under the Area's key, for everyone in it.
    expect(store.areaOf(section.id), family);
    expect(unsorted(), isEmpty);
  });

  testWidgets('a capture hidden from AI is offered no Note of a Shared Area '
      '(#267)', (tester) async {
    final family = store.share(places.addArea('Family').id);
    notes.capture('Milk for the family', place: family);
    notes.capture('Sauce', place: places.addArea('Home').id);
    tick();
    await start(tester);
    await openTextCapture(tester);
    await tap(tester, find.byKey(const ValueKey('capturePhoto')));
    // The photo wrote the capture; hidden, as by its Area.
    store.setHiddenFromAi(
      notes.all().firstWhere((n) => n.text.isEmpty).id,
      true,
    );
    await tap(tester, addToNote);

    expect(cards(), ['Sauce']);
  });

  testWidgets('Notes in the trash are not offered', (tester) async {
    final gone = notes.capture('Old sauce');
    notes.trash(gone.id);
    tick();
    notes.capture('Sauce');
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'more garlic');
    await tester.pump();
    await tap(tester, addToNote);

    expect(cards(), ['Sauce']);
  });

  testWidgets('not offered once a Frog is set up for the Note itself', (
    tester,
  ) async {
    store
      ..advanced = true
      ..setQuickNoteShows(QuickNoteControl.schedule, true);
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'eat it');
    await tester.pump();
    expect(addToNote, findsOneWidget);
    await tap(tester, find.byTooltip('Routine, Reminder, Frog or Session'));
    await tap(tester, find.text('Frog').last);

    expect(addToNote, findsNothing);
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

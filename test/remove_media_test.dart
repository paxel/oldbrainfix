import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/photo_views.dart';
import 'package:oldbrainfix/src/voice_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'card_menu.dart';
import 'fake_photos.dart';
import 'reveal.dart';

final talk = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);
final hum = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 2]);

void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        photos: FakePhotos(nextPhoto: onePixelPng),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Taps [finder] after scrolling it on screen: the card's ⋮ is longer
  /// than the test surface, and its menu scrolls.
  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
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

  /// A Note with a photo with a Photo Text, one without, a recording with
  /// a Transcript and one where no words were heard.
  Note mixed() {
    final captured = notes.capture(
      'doctor',
      photos: [onePixelPng, otherPng],
      recordings: [talk, hum],
    );
    final [spoken, silent] = captured.recordings;
    // The second photo is not read yet: no Photo Text at all.
    notes
      ..setPhotoText(captured.id, captured.photos.first, 'Room 12')
      ..setTranscript(captured.id, spoken, 'Bring the insurance card')
      ..setTranscript(captured.id, silent, '');
    return notes.byId(captured.id)!;
  }

  Finder inCard(Finder finder) =>
      find.descendant(of: find.byType(NoteCard), matching: finder);

  Finder inEditor(Finder finder) =>
      find.descendant(of: find.byType(NoteEditorScreen), matching: finder);

  /// Scrolls the editor back to its top, where the Note's text is.
  Future<void> top(WidgetTester tester) async {
    await tester.drag(find.byType(Scrollable).first, const Offset(0, 5000));
    await tester.pumpAndSettle();
  }

  group('the card\'s ⋮', () {
    testWidgets('offers only photos and recordings with a text', (
      tester,
    ) async {
      final note = mixed();
      await start(tester);
      await tap(tester, find.text('1 Unsorted'));
      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await openCardGroup(tester, 'note-remove-');

      final [read, blank] = note.photos;
      final [spoken, silent] = note.recordings;
      expect(find.byKey(ValueKey('note-remove-$read')), findsOneWidget);
      expect(find.byKey(ValueKey('note-remove-$spoken')), findsOneWidget);
      expect(find.byKey(ValueKey('note-remove-$blank')), findsNothing);
      expect(find.byKey(ValueKey('note-remove-$silent')), findsNothing);
      // Each entry says which one it is by the start of its text.
      expect(find.text('Remove photo, keep text'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(PopupMenuItem<VoidCallback>),
          matching: find.text('Room 12'),
        ),
        findsOneWidget,
      );
      expect(find.text('Remove recording, keep text'), findsOneWidget);
    });

    testWidgets('removes the photo after asking; its words go into the '
        'Note\'s text, on the card and in the editor', (tester) async {
      final note = mixed();
      final read = note.photos.first;
      await start(tester);
      await tap(tester, find.text('1 Unsorted'));
      expect(inCard(find.byType(NotePhoto)), findsNWidgets(2));
      // The detailed view shows the Photo Text under its photo.
      expect(inCard(find.text('Room 12')), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await openCardGroup(tester, 'note-remove-');
      await tap(tester, find.byKey(ValueKey('note-remove-$read')));

      expect(
        find.text('Remove this photo and keep its words in the text?'),
        findsOneWidget,
      );
      // Filled on the error colour, as every step that removes (obf-1081).
      final confirm = tester.widget<Material>(
        find.descendant(
          of: find.byKey(const ValueKey('remove-media-confirm')),
          matching: find.byType(Material),
        ),
      );
      final context = tester.element(find.byType(AlertDialog));
      expect(confirm.color, Theme.of(context).colorScheme.error);
      await tap(tester, find.text('Remove'));

      final after = notes.byId(note.id)!;
      expect(after.photos, [note.photos.last]);
      expect(after.photoTexts, isEmpty);
      expect(after.text, 'doctor\n\nRoom 12');
      expect(inCard(find.byType(NotePhoto)), findsOneWidget);
      final text = inCard(find.textContaining('Room 12', findRichText: true));
      expect(text, findsOneWidget);

      // The card got shorter: its first line may sit above the screen.
      await reveal(tester, text);
      await tester.longPress(text);
      await tester.pumpAndSettle();
      expect(inEditor(find.byType(NotePhoto)), findsOneWidget);
      expect(inEditor(find.text('doctor\n\nRoom 12')), findsOneWidget);
    });

    testWidgets('search finds the Note by the words of a removed photo', (
      tester,
    ) async {
      final note = mixed();
      notes
        ..removePhoto(note.id, note.photos.first)
        ..capture('call the garage');
      await start(tester);
      await tap(tester, find.byKey(const ValueKey('browse')));
      await tester.enterText(
        find.descendant(
          of: find.byType(AreasScreen),
          matching: find.byType(TextField),
        ),
        'room 12',
      );
      await tester.pump(BrowserSearchField.pause);
      await tester.pumpAndSettle();

      expect(find.byType(NoteCard), findsOneWidget);
      // In the Note's text now, and among the places found (obf-1047).
      expect(
        inCard(find.textContaining('Room 12', findRichText: true)),
        findsWidgets,
      );
    });

    testWidgets('Cancel keeps the recording', (tester) async {
      final note = mixed();
      final spoken = note.recordings.first;
      await start(tester);
      await tap(tester, find.text('1 Unsorted'));
      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await openCardGroup(tester, 'note-remove-');
      await tap(tester, find.byKey(ValueKey('note-remove-$spoken')));
      expect(
        find.text('Remove this recording and keep its words in the text?'),
        findsOneWidget,
      );
      await tap(tester, find.text('Cancel'));

      expect(notes.byId(note.id)!.recordings, note.recordings);
      expect(inCard(find.byType(RecordingButton)), findsNWidgets(2));
      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await openCardGroup(tester, 'note-remove-');
      expect(find.byKey(ValueKey('note-remove-$spoken')), findsOneWidget);
    });
  });

  group('the editor', () {
    testWidgets('has the button only under a recording with a Transcript; '
        'removed, its words show in the text field and stay there', (
      tester,
    ) async {
      final note = mixed();
      final [spoken, silent] = note.recordings;
      await start(tester);
      await edit(tester, note.id);

      final button = find.byKey(ValueKey('remove:$spoken'));
      await reveal(tester, button);
      // It removes: filled on the error colour (obf-1107).
      expect(
        tester
            .widget<Material>(
              find
                  .descendant(of: button, matching: find.byType(Material))
                  .first,
            )
            .color,
        Theme.of(tester.element(button)).colorScheme.error,
      );
      expect(
        find.byKey(ValueKey('remove:$silent'), skipOffstage: false),
        findsNothing,
      );
      Finder recording(String hash) => find.byWidgetPredicate(
        (w) => w is RecordingButton && w.hash == hash,
        // Just above the button, maybe scrolled out of view.
        skipOffstage: false,
      );
      expect(recording(spoken), findsOneWidget);
      await tap(tester, button);
      await tap(tester, find.byKey(const ValueKey('remove-media-confirm')));

      const words = 'doctor\n\nBring the insurance card';
      final after = notes.byId(note.id)!;
      expect(after.text, words);
      expect(after.transcripts.keys, [silent]);
      expect(recording(spoken), findsNothing);
      expect(
        find.byKey(ValueKey('remove:$spoken'), skipOffstage: false),
        findsNothing,
      );
      // The page grew shorter: from the top again.
      await top(tester);
      expect(inEditor(find.text(words)), findsOneWidget);

      // Leaving saves the field, the words in it.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)!.text, words);
      await tap(tester, find.text('1 Unsorted'));
      expect(inCard(find.byType(RecordingButton)), findsOneWidget);
      expect(
        inCard(find.textContaining('Bring the insurance', findRichText: true)),
        findsOneWidget,
      );
    });

    testWidgets('Undo takes the words out of the text again; the recording '
        'stays removed', (tester) async {
      final note = mixed();
      final spoken = note.recordings.first;
      await start(tester);
      await edit(tester, note.id);
      await reveal(tester, find.byKey(ValueKey('remove:$spoken')));
      await tap(tester, find.byKey(ValueKey('remove:$spoken')));
      await tap(tester, find.byKey(const ValueKey('remove-media-confirm')));
      expect(notes.byId(note.id)!.text, 'doctor\n\nBring the insurance card');

      await tap(tester, find.byKey(const ValueKey('editor-undo')));
      expect(notes.byId(note.id)!.text, 'doctor');
      await top(tester);
      expect(inEditor(find.text('doctor')), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('editor-redo')));
      expect(notes.byId(note.id)!.text, 'doctor\n\nBring the insurance card');
      expect(notes.byId(note.id)!.recordings, [note.recordings.last]);
    });

    testWidgets('Cancel keeps the photo', (tester) async {
      final note = mixed();
      final read = note.photos.first;
      await start(tester);
      await edit(tester, note.id);
      await reveal(tester, find.byKey(ValueKey('remove:$read')));
      await tap(tester, find.byKey(ValueKey('remove:$read')));
      await tap(tester, find.text('Cancel'));

      expect(notes.byId(note.id)!.photos, note.photos);
      expect(find.byKey(ValueKey('remove:$read')), findsOneWidget);
    });

    testWidgets('a merged Note\'s photo goes from its Section; its words '
        'become the Section\'s text and stay', (tester) async {
      final into = notes.capture('sauce');
      now = now.add(const Duration(minutes: 5));
      final merged = notes.capture('', photos: [onePixelPng]);
      final hash = merged.photos.single;
      notes
        ..setPhotoText(merged.id, hash, 'Tomatoes 400 g')
        ..merge(merged.id, into.id);
      await start(tester);
      await edit(tester, into.id);

      await reveal(tester, find.byKey(ValueKey('remove:$hash')));
      await tap(tester, find.byKey(ValueKey('remove:$hash')));
      await tap(tester, find.byKey(const ValueKey('remove-media-confirm')));

      final section = notes.sections(into.id).single;
      expect(section.photos, isEmpty);
      expect(section.photoTexts, isEmpty);
      expect(section.text, 'Tomatoes 400 g');
      expect(inEditor(find.byType(NotePhoto)), findsNothing);
      // The Section's field shows them, or leaving would write '' back.
      await tester.scrollUntilVisible(
        inEditor(find.text('Tomatoes 400 g')),
        -100,
        scrollable: find.byType(Scrollable).first,
      );
      expect(inEditor(find.text('Tomatoes 400 g')), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.sections(into.id).single.text, 'Tomatoes 400 g');
      expect(notes.byId(into.id)!.text, 'sauce');
    });

    testWidgets('German fits on a narrow phone', (tester) async {
      tester.view.physicalSize = const Size(1080, 2340);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      final note = mixed();
      final [read, _] = note.photos;
      await start(tester);
      await edit(tester, note.id);

      await reveal(tester, find.byKey(ValueKey('remove:$read')));
      expect(find.text('Foto entfernen, Text behalten'), findsOneWidget);
      await tap(tester, find.byKey(ValueKey('remove:$read')));
      expect(
        find.text('Dieses Foto entfernen und seine Worte im Text behalten?'),
        findsOneWidget,
      );
      expect(find.text('Entfernen'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tap(tester, find.text('Abbrechen'));

      final spoken = note.recordings.first;
      await reveal(tester, find.byKey(ValueKey('remove:$spoken')));
      expect(find.text('Aufnahme entfernen, Text behalten'), findsOneWidget);
      expect(tester.takeException(), isNull);

      // The card's ⋮ holds both, each with the start of its text.
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      await tap(tester, find.text('1 unsortiert'));
      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await openCardGroup(tester, 'note-remove-');
      await reveal(tester, find.text('Aufnahme entfernen, Text behalten'));
      expect(find.text('Foto entfernen, Text behalten'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(ValueKey('note-remove-$spoken')),
          matching: find.text('Bring the insurance card'),
        ),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    });
  });
}

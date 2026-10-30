import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_ocr.dart';
import 'fake_photos.dart';
import 'fake_speech.dart';
import 'reveal.dart';

final talk = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);
final hum = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 2]);
final mumble = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 3]);

/// A text in a format this version does not know.
const _future = '{"format":"x-future","text":"# Shopping"}';

void main() {
  late Store store;
  late Notes notes;
  late FakeSpeech speech;
  late FakeOcr ocr;

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 10, 28, 8))
      ..starterSetOffered = true;
    notes = Notes(store);
    speech = FakeSpeech(installed: WhisperModel.base.sha256);
    ocr = FakeOcr();
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    // Tall enough that the editor builds all of a Note at once, so what
    // is not found is not there.
    tester.view
      ..physicalSize = const Size(1080, 9000)
      ..devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        photos: FakePhotos(nextPhoto: onePixelPng),
        speech: speech,
        ocr: ocr,
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Scrolls the editor back to its top, then down to [finder]: a
  /// change may have moved it above the screen.
  Future<void> seek(WidgetTester tester, Finder finder) async {
    await tester.drag(find.byType(Scrollable).first, const Offset(0, 5000));
    await tester.pumpAndSettle();
    await reveal(tester, finder);
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await seek(tester, finder);
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

  /// A Note with [text], a photo with a Photo Text, a recording with a
  /// Transcript and one where no words were heard.
  Note mixed([String text = 'doctor']) {
    final captured = notes.capture(
      text,
      photos: [onePixelPng],
      recordings: [talk, hum],
    );
    final [spoken, silent] = captured.recordings;
    notes
      ..setPhotoText(captured.id, captured.photos.single, 'Room 12')
      ..setTranscript(captured.id, spoken, 'Bring the insurance card')
      ..setTranscript(captured.id, silent, '');
    return notes.byId(captured.id)!;
  }

  Finder key(String name, String hash) => find.byKey(ValueKey('$name:$hash'));

  /// The Note's text field as it shows, scrolled back into view.
  Future<String> field(WidgetTester tester) async {
    await tester.drag(find.byType(Scrollable).first, const Offset(0, 5000));
    await tester.pumpAndSettle();
    return tester
        .widget<TextField>(find.byType(TextField).first)
        .controller!
        .text;
  }

  group('in the Note editor', () {
    testWidgets('Replace makes the Transcript the Note text', (tester) async {
      final note = mixed();
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, key('replace-text', note.recordings.first));

      expect(notes.byId(note.id)!.text, 'Bring the insurance card');
      expect(await field(tester), 'Bring the insurance card');
      // Leaving writes nothing more.
      final entries = store.history(note.id).length;
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(store.history(note.id).length, entries);
    });

    testWidgets('Append adds the Photo Text on a line of its own', (
      tester,
    ) async {
      final note = mixed('doctor  ');
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, key('append-text', note.photos.single));

      expect(notes.byId(note.id)!.text, 'doctor\nRoom 12');
      expect(await field(tester), 'doctor\nRoom 12');
    });

    testWidgets('Append into a formatted Note writes the Transcript '
        'escaped, so it reads as itself', (tester) async {
      final captured = notes.capture(
        '**doctor**',
        format: TextFormats.markdown,
        recordings: [talk],
      );
      notes.setTranscript(
        captured.id,
        captured.recordings.single,
        'Bring *2* cards',
      );
      await start(tester);
      await edit(tester, captured.id);
      await tap(tester, key('append-text', captured.recordings.single));

      final note = notes.byId(captured.id)!;
      expect(note.text, '**doctor**\nBring \\*2\\* cards');
      expect(note.textFormat, TextFormats.markdown);
      expect(note.visibleText, 'doctor\nBring *2* cards');
      expect(await field(tester), '**doctor**\nBring \\*2\\* cards');
    });

    testWidgets('Append saves what was typed first', (tester) async {
      final note = mixed();
      await start(tester);
      await edit(tester, note.id);
      await tester.enterText(find.byType(TextField).first, 'dentist');
      await tap(tester, key('append-text', note.recordings.first));

      expect(notes.byId(note.id)!.text, 'dentist\nBring the insurance card');
    });

    testWidgets('Append to a Note without text gives it the Transcript', (
      tester,
    ) async {
      final note = mixed('');
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, key('append-text', note.recordings.first));

      expect(notes.byId(note.id)!.text, 'Bring the insurance card');
    });

    testWidgets('no Replace or Append where no words were heard', (
      tester,
    ) async {
      final note = mixed();
      await start(tester);
      await edit(tester, note.id);
      final silent = note.recordings.last;
      await seek(tester, find.text('No words heard.'));
      expect(key('replace-text', silent), findsNothing);
      expect(key('append-text', silent), findsNothing);
      expect(key('remove-text', silent), findsNothing);
      expect(key('again', silent), findsOneWidget);
    });

    testWidgets('no Replace or Append for a text in an unknown format', (
      tester,
    ) async {
      final note = mixed();
      store.append(note.id, NoteKeys.text, _future);
      await start(tester);
      await edit(tester, note.id);
      final spoken = note.recordings.first;
      await seek(tester, key('remove-text', spoken));
      expect(key('replace-text', spoken), findsNothing);
      expect(key('append-text', spoken), findsNothing);
      expect(key('replace-text', note.photos.single), findsNothing);
    });

    testWidgets('Remove asks once, removes the Transcript and keeps the '
        'recording; only "again" stays', (tester) async {
      final note = mixed();
      final spoken = note.recordings.first;
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, key('remove-text', spoken));

      expect(find.text('Remove this Transcript?'), findsOneWidget);
      final confirm = tester.widget<TextButton>(
        find.byKey(const ValueKey('remove-media-confirm')),
      );
      final context = tester.element(find.byType(AlertDialog));
      expect(
        confirm.style?.foregroundColor?.resolve({}),
        Theme.of(context).colorScheme.error,
      );
      await tester.tap(find.text('Remove'));
      await tester.pumpAndSettle();

      final after = notes.byId(note.id)!;
      expect(after.recordings, note.recordings);
      expect(after.removedTranscripts, [spoken]);
      expect(find.text('Bring the insurance card'), findsNothing);
      await seek(tester, key('again', spoken));
      // The silent one still says so; the removed one says nothing.
      await seek(tester, find.text('No words heard.'));
      expect(find.text('No words heard.'), findsOneWidget);
      expect(key('replace-text', spoken), findsNothing);
      expect(key('append-text', spoken), findsNothing);
      expect(key('remove-text', spoken), findsNothing);
      expect(key('again', spoken), findsOneWidget);
    });

    testWidgets('Cancel keeps the Photo Text', (tester) async {
      final note = mixed();
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, key('remove-text', note.photos.single));
      expect(find.text('Remove this Photo Text?'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(notes.byId(note.id)!.photoTexts, {note.photos.single: 'Room 12'});
    });

    testWidgets('Transcribe again brings a removed Transcript back', (
      tester,
    ) async {
      final note = mixed();
      final spoken = note.recordings.first;
      notes.removeTranscript(note.id, spoken);
      speech.transcripts[1] = 'Bring the card';
      await start(tester);
      expect(speech.transcribed, isEmpty, reason: 'not on its own');
      await edit(tester, note.id);
      expect(find.text('Bring the insurance card'), findsNothing);
      await tap(tester, key('again', spoken));

      expect(speech.transcribed, [talk]);
      final after = notes.byId(note.id)!;
      expect(after.transcripts[spoken], 'Bring the card');
      expect(after.removedTranscripts, isEmpty);
      expect(find.text('Bring the card'), findsOneWidget);
      expect(key('replace-text', spoken), findsOneWidget);
    });

    testWidgets('Transcribe again replaces a Transcript and shows it is '
        'running meanwhile', (tester) async {
      final note = mixed();
      final spoken = note.recordings.first;
      speech
        ..transcripts[1] = 'Bring the card'
        ..transcribeGate = Completer<void>();
      await start(tester);
      await edit(tester, note.id);
      // The button spins meanwhile: nothing settles.
      await seek(tester, key('again', spoken));
      await tester.tap(key('again', spoken));
      await tester.pump();
      await tester.pump();
      expect(find.text('Transcribing…'), findsOneWidget);
      expect(tester.widget<TextButton>(key('again', spoken)).onPressed, isNull);

      speech.transcribeGate!.complete();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)!.transcripts[spoken], 'Bring the card');
      expect(find.text('Bring the card'), findsOneWidget);
      expect(find.text('Bring the insurance card'), findsNothing);
    });

    testWidgets('Transcribe again tries a recording that failed before', (
      tester,
    ) async {
      final note = notes.capture('call', recordings: [mumble]);
      final hash = note.recordings.single;
      await start(tester);
      expect(speech.transcribed, [mumble]);
      expect(notes.byId(note.id)!.transcripts, isEmpty);

      speech.transcripts[3] = 'Call Ann';
      await edit(tester, note.id);
      await tap(tester, key('again', hash));
      expect(speech.transcribed, [mumble, mumble]);
      expect(notes.byId(note.id)!.transcripts[hash], 'Call Ann');
    });

    testWidgets('a failed Transcribe again is announced at the top', (
      tester,
    ) async {
      final note = mixed();
      speech.transcripts.remove(1);
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, key('again', note.recordings.first));

      expect(find.text('A recording could not be transcribed.'), findsOne);
      expect(
        notes.byId(note.id)!.transcripts[note.recordings.first],
        'Bring the insurance card',
      );
      await tester.tap(find.text('A recording could not be transcribed.'));
      await tester.pumpAndSettle();
      expect(find.text('Recording not transcribed'), findsOneWidget);
    });

    testWidgets('no Transcribe again without a speech model', (tester) async {
      speech.installed = null;
      final note = mixed();
      await start(tester);
      await edit(tester, note.id);
      await seek(tester, key('again', note.photos.single));
      await seek(tester, key('remove-text', note.recordings.first));
      expect(key('again', note.recordings.first), findsNothing);
    });

    testWidgets('Read the text again brings a removed Photo Text back', (
      tester,
    ) async {
      final note = mixed();
      final photo = note.photos.single;
      notes.removePhotoText(note.id, photo);
      ocr.texts[onePixelPng.length] = 'Room 14';
      await start(tester);
      expect(ocr.readPhotos, isEmpty, reason: 'not on its own');
      await edit(tester, note.id);
      expect(find.text('Room 12'), findsNothing);
      expect(key('replace-text', photo), findsNothing);
      await tap(tester, key('again', photo));

      final after = notes.byId(note.id)!;
      expect(after.photoTexts[photo], 'Room 14');
      expect(after.removedPhotoTexts, isEmpty);
      expect(find.text('Room 14'), findsOneWidget);
    });

    testWidgets('the text of a removed recording can still be taken over, '
        'not removed or made again', (tester) async {
      final note = mixed();
      final spoken = note.recordings.first;
      notes.removeRecording(note.id, spoken);
      await start(tester);
      await edit(tester, note.id);
      await seek(tester, key('replace-text', spoken));
      expect(key('remove-text', spoken), findsNothing);
      expect(key('again', spoken), findsNothing);
      await tap(tester, key('append-text', spoken));
      expect(notes.byId(note.id)!.text, 'doctor\nBring the insurance card');
    });
  });

  group('in a Section', () {
    /// "doctor" merged into "Health": its texts show in a Section.
    Future<(Note, String)> merged(WidgetTester tester) async {
      final note = mixed();
      final into = notes.capture('Health');
      notes.merge(note.id, into.id);
      await start(tester);
      await edit(tester, into.id);
      return (note, into.id);
    }

    testWidgets('Replace and Append change the Section\'s text', (
      tester,
    ) async {
      final (note, into) = await merged(tester);
      await tap(tester, key('replace-text', note.recordings.first));
      expect(notes.byId(note.id)!.text, 'Bring the insurance card');
      await tap(tester, key('append-text', note.photos.single));
      expect(notes.byId(note.id)!.text, 'Bring the insurance card\nRoom 12');
      expect(notes.byId(into)!.text, 'Health');
      expect(await field(tester), 'Health');
    });

    testWidgets('Remove and Transcribe again work on the Section', (
      tester,
    ) async {
      final (note, _) = await merged(tester);
      final spoken = note.recordings.first;
      await tap(tester, key('remove-text', spoken));
      await tester.tap(find.text('Remove'));
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)!.removedTranscripts, [spoken]);
      expect(key('replace-text', spoken), findsNothing);

      speech.transcripts[1] = 'Bring the card';
      await tap(tester, key('again', spoken));
      expect(notes.byId(note.id)!.transcripts[spoken], 'Bring the card');
      expect(key('replace-text', spoken), findsOneWidget);
    });
  });
}

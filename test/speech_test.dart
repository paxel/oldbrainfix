import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/speech_model_screen.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_speech.dart';
import 'fake_voice.dart';

/// Stand-ins for two recordings, told apart by their last byte.
final first = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);
final second = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 2]);

const wrongHash =
    '0000000000000000000000000000000000000000000000000000000000000000';

void main() {
  late Store store;
  late Notes notes;
  late FakeSpeech speech;
  late FakeVoice voice;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    speech = FakeSpeech();
    voice = FakeVoice(nextRecording: first);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester, {SpeechPlatform? platform}) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, voice: voice, speech: platform ?? speech),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Taps [finder] while a progress bar may run, which never settles.
  Future<void> press(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pump();
    await tester.pump();
  }

  final modelButton = find.byIcon(Icons.record_voice_over_outlined);

  /// Opens the speech model page from the notes browser.
  Future<void> openModel(WidgetTester tester) async {
    await tap(tester, find.byIcon(Icons.search));
    expect(find.byType(AreasScreen), findsOneWidget);
    await tap(tester, modelButton);
    expect(find.byType(SpeechModelScreen), findsOneWidget);
  }

  final download = find.text('Download (148 MB)');
  final pick = find.text('Use a file of my own');
  final ready = find.text(
    'The speech model is ready. Voice Notes are transcribed on this phone.',
  );

  group('the speech model page', () {
    testWidgets('is in the notes browser where there is a speech layer', (
      tester,
    ) async {
      await start(tester, platform: const NoSpeech());
      await tap(tester, find.byIcon(Icons.search));
      expect(modelButton, findsNothing);
    });

    testWidgets('tells the size and where the model comes from', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      expect(find.textContaining('about 148 MB'), findsOneWidget);
      expect(
        find.textContaining('only from Old Brain Fix\'s own release on GitHub'),
        findsOneWidget,
      );
      expect(download, findsOneWidget);
      expect(pick, findsOneWidget);
      // Nothing is fetched without asking.
      expect(speech.downloads, isEmpty);
    });

    testWidgets('downloads from the project\'s release with progress', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await press(tester, download);

      expect(speech.downloads, [WhisperModel.url]);
      expect(
        WhisperModel.url.toString(),
        'https://github.com/paxel/oldbrainfix/releases/download/'
        'whisper-model-v1/ggml-base.bin',
      );
      speech.report(50 * 1000 * 1000, 147951465);
      await tester.pump();
      expect(find.text('Downloading: 50 of 148 MB'), findsOneWidget);
      final bar = tester.widget<LinearProgressIndicator>(
        find.byType(LinearProgressIndicator),
      );
      expect(bar.value, closeTo(50e6 / 147951465, 1e-9));

      speech.bring(WhisperModel.sha256);
      await tester.pumpAndSettle();
      expect(ready, findsOneWidget);
      expect(speech.installed, WhisperModel.sha256);
    });

    testWidgets('a download with the wrong checksum is rejected', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await press(tester, download);
      speech.bring(wrongHash);
      await tester.pumpAndSettle();

      expect(
        find.text('That is not the expected speech model, so it was not kept.'),
        findsOneWidget,
      );
      expect(speech.installed, isNull);
      expect(speech.side, isNull);
      // Once at the start, once for the wrong model.
      expect(speech.discards, 2);
      expect(download, findsOneWidget);
    });

    testWidgets('a download that breaks off says so and keeps nothing', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await press(tester, download);
      speech.bring(null);
      await tester.pumpAndSettle();

      expect(
        find.text(
          'The download did not finish, and nothing was kept. '
          'Try again later.',
        ),
        findsOneWidget,
      );
      expect(speech.installed, isNull);
      expect(download, findsOneWidget);
    });

    testWidgets('a cancelled download keeps nothing and says nothing', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await press(tester, download);
      await press(tester, find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(speech.cancels, 1);
      expect(speech.installed, isNull);
      expect(find.textContaining('did not finish'), findsNothing);
      expect(download, findsOneWidget);
    });

    testWidgets('a picked file is kept when it is the pinned model', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await press(tester, pick);
      expect(speech.picks, 1);
      speech.bring(WhisperModel.sha256);
      await tester.pumpAndSettle();

      expect(ready, findsOneWidget);
      expect(speech.installed, WhisperModel.sha256);
      expect(speech.downloads, isEmpty);
    });

    testWidgets('a picked file that is not the pinned model is rejected', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await press(tester, pick);
      speech.bring(wrongHash);
      await tester.pumpAndSettle();

      expect(find.textContaining('not the expected'), findsOneWidget);
      expect(speech.installed, isNull);
    });

    testWidgets('picking no file changes nothing', (tester) async {
      await start(tester);
      await openModel(tester);
      await press(tester, pick);
      speech.bring(null);
      await tester.pumpAndSettle();

      expect(find.textContaining('did not finish'), findsNothing);
      expect(pick, findsOneWidget);
    });

    testWidgets('a model in place that is not the pinned one is dropped', (
      tester,
    ) async {
      speech.installed = wrongHash;
      notes.capture('', recordings: [first]);
      speech.transcripts[1] = 'never';
      await start(tester);

      expect(speech.removes, 1);
      expect(speech.transcribed, isEmpty);
      await openModel(tester);
      expect(download, findsOneWidget);
    });

    testWidgets('a fetch cut off when the app was closed is dropped', (
      tester,
    ) async {
      speech.side = WhisperModel.sha256;
      await start(tester);
      expect(speech.side, isNull);
      expect(speech.installed, isNull);
    });

    testWidgets('says so where Whisper does not run', (tester) async {
      speech.runs = false;
      await start(tester);
      await openModel(tester);
      expect(
        find.text(
          'Transcripts are not available on this device. '
          'Voice recordings are kept as they are.',
        ),
        findsOneWidget,
      );
      expect(download, findsNothing);
    });
  });

  group('Transcripts', () {
    testWidgets('nothing is transcribed without the model', (tester) async {
      notes.capture('', recordings: [first]);
      speech.transcripts[1] = 'buy more sauce';
      await start(tester);
      expect(speech.transcribed, isEmpty);
      expect(notes.all().single.transcripts, isEmpty);
    });

    testWidgets('recordings from before the model are transcribed once it '
        'comes', (tester) async {
      final note = notes.capture('', recordings: [first]);
      speech.transcripts[1] = 'buy more sauce';
      await start(tester);
      await openModel(tester);
      await press(tester, download);
      speech.bring(WhisperModel.sha256);
      await tester.pumpAndSettle();

      expect(speech.transcribed, [first]);
      final read = notes.byId(note.id);
      expect(read?.transcripts, {read?.recordings.single: 'buy more sauce'});
      // The recording stays.
      expect(notes.recordingBytes(read?.recordings.single ?? ''), first);
      expect(speech.idles, greaterThan(0));
    });

    testWidgets('a new voice Note is transcribed after it is saved, without '
        'holding Capture up', (tester) async {
      speech
        ..installed = WhisperModel.sha256
        ..transcripts[1] = 'call the garage'
        ..transcribeGate = Completer<void>();
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      await tap(tester, find.byIcon(Icons.mic_none));
      await tap(tester, find.byKey(const ValueKey('stopRecording')));

      // Saved and back on the Agenda while Whisper still works.
      expect(find.byType(CaptureScreen), findsNothing);
      expect(speech.transcribed, [first]);
      final card = find.byType(NoteCard);
      expect(
        find.descendant(of: card, matching: find.text('Transcribing…')),
        findsOneWidget,
      );

      speech.transcribeGate?.complete();
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: card, matching: find.text('call the garage')),
        findsOneWidget,
      );
      expect(find.text('Transcribing…'), findsNothing);
    });

    testWidgets('a recording that cannot be transcribed is not tried again '
        'until the next start', (tester) async {
      speech.installed = WhisperModel.sha256;
      final note = notes.capture('', recordings: [first]);
      await start(tester);
      expect(speech.transcribed, [first]);
      expect(notes.byId(note.id)?.transcripts, isEmpty);

      notes.edit(note.id, 'something else changes the log');
      await tester.pumpAndSettle();
      expect(speech.transcribed, [first]);
      expect(notes.byId(note.id)?.recordings, hasLength(1));
    });

    testWidgets('silence is kept as "no words heard"', (tester) async {
      speech
        ..installed = WhisperModel.sha256
        ..transcripts[1] = '';
      final note = notes.capture('', recordings: [first]);
      await start(tester);
      expect(notes.byId(note.id)?.transcripts.values, ['']);
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), notes.byId(note.id)!));
      await tester.pumpAndSettle();
      expect(find.text('No words heard.'), findsOneWidget);
    });

    testWidgets('the editor shows each recording\'s Transcript, a merged '
        'Note\'s too', (tester) async {
      speech
        ..installed = WhisperModel.sha256
        ..transcripts[1] = 'sauce from the shop'
        ..transcripts[2] = 'and fresh basil';
      final into = notes.capture('shopping', recordings: [first]);
      now = now.add(const Duration(minutes: 5));
      final merged = notes.capture('', recordings: [second]);
      notes.merge(merged.id, into.id);
      await start(tester);

      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), notes.byId(into.id)!));
      await tester.pumpAndSettle();
      final editor = find.byType(NoteEditorScreen);
      expect(
        find.descendant(of: editor, matching: find.text('sauce from the shop')),
        findsOneWidget,
      );
      await tester.scrollUntilVisible(
        find.text('and fresh basil'),
        100,
        scrollable: find
            .descendant(of: editor, matching: find.byType(Scrollable))
            .first,
      );
      expect(find.text('and fresh basil'), findsOneWidget);
    });

    testWidgets('the editor shows a Transcript written while it is open, '
        'further down than it had built', (tester) async {
      speech
        ..installed = WhisperModel.sha256
        ..transcripts[1] = 'the garage closes at six'
        ..transcribeGate = Completer<void>();
      // A long text pushes the recording far below the screen.
      final note = notes.capture(
        List.filled(80, 'car').join('\n'),
        recordings: [first],
      );
      await start(tester);
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), notes.byId(note.id)!));
      await tester.pumpAndSettle();
      final editor = find.byType(NoteEditorScreen);
      expect(editor, findsOneWidget);
      expect(find.byType(TranscriptText, skipOffstage: false), findsNothing);

      speech.transcribeGate?.complete();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)?.transcripts.values, [
        'the garage closes at six',
      ]);
      await tester.scrollUntilVisible(
        find.text('the garage closes at six'),
        200,
        scrollable: find
            .descendant(of: editor, matching: find.byType(Scrollable))
            .first,
      );
      expect(find.text('the garage closes at six'), findsOneWidget);
    });

    testWidgets('the notes browser finds a Note by its Transcript', (
      tester,
    ) async {
      speech
        ..installed = WhisperModel.sha256
        ..transcripts[1] = 'buy more sauce';
      notes.capture('', recordings: [first]);
      notes.capture('call the garage');
      await start(tester);
      await tap(tester, find.byIcon(Icons.search));
      await tester.enterText(
        find.descendant(
          of: find.byType(AreasScreen),
          matching: find.byType(TextField),
        ),
        'SAUCE',
      );
      await tester.pumpAndSettle();

      final card = find.byType(NoteCard);
      expect(card, findsOneWidget);
      expect(
        find.descendant(of: card, matching: find.text('buy more sauce')),
        findsOneWidget,
      );
    });
  });
}

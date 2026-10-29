import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/speech_model_screen.dart';
import 'package:oldbrainfix/src/speech_offer_screen.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_speech.dart';
import 'fake_voice.dart';
import 'open_settings.dart';
import 'open_capture.dart';

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

  final modelButton = find.text('Speech model');

  /// Opens the speech model page from Settings.
  Future<void> openModel(WidgetTester tester) async {
    await openDeviceSettings(tester);
    await tester.ensureVisible(modelButton);
    await tap(tester, modelButton);
    expect(find.byType(SpeechModelScreen), findsOneWidget);
  }

  final download = find.text('Download (148 MB)');
  final base = find.byKey(const ValueKey('speech-model-base'));
  final small = find.byKey(const ValueKey('speech-model-small'));
  final missing = find.text('No speech model yet');

  /// Chooses the model option [finder] on the speech model page, which
  /// starts its download.
  Future<void> choose(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await press(tester, finder);
  }

  /// Whether the option [finder] is the chosen one.
  bool chosen(WidgetTester tester, Finder finder) =>
      tester
          .widget<RadioGroup<WhisperModel?>>(
            find.byType(RadioGroup<WhisperModel?>),
          )
          .groupValue ==
      tester.widget<RadioListTile<WhisperModel?>>(finder).value;
  final pick = find.text('Use a file of my own');
  final ready = find.text(
    'The speech model is ready. Voice Notes are transcribed on this phone.',
  );

  group('the speech model page', () {
    testWidgets('is in Settings where there is a speech layer', (tester) async {
      await start(tester, platform: const NoSpeech());
      await openDeviceSettings(tester);
      expect(modelButton, findsNothing);
    });

    testWidgets('tells the sizes, where the models come from and whose they '
        'are', (tester) async {
      await start(tester);
      await openModel(tester);
      expect(find.text('Base, 148 MB'), findsOneWidget);
      expect(
        find.text('Faster, and enough for English or German alone.'),
        findsOneWidget,
      );
      expect(find.text('Small, 488 MB'), findsOneWidget);
      expect(
        find.text(
          'Better with English words in German speech, but slower, '
          'and it needs more memory.',
        ),
        findsOneWidget,
      );
      expect(
        find.textContaining('Only the chosen model is kept'),
        findsOneWidget,
      );
      expect(find.textContaining('ggml-base.bin or ggml-small.bin'), findsOne);
      // None chosen while there is none.
      expect(chosen(tester, base), isFalse);
      expect(chosen(tester, small), isFalse);
      expect(
        find.textContaining('only from Old Brain Fix\'s own release on GitHub'),
        findsOneWidget,
      );
      expect(
        find.textContaining('Whisper by OpenAI, Copyright (c) 2022 OpenAI'),
        findsOneWidget,
      );
      expect(missing, findsOneWidget);
      expect(pick, findsOneWidget);
      // Nothing is fetched without asking.
      expect(speech.downloads, isEmpty);
    });

    testWidgets('downloads from the project\'s release with progress', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, base);

      expect(speech.downloads, [WhisperModel.base.url]);
      expect(
        WhisperModel.base.url.toString(),
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

      speech.bring(WhisperModel.base.sha256);
      await tester.pumpAndSettle();
      expect(ready, findsOneWidget);
      expect(speech.installed, WhisperModel.base.sha256);
    });

    testWidgets('a download with the wrong checksum is rejected', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, base);
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
      expect(missing, findsOneWidget);
    });

    testWidgets('a download that breaks off says so and keeps nothing', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, base);
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
      expect(missing, findsOneWidget);
    });

    testWidgets('a cancelled download keeps nothing and says nothing', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, base);
      await press(tester, find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(speech.cancels, 1);
      expect(speech.installed, isNull);
      expect(find.textContaining('did not finish'), findsNothing);
      expect(missing, findsOneWidget);
    });

    testWidgets('a picked file is kept when it is the pinned model', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, pick);
      expect(speech.picks, 1);
      speech.bring(WhisperModel.base.sha256);
      await tester.pumpAndSettle();

      expect(ready, findsOneWidget);
      expect(speech.installed, WhisperModel.base.sha256);
      expect(speech.downloads, isEmpty);
    });

    testWidgets('a picked file that is not the pinned model is rejected', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, pick);
      speech.bring(wrongHash);
      await tester.pumpAndSettle();

      expect(find.textContaining('not the expected'), findsOneWidget);
      expect(speech.installed, isNull);
    });

    testWidgets('picking no file changes nothing', (tester) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, pick);
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
      expect(missing, findsOneWidget);
    });

    testWidgets('a fetch cut off when the app was closed is dropped', (
      tester,
    ) async {
      speech.side = WhisperModel.base.sha256;
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
      expect(base, findsNothing);
    });
    testWidgets('choosing small downloads it and switches to it; the base '
        'model stays in use until small is in place', (tester) async {
      speech.installed = WhisperModel.base.sha256;
      notes.capture('', recordings: [first]);
      speech.transcripts[1] = 'before';
      await start(tester);
      await openModel(tester);
      expect(ready, findsOneWidget);
      expect(chosen(tester, base), isTrue);

      await choose(tester, small);
      expect(speech.downloads, [WhisperModel.small.url]);
      expect(
        WhisperModel.small.url.toString(),
        'https://github.com/paxel/oldbrainfix/releases/download/'
        'whisper-model-v1/ggml-small.bin',
      );
      expect(chosen(tester, small), isTrue);
      // Still transcribing with base meanwhile.
      notes.capture('', recordings: [second]);
      speech.transcripts[2] = 'meanwhile';
      await tester.pump();
      await tester.pump();
      expect(speech.transcribed, [first, second]);
      expect(speech.installed, WhisperModel.base.sha256);

      speech.bring(WhisperModel.small.sha256);
      await tester.pumpAndSettle();
      expect(ready, findsOneWidget);
      // The new one took the old one's place: only small is kept.
      expect(speech.installed, WhisperModel.small.sha256);
      expect(speech.side, isNull);
      expect(chosen(tester, small), isTrue);
      final model = SpeechScope.of(
        tester.element(find.byType(SpeechModelScreen)),
      )!.model;
      expect(model.current, WhisperModel.small);

      // And back to base the same way.
      await choose(tester, base);
      expect(speech.downloads.last, WhisperModel.base.url);
      speech.bring(WhisperModel.base.sha256);
      await tester.pumpAndSettle();
      expect(speech.installed, WhisperModel.base.sha256);
      expect(chosen(tester, base), isTrue);
    });

    testWidgets('a switch that breaks off keeps the model in use', (
      tester,
    ) async {
      speech.installed = WhisperModel.base.sha256;
      await start(tester);
      await openModel(tester);
      await choose(tester, small);
      speech.bring(null);
      await tester.pumpAndSettle();

      expect(find.textContaining('did not finish'), findsOneWidget);
      expect(speech.installed, WhisperModel.base.sha256);
      expect(ready, findsOneWidget);
      expect(chosen(tester, base), isTrue);
    });

    testWidgets('a cancelled switch keeps the model in use', (tester) async {
      speech.installed = WhisperModel.base.sha256;
      await start(tester);
      await openModel(tester);
      await choose(tester, small);
      await tester.ensureVisible(find.text('Cancel'));
      await press(tester, find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(speech.cancels, 1);
      expect(speech.installed, WhisperModel.base.sha256);
      expect(chosen(tester, base), isTrue);
    });

    testWidgets('a download of small that brings base is not kept', (
      tester,
    ) async {
      await start(tester);
      await openModel(tester);
      await choose(tester, small);
      speech.bring(WhisperModel.base.sha256);
      await tester.pumpAndSettle();

      expect(find.textContaining('not the expected'), findsOneWidget);
      expect(speech.installed, isNull);
      expect(missing, findsOneWidget);
    });

    testWidgets('a picked file is kept as small when it is small', (
      tester,
    ) async {
      speech.installed = WhisperModel.base.sha256;
      await start(tester);
      await openModel(tester);
      await choose(tester, pick);
      speech.bring(WhisperModel.small.sha256);
      await tester.pumpAndSettle();

      expect(ready, findsOneWidget);
      expect(speech.installed, WhisperModel.small.sha256);
      expect(chosen(tester, small), isTrue);
      expect(speech.downloads, isEmpty);
    });

    testWidgets('small in place is kept at the next start', (tester) async {
      speech.installed = WhisperModel.small.sha256;
      notes.capture('', recordings: [first]);
      speech.transcripts[1] = 'with small';
      await start(tester);

      expect(speech.removes, 0);
      expect(speech.transcribed, [first]);
      await openModel(tester);
      expect(ready, findsOneWidget);
      expect(chosen(tester, small), isTrue);
    });

    testWidgets('the German page fits the phone', (tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 2.75;
      addTearDown(tester.view.reset);
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      speech.installed = WhisperModel.base.sha256;
      await start(tester);
      await openDeviceSettings(tester);
      await tester.ensureVisible(find.text('Sprachmodell'));
      await tap(tester, find.text('Sprachmodell'));
      expect(find.byType(SpeechModelScreen), findsOneWidget);

      expect(find.text('Basis, 148 MB'), findsOneWidget);
      expect(find.text('Klein, 488 MB'), findsOneWidget);
      await tester.ensureVisible(find.text('Eigene Datei verwenden'));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('ggml-base.bin oder ggml-small.bin'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    });
  });

  group('the first-start offer', () {
    final notNow = find.text('Not now');
    final mobileHint = find.byKey(const ValueKey('speech-offer-mobile-data'));

    /// A first start up to the Starter Set's Start.
    Future<void> firstStart(
      WidgetTester tester, {
      SpeechPlatform? platform,
    }) async {
      store.starterSetOffered = false;
      await start(tester, platform: platform);
      await tap(tester, find.byKey(const ValueKey('first-new')));
      expect(find.byType(StarterSetScreen), findsOneWidget);
      expect(find.byType(SpeechOfferScreen), findsNothing);
      await tap(tester, find.text('Start'));
    }

    testWidgets('comes after the Starter Set; Not now goes on without the '
        'model, which Settings still offers', (tester) async {
      await firstStart(tester);
      expect(find.byType(SpeechOfferScreen), findsOneWidget);
      expect(find.text('Transcribe voice Notes on the phone?'), findsOneWidget);
      expect(find.textContaining('speech model (148 MB)'), findsOneWidget);
      expect(mobileHint, findsNothing);

      await tap(tester, notNow);
      expect(find.byType(SpeechOfferScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(speech.downloads, isEmpty);

      await openModel(tester);
      expect(base, findsOneWidget);
      expect(pick, findsOneWidget);
    });

    testWidgets('Download starts the download in the background and the '
        'setup goes on at once', (tester) async {
      await firstStart(tester);
      await tap(tester, download);

      expect(speech.downloads, [WhisperModel.base.url]);
      expect(find.byType(SpeechOfferScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(
        SpeechScope.of(tester.element(find.byType(AgendaScreen)))!.model.status,
        ModelStatus.fetching,
      );

      speech.bring(WhisperModel.base.sha256);
      await tester.pumpAndSettle();
      expect(speech.installed, WhisperModel.base.sha256);
    });

    testWidgets('a background download that breaks off is announced', (
      tester,
    ) async {
      await firstStart(tester);
      await tap(tester, download);

      speech.bring(null);
      await tester.pumpAndSettle();
      expect(
        find.text(
          'The download did not finish, and nothing was kept. '
          'Try again later.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('says so beside the button on mobile data', (tester) async {
      speech.mobileData = true;
      await firstStart(tester);
      expect(mobileHint, findsOneWidget);
      expect(
        find.text('You are on mobile data. The download uses 148 MB of it.'),
        findsOneWidget,
      );
    });

    testWidgets('is not shown on a phone that finished setup', (tester) async {
      await start(tester);
      expect(find.byType(SpeechOfferScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('is not shown where Whisper does not run', (tester) async {
      await firstStart(tester, platform: FakeSpeech(runs: false));
      expect(find.byType(SpeechOfferScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('is not shown where a model is in place', (tester) async {
      await firstStart(
        tester,
        platform: FakeSpeech(installed: WhisperModel.base.sha256),
      );
      expect(find.byType(SpeechOfferScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('is not shown without a speech layer', (tester) async {
      await firstStart(tester, platform: const NoSpeech());
      expect(find.byType(SpeechOfferScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
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
      await choose(tester, base);
      speech.bring(WhisperModel.base.sha256);
      await tester.pumpAndSettle();

      expect(speech.transcribed, [first]);
      final read = notes.byId(note.id);
      expect(read?.transcripts, {read?.recordings.single: 'buy more sauce'});
      // The recording stays.
      expect(notes.recordingBytes(read?.recordings.single ?? ''), first);
      expect(speech.idles, greaterThan(0));
    });

    testWidgets('Whisper is led in with the Area and Topic names and the '
        'app\'s words', (tester) async {
      final places = Places(store);
      final bikes = places.addArea('Fahrradladen');
      places.addTopic(bikes.id, 'Reifenwechsel');
      speech
        ..installed = WhisperModel.base.sha256
        ..transcripts[1] = 'Reifen';
      notes.capture('', recordings: [first]);
      await start(tester);

      final prompts = speech.prompts.single;
      expect(prompts.english, 'Fahrradladen, Reifenwechsel.');
      expect(prompts.german, startsWith(TranscriptPrompts.germanLeadIn));
      expect(prompts.german, contains('Meeting'));
      expect(prompts.german, contains('Deadline'));
      expect(prompts.german, contains('Unsorted, Routine, Agenda.'));
      expect(prompts.german, endsWith('Fahrradladen, Reifenwechsel.'));
    });

    testWidgets('in German the prompt names places and words as the user '
        'reads them', (tester) async {
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      Places(store).plantStarterSet({'household', 'kitchen'});
      speech
        ..installed = WhisperModel.base.sha256
        ..transcripts[1] = 'Küche';
      notes.capture('', recordings: [first]);
      await start(tester);

      final prompts = speech.prompts.single;
      expect(prompts.english, 'Haushalt, Küche.');
      expect(prompts.german, contains('Unsortiert, Routine, Agenda.'));
      expect(prompts.german, endsWith('Haushalt, Küche.'));
      expect(
        prompts.german.runes.length,
        lessThanOrEqualTo(TranscriptPrompts.maxLength),
      );
    });

    testWidgets('a new voice Note is transcribed after it is saved, without '
        'holding Capture up', (tester) async {
      speech
        ..installed = WhisperModel.base.sha256
        ..transcripts[1] = 'call the garage'
        ..transcribeGate = Completer<void>();
      await start(tester);
      await openTextCapture(tester);
      await tap(tester, find.byIcon(Icons.mic_none));
      await tap(tester, find.byKey(const ValueKey('stopRecording')));
      await tap(tester, find.text('Save'));

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
      speech.installed = WhisperModel.base.sha256;
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
        ..installed = WhisperModel.base.sha256
        ..transcripts[1] = '';
      final note = notes.capture('', recordings: [first]);
      await start(tester);
      expect(notes.byId(note.id)?.transcripts.values, ['']);
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), notes.byId(note.id)!));
      await tester.pumpAndSettle();
      expect(find.text('No words heard.'), findsOneWidget);
    });

    testWidgets('Whisper repeating its lead-in is kept as "no words heard", '
        'speech after it as said', (tester) async {
      final places = Places(store);
      for (final name in ['Arzt', 'Work', 'Meetings']) {
        places.addArea(name);
      }
      speech
        ..installed = WhisperModel.base.sha256
        ..transcripts[1] = ' ${TranscriptPrompts.germanLeadIn}'
        ..transcripts[2] =
            '${TranscriptPrompts.germanLeadIn} Arzt, Meetings, Work. '
            'Morgen um zehn zum Arzt.';
      final echo = notes.capture('', recordings: [first]);
      final said = notes.capture('', recordings: [second]);
      await start(tester);
      expect(notes.byId(echo.id)?.transcripts.values, ['']);
      expect(notes.byId(said.id)?.transcripts.values, [
        'Morgen um zehn zum Arzt.',
      ]);
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, places, notes.byId(echo.id)!));
      await tester.pumpAndSettle();
      expect(find.text('No words heard.'), findsOneWidget);
      expect(find.textContaining('Notiz auf Deutsch'), findsNothing);
    });

    testWidgets('the editor shows each recording\'s Transcript, a merged '
        'Note\'s too', (tester) async {
      speech
        ..installed = WhisperModel.base.sha256
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
        ..installed = WhisperModel.base.sha256
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
        ..installed = WhisperModel.base.sha256
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

  test(
    'the channel hands the recording and both prompts to the phone',
    () async {
      TestWidgetsFlutterBinding.ensureInitialized();
      const channel = MethodChannel('io.github.paxel.oldbrainfix/speech');
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      MethodCall? asked;
      messenger.setMockMethodCallHandler(channel, (call) async {
        asked = call;
        return 'heard';
      });
      addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

      final text = await AndroidSpeech().transcribe(
        first,
        const TranscriptPrompts(english: 'Work.', german: 'Notiz. Work.'),
      );

      expect(text, 'heard');
      expect(asked?.method, 'transcribe');
      expect(asked?.arguments, {
        'bytes': first,
        'english': 'Work.',
        'german': 'Notiz. Work.',
      });
    },
  );
}

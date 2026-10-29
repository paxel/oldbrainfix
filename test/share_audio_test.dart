import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/failure_dialog.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix/src/voice_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_speech.dart';
import 'fake_voice.dart';

/// Stand-ins for two shared sound files, an Ogg voice message and an
/// m4a file: the app keeps any bytes as they are.
final message = Uint8List.fromList([0x4F, 0x67, 0x67, 0x53, 0, 1]);
final memo = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 2]);

const whatsapp = SourceApp('com.whatsapp', label: 'WhatsApp');

void main() {
  late Store store;
  late Notes notes;
  late FakePhotos photos;
  late FakeVoice voice;
  late FakeSpeech speech;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    photos = FakePhotos();
    voice = FakeVoice();
    speech = FakeSpeech(installed: WhisperModel.base.sha256);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        photos: photos,
        voice: voice,
        speech: speech,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Finder inCard(Finder finder) =>
      find.descendant(of: find.byType(NoteCard), matching: finder);

  group('a shared sound file', () {
    testWidgets('becomes a voice Note from its app, transcribed, kept when '
        'the user just goes back', (tester) async {
      speech.transcripts[1] = 'pick up the parcel';
      await start(tester);
      photos.share(
        SharedItem(recordings: [SharedRecording(message)], sourceApp: whatsapp),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(find.byType(RecordingButton), findsOneWidget);
      // Written at once, before the user did anything.
      final note = notes.all().single;
      expect(note.recordings, hasLength(1));
      expect(notes.recordingBytes(note.recordings.single), message);
      expect(note.context.sourceApp?.package, 'com.whatsapp');
      expect(note.context.sourceApp?.name, 'WhatsApp');
      // Transcribed with the same model and prompts as a recording.
      expect(speech.transcribed, [message]);
      expect(speech.prompts, hasLength(1));

      final dynamic navigator = tester.state(find.byType(Navigator).first);
      await (navigator as NavigatorState).maybePop();
      await tester.pumpAndSettle();

      final saved = notes.byId(note.id)!;
      expect(saved.isUnsorted, isTrue);
      expect(saved.nudge, now.add(Notes.defaultNudge));
      expect(saved.transcripts.values, ['pick up the parcel']);
      expect(inCard(find.text('pick up the parcel')), findsOneWidget);
      // Nothing failed: nothing announced.
      expect(
        find.text('A shared recording cannot be transcribed.'),
        findsNothing,
      );
    });

    testWidgets('several make one Note with a recording each, next to the '
        'text', (tester) async {
      await start(tester);
      photos.share(
        SharedItem(
          text: 'From the plumber',
          recordings: [SharedRecording(message), SharedRecording(memo)],
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(RecordingButton), findsNWidgets(2));
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.text, 'From the plumber');
      expect(
        [for (final hash in note.recordings) notes.recordingBytes(hash)],
        [message, memo],
      );
    });

    testWidgets('plays on its card at the chosen speed', (tester) async {
      store.playbackSpeed = 1.5;
      await start(tester);
      photos.share(SharedItem(recordings: [SharedRecording(message)]));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Save'));

      await tap(tester, inCard(find.byIcon(Icons.play_arrow)));
      expect(voice.played, [message]);
      expect(voice.speed, 1.5);
    });

    testWidgets('can be removed and its Transcript kept', (tester) async {
      speech.transcripts[1] = 'pick up the parcel';
      await start(tester);
      photos.share(SharedItem(recordings: [SharedRecording(message)]));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Save'));
      final note = notes.all().single;
      final hash = note.recordings.single;

      await tap(tester, find.byKey(const ValueKey('note-menu')));
      await tap(tester, find.byKey(ValueKey('note-remove-$hash')));
      await tap(tester, find.text('Remove'));

      final after = notes.byId(note.id)!;
      expect(after.recordings, isEmpty);
      expect(after.removedRecordings, [hash]);
      expect(inCard(find.text('pick up the parcel')), findsOneWidget);
    });
  });

  group('a shared sound file this Device cannot transcribe', () {
    testWidgets('still makes the Note and announces why', (tester) async {
      await start(tester);
      photos.share(
        SharedItem(
          recordings: [
            SharedRecording(
              message,
              problem: SoundProblem.unsupported,
              format: 'audio/x-rare',
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();

      // Kept as it is, over the announcement.
      expect(find.byType(CaptureScreen), findsOneWidget);
      final note = notes.all().single;
      expect(notes.recordingBytes(note.recordings.single), message);
      expect(find.text('A shared recording cannot be transcribed.'), findsOne);

      await tap(tester, find.text('A shared recording cannot be transcribed.'));
      expect(find.text('Recording not transcribed'), findsOneWidget);
      expect(
        find.textContaining(
          'This phone cannot decode the format of the shared file.',
        ),
        findsOneWidget,
      );
      expect(
        find.textContaining('share it again as m4a or mp3'),
        findsOneWidget,
      );
      expect(find.text('unsupported: audio/x-rare'), findsOneWidget);
      expect(find.text('Report'), findsOneWidget);
      await tap(tester, find.text('Close'));

      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(CaptureScreen), findsOneWidget);
      await tap(tester, find.text('Save'));
      expect(notes.all().single.recordings, hasLength(1));
    });

    testWidgets('names a file without sound, listing the failed ones', (
      tester,
    ) async {
      await start(tester);
      photos.share(
        SharedItem(
          recordings: [
            SharedRecording(message, problem: SoundProblem.unreadable),
            SharedRecording(memo),
            SharedRecording(
              Uint8List.fromList([9]),
              problem: SoundProblem.unsupported,
              format: 'audio/x-rare',
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      expect(notes.all().single.recordings, hasLength(3));

      await tap(tester, find.text('Shared recordings cannot be transcribed.'));
      expect(
        find.textContaining('No sound was found in the shared file'),
        findsOneWidget,
      );
      expect(
        find.text('unreadable\nunsupported: audio/x-rare'),
        findsOneWidget,
      );
    });

    testWidgets('is announced in German', (tester) async {
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await start(tester);
      photos.share(
        SharedItem(
          recordings: [
            SharedRecording(message, problem: SoundProblem.unreadable),
          ],
        ),
      );
      await tester.pumpAndSettle();

      await tap(
        tester,
        find.text('Eine geteilte Aufnahme kann nicht transkribiert werden.'),
      );
      expect(find.text('Aufnahme nicht transkribiert'), findsOneWidget);
      expect(find.text('Schließen'), findsOneWidget);
      expect(find.text('Melden'), findsOneWidget);
    });
  });

  testWidgets('Report hands the full text on and closes the dialog', (
    tester,
  ) async {
    final reported = <String>[];
    await start(tester);
    final context = tester.element(find.byType(Scaffold).first);
    unawaited(
      showFailure(
        context,
        title: 'Title',
        cause: 'Cause.',
        fix: 'Fix.',
        details: 'unsupported: audio/x-rare',
        report: (body) async => reported.add(body),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Cause. Fix.'), findsOneWidget);

    await tap(tester, find.text('Report'));
    expect(reported, ['unsupported: audio/x-rare']);
    expect(find.byType(AlertDialog), findsNothing);
  });

  test('the channel hands over the sound files with their problems', () async {
    TestWidgetsFlutterBinding.ensureInitialized();
    const channel = MethodChannel('io.github.paxel.oldbrainfix/photos');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      expect(call.method, 'shared');
      return {
        'subject': null,
        'text': null,
        'photos': <Object?>[],
        'recordings': [
          {'bytes': message, 'problem': null, 'format': null},
          {'bytes': memo, 'problem': 'unsupported', 'format': 'audio/x-rare'},
          {
            'bytes': Uint8List.fromList([7]),
            'problem': 'unreadable',
          },
          {'problem': 'unreadable'},
        ],
        'sourcePackage': 'com.whatsapp',
        'sourceLabel': 'WhatsApp',
      };
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    final shared = await AndroidPhotos().takeShared();

    final [plain, rare, broken] = shared!.recordings;
    expect(plain.bytes, message);
    expect(plain.problem, isNull);
    expect(rare.bytes, memo);
    expect(rare.problem, SoundProblem.unsupported);
    expect(rare.format, 'audio/x-rare');
    expect(broken.problem, SoundProblem.unreadable);
    expect(broken.format, isNull);
    expect(shared.sourceApp?.package, 'com.whatsapp');
  });
}

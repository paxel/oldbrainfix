import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/voice_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_voice.dart';
import 'reveal.dart';

/// Stand-ins for two recordings: the app keeps any bytes as they are.
final first = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);
final second = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 2]);

void main() {
  late Store store;
  late Notes notes;
  late FakeVoice voice;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    voice = FakeVoice(nextRecording: first);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, voice: voice));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  final mic = find.byIcon(Icons.mic_none);
  final stop = find.byKey(const ValueKey('stopRecording'));
  String elapsed(WidgetTester tester) =>
      tester.widget<Text>(find.byKey(const ValueKey('elapsed'))).data ?? '';

  /// Opens text Capture from the Agenda's +.
  Future<void> openCapture(WidgetTester tester) async {
    await tap(tester, find.byIcon(Icons.add));
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  group('the mic button', () {
    testWidgets('is not offered where there is no microphone', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openCapture(tester);
      expect(mic, findsNothing);
    });

    testWidgets('text Capture never asks for the microphone', (tester) async {
      voice.allowed = false;
      await start(tester);
      await openCapture(tester);
      expect(mic, findsOneWidget);
      await tester.enterText(find.byType(TextField), 'milk');
      await tap(tester, find.text('Save'));

      expect(voice.requests, 0);
      expect(voice.starts, 0);
      expect(notes.all().single.recordings, isEmpty);
    });

    testWidgets('records at once when the microphone is allowed', (
      tester,
    ) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);

      expect(find.byType(AlertDialog), findsNothing);
      expect(voice.recording, isTrue);
      expect(stop, findsOneWidget);
      expect(find.text('Recording…'), findsOneWidget);
      // One big stop button: no hold-to-talk, the Nudges stay.
      expect(find.byType(NudgeButtons), findsOneWidget);
      expect(find.text('Save'), findsNothing);
      expect(mic, findsNothing);

      await tap(tester, stop);
    });
  });

  group('the microphone permission', () {
    testWidgets('is explained and asked for the first time', (tester) async {
      voice.allowed = false;
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);

      expect(find.text('Use the microphone?'), findsOneWidget);
      expect(voice.requests, 0);
      await tap(tester, find.text('Allow'));

      expect(voice.requests, 1);
      expect(voice.recording, isTrue);
      await tap(tester, stop);
      expect(notes.all().single.recordings, hasLength(1));
    });

    testWidgets('"Not now" records nothing and asks nothing', (tester) async {
      voice.allowed = false;
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, find.text('Not now'));

      expect(voice.requests, 0);
      expect(voice.starts, 0);
      expect(find.text('Voice Notes need the microphone.'), findsOneWidget);
      // Typing is still there.
      await tap(tester, find.text('Type instead'));
      await tester.enterText(find.byType(TextField), 'milk');
      await tap(tester, find.text('Save'));
      expect(notes.all().single.text, 'milk');
    });

    testWidgets('denied, it can be asked for again', (tester) async {
      voice
        ..allowed = false
        ..allowOnRequest = false;
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, find.text('Allow'));
      expect(voice.starts, 0);
      expect(find.text('Voice Notes need the microphone.'), findsOneWidget);

      voice.allowOnRequest = true;
      await tap(tester, find.widgetWithText(FilledButton, 'Allow'));
      await tap(tester, find.widgetWithText(TextButton, 'Allow'));
      expect(voice.requests, 2);
      expect(voice.recording, isTrue);
      await tap(tester, stop);
    });
  });

  group('recording', () {
    testWidgets('shows the elapsed time', (tester) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      expect(elapsed(tester), '0:00');

      await tester.pump(const Duration(seconds: 3));
      expect(elapsed(tester), '0:03');
      await tester.pump(const Duration(seconds: 62));
      expect(elapsed(tester), '1:05');

      await tap(tester, stop);
    });

    testWidgets('the stop button saves the Note, nudged in 1 h', (
      tester,
    ) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, stop);

      expect(voice.stops, 1);
      expect(voice.recording, isFalse);
      expect(find.byType(CaptureScreen), findsNothing);
      final note = notes.all().single;
      expect(note.text, '');
      expect(note.recordings, [store.putAudio(first)]);
      expect(notes.recordingBytes(note.recordings.single), first);
      expect(note.isUnsorted, isTrue);
      expect(note.nudge, now.add(Notes.defaultNudge));
      // Its card shows on the Agenda with a play button.
      expect(
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.byType(RecordingButton),
        ),
        findsOneWidget,
      );
    });

    testWidgets('a Nudge button stops and saves with its time', (tester) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, find.byKey(const ValueKey('nudge10')));

      expect(voice.recording, isFalse);
      final note = notes.all().single;
      expect(note.recordings, hasLength(1));
      expect(note.nudge, now.add(const Duration(minutes: 10)));
    });

    testWidgets('typed text and the recording go into one Note', (
      tester,
    ) async {
      await start(tester);
      await openCapture(tester);
      await tester.enterText(find.byType(TextField), 'call the plumber');
      await tap(tester, mic);
      // The words stay in sight while recording.
      expect(find.text('call the plumber'), findsOneWidget);
      await tap(tester, stop);

      final note = notes.all().single;
      expect(note.text, 'call the plumber');
      expect(note.recordings, hasLength(1));
    });

    testWidgets('Back stops and keeps the recording', (tester) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tester.pump(const Duration(seconds: 2));
      final dynamic navigator = tester.state(find.byType(Navigator).first);
      await (navigator as NavigatorState).maybePop();
      await tester.pumpAndSettle();

      expect(voice.stops, 1);
      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all().single.recordings, hasLength(1));
    });

    testWidgets('the app going to the background keeps the recording', (
      tester,
    ) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();

      // Stopped once, written at once.
      expect(voice.stops, 1);
      expect(voice.recording, isFalse);
      final captured = notes.all().single;
      expect(captured.recordings, hasLength(1));

      for (final state in [
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
      // Back on the text field with the recording; words can follow.
      expect(find.byType(RecordingButton), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'for the car');
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.id, captured.id);
      expect(note.text, 'for the car');
      expect(note.recordings, hasLength(1));
      expect(voice.stops, 1);
    });

    testWidgets('a second recording goes into the same Note', (tester) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
      voice.nextRecording = second;
      await tap(tester, mic);
      await tap(tester, stop);

      expect(notes.all().single.recordings, [
        store.putAudio(first),
        store.putAudio(second),
      ]);
    });

    testWidgets('nothing usable recorded captures nothing', (tester) async {
      voice.nextRecording = null;
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, stop);

      expect(voice.stops, 1);
      expect(notes.all(), isEmpty);
    });

    testWidgets('voice Capture records as soon as it opens', (tester) async {
      await start(tester);
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(captureNote(agenda, notes, voice: true, onChanged: () {}));
      await tester.pumpAndSettle();

      expect(voice.recording, isTrue);
      expect(stop, findsOneWidget);
      await tap(tester, stop);
      expect(notes.all().single.recordings, hasLength(1));
    });
  });

  group('playing back', () {
    testWidgets('the card plays and stops its recording', (tester) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, stop);
      final play = find.descendant(
        of: find.byType(NoteCard),
        matching: find.byIcon(Icons.play_arrow),
      );
      await tap(tester, play);

      expect(voice.played, [first]);
      expect(find.byIcon(Icons.stop), findsOneWidget);
      await tap(tester, find.byIcon(Icons.stop));
      expect(voice.stoppedPlaying, 1);
      expect(find.byIcon(Icons.play_arrow), findsOneWidget);

      // Played to the end, it shows play again.
      await tap(tester, play);
      voice.finishPlaying();
      await tester.pumpAndSettle();
      expect(find.byIcon(Icons.play_arrow), findsOneWidget);
      expect(voice.played, [first, first]);
    });

    testWidgets('recording again stops the playing', (tester) async {
      await start(tester);
      await openCapture(tester);
      await tap(tester, mic);
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
      await tap(tester, find.byIcon(Icons.play_arrow));
      expect(voice.played, [first]);

      voice.nextRecording = second;
      await tap(tester, mic);
      expect(voice.stoppedPlaying, 1);
      await tap(tester, stop);
      // Both cards' buttons show play again.
      expect(find.byIcon(Icons.stop), findsNothing);
      expect(find.byIcon(Icons.play_arrow), findsNWidgets(2));
    });

    testWidgets('the editor plays its own and a merged Note\'s recordings', (
      tester,
    ) async {
      final into = notes.capture('sauce', recordings: [first]);
      now = now.add(const Duration(minutes: 5));
      final merged = notes.capture('', recordings: [second]);
      notes.merge(merged.id, into.id);
      await start(tester);
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), notes.byId(into.id)!));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);

      final buttons = find.descendant(
        of: find.byType(NoteEditorScreen),
        matching: find.byType(RecordingButton),
      );
      Finder play(Finder button) =>
          find.descendant(of: button, matching: find.byIcon(Icons.play_arrow));
      await tap(tester, play(buttons.first));
      expect(voice.played, [first]);
      // The merged Note's Section sits low on the page.
      final low = find.byWidgetPredicate(
        (w) => w is RecordingButton && w.hash == merged.recordings.single,
      );
      await reveal(tester, low);
      await tap(tester, play(low));
      expect(voice.played, [first, second]);
    });
  });

  group('the playback speed', () {
    final label = find.descendant(
      of: find.byType(NoteCard),
      matching: find.byKey(const ValueKey('playbackSpeed')),
    );
    String shown(WidgetTester tester) => tester
        .widget<Text>(find.descendant(of: label, matching: find.byType(Text)))
        .data!;

    /// Records a voice Note, whose card shows on the Agenda.
    Future<void> record(WidgetTester tester) async {
      await openCapture(tester);
      await tap(tester, mic);
      await tap(tester, stop);
    }

    testWidgets('changes at once, while playing, and is kept', (tester) async {
      await start(tester);
      await record(tester);
      expect(shown(tester), '1×');
      await tap(tester, find.byIcon(Icons.play_arrow));
      expect(voice.speed, 1);

      await tap(tester, label);
      // The current speed is the one marked.
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('playbackSpeed-1.0')),
          matching: find.byIcon(Icons.radio_button_checked),
        ),
        findsOneWidget,
      );
      expect(find.byIcon(Icons.radio_button_unchecked), findsNWidgets(4));
      await tap(tester, find.text('×1.5'));

      expect(shown(tester), '1.5×');
      // The recording playing goes faster without starting over.
      expect(voice.speed, 1.5);
      expect(voice.played, [first]);
      expect(store.playbackSpeed, 1.5);

      // After a restart the player gets it before it plays; the Note is
      // no longer just captured, so its editor shows it.
      await tester.pumpWidget(const SizedBox());
      final again = FakeVoice();
      await tester.pumpWidget(OldBrainFixApp(store: store, voice: again));
      await tester.pumpAndSettle();
      final agenda = tester.element(find.byType(AgendaScreen));
      unawaited(openNote(agenda, notes, Places(store), notes.all().single));
      await tester.pumpAndSettle();
      final editor = find.byType(NoteEditorScreen);
      final inEditor = find.descendant(
        of: editor,
        matching: find.byKey(const ValueKey('playbackSpeed')),
      );
      expect(
        tester
            .widget<Text>(
              find.descendant(of: inEditor, matching: find.byType(Text)),
            )
            .data,
        '1.5×',
      );
      await tap(
        tester,
        find.descendant(of: editor, matching: find.byIcon(Icons.play_arrow)),
      );
      expect(again.speed, 1.5);
      expect(again.played, [first]);
    });

    testWidgets('is written the German way', (tester) async {
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      store.playbackSpeed = 1.5;
      await start(tester);
      await record(tester);
      expect(shown(tester), '1,5×');

      await tap(tester, label);
      expect(find.text('×0,75'), findsOneWidget);
      await tap(tester, find.text('×0,5'));
      expect(shown(tester), '0,5×');
      expect(store.playbackSpeed, 0.5);
    });

    testWidgets('the one kept before, as the app kept it in 0.1.1, carries '
        'over', (tester) async {
      store.setLocalSetting('playbackSpeed', '1.5');
      await start(tester);
      await record(tester);
      expect(shown(tester), '1.5×');
      await tap(tester, find.byIcon(Icons.play_arrow));
      expect(voice.speed, 1.5);
      expect(store.localSetting('playbackSpeed'), '1.5');
    });

    testWidgets('anything unknown kept plays at normal speed', (tester) async {
      store.setLocalSetting('playbackSpeed', '7');
      expect(store.playbackSpeed, 1);
    });
  });
}

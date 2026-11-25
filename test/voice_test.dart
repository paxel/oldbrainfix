import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/recording_pause.dart';
import 'package:oldbrainfix/src/voice_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_voice.dart';
import 'reveal.dart';
import 'open_capture.dart';
import 'open_settings.dart';

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
    await openTextCapture(tester);
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  /// Opens voice Capture from the Agenda's +, recording at once; the
  /// quick note has no Voice of its own (#352).
  Future<void> openVoice(WidgetTester tester) async {
    await openCaptureBy(tester, 'voice');
    expect(find.byType(CaptureScreen), findsOneWidget);
  }

  /// Opens the editor of the Note [note] from the Agenda.
  Future<void> edit(WidgetTester tester, Note note) async {
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, Places(store), note));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  final editorVoice = find.byKey(const ValueKey('editor-voice'));

  group('voice', () {
    testWidgets('is not offered where there is no microphone', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tap(tester, find.byIcon(Icons.add));
      expect(fanEntry('voice'), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();
      final note = notes.capture('milk');
      await edit(tester, note);
      expect(editorVoice, findsNothing);
    });

    testWidgets('text Capture never asks for the microphone, and offers no '
        'Voice (#352)', (tester) async {
      voice.allowed = false;
      await start(tester);
      await openCapture(tester);
      expect(mic, findsNothing);
      await tester.enterText(find.byType(TextField), 'milk');
      await saveCapture(tester);

      expect(voice.requests, 0);
      expect(voice.starts, 0);
      expect(notes.all().single.recordings, isEmpty);
    });

    testWidgets('records at once when the microphone is allowed', (
      tester,
    ) async {
      await start(tester);
      await openVoice(tester);

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
      await openVoice(tester);

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
      await openVoice(tester);
      await tap(tester, find.text('Not now'));

      expect(voice.requests, 0);
      expect(voice.starts, 0);
      expect(find.text('Voice Notes need the microphone.'), findsOneWidget);
      // Typing is still there.
      await tap(tester, find.text('Type instead'));
      await tester.enterText(find.byType(TextField), 'milk');
      await saveCapture(tester);
      expect(notes.all().single.text, 'milk');
    });

    testWidgets('denied, it can be asked for again', (tester) async {
      voice
        ..allowed = false
        ..allowOnRequest = false;
      await start(tester);
      await openVoice(tester);
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
      await openVoice(tester);
      expect(elapsed(tester), '0:00');

      await tester.pump(const Duration(seconds: 3));
      expect(elapsed(tester), '0:03');
      await tester.pump(const Duration(seconds: 62));
      expect(elapsed(tester), '1:05');

      await tap(tester, stop);
    });

    testWidgets('the stop button comes back to the capture screen with the '
        'recording and room to type; leaving keeps both, nudged in 1 h', (
      tester,
    ) async {
      await start(tester);
      await openVoice(tester);
      await tap(tester, stop);

      expect(voice.stops, 1);
      expect(voice.recording, isFalse);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(stop, findsNothing);
      expect(
        find.descendant(
          of: find.byType(CaptureScreen),
          matching: find.byType(RecordingButton),
        ),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('captureToEditor')), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'plumber');
      await saveCapture(tester);

      expect(find.byType(CaptureScreen), findsNothing);
      final note = notes.all().single;
      expect(note.text, 'plumber');
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
      await openVoice(tester);
      await tap(tester, find.byKey(const ValueKey('nudge10')));

      expect(voice.recording, isFalse);
      final note = notes.all().single;
      expect(note.recordings, hasLength(1));
      expect(note.nudge, now.add(const Duration(minutes: 10)));
    });

    testWidgets('the editor\'s Voice records into the open Note on a page '
        'with the big stop button, back to the editor after Stop (#352)', (
      tester,
    ) async {
      final note = notes.capture('call the plumber');
      await start(tester);
      await edit(tester, note);
      await tap(tester, editorVoice);

      expect(voice.recording, isTrue);
      expect(stop, findsOneWidget);
      expect(find.text('Recording…'), findsOneWidget);
      await tester.pump(const Duration(seconds: 2));
      expect(elapsed(tester), '0:02');
      await tap(tester, stop);

      expect(voice.recording, isFalse);
      expect(stop, findsNothing);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      final recorded = notes.byId(note.id)!;
      expect(recorded.text, 'call the plumber');
      expect(recorded.recordings, [store.putAudio(first)]);
      expect(notes.all(), hasLength(1));
    });

    testWidgets('the editor\'s Voice asks for the microphone the first time, '
        'and Back ends and keeps the recording (#352)', (tester) async {
      voice.allowed = false;
      final note = notes.capture('call the plumber');
      await start(tester);
      await edit(tester, note);
      await tap(tester, editorVoice);
      expect(find.text('Use the microphone?'), findsOneWidget);
      await tap(tester, find.text('Allow'));
      expect(voice.recording, isTrue);

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(voice.recording, isFalse);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(notes.byId(note.id)!.recordings, hasLength(1));
    });

    testWidgets('a voice Capture asked for while the editor records ends and '
        'keeps that recording first: one at a time (#350, #352)', (
      tester,
    ) async {
      final note = notes.capture('call the plumber');
      await start(tester);
      await edit(tester, note);
      await tap(tester, editorVoice);
      expect(voice.starts, 1);

      // The Agenda lies under the editor, offstage.
      final agenda = tester.element(
        find.byType(AgendaScreen, skipOffstage: false),
      );
      unawaited(captureNote(agenda, notes, voice: true, onChanged: () {}));
      await tester.pumpAndSettle();
      expect(voice.starts, 2);
      expect(voice.stops, 1);
      expect(notes.byId(note.id)!.recordings, [store.putAudio(first)]);

      voice.nextRecording = second;
      await tap(tester, find.byKey(const ValueKey('stopRecording')).last);
      final captured = notes.all().where((n) => n.id != note.id).single;
      expect(captured.recordings, [store.putAudio(second)]);
    });

    testWidgets('Back stops and keeps the recording', (tester) async {
      await start(tester);
      await openVoice(tester);
      await tester.pump(const Duration(seconds: 2));
      final dynamic navigator = tester.state(find.byType(Navigator).first);
      await (navigator as NavigatorState).maybePop();
      await tester.pumpAndSettle();

      expect(voice.stops, 1);
      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all().single.recordings, hasLength(1));
    });

    testWidgets('on Android, leaving the app or locking the phone ends the '
        'recording and keeps it, back on the text field (#368)', (
      tester,
    ) async {
      await start(tester);
      await openVoice(tester);
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
      expect(captured.recordings, [store.putAudio(first)]);

      for (final state in [
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
      // Back on the text field with the recording; words can follow.
      expect(stop, findsNothing);
      expect(find.byType(RecordingButton), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'for the car');
      await saveCapture(tester);

      final note = notes.all().single;
      expect(note.id, captured.id);
      expect(note.text, 'for the car');
      expect(note.recordings, hasLength(1));
      expect(voice.stops, 1);
    });

    testWidgets('on Android, leaving the app ends the editor\'s recording, '
        'keeps it in the Note and closes its page (#368)', (tester) async {
      final note = notes.capture('call the plumber');
      await start(tester);
      await edit(tester, note);
      await tap(tester, editorVoice);
      expect(voice.recording, isTrue);

      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();

      expect(voice.stops, 1);
      expect(voice.recording, isFalse);
      expect(stop, findsNothing);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(notes.byId(note.id)!.recordings, [store.putAudio(first)]);
    });

    testWidgets('on the iPhone the recording goes on in the background and '
        'with the phone locked, and shows as running back in the app (#350)', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      try {
        await start(tester);
        await openVoice(tester);
        await tester.pump(const Duration(seconds: 2));
        for (final state in [
          AppLifecycleState.inactive,
          AppLifecycleState.hidden,
          AppLifecycleState.paused,
        ]) {
          tester.binding.handleAppLifecycleStateChanged(state);
        }
        await tester.pump(const Duration(seconds: 3));

        expect(voice.stops, 0);
        expect(voice.recording, isTrue);

        for (final state in [
          AppLifecycleState.hidden,
          AppLifecycleState.inactive,
          AppLifecycleState.resumed,
        ]) {
          tester.binding.handleAppLifecycleStateChanged(state);
        }
        await tester.pump();
        // Still recording, the time counted on.
        expect(stop, findsOneWidget);
        expect(find.text('Recording…'), findsOneWidget);
        expect(elapsed(tester), '0:05');
        await tap(tester, stop);

        expect(voice.stops, 1);
        final note = notes.all().single;
        expect(note.recordings, [store.putAudio(first)]);
      } finally {
        debugDefaultTargetPlatformOverride = null;
      }
    });

    testWidgets('a second recording in the editor goes into the same Note', (
      tester,
    ) async {
      await start(tester);
      await openVoice(tester);
      await tap(tester, stop);
      await tap(tester, find.byKey(const ValueKey('captureToEditor')));
      voice.nextRecording = second;
      await tap(tester, editorVoice);
      await tap(tester, stop);

      expect(notes.all().single.recordings, [
        store.putAudio(first),
        store.putAudio(second),
      ]);
    });

    testWidgets('nothing usable recorded captures nothing', (tester) async {
      voice.nextRecording = null;
      await start(tester);
      await openVoice(tester);
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

  group('a pause (obf-1053)', () {
    /// The microphone hears [loudness] for [time].
    Future<void> hear(
      WidgetTester tester,
      double? loudness,
      Duration time,
    ) async {
      voice.loudness = loudness;
      await tester.pump(time);
    }

    const speech = 0.3;
    const quiet = 0.005;

    testWidgets('is ignored by default: a recording goes on, its level not '
        'even asked', (tester) async {
      await start(tester);
      await openVoice(tester);
      await hear(tester, speech, const Duration(seconds: 1));
      await hear(tester, quiet, const Duration(seconds: 30));

      expect(store.localSetting(recordingPauseKey), isNull);
      expect(voice.recording, isTrue);
      expect(voice.levelAsks, 0);
      await tap(tester, stop);
    });

    testWidgets('of the set length after speaking ends the quick note\'s '
        'recording and keeps it, as Stop does; speaking again counts anew', (
      tester,
    ) async {
      setRecordingPause(store, 2);
      await start(tester);
      await openVoice(tester);
      await hear(tester, speech, const Duration(seconds: 1));
      await hear(tester, quiet, const Duration(milliseconds: 1500));
      expect(voice.recording, isTrue);
      await hear(tester, speech, const Duration(milliseconds: 400));
      await hear(tester, quiet, const Duration(milliseconds: 1600));
      expect(voice.recording, isTrue);

      await tester.pump(const Duration(milliseconds: 600));
      await tester.pumpAndSettle();
      expect(voice.recording, isFalse);
      expect(voice.stops, 1);
      // Back on the quick note with the recording and room to type.
      expect(stop, findsNothing);
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(CaptureScreen),
          matching: find.byType(RecordingButton),
        ),
        findsOneWidget,
      );
      await saveCapture(tester);
      expect(notes.all().single.recordings, [store.putAudio(first)]);
    });

    testWidgets('before anything was said, or while the level is not known, '
        'ends nothing', (tester) async {
      setRecordingPause(store, 2);
      await start(tester);
      await openVoice(tester);
      await hear(tester, quiet, const Duration(seconds: 20));
      expect(voice.recording, isTrue);
      await hear(tester, speech, const Duration(seconds: 1));
      await hear(tester, null, const Duration(seconds: 20));
      expect(voice.recording, isTrue);
      expect(voice.levelAsks, greaterThan(0));
      await tap(tester, stop);
      expect(voice.stops, 1);
    });

    testWidgets('ends the editor\'s recording, which goes into its Note, and '
        'closes its page', (tester) async {
      setRecordingPause(store, 3);
      final note = notes.capture('call the plumber');
      await start(tester);
      await edit(tester, note);
      await tap(tester, editorVoice);
      await hear(tester, speech, const Duration(seconds: 1));
      await hear(tester, quiet, const Duration(milliseconds: 2600));
      expect(voice.recording, isTrue);
      await hear(tester, quiet, const Duration(milliseconds: 600));
      await tester.pumpAndSettle();

      expect(voice.recording, isFalse);
      expect(stop, findsNothing);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(notes.byId(note.id)!.recordings, [store.putAudio(first)]);
    });

    testWidgets('ends the recording the iPhone goes on with locked, kept as '
        'Stop keeps it (#350)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      try {
        setRecordingPause(store, 2);
        await start(tester);
        await openVoice(tester);
        for (final state in [
          AppLifecycleState.inactive,
          AppLifecycleState.hidden,
          AppLifecycleState.paused,
        ]) {
          tester.binding.handleAppLifecycleStateChanged(state);
        }
        await hear(tester, speech, const Duration(seconds: 1));
        expect(voice.recording, isTrue);
        await hear(tester, quiet, const Duration(milliseconds: 2200));
        expect(voice.recording, isFalse);
        expect(voice.stops, 1);

        for (final state in [
          AppLifecycleState.hidden,
          AppLifecycleState.inactive,
          AppLifecycleState.resumed,
        ]) {
          tester.binding.handleAppLifecycleStateChanged(state);
        }
        await tester.pumpAndSettle();
        expect(stop, findsNothing);
        expect(find.byType(CaptureScreen), findsOneWidget);
        await saveCapture(tester);
        expect(notes.all().single.recordings, [store.putAudio(first)]);
      } finally {
        debugDefaultTargetPlatformOverride = null;
      }
    });

    testWidgets('Stop within a pause keeps the recording once', (tester) async {
      setRecordingPause(store, 2);
      await start(tester);
      await openVoice(tester);
      await hear(tester, speech, const Duration(seconds: 1));
      await hear(tester, quiet, const Duration(seconds: 1));
      await tap(tester, stop);
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();

      expect(voice.stops, 1);
      await saveCapture(tester);
      expect(notes.all().single.recordings, hasLength(1));
    });

    testWidgets('is chosen in Settings ▸ Capture, Off at first, and kept on '
        'this Device', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      final choice = find.byKey(const ValueKey('recordingPause'));
      await tester.scrollUntilVisible(
        choice,
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.pumpAndSettle();
      expect(find.text('End a recording after a pause'), findsOneWidget);
      ChoiceChip chip(String label) => tester.widget<ChoiceChip>(
        find.ancestor(
          of: find.descendant(of: choice, matching: find.text(label)),
          matching: find.byType(ChoiceChip),
        ),
      );
      expect(
        [
          for (final o in ['Off', '2 s', '3 s', '5 s', '10 s'])
            chip(o).selected,
        ],
        [true, false, false, false, false],
      );

      await tap(
        tester,
        find.descendant(of: choice, matching: find.text('5 s')),
      );
      expect(store.localSetting(recordingPauseKey), '5');
      expect(recordingPause(store), const Duration(seconds: 5));
      expect(chip('5 s').selected, isTrue);

      await tap(
        tester,
        find.descendant(of: choice, matching: find.text('Off')),
      );
      expect(store.localSetting(recordingPauseKey), isNull);
      expect(recordingPause(store), isNull);
    });

    testWidgets('is not offered where there is no microphone', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openDeviceSettings(tester);
      expect(find.byKey(const ValueKey('recordingPause')), findsNothing);
    });

    test('anything unknown kept is Off', () {
      store.setLocalSetting(recordingPauseKey, '4');
      expect(recordingPause(store), isNull);
      store.setLocalSetting(recordingPauseKey, 'soon');
      expect(recordingPause(store), isNull);
      expect(() => setRecordingPause(store, 4), throwsArgumentError);
    });
  });

  group('playing back', () {
    testWidgets('the card plays and stops its recording', (tester) async {
      await start(tester);
      await openVoice(tester);
      await tap(tester, stop);
      await saveCapture(tester);
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
      final note = notes.capture('sauce', recordings: [first]);
      await start(tester);
      await edit(tester, note);
      final play = find.descendant(
        of: find.byType(NoteEditorScreen),
        matching: find.byIcon(Icons.play_arrow),
      );
      await reveal(tester, play);
      await tap(tester, play);
      expect(voice.played, [first]);

      voice.nextRecording = second;
      await tap(tester, editorVoice);
      expect(voice.stoppedPlaying, 1);
      await tap(tester, stop);
      expect(notes.byId(note.id)!.recordings, hasLength(2));
      // Its buttons show play again.
      expect(
        find.descendant(
          of: find.byType(NoteEditorScreen),
          matching: find.byIcon(Icons.stop),
        ),
        findsNothing,
      );
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

      Finder play(Finder button) =>
          find.descendant(of: button, matching: find.byIcon(Icons.play_arrow));
      Finder of(String hash) =>
          find.byWidgetPredicate((w) => w is RecordingButton && w.hash == hash);
      // The merged Note's Section reads on in the Note's text (#307), the
      // Note's own recording under the badges, lower on the page.
      final section = of(merged.recordings.single);
      await reveal(tester, section);
      await tap(tester, play(section));
      expect(voice.played, [second]);
      final own = of(into.recordings.single);
      await reveal(tester, own);
      await tap(tester, play(own));
      expect(voice.played, [second, first]);
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
      await openVoice(tester);
      await tap(tester, stop);
      await saveCapture(tester);
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

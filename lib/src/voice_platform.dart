import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'l10n.dart';

/// The platform seam for voice Notes: the microphone and playing a
/// recording back (android/…/Voice.kt). Widget tests hand in a fake.
abstract interface class VoicePlatform {
  /// Whether this Device can record at all; the mic button only shows
  /// where it can.
  bool get available;

  /// Whether the user allowed the microphone. Never asks.
  Future<bool> granted();

  /// Asks the user for the microphone; whether it is allowed now.
  Future<bool> request();

  /// Starts recording; whether it did.
  Future<bool> start();

  /// Ends the recording; its bytes, or null when nothing usable was
  /// recorded.
  Future<Uint8List?> stop();

  /// Plays the recording [bytes], stopping whatever played before.
  /// Whether it plays.
  Future<bool> play(Uint8List bytes);

  /// Stops playing.
  Future<void> stopPlaying();

  /// A recording played to its end.
  Stream<void> get playbackEnded;
}

/// No microphone: the default where none is handed in.
class NoVoice implements VoicePlatform {
  const NoVoice();

  @override
  bool get available => false;

  @override
  Future<bool> granted() async => false;

  @override
  Future<bool> request() async => false;

  @override
  Future<bool> start() async => false;

  @override
  Future<Uint8List?> stop() async => null;

  @override
  Future<bool> play(Uint8List bytes) async => false;

  @override
  Future<void> stopPlaying() async {}

  @override
  Stream<void> get playbackEnded => const Stream.empty();
}

/// Android's own MediaRecorder and MediaPlayer behind the app's method
/// channel; no Google library.
class AndroidVoice implements VoicePlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/voice');
  final _ended = StreamController<void>.broadcast();

  AndroidVoice() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'played') _ended.add(null);
    });
  }

  @override
  bool get available => true;

  @override
  Future<bool> granted() => _ask('granted');

  @override
  Future<bool> request() => _ask('request');

  @override
  Future<bool> start() => _ask('start');

  @override
  Future<Uint8List?> stop() async {
    try {
      return await _channel.invokeMethod<Uint8List>('stop');
    } on PlatformException {
      return null;
    }
  }

  @override
  Future<bool> play(Uint8List bytes) => _ask('play', bytes);

  @override
  Future<void> stopPlaying() => _ask('stopPlaying');

  @override
  Stream<void> get playbackEnded => _ended.stream;

  static Future<bool> _ask(String method, [Object? arguments]) async {
    try {
      return await _channel.invokeMethod<bool>(method, arguments) ?? false;
    } on PlatformException {
      return false;
    }
  }
}

/// The one recording playing at a time, by its hash, for every play
/// button to show; null while none plays.
class Playback extends ValueNotifier<String?> {
  final VoicePlatform voice;

  /// The bytes of a stored recording by its hash, or null when they are
  /// not on this Device.
  final Uint8List? Function(String hash) bytes;

  late final StreamSubscription<void> _ended;

  Playback({required this.voice, required this.bytes}) : super(null) {
    _ended = voice.playbackEnded.listen((_) => value = null);
  }

  /// Plays the recording [hash], or stops it when it is the one playing.
  Future<void> toggle(String hash) async {
    if (value == hash) return stop();
    final recording = bytes(hash);
    if (recording == null) return;
    value = await voice.play(recording) ? hash : null;
  }

  /// Stops playing, if anything plays.
  Future<void> stop() async {
    if (value == null) return;
    value = null;
    await voice.stopPlaying();
  }

  @override
  void dispose() {
    unawaited(_ended.cancel());
    super.dispose();
  }
}

/// Hands the app's [VoicePlatform] and its [Playback] down to every
/// screen.
class VoiceScope extends InheritedWidget {
  final VoicePlatform voice;

  /// Null where nothing can be played back.
  final Playback? playback;

  const VoiceScope({
    super.key,
    required this.voice,
    this.playback,
    required super.child,
  });

  static VoiceScope? _of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<VoiceScope>();

  /// The app's [VoicePlatform]; read once, so it does not listen.
  static VoicePlatform of(BuildContext context) =>
      _of(context)?.voice ?? const NoVoice();

  /// The app's [Playback], or null where there is none.
  static Playback? playbackOf(BuildContext context) => _of(context)?.playback;

  @override
  bool updateShouldNotify(VoiceScope oldWidget) =>
      voice != oldWidget.voice || playback != oldWidget.playback;
}

/// Plays or stops the voice recording [hash]: a compact icon button on a
/// card, a labelled one in the editor when [compact] is false.
class RecordingButton extends StatelessWidget {
  final String hash;
  final bool compact;

  const RecordingButton({super.key, required this.hash, this.compact = true});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final playback = VoiceScope.playbackOf(context);
    if (playback == null) {
      return Icon(Icons.mic_none, semanticLabel: t.voiceRecording);
    }
    return ValueListenableBuilder<String?>(
      valueListenable: playback,
      builder: (context, playing, _) {
        final on = playing == hash;
        final icon = Icon(on ? Icons.stop : Icons.play_arrow);
        void onPressed() => unawaited(playback.toggle(hash));
        if (compact) {
          return IconButton.filledTonal(
            tooltip: on ? t.stopPlaying : t.playRecording,
            onPressed: onPressed,
            icon: icon,
          );
        }
        return FilledButton.tonalIcon(
          onPressed: onPressed,
          icon: icon,
          label: Text(on ? t.stopPlaying : t.voiceRecording),
        );
      },
    );
  }
}

/// Explains why voice Notes need the microphone and that the recording
/// stays on the user's Devices, then asks for it when the user agrees.
/// Returns whether it is allowed now.
Future<bool> explainAndAskMicrophone(
  BuildContext context,
  VoicePlatform voice,
) async {
  final agreed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        icon: const Icon(Icons.mic_none),
        title: Text(t.microphoneExplainTitle),
        content: Text(t.microphoneExplainBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.notNow),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.allowAccess),
          ),
        ],
      );
    },
  );
  if (agreed != true) return false;
  return voice.request();
}

/// [elapsed] as a recording's running time: minutes and seconds, hours
/// in front once there are any.
String elapsedText(Duration elapsed) {
  String two(int n) => n.toString().padLeft(2, '0');
  final seconds = two(elapsed.inSeconds % 60);
  if (elapsed.inHours > 0) {
    return '${elapsed.inHours}:${two(elapsed.inMinutes % 60)}:$seconds';
  }
  return '${elapsed.inMinutes}:$seconds';
}

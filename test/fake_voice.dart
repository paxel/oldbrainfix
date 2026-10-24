import 'dart:async';
import 'dart:typed_data';

import 'package:oldbrainfix/src/voice_platform.dart';

/// A [VoicePlatform] the test drives: whether the microphone is allowed,
/// what a recording turns out as, and what was played.
class FakeVoice implements VoicePlatform {
  /// Whether the microphone is allowed.
  bool allowed;

  /// Whether the user allows it when asked.
  bool allowOnRequest;

  /// What the next recording is when it is stopped; null as if nothing
  /// usable was recorded.
  Uint8List? nextRecording;

  /// How often the user was asked for the microphone.
  int requests = 0;

  /// How often recording was started and stopped.
  int starts = 0;
  int stops = 0;

  /// Whether a recording runs.
  bool recording = false;

  /// The recordings played, in order.
  final played = <Uint8List>[];

  /// How often playing was stopped.
  int stoppedPlaying = 0;

  final _ended = StreamController<void>.broadcast();

  FakeVoice({
    this.allowed = true,
    this.allowOnRequest = true,
    this.nextRecording,
  });

  /// The recording playing reaches its end.
  void finishPlaying() => _ended.add(null);

  @override
  bool get available => true;

  @override
  Future<bool> granted() async => allowed;

  @override
  Future<bool> request() async {
    requests++;
    allowed = allowOnRequest;
    return allowed;
  }

  @override
  Future<bool> start() async {
    if (!allowed) return false;
    starts++;
    recording = true;
    return true;
  }

  @override
  Future<Uint8List?> stop() async {
    stops++;
    recording = false;
    return nextRecording;
  }

  @override
  Future<bool> play(Uint8List bytes) async {
    played.add(bytes);
    return true;
  }

  @override
  Future<void> stopPlaying() async => stoppedPlaying++;

  @override
  Stream<void> get playbackEnded => _ended.stream;
}

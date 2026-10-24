import 'dart:async';
import 'dart:typed_data';

import 'package:oldbrainfix/src/speech_platform.dart';

/// A [SpeechPlatform] the test drives: whether Whisper runs, which model
/// is in use, what a download or pick brings, and what each recording
/// is transcribed as.
class FakeSpeech implements SpeechPlatform {
  /// Whether Whisper runs on this Device.
  bool runs;

  /// The SHA-256 of the model in use; null while there is none.
  String? installed;

  /// The SHA-256 of the model on the side; null while there is none.
  String? side;

  /// What the running download or pick brings, once the test completes
  /// it; null for "nothing".
  Completer<String?>? fetching;

  /// What each recording is transcribed as, by its bytes' last byte; a
  /// recording missing here cannot be transcribed.
  final transcripts = <int, String>{};

  /// Holds transcription back until the test completes it, when set.
  Completer<void>? transcribeGate;

  /// What was asked for, in order.
  final downloads = <Uri>[];
  final transcribed = <Uint8List>[];
  var picks = 0;
  var cancels = 0;
  var discards = 0;
  var removes = 0;
  var idles = 0;

  final _progress = StreamController<ModelProgress>.broadcast();

  FakeSpeech({this.runs = true, this.installed});

  /// The download has come [received] bytes of [total].
  void report(int received, int? total) =>
      _progress.add((received: received, total: total));

  /// The running download or pick brings a model with [hash], or
  /// nothing when null.
  void bring(String? hash) {
    final fetching = this.fetching;
    this.fetching = null;
    fetching?.complete(hash);
  }

  Future<String?> _fetch() async {
    final fetching = this.fetching = Completer<String?>();
    final hash = await fetching.future;
    side = hash;
    return hash;
  }

  @override
  bool get available => true;

  @override
  Future<bool> supported() async => runs;

  @override
  Future<String?> modelHash() async => installed;

  @override
  Future<String?> download(Uri url) {
    downloads.add(url);
    return _fetch();
  }

  @override
  Stream<ModelProgress> get progress => _progress.stream;

  @override
  Future<String?> pick() {
    picks++;
    return _fetch();
  }

  @override
  Future<void> cancel() async {
    cancels++;
    bring(null);
  }

  @override
  Future<bool> install() async {
    if (side == null) return false;
    installed = side;
    side = null;
    return true;
  }

  @override
  Future<void> discard() async {
    discards++;
    side = null;
  }

  @override
  Future<void> remove() async {
    removes++;
    installed = null;
  }

  @override
  Future<String?> transcribe(Uint8List bytes) async {
    transcribed.add(bytes);
    await transcribeGate?.future;
    return transcripts[bytes.last];
  }

  @override
  Future<void> idle() async => idles++;
}

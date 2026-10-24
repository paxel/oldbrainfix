import 'dart:async';

import 'package:oldbrainfix/src/capture_requests.dart';

/// A [CaptureRequests] the test drives: a shortcut, tile, widget or
/// hardware button asking for a capture screen.
class FakeCaptureRequests implements CaptureRequests {
  /// The capture asked for and not taken yet; one at a time, as on the
  /// Device.
  CaptureMode? waiting;

  /// How often the app looked for a request.
  int looked = 0;

  final _arrived = StreamController<void>.broadcast();

  FakeCaptureRequests({this.waiting});

  /// An entry point asks for [mode] while the app runs.
  void request(CaptureMode mode) {
    waiting = mode;
    _arrived.add(null);
  }

  @override
  Future<CaptureMode?> take() async {
    looked++;
    final mode = waiting;
    waiting = null;
    return mode;
  }

  @override
  Stream<void> get arrived => _arrived.stream;
}

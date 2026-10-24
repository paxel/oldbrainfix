import 'dart:async';

import 'package:flutter/services.dart';

/// Which capture screen a quick entry point opens.
enum CaptureMode { text, voice, photo }

/// The platform seam for the quick entry points: app shortcuts, Quick
/// Settings tiles, widgets and hardware buttons, each asking for one
/// [CaptureMode] (android/…/CaptureEntry.kt). Widget tests hand in a
/// fake.
abstract interface class CaptureRequests {
  /// The capture asked for and not opened yet, or null; taking clears it.
  Future<CaptureMode?> take();

  /// A capture was asked for while the app runs.
  Stream<void> get arrived;
}

/// No entry points: the default where none is handed in.
class NoCaptureRequests implements CaptureRequests {
  const NoCaptureRequests();

  @override
  Future<CaptureMode?> take() async => null;

  @override
  Stream<void> get arrived => const Stream.empty();
}

/// Android's shortcuts, tiles and widgets behind the app's method
/// channel.
class AndroidCaptureRequests implements CaptureRequests {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/capture');
  final _arrived = StreamController<void>.broadcast();

  AndroidCaptureRequests() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'requested') _arrived.add(null);
    });
  }

  @override
  Future<CaptureMode?> take() async {
    final String? name;
    try {
      name = await _channel.invokeMethod<String>('take');
    } on PlatformException {
      return null;
    }
    for (final mode in CaptureMode.values) {
      if (mode.name == name) return mode;
    }
    return null;
  }

  @override
  Stream<void> get arrived => _arrived.stream;
}

import 'dart:async';
import 'dart:typed_data';

import 'package:oldbrainfix/src/photo_platform.dart';

/// A valid 1×1 PNG: photo bytes widget tests can decode.
final Uint8List onePixelPng = Uint8List.fromList(const [
  0x89,
  0x50,
  0x4E,
  0x47,
  0x0D,
  0x0A,
  0x1A,
  0x0A,
  0x00,
  0x00,
  0x00,
  0x0D,
  0x49,
  0x48,
  0x44,
  0x52,
  0x00,
  0x00,
  0x00,
  0x01,
  0x00,
  0x00,
  0x00,
  0x01,
  0x08,
  0x06,
  0x00,
  0x00,
  0x00,
  0x1F,
  0x15,
  0xC4,
  0x89,
  0x00,
  0x00,
  0x00,
  0x0D,
  0x49,
  0x44,
  0x41,
  0x54,
  0x78,
  0x9C,
  0x63,
  0xF8,
  0xCF,
  0xC0,
  0xF0,
  0x1F,
  0x00,
  0x05,
  0x00,
  0x01,
  0xFF,
  0x89,
  0x99,
  0x3D,
  0x1D,
  0x00,
  0x00,
  0x00,
  0x00,
  0x49,
  0x45,
  0x4E,
  0x44,
  0xAE,
  0x42,
  0x60,
  0x82,
]);

/// Another valid photo, different bytes: [onePixelPng] with a trailing
/// byte after its end.
final Uint8List otherPng = Uint8List.fromList([...onePixelPng, 0]);

/// A [PhotoPlatform] the test drives: what the camera hands back, and
/// what other apps share in.
class FakePhotos implements PhotoPlatform {
  /// What the next photo taken is; null as if the user backed out.
  Uint8List? nextPhoto;

  /// When set, taking a photo waits for it, as while the camera app is
  /// open; it overrides [nextPhoto].
  Completer<Uint8List?>? camera;

  /// How often the camera was opened.
  int taken = 0;

  /// The shares not taken yet, oldest first.
  final waiting = <SharedItem>[];

  final _arrived = StreamController<void>.broadcast();

  FakePhotos({this.nextPhoto});

  /// Another app shares [item] while the app runs.
  void share(SharedItem item) {
    waiting.add(item);
    _arrived.add(null);
  }

  @override
  bool get available => true;

  @override
  Future<Uint8List?> takePhoto() {
    taken++;
    return camera?.future ?? Future.value(nextPhoto);
  }

  @override
  Future<SharedItem?> takeShared() async =>
      waiting.isEmpty ? null : waiting.removeAt(0);

  @override
  Stream<void> get sharedArrived => _arrived.stream;
}

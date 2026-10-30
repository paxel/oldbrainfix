import 'dart:async';
import 'dart:typed_data';

import 'package:oldbrainfix/src/ocr_platform.dart';

/// An [OcrPlatform] the test drives: whether RustO runs, and what text
/// each photo holds.
class FakeOcr implements OcrPlatform {
  /// Whether RustO runs on this Device.
  bool runs;

  /// The text in each photo, by its bytes' length; a photo missing here
  /// cannot be read.
  final texts = <int, String>{};

  /// Holds reading back until the test completes it, when set.
  Completer<void>? readGate;

  /// What was asked for, in order.
  final readPhotos = <Uint8List>[];
  var idles = 0;

  FakeOcr({this.runs = true});

  @override
  Future<bool> supported() async => runs;

  @override
  Future<String?> read(Uint8List jpeg) async {
    readPhotos.add(jpeg);
    await readGate?.future;
    return texts[jpeg.length];
  }

  @override
  Future<void> idle() async => idles++;
}

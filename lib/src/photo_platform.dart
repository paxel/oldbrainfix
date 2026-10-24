import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Something another app shared into Old Brain Fix: its text, links
/// included, and its photos.
class SharedItem {
  final String text;
  final List<Uint8List> photos;

  const SharedItem({this.text = '', this.photos = const []});

  bool get isEmpty => text.trim().isEmpty && photos.isEmpty;
}

/// The text of a share: the [subject] a sharing app sends with a link,
/// such as a page's title, goes on top of the [text] unless the text
/// already holds it.
String sharedText(String? subject, String? text) {
  final s = subject?.trim() ?? '';
  final t = text?.trim() ?? '';
  if (s.isEmpty) return t;
  if (t.isEmpty) return s;
  if (t.contains(s)) return t;
  return '$s\n$t';
}

/// The platform seam for photos: the camera and what other apps share in
/// (android/…/Photos.kt, ShareActivity.kt). Widget tests hand in a fake.
abstract interface class PhotoPlatform {
  /// Whether this Device has a camera to take photos with; the camera
  /// button only shows where it has.
  bool get available;

  /// Takes a photo with the Device's camera app; its bytes as a JPEG
  /// without metadata, or null when none was taken.
  Future<Uint8List?> takePhoto();

  /// What was shared in and not taken yet, or null; taking clears it.
  Future<SharedItem?> takeShared();

  /// Something was shared in while the app runs.
  Stream<void> get sharedArrived;
}

/// No camera and no shares: the default where none is handed in.
class NoPhotos implements PhotoPlatform {
  const NoPhotos();

  @override
  bool get available => false;

  @override
  Future<Uint8List?> takePhoto() async => null;

  @override
  Future<SharedItem?> takeShared() async => null;

  @override
  Stream<void> get sharedArrived => const Stream.empty();
}

/// Android's own camera intent and share target behind the app's method
/// channel; no Google library.
class AndroidPhotos implements PhotoPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/photos');
  final _shared = StreamController<void>.broadcast();

  AndroidPhotos() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'shared') _shared.add(null);
    });
  }

  @override
  bool get available => true;

  @override
  Future<Uint8List?> takePhoto() async {
    try {
      return await _channel.invokeMethod<Uint8List>('take');
    } on PlatformException {
      return null;
    }
  }

  @override
  Future<SharedItem?> takeShared() async {
    final Map<String, Object?>? raw;
    try {
      raw = await _channel.invokeMapMethod<String, Object?>('shared');
    } on PlatformException {
      return null;
    }
    if (raw == null) return null;
    final item = SharedItem(
      text: sharedText(raw['subject'] as String?, raw['text'] as String?),
      photos: [
        for (final photo in raw['photos'] as List? ?? const [])
          if (photo is Uint8List) photo,
      ],
    );
    return item.isEmpty ? null : item;
  }

  @override
  Stream<void> get sharedArrived => _shared.stream;
}

/// Hands the app's [PhotoPlatform] and the stored photo bytes down to
/// every screen.
class PhotoScope extends InheritedWidget {
  final PhotoPlatform photos;

  /// The bytes of a stored photo by its hash, or null when they are not
  /// on this Device.
  final Uint8List? Function(String hash) bytes;

  const PhotoScope({
    super.key,
    required this.photos,
    required this.bytes,
    required super.child,
  });

  static PhotoScope? _of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PhotoScope>();

  /// The app's [PhotoPlatform]; read once, so it does not listen.
  static PhotoPlatform of(BuildContext context) =>
      _of(context)?.photos ?? const NoPhotos();

  /// The bytes of the stored photo [hash], or null.
  static Uint8List? bytesOf(BuildContext context, String hash) =>
      _of(context)?.bytes(hash);

  @override
  bool updateShouldNotify(PhotoScope oldWidget) =>
      photos != oldWidget.photos || bytes != oldWidget.bytes;
}

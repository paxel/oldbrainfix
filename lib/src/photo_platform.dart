import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Something another app shared into Old Brain Fix: its text, links
/// included, its photos, its sound files as voice recordings, and the app
/// it came from when Android said.
class SharedItem {
  final String text;
  final List<Uint8List> photos;
  final List<SharedRecording> recordings;
  final SourceApp? sourceApp;

  const SharedItem({
    this.text = '',
    this.photos = const [],
    this.recordings = const [],
    this.sourceApp,
  });

  bool get isEmpty =>
      text.trim().isEmpty && photos.isEmpty && recordings.isEmpty;
}

/// Why a sound file shared in cannot be transcribed on this Device.
enum SoundProblem {
  /// No sound was found in it: not a sound file, or a damaged one.
  unreadable,

  /// This Device has no decoder for its format.
  unsupported,
}

/// A sound file shared in, kept as it is as a voice recording; [problem]
/// says why it cannot be transcribed, null when it can, and [format] is
/// its MIME type when that is known.
class SharedRecording {
  final Uint8List bytes;
  final SoundProblem? problem;
  final String? format;

  const SharedRecording(this.bytes, {this.problem, this.format});
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

/// What became of a photo saved into the Device's Pictures (#149).
enum PhotoSaved {
  /// It is in the Device's Pictures.
  saved,

  /// This Android is older than 10, where saving there needs a storage
  /// permission the app does not ask for.
  tooOld,

  /// It could not be written there.
  failed,
}

/// The platform seam for photos: the camera, the system's picture picker,
/// what other apps share in, sound files included, the clipboard for
/// Paste, and a photo handed out of its Note (android/…/Photos.kt,
/// ShareActivity.kt, Clipboards.kt, PhotoOut.kt). Widget tests hand in a
/// fake.
abstract interface class PhotoPlatform {
  /// Whether this Device has a camera to take photos with; the camera
  /// button only shows where it has.
  bool get available;

  /// Whether this Device has a picker for its pictures; the Gallery
  /// button only shows where it has.
  bool get pickable;

  /// Takes a photo with the Device's camera app; its bytes as a JPEG
  /// without metadata, or null when none was taken.
  Future<Uint8List?> takePhoto();

  /// Opens the system's picture picker, several pictures at once; each
  /// picked one as a JPEG without metadata, none when the picker was
  /// closed without picking.
  Future<List<Uint8List>> pickPhotos();

  /// Whether this Device's clipboard can be read for Paste; the Paste
  /// buttons only show where it can.
  bool get pasteable;

  /// What the clipboard holds, read now and only now, as Paste was
  /// tapped: its text and its pictures, each as a JPEG without metadata;
  /// null when it holds neither or cannot be read.
  Future<SharedItem?> takeClipboard();

  /// What was shared in and not taken yet, or null; taking clears it.
  Future<SharedItem?> takeShared();

  /// Something was shared in while the app runs.
  Stream<void> get sharedArrived;

  /// Saves the photo file at [path], written where
  /// [BundlePlatform.exportPath] said, into the Device's Pictures as
  /// [name] of MIME [type] (#149).
  Future<PhotoSaved> savePhoto(String path, String name, String type);

  /// Puts the photo file at [path], written as for [savePhoto], of MIME
  /// [type] on the clipboard as a picture (#149); whether it is there.
  Future<bool> copyPhoto(String path, String type);
}

/// No camera and no shares: the default where none is handed in.
class NoPhotos implements PhotoPlatform {
  const NoPhotos();

  @override
  bool get available => false;

  @override
  bool get pickable => false;

  @override
  Future<Uint8List?> takePhoto() async => null;

  @override
  Future<List<Uint8List>> pickPhotos() async => const [];

  @override
  bool get pasteable => false;

  @override
  Future<SharedItem?> takeClipboard() async => null;

  @override
  Future<SharedItem?> takeShared() async => null;

  @override
  Stream<void> get sharedArrived => const Stream.empty();

  @override
  Future<PhotoSaved> savePhoto(String path, String name, String type) async =>
      PhotoSaved.failed;

  @override
  Future<bool> copyPhoto(String path, String type) async => false;
}

/// Android's own camera intent, picture picker and share target behind the
/// app's method channel; no Google library.
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
  bool get pickable => true;

  @override
  Future<Uint8List?> takePhoto() async {
    try {
      return await _channel.invokeMethod<Uint8List>('take');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  @override
  Future<List<Uint8List>> pickPhotos() async {
    try {
      final picked = await _channel.invokeListMethod<Object?>('pick');
      return [...?picked?.whereType<Uint8List>()];
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }

  @override
  bool get pasteable => true;

  @override
  Future<SharedItem?> takeClipboard() async {
    final Map<String, Object?>? raw;
    try {
      raw = await _channel.invokeMapMethod<String, Object?>('paste');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
    if (raw == null) return null;
    final item = SharedItem(
      text: raw['text'] as String? ?? '',
      photos: [
        for (final photo in raw['photos'] as List? ?? const [])
          if (photo is Uint8List) photo,
      ],
    );
    return item.isEmpty ? null : item;
  }

  @override
  Future<SharedItem?> takeShared() async {
    final Map<String, Object?>? raw;
    try {
      raw = await _channel.invokeMapMethod<String, Object?>('shared');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
    if (raw == null) return null;
    final package = raw['sourcePackage'];
    final item = SharedItem(
      text: sharedText(raw['subject'] as String?, raw['text'] as String?),
      photos: [
        for (final photo in raw['photos'] as List? ?? const [])
          if (photo is Uint8List) photo,
      ],
      recordings: [
        for (final recording in raw['recordings'] as List? ?? const [])
          if (recording case {'bytes': final Uint8List bytes})
            SharedRecording(
              bytes,
              problem: switch (recording['problem']) {
                'unreadable' => SoundProblem.unreadable,
                'unsupported' => SoundProblem.unsupported,
                _ => null,
              },
              format: recording['format'] as String?,
            ),
      ],
      sourceApp: package is String && package.isNotEmpty
          ? SourceApp(package, label: raw['sourceLabel'] as String?)
          : null,
    );
    return item.isEmpty ? null : item;
  }

  @override
  Stream<void> get sharedArrived => _shared.stream;

  @override
  Future<PhotoSaved> savePhoto(String path, String name, String type) async {
    final String? saved;
    try {
      saved = await _channel.invokeMethod<String>('save', {
        'path': path,
        'name': name,
        'type': type,
      });
    } on PlatformException {
      return PhotoSaved.failed;
    } on MissingPluginException {
      return PhotoSaved.failed;
    }
    return switch (saved) {
      'saved' => PhotoSaved.saved,
      'old' => PhotoSaved.tooOld,
      _ => PhotoSaved.failed,
    };
  }

  @override
  Future<bool> copyPhoto(String path, String type) async {
    try {
      return await _channel.invokeMethod<bool>('copy', {
            'path': path,
            'type': type,
          }) ??
          false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }
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

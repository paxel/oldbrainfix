import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// A file on this Device, picked with the system's file picker or shared
/// in by another app (#267, ADR 0047): a copy the app made in its cache
/// at [path] while it could read it, its [name] and its media [type] as
/// the app it came from said, "" where it said none. The copy goes once
/// it is attached, or left.
class DeviceFile {
  final String path;
  final String name;
  final String type;

  const DeviceFile({required this.path, required this.name, this.type = ''});
}

/// One page of a PDF read by the app's native library (`oldbrainfix-pdf`
/// beside RustO, ADR 0047): its own [text], and its picture as a JPEG or
/// PNG when it is a scan whose text the OCR reads.
class PdfPage {
  final String text;
  final Uint8List? scan;

  const PdfPage({this.text = '', this.scan});
}

/// The platform seam for Attachments: the system's file picker, opening
/// a file with the system's app, the PDF reader of the app's native
/// library (android/…/Files.kt, android/ocr; the iPhone's FilesChannel),
/// and the system's own PDF renderer for the viewer (#286).
/// Widget tests hand in a fake.
abstract interface class FilePlatform {
  /// Whether this Device has a file picker; the File button only shows
  /// where it has.
  bool get pickable;

  /// Opens the system's file picker, several files of any kind at once;
  /// none when it was closed without picking.
  Future<List<DeviceFile>> pickFiles();

  /// Opens the file at [path], written where [BundlePlatform.exportPath]
  /// said, of media [type] with the system's app for it; whether one
  /// took it.
  Future<bool> open(String path, String type);

  /// How many pages the PDF at [path] has; null where this Device has no
  /// PDF reader or the file reads as none.
  Future<int?> pdfPages(String path);

  /// The page [index] of the PDF at [path], counted from 0; null where it
  /// cannot be read.
  Future<PdfPage?> pdfPage(String path, int index);

  /// The size of each page of the PDF at [path] in points, as this
  /// Device's own renderer reads it (#286): Android's `PdfRenderer`, the
  /// iPhone's PDFKit; null where it cannot show the PDF, as one locked
  /// with a password.
  Future<List<Size>?> pdfLayout(String path);

  /// The page [index] of the PDF at [path] drawn on white [width] pixels
  /// wide, at most [maxRenderSide] pixels a side and [maxRenderPixels] in
  /// all, as a PNG; null where it cannot be drawn.
  Future<Uint8List?> pdfRender(String path, int index, int width);
}

/// The longest side a PDF page is drawn at, in pixels (#286).
const maxRenderSide = 4096;

/// The most pixels a PDF page is drawn with (#286): 32 MB as a bitmap.
const maxRenderPixels = 8 * 1024 * 1024;

/// The sizes of a channel's answer to `pdfLayout`: each page's width and
/// height in points, one after the other; null for anything else.
List<Size>? pdfSizes(List<Object?>? raw) {
  if (raw == null || raw.length.isOdd) return null;
  final sizes = <Size>[];
  for (var i = 0; i < raw.length; i += 2) {
    final (width, height) = (raw[i], raw[i + 1]);
    if (width is! num || height is! num || width <= 0 || height <= 0) {
      return null;
    }
    sizes.add(Size(width.toDouble(), height.toDouble()));
  }
  return sizes;
}

/// No picker and no reader: the default where none is handed in.
class NoFiles implements FilePlatform {
  const NoFiles();

  @override
  bool get pickable => false;

  @override
  Future<List<DeviceFile>> pickFiles() async => const [];

  @override
  Future<bool> open(String path, String type) async => false;

  @override
  Future<int?> pdfPages(String path) async => null;

  @override
  Future<PdfPage?> pdfPage(String path, int index) async => null;

  @override
  Future<List<Size>?> pdfLayout(String path) async => null;

  @override
  Future<Uint8List?> pdfRender(String path, int index, int width) async => null;
}

/// The app's own channel to Android's file picker and views and to the
/// PDF reader built into its native library; the iPhone answers on the
/// same channel. No Google library.
class AndroidFiles implements FilePlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/files');

  const AndroidFiles();

  @override
  bool get pickable => true;

  @override
  Future<List<DeviceFile>> pickFiles() async =>
      deviceFiles(await _ask<List<Object?>>('pick'));

  @override
  Future<bool> open(String path, String type) async =>
      await _ask<bool>('open', {'path': path, 'type': type}) ?? false;

  @override
  Future<int?> pdfPages(String path) async {
    final pages = await _ask<int>('pdfPages', path);
    return pages == null || pages < 0 ? null : pages;
  }

  @override
  Future<PdfPage?> pdfPage(String path, int index) async {
    final page = await _ask<Map<Object?, Object?>>('pdfPage', {
      'path': path,
      'index': index,
    });
    if (page == null) return null;
    return PdfPage(
      text: page['text'] as String? ?? '',
      scan: page['scan'] as Uint8List?,
    );
  }

  @override
  Future<List<Size>?> pdfLayout(String path) async =>
      pdfSizes(await _ask<List<Object?>>('pdfLayout', path));

  @override
  Future<Uint8List?> pdfRender(String path, int index, int width) =>
      _ask<Uint8List>('pdfRender', {
        'path': path,
        'index': index,
        'width': width,
      });

  static Future<T?> _ask<T>(String method, [Object? arguments]) async {
    try {
      return await _channel.invokeMethod<T>(method, arguments);
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }
}

/// The files of a channel's answer: each a map of its `path`, `name` and
/// `type`; anything else is left out.
List<DeviceFile> deviceFiles(List<Object?>? raw) => [
  for (final file in raw ?? const <Object?>[])
    if (file case {'path': final String path} when path.isNotEmpty)
      DeviceFile(
        path: path,
        name: switch (file['name']) {
          final String name => name,
          _ => '',
        },
        type: switch (file['type']) {
          final String type => type,
          _ => '',
        },
      ),
];

/// Lets go of the copies [files] made on this Device, once attached or
/// left; one already gone is fine.
void dropDeviceFiles(Iterable<DeviceFile> files) {
  for (final file in files) {
    try {
      File(file.path).deleteSync();
    } on FileSystemException {
      // Gone already.
    }
  }
}

/// Hands the app's [FilePlatform] down to every screen.
class FileScope extends InheritedWidget {
  final FilePlatform files;

  const FileScope({super.key, required this.files, required super.child});

  /// The app's [FilePlatform]; read once, so it does not listen.
  static FilePlatform of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<FileScope>()?.files ??
      const NoFiles();

  @override
  bool updateShouldNotify(FileScope oldWidget) => files != oldWidget.files;
}

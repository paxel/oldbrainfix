import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix/src/file_platform.dart';

/// A [FilePlatform] the test drives: what the file picker hands back, as
/// copies in [dir], what opens with another app, and the PDFs the
/// native reader reads, by their bytes' length.
class FakeFiles implements FilePlatform {
  /// Where the picked files are put, as Android copies them aside.
  final Directory dir;

  /// The files the next pick hands back, by their names and bytes, with
  /// their media types; empty as if the picker was closed.
  List<({String name, Uint8List bytes, String type})> nextPicked = const [];

  /// How often the picker was opened.
  int picks = 0;

  /// The files opened with another app, with their types and bytes.
  final opened = <({String type, List<int> bytes})>[];

  /// Whether another app takes a file to open.
  bool opens = true;

  /// Each PDF the reader knows, by its bytes' length: its pages.
  final pdfs = <int, List<PdfPage>>{};

  /// What the reader was asked for, in order.
  final read = <String>[];

  FakeFiles(this.dir);

  /// [nextPicked] written into [dir] as the picker's copies.
  List<DeviceFile> copies() => [
    for (final (i, file) in nextPicked.indexed)
      () {
        final copy = File('${dir.path}/picked-$picks-$i')
          ..writeAsBytesSync(file.bytes);
        return DeviceFile(path: copy.path, name: file.name, type: file.type);
      }(),
  ];

  @override
  bool get pickable => true;

  @override
  Future<List<DeviceFile>> pickFiles() async {
    picks++;
    return copies();
  }

  @override
  Future<bool> open(String path, String type) async {
    opened.add((type: type, bytes: File(path).readAsBytesSync()));
    return opens;
  }

  @override
  Future<int?> pdfPages(String path) async {
    read.add('pages');
    return pdfs[File(path).lengthSync()]?.length;
  }

  @override
  Future<PdfPage?> pdfPage(String path, int index) async {
    read.add('page $index');
    final pages = pdfs[File(path).lengthSync()];
    return pages == null || index >= pages.length ? null : pages[index];
  }
}

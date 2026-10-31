import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';

/// Reads the files of the [folder] picked through [folders] and makes
/// Notes of them in a new Area named [areaName] (ADR 0036). Only what
/// may become a Note is read, and none larger than
/// [Imports.maxFileBytes]; every other file is handed over unread, so
/// the core counts it as skipped. A file that cannot be read throws, and
/// then nothing is written.
Future<ImportSummary> importFolder(
  Store store,
  FolderPlatform folders,
  String folder,
  String areaName,
) async {
  final files = <ImportFile>[];
  for (final file in await folders.files(folder)) {
    files.add(await _read(folders, file));
  }
  return Imports(store).folder(areaName, files);
}

Future<ImportFile> _read(FolderPlatform folders, FolderFile file) async {
  if (!mayBecomeNote(file.path)) {
    return ImportFile(file.path, file.modified, Uint8List(0));
  }
  final bytes = file.size > Imports.maxFileBytes
      ? null
      : await folders.readFile(file, Imports.maxFileBytes);
  return ImportFile(file.path, file.modified, bytes ?? _tooLarge);
}

/// What the core takes as larger than it reads, handed over for every
/// such file: only its length counts, so one serves them all.
final _tooLarge = Uint8List(Imports.maxFileBytes + 1);

/// Whether the file at [path] may become a Note, as the core takes it:
/// a Markdown (`.md`, `.markdown`) or text (`.txt`) file, in any case of
/// ASCII letters, not a hidden one. Others are not worth reading.
bool mayBecomeNote(String path) {
  final name = path.substring(path.lastIndexOf('/') + 1);
  if (name.startsWith('.')) return false;
  final dot = name.lastIndexOf('.');
  if (dot < 0) return false;
  final extension = String.fromCharCodes([
    for (final c in name.substring(dot + 1).codeUnits)
      c >= 0x41 && c <= 0x5a ? c + 0x20 : c,
  ]);
  return const {'md', 'markdown', 'txt'}.contains(extension);
}

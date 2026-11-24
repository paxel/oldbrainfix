import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';
import 'import_run.dart';

/// Reads the files of the [folder] picked through [folders] and makes
/// Notes of them in a new Area named [areaName] (ADR 0036). Only what
/// may become a Note is read, and none larger than
/// [Imports.maxFileBytes]; every other file is handed over unread, so
/// the core counts it as skipped. A file that cannot be read throws, and
/// then nothing is written. [progress] hears how far the writing is;
/// [stop] is asked before each Note (obf-1078). The files [listed]
/// already, by [folderLook], are not listed again; with [fillIn], the
/// Area of an earlier Import of the [folder], only those it did not
/// make go into it (obf-1078).
Future<ImportSummary> importFolder(
  Store store,
  FolderPlatform folders,
  String folder,
  String areaName, {
  List<FolderFile>? listed,
  String? fillIn,
  ImportProgress? progress,
  Stopping? stop,
}) async {
  final files = <ImportFile>[];
  for (final file in listed ?? await folders.files(folder)) {
    files.add(await _read(folders, file));
  }
  return Imports(store).folder(
    areaName,
    files,
    from: folder,
    fillIn: fillIn,
    progress: progress,
    stop: stop,
  );
}

/// What a look at a picked source found before any of its files is read
/// (obf-1078): what it lists, and the [earlier] Import of it a new one
/// may fill in, null when there is none.
typedef Looked<L> = ({L listed, EarlierImport? earlier});

/// The files of [folder] and the earlier Import of it, as a job for a
/// [JobRunner] (obf-1078).
LongWork<Looked<List<FolderFile>>> folderLook(String folder) =>
    (store, folders, progress, stop) async {
      final files = await folders.files(folder);
      return (
        listed: files,
        earlier: Imports(store).earlier(ImportSourceKind.folder, folder, [
          for (final file in files) file.path,
        ]),
      );
    };

/// [importFolder] of [folder] into [areaName] as a job for a
/// [JobRunner] (obf-1061), of the files [listed] and filling in [fillIn]
/// when given (obf-1078).
ImportWork folderImport(
  String folder,
  String areaName, {
  List<FolderFile>? listed,
  String? fillIn,
}) =>
    (store, folders, progress, stop) => importFolder(
      store,
      folders,
      folder,
      areaName,
      listed: listed,
      fillIn: fillIn,
      progress: progress,
      stop: stop,
    );

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
/// a Markdown (`.md`, `.markdown`), text (`.txt`), AsciiDoc (`.adoc`,
/// `.asciidoc`, `.asc`), Org (`.org`), reStructuredText (`.rst`) or HTML
/// (`.html`, `.htm`) file, in any case of ASCII letters, not a hidden
/// one. Others are not worth reading.
bool mayBecomeNote(String path) {
  final name = path.substring(path.lastIndexOf('/') + 1);
  if (name.startsWith('.')) return false;
  final dot = name.lastIndexOf('.');
  if (dot < 0) return false;
  final extension = String.fromCharCodes([
    for (final c in name.substring(dot + 1).codeUnits)
      c >= 0x41 && c <= 0x5a ? c + 0x20 : c,
  ]);
  return const {'md', 'markdown', 'txt'}.contains(extension) ||
      ImportFormats.ofExtension(extension) != null;
}

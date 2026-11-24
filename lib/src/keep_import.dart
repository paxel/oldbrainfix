import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';
import 'import_run.dart';

/// Reads the Google Keep notes of the Takeout export in the [folder]
/// picked through [folders], unpacked, and makes Notes of them in a new
/// Area named [areaName] (ADR 0036, Google Keep). Only the files
/// [Imports.keepReads] takes are handed over, none read beyond its
/// limit. A file that cannot be read throws, and then nothing is
/// written. [progress] hears how far the writing is; [stop] is asked
/// before each Note (obf-1078).
Future<ImportSummary> importKeepFolder(
  Store store,
  FolderPlatform folders,
  String folder,
  String areaName, {
  ImportProgress? progress,
  Stopping? stop,
}) async {
  final files = await folders.files(folder);
  final kept = _ofKeep([for (final f in files) f.path]).toSet();
  final over = <int, Uint8List>{};
  return Imports(store).keep(
    areaName,
    [
      for (final file in files)
        if (kept.contains(file.path)) ?await _read(folders, file, over),
    ],
    progress: progress,
    stop: stop,
  );
}

/// As [importKeepFolder] for the Takeout export as the [zip] picked
/// through [folders]: the zip's files the core takes are copied aside,
/// none beyond its limit and one byte more, read from there and deleted
/// afterwards, also when the Import fails.
Future<ImportSummary> importKeepZip(
  Store store,
  FolderPlatform folders,
  String zip,
  String areaName, {
  ImportProgress? progress,
  Stopping? stop,
}) async {
  try {
    final wanted = {
      for (final path in _ofKeep(await folders.zipEntries(zip)))
        path: ?Imports.keepReads(path),
    };
    final entries = <ImportFile>[];
    final over = <int, Uint8List>{};
    for (final file in await folders.unzip(zip, wanted)) {
      if (await _read(folders, file, over) case final entry?) {
        entries.add(entry);
      }
    }
    return Imports(store)
        .keep(areaName, entries, progress: progress, stop: stop);
  } finally {
    await folders.dropUnzipped();
  }
}

/// [importKeepZip] of [zip], or [importKeepFolder] of [source] when it
/// is no [zip], into [areaName], as a job for a [JobRunner]
/// (obf-1061).
ImportWork keepImport(String source, String areaName, {required bool zip}) =>
    (store, folders, progress, stop) => zip
    ? importKeepZip(
        store,
        folders,
        source,
        areaName,
        progress: progress,
        stop: stop,
      )
    : importKeepFolder(
        store,
        folders,
        source,
        areaName,
        progress: progress,
        stop: stop,
      );

/// The paths of [paths] the core reads of a Keep export: those
/// [Imports.keepReads] takes, and when any of them lies in a folder named
/// `Keep`, as in a Takeout of several Google products, only those, as
/// the core counts them; so no photo of another product is read.
List<String> _ofKeep(List<String> paths) {
  final taken = [
    for (final path in paths)
      if (Imports.keepReads(path) != null) path,
  ];
  final inKeep = taken.where(_inKeep).toList();
  return inKeep.isEmpty ? taken : inKeep;
}

/// Whether the file at [path] lies in a folder named `Keep`, in any case
/// of ASCII letters, as the core takes it.
bool _inKeep(String path) {
  final folders = path.split(RegExp(r'[/\\]'));
  if (folders.length < 2) return false;
  final name = String.fromCharCodes([
    for (final c in folders[folders.length - 2].codeUnits)
      c >= 0x41 && c <= 0x5a ? c + 0x20 : c,
  ]);
  return name == 'keep';
}

/// [file] by its path with its bytes, or as larger than the core reads:
/// its limit and one byte more, never read; null for a file the core
/// does not read. What stands for such a file
/// is kept in [over] by its limit: only its length counts, so one serves
/// every file of that limit.
Future<ImportFile?> _read(
  FolderPlatform folders,
  FolderFile file,
  Map<int, Uint8List> over,
) async {
  final limit = Imports.keepReads(file.path);
  if (limit == null) return null;
  final bytes = file.size > limit ? null : await folders.readFile(file, limit);
  return ImportFile(
    file.path,
    null,
    bytes ?? over.putIfAbsent(limit, () => Uint8List(limit + 1)),
  );
}

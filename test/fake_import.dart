import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A folder to import, in memory: the picker answers [picks], and its
/// files are those [put] in, by their path inside it.
class FakeImportFolders implements FolderPlatform {
  /// What the picker answers; null backs out.
  String? picks = 'content://tree/primary%3ANotes';

  /// The folder's own name.
  String name = 'Notes';

  /// Throws on listing, as a vanished drive does.
  bool fails = false;

  /// Holds the listing until it completes, as a slow drive does.
  Completer<void>? hold;

  final _files = <FolderFile>[];
  final _bytes = <String, Uint8List>{};

  /// The paths read, in order.
  final read = <String>[];

  /// Each folder whose grant was given up, and the folders kept then.
  final released = <String>[];
  final kept = <List<String>>[];

  /// Puts a file at [path] changed [modified], holding [bytes]; [size]
  /// says what the platform says of it, its length when not given.
  void put(String path, DateTime? modified, List<int> bytes, {int? size}) {
    _files.add(
      FolderFile(
        path: path,
        modified: modified,
        size: size ?? bytes.length,
        ref: 'doc:$path',
      ),
    );
    _bytes['doc:$path'] = Uint8List.fromList(bytes);
  }

  @override
  bool get available => true;

  @override
  Future<String?> pick() async => picks;

  @override
  Future<String> displayName(String folder) async => name;

  @override
  SyncFolder open(String folder) => MemorySyncFolder();

  @override
  BackupFolder openBackups(String folder) => LocalBackupFolder(folder);

  @override
  Future<List<FolderFile>> files(String folder) async {
    await hold?.future;
    if (fails) throw FileSystemException('Gone', folder);
    return [..._files];
  }

  @override
  Future<Uint8List?> readFile(FolderFile file, int limit) async {
    read.add(file.path);
    final bytes = _bytes[file.ref]!;
    return bytes.length > limit ? null : bytes;
  }

  @override
  Future<void> release(String folder, List<String> keep) async {
    released.add(folder);
    kept.add(keep);
  }

  @override
  Future<String?> autoBackupDir() async => null;

  @override
  Future<void> setAutoBackup(bool on) async {}
}

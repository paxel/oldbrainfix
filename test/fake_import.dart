import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A folder or a zip to import, in memory: the folder picker answers
/// [picks], and its files are those [put] in, by their path inside it;
/// the file picker answers [zipPicks], and the zip's files are those
/// [putZip] in.
class FakeImportFolders implements FolderPlatform {
  /// What the picker answers; null backs out.
  String? picks = 'content://tree/primary%3ANotes';

  /// The folder's own name.
  String name = 'Notes';

  /// Throws on listing, as a vanished drive does.
  bool fails = false;

  /// Holds the listing until it completes, as a slow drive does.
  Completer<void>? hold;

  /// What the file picker answers; null backs out.
  String? zipPicks = 'content://downloads/takeout-20261101.zip';

  /// Throws on listing the zip, as one that is no zip does.
  bool zipFails = false;

  final _zip = <String, Uint8List>{};

  /// The paths and limits asked of [unzip], each time.
  final unzipped = <Map<String, int>>[];

  /// How often the copies of [unzip] were deleted.
  var dropped = 0;

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

  /// Puts a file at [path] inside the zip, holding [bytes].
  void putZip(String path, List<int> bytes) =>
      _zip[path] = Uint8List.fromList(bytes);

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
  Future<String?> pickZip() async => zipPicks;

  @override
  Future<List<String>> zipEntries(String zip) async {
    await hold?.future;
    if (zipFails) throw FileSystemException('Not a zip', zip);
    return [..._zip.keys];
  }

  @override
  Future<List<FolderFile>> unzip(String zip, Map<String, int> wanted) async {
    unzipped.add(wanted);
    return [
      for (final MapEntry(key: path, value: bytes) in _zip.entries)
        if (wanted[path] case final limit?)
          () {
            // At most the limit and one byte more is copied.
            final copied = bytes.length > limit + 1
                ? Uint8List.sublistView(bytes, 0, limit + 1)
                : bytes;
            _bytes['zip:$path'] = copied;
            return FolderFile(
              path: path,
              modified: null,
              size: copied.length,
              ref: 'zip:$path',
            );
          }(),
    ];
  }

  @override
  Future<void> dropUnzipped() async => dropped++;

  @override
  Future<String?> autoBackupDir() async => null;

  @override
  Future<void> setAutoBackup(bool on) async {}

  @override
  Future<String?> fileApp(String folder, String path) async => null;

  @override
  Future<bool> openFile(String folder, String path) async => false;
}

import 'dart:io';
import 'dart:typed_data';

/// Photo and voice recording bytes live outside the log in
/// content-addressed storage, keyed by their SHA-256 (Catlog's blob
/// store, copied and adapted): a directory next to the database file, or
/// memory for in-memory stores. The log only names a blob by its hash, so
/// whatever carries the log carries the reference, and the bytes can
/// follow on their own.
abstract interface class Blobs {
  /// Keeps [bytes] under [hash]; a hash already held stays as it is.
  void put(String hash, Uint8List bytes);

  /// The bytes under [hash], or null when they are not here (yet).
  Uint8List? get(String hash);

  /// Lets go of the bytes under [hash]; returns whether they were here.
  bool remove(String hash);
}

/// [Blobs] as one file per blob in [dir].
class FileBlobs implements Blobs {
  final Directory dir;

  FileBlobs(String path) : dir = Directory(path) {
    dir.createSync(recursive: true);
  }

  /// A blob name is its SHA-256, nothing else: a hash arriving from
  /// another Device ("../../x") must never leave this directory. Anything
  /// else is a blob that is not here, never an error.
  static final _hex64 = RegExp(r'^[0-9a-f]{64}$');

  File? _file(String hash) =>
      _hex64.hasMatch(hash) ? File('${dir.path}/$hash') : null;

  @override
  void put(String hash, Uint8List bytes) {
    final file = _file(hash);
    if (file == null || file.existsSync()) return;
    // Written beside, then renamed: a kill mid-write leaves a stray
    // .tmp, never a cut-off blob that counts as present.
    final tmp = File('${file.path}.tmp');
    tmp.writeAsBytesSync(bytes, flush: true);
    tmp.renameSync(file.path);
  }

  @override
  Uint8List? get(String hash) {
    final file = _file(hash);
    return file != null && file.existsSync() ? file.readAsBytesSync() : null;
  }

  @override
  bool remove(String hash) {
    final file = _file(hash);
    if (file == null || !file.existsSync()) return false;
    file.deleteSync();
    return true;
  }
}

/// [Blobs] in memory: gone with the store. For tests.
class MemoryBlobs implements Blobs {
  final _held = <String, Uint8List>{};

  @override
  void put(String hash, Uint8List bytes) =>
      _held.putIfAbsent(hash, () => bytes);

  @override
  Uint8List? get(String hash) => _held[hash];

  @override
  bool remove(String hash) => _held.remove(hash) != null;
}

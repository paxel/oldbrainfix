import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

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

  /// Whether the bytes under [hash] are here, without reading them.
  bool has(String hash);

  /// Keeps the bytes of the file at [path] as they are, under their
  /// SHA-256, and returns that hash; a hash already held stays as it is.
  /// Read in chunks, so a large file is never held whole: for an attached
  /// file of up to 256 MiB, run off the UI isolate, which a [FileBlobs]
  /// can be sent to.
  String putFrom(String path);
}

/// The SHA-256 of the bytes read from [input] in chunks of a MiB, each
/// handed to [also] as it is read.
String _hashOf(RandomAccessFile input, [void Function(Uint8List)? also]) {
  final out = _DigestSink();
  final hashing = sha256.startChunkedConversion(out);
  while (true) {
    final chunk = input.readSync(1 << 20);
    if (chunk.isEmpty) break;
    hashing.add(chunk);
    also?.call(chunk);
  }
  hashing.close();
  out.close();
  return out.value.toString();
}

/// Where a chunked SHA-256 lands.
class _DigestSink implements Sink<Digest> {
  late Digest value;

  @override
  void add(Digest data) => value = data;

  @override
  void close() {}
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

  @override
  bool has(String hash) => _file(hash)?.existsSync() ?? false;

  @override
  String putFrom(String path) {
    // Copied beside under a name of its own while it is hashed, then
    // renamed to its hash: a kill midway leaves a stray .tmp, and two
    // files kept at once never write into one another.
    final tmp = File(
      '${dir.path}/in-${Random.secure().nextInt(1 << 32).toRadixString(16)}'
      '-${Random.secure().nextInt(1 << 32).toRadixString(16)}.tmp',
    );
    final input = File(path).openSync();
    final String hash;
    try {
      final output = tmp.openSync(mode: FileMode.writeOnly);
      try {
        hash = _hashOf(input, output.writeFromSync);
        output.flushSync();
      } finally {
        output.closeSync();
      }
    } catch (_) {
      // A full disk midway leaves nothing beside.
      if (tmp.existsSync()) tmp.deleteSync();
      rethrow;
    } finally {
      input.closeSync();
    }
    final file = _file(hash);
    if (file == null || file.existsSync()) {
      tmp.deleteSync();
    } else {
      tmp.renameSync(file.path);
    }
    return hash;
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

  @override
  bool has(String hash) => _held.containsKey(hash);

  @override
  String putFrom(String path) {
    final bytes = File(path).readAsBytesSync();
    final hash = sha256.convert(bytes).toString();
    put(hash, bytes);
    return hash;
  }
}

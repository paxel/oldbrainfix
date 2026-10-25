import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive_io.dart';

import 'area_keys.dart';
import 'conflicts.dart';
import 'entry.dart';
import 'folder_sync.dart';
import 'pairing.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// Sync by bundle file (ADR 0003): what is new for a paired Device, or
/// everything, in one file sent through any messenger and imported on
/// the other Device. Inside the zip lies the shared folder's layout
/// (ADR 0012), sealed the same way: the manifest, the segments of
/// entries per key, the keys file sealed with the personal key and the
/// blobs, so one reader serves both and a Device lacking the keys reads
/// nothing. What comes in is checked and applied as from the folder:
/// signatures, Conflicts from causal contexts, a Moment before it.
///
/// Every file is stored, not deflated: sealed bytes do not shrink. The
/// zip's dates are all 1980-01-01, so it tells nothing of when it was
/// written.
const bundleExtension = 'obfsync';

/// The type a bundle is shared as.
const bundleMimeType = 'application/vnd.oldbrainfix.sync';

/// The name a bundle written on [day] goes by.
String bundleFileName(DateTime day) {
  String two(int n) => n.toString().padLeft(2, '0');
  return 'oldbrainfix-${day.year}-${two(day.month)}-${two(day.day)}'
      '.$bundleExtension';
}

/// A file that is no bundle this app can read: no zip, damaged, or
/// without a manifest it understands.
class NotABundle implements Exception {
  const NotABundle();

  @override
  String toString() => 'NotABundle';
}

/// A bundle written by a newer version of the app: it holds only
/// manifests of a format this version does not read (ADR 0019). This
/// app needs an update to read it.
class BundleNeedsUpdate implements Exception {
  const BundleNeedsUpdate();

  @override
  String toString() => 'BundleNeedsUpdate';
}

/// What a bundle import did.
class BundleResult {
  /// The entries that were new here.
  final List<Entry> applied;
  final int blobsIn;

  /// Segments sealed with a key this Device does not hold: it was never
  /// paired with the Device that wrote them.
  final int unreadable;
  final Moment? moment;
  final ImportReport report;
  final List<Conflict> conflicts;

  /// The Device that wrote the bundle, and the name it goes by here;
  /// empty when unknown.
  final String from;
  final String fromName;

  /// Whether the own Profile changed, or "Which one is you?" came up
  /// (ADR 0017).
  final bool profileChanged;

  const BundleResult({
    required this.applied,
    required this.blobsIn,
    required this.unreadable,
    required this.moment,
    required this.report,
    required this.conflicts,
    required this.from,
    required this.fromName,
    this.profileChanged = false,
  });

  /// Nothing could be read for want of keys: the Devices have to be
  /// paired first.
  bool get needsPairing => applied.isEmpty && unreadable > 0;
}

/// A file inside a bundle larger than this is not one of ours and is
/// left out: a small zip that unpacks to gigabytes must not take the
/// app down. Segments are closed at [segmentCap]; blobs are photos and
/// voice recordings.
const maxBundleFileBytes = 16 << 20;
const maxBundleBlobBytes = 256 << 20;

/// Writes a bundle to [path] and returns how many entries and blobs it
/// carries; null, writing nothing, when there is nothing to send. For
/// the paired Device [forDevice] it holds what lies beyond what that
/// Device last said it holds ([Pairing.heldBy]), else everything; the
/// blobs only of the entries in it. For another person's Device, paired
/// by invite, only what lies in the Areas it shares, with the whole
/// history of Notes filed into them meanwhile (ADR 0013). A bundle that
/// never arrived loses nothing: the next one for the same Device carries
/// it again.
///
/// The nonces come from [nonce] in the folder's order; the fixture
/// generator hands in a counter.
Future<({int entries, int blobs})?> writeBundle(
  Store store,
  String path, {
  String? forDevice,
  Uint8List Function()? nonce,
}) async {
  final entries = forDevice != null && store.isSharedPeer(forDevice)
      ? store.entriesFor(forDevice, store.heldBy(forDevice))
      : store.entriesSince(
          forDevice == null ? const {} : store.heldBy(forDevice),
        );
  if (entries.isEmpty) return null;
  final blobs = <String>{
    for (final e in entries)
      for (final (kind, prefix) in Sync.blobKinds)
        if (e.field.startsWith(prefix))
          '$kind/${e.field.substring(prefix.length)}',
  };
  // Straight into the file, one sealed file after the other: a bundle of
  // every photo is never in memory as a whole.
  final encoder = ZipFileEncoder()..create(path, modified: DateTime.utc(1980));
  try {
    return await writeSealed(
      store,
      _ZipWriteFolder(encoder),
      entries,
      blobs: blobs,
      nonce: nonce ?? () => secretBytes(sealNonceBytes),
    );
  } finally {
    encoder.closeSync();
  }
}

/// Imports the bundle at [path]: takes in what is new in it and the
/// blobs the log lacks, as a folder sync does, and keeps what the
/// writer said it holds. Throws [NotABundle] for a file that is none,
/// and [BundleNeedsUpdate] for one holding only newer formats.
Future<BundleResult> importBundle(Store store, String path) async {
  final InputFileStream input;
  try {
    input = InputFileStream(path);
  } on FileSystemException {
    throw const NotABundle();
  }
  try {
    final Archive archive;
    try {
      archive = ZipDecoder().decodeStream(input);
    } on Object {
      // Whatever the decoder chokes on: no zip, or a damaged one.
      throw const NotABundle();
    }
    final folder = _ZipReadFolder(archive);
    final writers = <String, FolderManifest>{};
    var newer = false;
    for (final name in await folder.list('')) {
      if (!name.endsWith('.manifest')) continue;
      final bytes = await folder.read('', name);
      final manifest = FolderManifest.parse(bytes);
      final device = name.substring(0, name.length - '.manifest'.length);
      if (device.isEmpty) continue;
      if (manifest != null) writers[device] = manifest;
      newer |= FolderManifest.isNewer(bytes);
    }
    if (writers.isEmpty) {
      throw newer ? const BundleNeedsUpdate() : const NotABundle();
    }
    final from = writers.keys.firstWhere(
      (d) => d != store.deviceId,
      orElse: () => writers.keys.first,
    );
    final name = store.pairedDevice(from)?.name ?? '';
    final FolderSyncResult read;
    try {
      read = await readSealed(
        store,
        folder,
        cause: MomentCause.bundle,
        label: name.isEmpty ? null : name,
      );
    } on FileSystemException {
      throw const NotABundle();
    }
    for (final MapEntry(:key, :value) in writers.entries) {
      store.noteHeldBy(key, value.vector);
    }
    return BundleResult(
      applied: read.applied,
      blobsIn: read.blobsIn,
      unreadable: read.unreadable,
      moment: read.moment,
      report: read.report,
      conflicts: read.conflicts,
      from: from,
      fromName: name,
      profileChanged: read.profileChanged,
    );
  } finally {
    input.closeSync();
  }
}

/// The path of [name] in [dir] inside the zip.
String _path(String dir, String name) => dir.isEmpty ? name : '$dir/$name';

/// A bundle being written, as the folder's writer sees it: every file
/// goes into the zip as it is written. Empty to read: a bundle is
/// written once, from nothing.
class _ZipWriteFolder implements SyncFolder {
  final ZipFileEncoder encoder;
  final _written = <String, Set<String>>{};

  _ZipWriteFolder(this.encoder);

  @override
  Future<List<String>> list(String dir) async =>
      (_written[dir]?.toList() ?? <String>[])..sort();

  @override
  Future<Uint8List?> read(String dir, String name) async => null;

  @override
  Future<void> write(String dir, String name, List<int> bytes) async {
    if (!_written.putIfAbsent(dir, () => {}).add(name)) {
      throw FileSystemException('Written twice into a bundle', name);
    }
    encoder.addArchiveFile(
      ArchiveFile.noCompress(_path(dir, name), bytes.length, bytes),
    );
  }

  @override
  Future<void> delete(String dir, String name) async {}

  @override
  Future<void> ensure(String dir) async {}
}

/// A bundle being read, as the folder's reader sees it: the zip's files
/// directly in the root, `keys` and `blobs`; anything else in it is not
/// looked at. Each file is unpacked when it is read.
class _ZipReadFolder implements SyncFolder {
  final Map<String, Map<String, ArchiveFile>> _dirs = {};

  _ZipReadFolder(Archive archive) {
    for (final file in archive.files) {
      if (!file.isFile) continue;
      final parts = file.name.split('/');
      final (dir, name) = switch (parts) {
        [final name] => ('', name),
        ['keys' || 'blobs', final name] => (parts[0], name),
        _ => ('', ''),
      };
      if (name.isEmpty) continue;
      _dirs.putIfAbsent(dir, () => {})[name] = file;
    }
  }

  @override
  Future<List<String>> list(String dir) async =>
      (_dirs[dir]?.keys.toList() ?? <String>[])..sort();

  @override
  Future<Uint8List?> read(String dir, String name) async {
    final file = _dirs[dir]?[name];
    if (file == null) return null;
    final cap = dir == 'blobs' ? maxBundleBlobBytes : maxBundleFileBytes;
    if (file.size > cap) return null;
    try {
      final bytes = file.readBytes();
      return bytes == null || bytes.length > cap ? null : bytes;
    } on Object {
      // A damaged file inside reads as absent.
      return null;
    }
  }

  @override
  Future<void> write(String dir, String name, List<int> bytes) async =>
      throw FileSystemException('A bundle is read only', name);

  @override
  Future<void> delete(String dir, String name) async =>
      throw FileSystemException('A bundle is read only', name);

  @override
  Future<void> ensure(String dir) async {}
}

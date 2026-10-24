import 'dart:io';
import 'dart:typed_data';

/// Photo bytes live outside the log in content-addressed storage, keyed
/// by their SHA-256 (Catlog's blob store, copied and adapted): a
/// directory next to the database file, or memory for in-memory stores.
/// The log only names a photo by its hash, so whatever carries the log
/// carries the reference, and the bytes can follow on their own.
abstract interface class PhotoBlobs {
  /// Keeps [bytes] under [hash]; a hash already held stays as it is.
  void put(String hash, Uint8List bytes);

  /// The bytes under [hash], or null when they are not here (yet).
  Uint8List? get(String hash);
}

/// [PhotoBlobs] as one file per photo in [dir].
class FilePhotoBlobs implements PhotoBlobs {
  final Directory dir;

  FilePhotoBlobs(String path) : dir = Directory(path) {
    dir.createSync(recursive: true);
  }

  /// A blob name is its SHA-256, nothing else: a hash arriving from
  /// another Device ("../../x") must never leave this directory. Anything
  /// else is a photo that is not here, never an error.
  static final _hex64 = RegExp(r'^[0-9a-f]{64}$');

  File? _file(String hash) =>
      _hex64.hasMatch(hash) ? File('${dir.path}/$hash') : null;

  @override
  void put(String hash, Uint8List bytes) {
    final file = _file(hash);
    if (file == null || file.existsSync()) return;
    // Written beside, then renamed: a kill mid-write leaves a stray
    // .tmp, never a cut-off photo that counts as present.
    final tmp = File('${file.path}.tmp');
    tmp.writeAsBytesSync(bytes, flush: true);
    tmp.renameSync(file.path);
  }

  @override
  Uint8List? get(String hash) {
    final file = _file(hash);
    return file != null && file.existsSync() ? file.readAsBytesSync() : null;
  }
}

/// [PhotoBlobs] in memory: gone with the store. For tests.
class MemoryPhotoBlobs implements PhotoBlobs {
  final _held = <String, Uint8List>{};

  @override
  void put(String hash, Uint8List bytes) =>
      _held.putIfAbsent(hash, () => bytes);

  @override
  Uint8List? get(String hash) => _held[hash];
}

/// [jpeg] without its APP1..APP15 and COM segments (Exif with its GPS
/// position, XMP, ICC, comments): a stored photo never says where it
/// was taken; the Note's own Capture Context does that. The picture data
/// is untouched, no re-encoding, so every Device that strips the same
/// file gets the same bytes. Bytes that are not a JPEG come back as they
/// are. Copied from Catlog's photo_privacy.dart.
Uint8List stripJpegMetadata(Uint8List jpeg) {
  if (jpeg.length < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8) return jpeg;
  final out = BytesBuilder(copy: false)..add(const [0xFF, 0xD8]);
  var i = 2;
  var changed = false;
  while (i + 4 <= jpeg.length && jpeg[i] == 0xFF) {
    final marker = jpeg[i + 1];
    if (marker == 0xD9) break; // EOI: nothing follows
    if (marker == 0xDA) break; // SOS: the scan runs to the end
    if (marker == 0xFF) {
      i++; // fill byte
      continue;
    }
    if (marker >= 0xD0 && marker <= 0xD7) {
      out.add(jpeg.sublist(i, i + 2)); // RSTn carries no length
      i += 2;
      continue;
    }
    final length = (jpeg[i + 2] << 8) | jpeg[i + 3];
    final end = i + 2 + length;
    if (end > jpeg.length) return jpeg; // truncated: leave it alone
    final drop = (marker >= 0xE1 && marker <= 0xEF) || marker == 0xFE;
    if (drop) {
      changed = true;
    } else {
      out.add(jpeg.sublist(i, end));
    }
    i = end;
  }
  if (!changed) return jpeg;
  out.add(jpeg.sublist(i));
  return out.toBytes();
}

import 'dart:typed_data';

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

import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A JPEG's skeleton: SOI, an Exif APP1 with a GPS-like payload, a JFIF
/// APP0, a comment, a quantisation table, then the scan up to EOI.
Uint8List jpegWithMetadata() => Uint8List.fromList([
  0xFF, 0xD8, // SOI
  0xFF, 0xE0, 0x00, 0x06, 0x4A, 0x46, 0x49, 0x46, // APP0 "JFIF"
  0xFF, 0xE1, 0x00, 0x08, 0x45, 0x78, 0x69, 0x66, 0x47, 0x50, // APP1 Exif
  0xFF, 0xFE, 0x00, 0x05, 0x68, 0x69, 0x21, // COM "hi!"
  0xFF, 0xDB, 0x00, 0x04, 0x01, 0x02, // DQT
  0xFF, 0xDA, 0x00, 0x03, 0x00, 0x11, 0x22, 0x33, // SOS and scan data
  0xFF, 0xD9, // EOI
]);

/// [jpegWithMetadata] as stripped: APP0, DQT and the scan stay.
final stripped = Uint8List.fromList([
  0xFF, 0xD8, // SOI
  0xFF, 0xE0, 0x00, 0x06, 0x4A, 0x46, 0x49, 0x46, // APP0 "JFIF"
  0xFF, 0xDB, 0x00, 0x04, 0x01, 0x02, // DQT
  0xFF, 0xDA, 0x00, 0x03, 0x00, 0x11, 0x22, 0x33, // SOS and scan data
  0xFF, 0xD9, // EOI
]);

void main() {
  group('stripping metadata', () {
    test('drops Exif and comments, keeps the picture', () {
      expect(stripJpegMetadata(jpegWithMetadata()), stripped);
    });

    test('a JPEG without metadata comes back as it is', () {
      expect(stripJpegMetadata(stripped), same(stripped));
    });

    test('bytes that are not a JPEG come back as they are', () {
      final png = Uint8List.fromList([0x89, 0x50, 0x4E, 0x47, 1, 2, 3]);
      expect(stripJpegMetadata(png), same(png));
    });
  });

  group('the store\'s photos', () {
    test('a photo is kept without metadata under its hash', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final hash = store.putPhoto(jpegWithMetadata());
      expect(hash, sha256.convert(stripped).toString());
      expect(store.photo(hash), stripped);
    });

    test('an unknown photo is null', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(store.photo('0' * 64), isNull);
    });

    test('photos live as files next to the log and survive a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final first = Store.open('${dir.path}/log.db');
      final hash = first.putPhoto(jpegWithMetadata());
      first.close();
      expect(File('${dir.path}/photos/$hash').existsSync(), isTrue);
      final second = Store.open('${dir.path}/log.db');
      addTearDown(second.close);
      expect(second.photo(hash), stripped);
    });

    test('a hash that is no hash never reaches outside the directory', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final blobs = FileBlobs('${dir.path}/photos');
      blobs.put('../escape', Uint8List.fromList([1]));
      expect(File('${dir.path}/escape').existsSync(), isFalse);
      expect(blobs.get('../escape'), isNull);
    });
  });

  group('photo Notes', () {
    final png = Uint8List.fromList([0x89, 0x50, 0x4E, 0x47, 9]);

    test('a Note captured with photos names them by hash', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final note = notes.capture('', photos: [jpegWithMetadata(), png]);
      expect(note.text, '');
      expect(note.photos, [
        sha256.convert(stripped).toString(),
        sha256.convert(png).toString(),
      ]);
      expect(notes.photoBytes(note.photos.first), stripped);
      expect(note.isUnsorted, isTrue);
    });

    test('a photo added later comes after the ones before', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final note = notes.capture('receipt', photos: [png]);
      final hash = notes.addPhoto(note.id, jpegWithMetadata());
      expect(notes.byId(note.id)?.photos, [
        sha256.convert(png).toString(),
        hash,
      ]);
    });

    test('photos go only to Notes', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(
        () => Notes(store).addPhoto('note:none', png),
        throwsArgumentError,
      );
    });

    test('photos are in the log and survive a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final first = Store.open('${dir.path}/log.db');
      final id = Notes(first).capture('', photos: [png]).id;
      first.close();
      final second = Store.open('${dir.path}/log.db');
      addTearDown(second.close);
      final note = Notes(second).byId(id);
      expect(note?.photos, [sha256.convert(png).toString()]);
      expect(Notes(second).photoBytes(note?.photos.single ?? ''), png);
    });

    test('a merged Note brings its photos as a Section', () {
      var now = DateTime.utc(2026, 10, 24, 8);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      final notes = Notes(store);
      final into = notes.capture('sauce', photos: [png]);
      now = now.add(const Duration(minutes: 5));
      final merged = notes.capture('the shelf', photos: [jpegWithMetadata()]);
      notes.merge(merged.id, into.id);
      final section = notes.sections(into.id).single;
      expect(section.id, merged.id);
      expect(section.photos, [sha256.convert(stripped).toString()]);
      expect(notes.byId(into.id)?.photos, [sha256.convert(png).toString()]);
    });

    test('a Note without photos has none', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(Notes(store).capture('milk').photos, isEmpty);
    });
  });
}

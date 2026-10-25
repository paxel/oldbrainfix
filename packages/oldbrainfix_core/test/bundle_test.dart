import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive_io.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'sync_test.dart' show device, exchange, texts, ticking;

Uint8List _photo(int n) => Uint8List.fromList([
  0xff,
  0xd8,
  for (var i = 0; i < 64; i++) (i * n) & 0xff,
  0xff,
  0xd9,
]);

void main() {
  late Directory dir;
  var n = 0;
  String path() => '${dir.path}/b${n++}.$bundleExtension';

  setUp(() => dir = Directory.systemTemp.createTempSync('bundle_test_'));
  tearDown(() => dir.deleteSync(recursive: true));

  test('everything goes across in one file, sealed, photos too', () async {
    final clock = ticking();
    final phone = device(clock, 'Ada');
    final desk = device(clock, 'Ada');
    final house = Places(desk).addArea('Household');
    exchange(phone, desk);
    Notes(desk).capture('Buy milk', place: house.id);
    final receipt = Notes(desk).capture('Receipt', photos: [_photo(3)]);
    final kids = Places(desk).addArea('Kids');
    Notes(desk).capture('School trip', place: kids.id);

    final file = path();
    final wrote = await writeBundle(desk, file);
    expect(wrote?.entries, desk.allEntries().length);
    expect(wrote?.blobs, 1);
    final raw = latin1.decode(File(file).readAsBytesSync());
    for (final secret in ['Buy milk', 'School', 'Kids', 'note:', 'area:']) {
      expect(raw, isNot(contains(secret)));
    }

    final got = await importBundle(phone, file);
    expect(got.applied, isNotEmpty);
    expect(got.blobsIn, 1);
    expect(got.unreadable, 0);
    expect(got.needsPairing, isFalse);
    expect(got.from, desk.deviceId);
    expect(got.moment?.cause, MomentCause.bundle);
    expect(texts(phone), texts(desk));
    final hash = Notes(desk).byId(receipt.id)!.photos.single;
    expect(phone.photo(hash), desk.photo(hash));
    // The key of the Area made after the meeting came with it.
    expect(phone.writingKey(kids.id)?.id, desk.writingKey(kids.id)?.id);

    final again = await importBundle(phone, file);
    expect(again.applied, isEmpty);
    expect(again.moment, isNull);
  });

  test('a bundle for a paired Device carries what it lacks', () async {
    final clock = ticking();
    final phone = device(clock, 'Ada');
    final desk = device(clock, 'Ada');
    Notes(desk).capture('Old', photos: [_photo(5)]);
    exchange(phone, desk);
    // The desk was told what the phone holds in the session.
    expect(desk.heldBy(phone.deviceId), isNotEmpty);
    expect(await writeBundle(desk, path(), forDevice: phone.deviceId), isNull);

    final before = desk.allEntries().length;
    Notes(desk).capture('New', photos: [_photo(9)]);
    final file = path();
    final wrote = await writeBundle(desk, file, forDevice: phone.deviceId);
    expect(wrote?.entries, desk.allEntries().length - before);
    expect(wrote?.blobs, 1, reason: 'only the photo of the new Note');
    final got = await importBundle(phone, file);
    expect(got.blobsIn, 1);
    expect(texts(phone), ['New', 'Old']);

    // What the desk wrote it holds is kept: the phone's bundle for it
    // carries only what is new on the phone.
    expect(phone.heldBy(desk.deviceId), desk.versionVector());
    Notes(phone).capture('From the phone');
    final back = path();
    expect(
      (await writeBundle(phone, back, forDevice: desk.deviceId))?.entries,
      lessThan(phone.allEntries().length),
    );
    await importBundle(desk, back);
    expect(texts(desk), texts(phone));
    // Forgetting the Device forgets what it held.
    phone.forgetDevice(desk.deviceId);
    expect(phone.heldBy(desk.deviceId), isEmpty);
  });

  test('a Device never paired reads nothing and is told to pair', () async {
    final clock = ticking();
    final desk = device(clock, 'Ada');
    Notes(desk).capture('Private');
    final file = path();
    await writeBundle(desk, file);
    final stranger = device(clock, 'Eve');
    final got = await importBundle(stranger, file);
    expect(got.applied, isEmpty);
    expect(got.needsPairing, isTrue);
    expect(texts(stranger), isEmpty);
  });

  test('concurrent edits through a bundle are a Conflict', () async {
    final clock = ticking();
    final phone = device(clock, 'Ada');
    final desk = device(clock, 'Ada');
    final note = Notes(phone).capture('Buy milk');
    exchange(phone, desk);
    Notes(phone).edit(note.id, 'Buy oat milk');
    Notes(desk).edit(note.id, 'Buy milk and eggs');
    final file = path();
    await writeBundle(phone, file);
    final got = await importBundle(desk, file);
    expect(got.conflicts, hasLength(1));
    expect(desk.conflicts(), hasLength(1));
  });

  test('a bundle leaves the shared folder to read in full', () async {
    final clock = ticking();
    final phone = device(clock, 'Ada');
    final desk = device(clock, 'Ada');
    exchange(phone, desk);
    Notes(desk).capture('First');
    final folder = MemorySyncFolder();
    await folderSync(desk, folder);
    final file = path();
    await writeBundle(desk, file);
    await importBundle(phone, file);
    expect(phone.localSettingsWithPrefix('folderRead:'), isEmpty);
    Notes(desk).capture('Second');
    await folderSync(desk, folder);
    await folderSync(phone, folder);
    expect(texts(phone), texts(desk));
  });

  test('a file that is no bundle says so', () async {
    final clock = ticking();
    final phone = device(clock, 'Ada');
    final junk = path();
    File(junk).writeAsStringSync('not a zip at all');
    expect(importBundle(phone, junk), throwsA(isA<NotABundle>()));
    expect(
      importBundle(phone, '${dir.path}/missing'),
      throwsA(isA<NotABundle>()),
    );
    // A zip, but not ours.
    final zip = path();
    final encoder = ZipFileEncoder()..create(zip);
    encoder.addArchiveFile(ArchiveFile.string('photo.jpg', 'x'));
    encoder.closeSync();
    expect(importBundle(phone, zip), throwsA(isA<NotABundle>()));
  });

  test('a bundle of only newer formats says this app needs an '
      'update', () async {
    final phone = device(ticking(), 'Ada');
    final later = path();
    final encoder = ZipFileEncoder()..create(later);
    encoder.addArchiveFile(
      ArchiveFile.string('abc.manifest', '{"format":2,"later":true}'),
    );
    encoder.closeSync();
    expect(importBundle(phone, later), throwsA(isA<BundleNeedsUpdate>()));
  });

  test('the file name tells the day', () {
    expect(
      bundleFileName(DateTime(2026, 3, 1)),
      'oldbrainfix-2026-03-01.obfsync',
    );
  });
}

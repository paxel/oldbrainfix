import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'sync_test.dart' show device, exchange, texts, ticking;

/// Every byte in [folder], each file's as one string, for looking for
/// what must not be there.
List<String> _contents(MemorySyncFolder folder) => [
  for (final files in folder.dirs.values)
    for (final bytes in files.values) latin1.decode(bytes),
];

Uint8List _photo(int n) => Uint8List.fromList(
  // A JPEG start, so the store keeps it as a photo, then the pattern.
  [0xff, 0xd8, for (var i = 0; i < 64; i++) (i * n) & 0xff, 0xff, 0xd9],
);

/// A folder that is gone: a drive unplugged, an access revoked.
class _GoneFolder implements SyncFolder {
  Never _gone() => throw const FileSystemException('gone');

  @override
  Future<List<String>> list(String dir) async => _gone();

  @override
  Future<Uint8List?> read(String dir, String name) async => _gone();

  @override
  Future<void> write(String dir, String name, List<int> bytes) async => _gone();

  @override
  Future<void> delete(String dir, String name) async => _gone();

  @override
  Future<void> ensure(String dir) async => _gone();
}

void main() {
  group('the folder setting', () {
    test('a round keeps how it went; a gone folder too', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      final folder = MemorySyncFolder();
      Notes(phone).capture('Hello');
      expect(phone.folderStatus, isNull);
      await folderRound(phone, folder);
      expect(phone.folderStatus?.failed, isFalse);
      final result = await folderRound(desk, folder);
      expect(result?.applied, isNotEmpty);
      expect(desk.folderStatus?.entriesIn, result?.applied.length);
      expect(desk.folderStatus?.unreadable, 0);
      expect(await folderRound(desk, _GoneFolder()), isNull);
      expect(desk.folderStatus?.failed, isTrue);
      final stranger = device(clock, 'Eve');
      await folderRound(stranger, folder);
      expect(stranger.folderStatus?.unreadable, greaterThan(0));
    });

    test(
      'another folder is read whole; forgetting touches nothing in it',
      () async {
        final clock = ticking();
        final phone = device(clock, 'Ada');
        final desk = device(clock, 'Ada');
        exchange(phone, desk);
        final folder = MemorySyncFolder();
        Notes(phone).capture('Hello');
        await folderSync(phone, folder);
        desk.chooseSyncFolder('/mnt/a');
        await folderRound(desk, folder);
        expect(desk.syncFolder, '/mnt/a');
        expect(desk.localSettingsWithPrefix('folderRead:'), isNotEmpty);
        desk.chooseSyncFolder('/mnt/a');
        expect(desk.folderStatus, isNotNull, reason: 'the same folder stays');
        desk.chooseSyncFolder('/mnt/b');
        expect(desk.localSettingsWithPrefix('folderRead:'), isEmpty);
        expect(desk.folderStatus, isNull);
        final files = folder.dirs.values.fold(0, (n, d) => n + d.length);
        desk.forgetSyncFolder();
        expect(desk.syncFolder, isNull);
        expect(folder.dirs.values.fold(0, (n, d) => n + d.length), files);
      },
    );
  });

  group('folder sync', () {
    test('two paired Devices converge through the folder alone', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final house = Places(desk).addArea('Household');
      exchange(phone, desk);
      // Apart since: each writes on its own.
      Notes(phone).capture('Buy milk', place: house.id);
      Notes(phone).capture('Call mum');
      final photo = Notes(desk).capture('Receipt', photos: [_photo(3)]);
      final kids = Places(desk).addArea('Kids');
      Notes(desk).capture('School trip', place: kids.id);

      final folder = MemorySyncFolder();
      final first = await folderSync(phone, folder);
      expect(first.applied, isEmpty);
      expect(first.entriesOut, phone.allEntries().length);
      final second = await folderSync(desk, folder);
      expect(second.applied, isNotEmpty);
      expect(second.moment?.cause, MomentCause.folder);
      final third = await folderSync(phone, folder);
      expect(third.applied, isNotEmpty);
      expect(third.blobsIn, 1);
      expect(texts(phone), texts(desk));
      expect(texts(phone), contains('School trip'));
      final hash = Notes(desk).byId(photo.id)!.photos.single;
      expect(phone.photo(hash), desk.photo(hash));
      // The key of the Area made after the meeting came through the
      // folder too.
      expect(phone.writingKey(kids.id)?.id, desk.writingKey(kids.id)?.id);
      // Nothing new: nothing moves.
      final quiet = await folderSync(desk, folder);
      expect(quiet.applied, isEmpty);
      expect(quiet.entriesOut, 0);
      expect(quiet.moment, isNull);
    });

    test('a Device that was never paired reads nothing', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      final doctor = Places(phone).addArea('Doctor');
      Notes(phone).capture('Blood test results', place: doctor.id);
      Notes(phone).capture('Feeling dizzy');
      Notes(phone).capture('X-ray', photos: [_photo(5)]);
      final folder = MemorySyncFolder();
      await folderSync(phone, folder);

      final stranger = device(clock, 'Eve');
      final result = await folderSync(stranger, folder);
      expect(result.applied, isEmpty);
      expect(result.unreadable, greaterThan(0));
      expect(Notes(stranger).all(), isEmpty);
      expect(stranger.missingBlobs(), isEmpty);

      for (final text in _contents(folder)) {
        for (final secret in [
          'Blood test',
          'dizzy',
          'Doctor',
          doctor.id,
          'note:',
          'area:',
          '"text"',
          'Ada',
        ]) {
          expect(text, isNot(contains(secret)));
        }
      }
      // The photo's hash names no file.
      final hash = Notes(phone).all().expand((n) => n.photos).single;
      expect(folder.dirs['blobs']!.keys, isNot(contains(contains(hash))));
      expect(folder.dirs['']!.keys, contains('.nomedia'));
    });

    test('only what is new goes out, appended to the open segment', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final folder = MemorySyncFolder();
      Notes(phone).capture('One');
      await folderSync(phone, folder);
      final manifest = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!;
      expect(manifest.generation, 1);
      expect(manifest.segments, hasLength(1));
      final first = folder.dirs['']![manifest.segments.single.name]!;
      Notes(phone).capture('Two');
      final again = await folderSync(phone, folder);
      expect(
        again.entriesOut,
        phone.allEntries().length - manifest.segments.single.lines,
      );
      final grown = folder.dirs['']![manifest.segments.single.name]!;
      expect(utf8.decode(grown), startsWith(utf8.decode(first)));
      final next = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!;
      expect(next.generation, 1);
      expect(next.vector, phone.versionVector());
      expect(next.segments.single.lines, phone.allEntries().length);
    });

    test('a full segment is closed and the next one opened', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      final folder = MemorySyncFolder();
      final long = 'x' * 2000;
      for (var i = 0; i < 40; i++) {
        Notes(phone).capture('$i $long');
      }
      await folderSync(phone, folder);
      final manifest = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!;
      expect(manifest.segments.length, greaterThan(1));
      final full = manifest.segments.first;
      final bytes = folder.dirs['']![full.name]!;
      expect(bytes.length, greaterThanOrEqualTo(segmentCap));
      Notes(phone).capture('After');
      await folderSync(phone, folder);
      expect(folder.dirs['']![full.name], bytes, reason: 'full stays full');
      await folderSync(desk, folder);
      expect(texts(desk), texts(phone));
    });

    test('a key that comes late fills the gap below the vector', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      final folder = MemorySyncFolder();
      // An Area the desk has no key for yet, then a Note outside it.
      final work = Places(phone).addArea('Work');
      Notes(phone).capture('Report', place: work.id);
      Notes(phone).capture('Loose thought');
      await folderSync(phone, folder);
      // The keys file is still on its way.
      final keys = Map.of(folder.dirs['keys']!);
      folder.dirs['keys']!.clear();
      final early = await folderSync(desk, folder);
      expect(early.unreadable, 1);
      expect(texts(desk), contains('Loose thought'));
      expect(texts(desk), isNot(contains('Report')));
      folder.dirs['keys']!.addAll(keys);
      final late = await folderSync(desk, folder);
      expect(late.unreadable, 0);
      expect(texts(desk), texts(phone));
    });

    test('a Note moved to another Area writes on under the new key', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final home = Places(phone).addArea('Home');
      final work = Places(phone).addArea('Work');
      exchange(phone, desk);
      final folder = MemorySyncFolder();
      final note = Notes(phone).capture('Tax forms', place: home.id);
      await folderSync(phone, folder);
      Notes(phone).file(note.id, work.id);
      Notes(phone).edit(note.id, 'Tax forms, signed');
      await folderSync(phone, folder);
      final manifest = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!;
      expect(
        {for (final s in manifest.segments) s.key},
        containsAll([
          phone.writingKey(home.id)!.id,
          phone.writingKey(work.id)!.id,
        ]),
      );
      await folderSync(desk, folder);
      expect(Notes(desk).byId(note.id)?.text, 'Tax forms, signed');
      expect(Notes(desk).byId(note.id)?.place, work.id);
    });

    test('concurrent edits through the folder are a Conflict', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final note = Notes(phone).capture('Buy milk');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Buy oat milk');
      Notes(desk).edit(note.id, 'Buy milk and eggs');
      final folder = MemorySyncFolder();
      await folderSync(phone, folder);
      final result = await folderSync(desk, folder);
      expect(result.conflicts, hasLength(1));
      expect(desk.conflicts(), hasLength(1));
    });

    test('entries no paired key signed are refused', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      final folder = MemorySyncFolder();
      Notes(phone).capture('Real');
      await folderSync(phone, folder);
      // Someone holding the keys writes in the phone's name.
      final name = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!.segments.single;
      final key = phone.areaKey(name.key)!;
      final forged = Entry(
        seq: -1,
        device: phone.deviceId,
        dseq: 99,
        entity: 'note:forged',
        field: NoteKeys.text,
        value: 'Forged',
        date: DateTime.utc(2026, 3, 2),
        author: 'Ada',
        recorded: DateTime.utc(2026, 3, 2),
        sig: phone.allEntries().first.sig,
      );
      final line = base64.encode(
        seal(
          key.key,
          utf8.encode(jsonEncode(forged.toJson())),
          aad: SealedAs.entry,
        ),
      );
      final bytes = folder.dirs['']![name.name]!;
      folder.dirs['']![name.name] = utf8.encode('${utf8.decode(bytes)}\n$line');
      final manifest = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!;
      folder.dirs['']![manifestName(phone.deviceId)] = FolderManifest(
        generation: manifest.generation,
        vector: manifest.vector,
        segments: [(name: name.name, key: name.key, lines: name.lines + 1)],
      ).encode();
      final result = await folderSync(desk, folder);
      expect(result.report.refusedCount, 1);
      expect(texts(desk), ['Real']);
    });

    test('a segment still on its way leaves its Device for later', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      final folder = MemorySyncFolder();
      Notes(phone).capture('Soon');
      await folderSync(phone, folder);
      final name = FolderManifest.parse(
        folder.dirs['']![manifestName(phone.deviceId)],
      )!.segments.single.name;
      final bytes = folder.dirs['']!.remove(name)!;
      expect((await folderSync(desk, folder)).applied, isEmpty);
      folder.dirs['']![name] = bytes;
      expect((await folderSync(desk, folder)).applied, isNotEmpty);
      expect(texts(desk), ['Soon']);
    });

    test('plain files carry the same folder', () async {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      Notes(phone).capture('On disk', photos: [_photo(7)]);
      final dir = Directory.systemTemp.createTempSync('oldbrainfix_folder_');
      try {
        await folderSync(phone, LocalSyncFolder(dir.path));
        await folderSync(desk, LocalSyncFolder(dir.path));
        expect(texts(desk), ['On disk']);
        expect(desk.missingBlobs(), isEmpty);
        expect(
          Directory('${dir.path}/$syncFolderRoot/blobs').listSync(),
          hasLength(1),
        );
        // A folder that was moved away is gone, not made again.
        final gone = await folderRound(
          desk,
          LocalSyncFolder('${dir.path}/moved'),
        );
        expect(gone, isNull);
        expect(desk.folderStatus?.failed, isTrue);
        expect(Directory('${dir.path}/moved').existsSync(), isFalse);
        // A name from a manifest never leaves the folder.
        await expectLater(
          LocalSyncFolder(dir.path).read('', '../secret'),
          throwsA(isA<FileSystemException>()),
        );
      } finally {
        dir.deleteSync(recursive: true);
      }
    });
  });
}

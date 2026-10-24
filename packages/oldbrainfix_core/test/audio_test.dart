import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Stand-ins for two voice recordings: the store keeps any bytes as
/// they are.
final first = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 1]);
final second = Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, 2]);

String hashOf(Uint8List bytes) => sha256.convert(bytes).toString();

void main() {
  group('the store\'s audio', () {
    test('a recording is kept as it is under its hash', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final hash = store.putAudio(first);
      expect(hash, hashOf(first));
      expect(store.audio(hash), first);
      // Recordings and photos are apart.
      expect(store.photo(hash), isNull);
    });

    test('the same recording twice is kept once', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(store.putAudio(first), store.putAudio(first));
    });

    test('an unknown recording is null', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(store.audio('0' * 64), isNull);
    });

    test('recordings live as files next to the log and survive a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final opened = Store.open('${dir.path}/log.db');
      final hash = opened.putAudio(first);
      opened.close();
      expect(File('${dir.path}/audio/$hash').existsSync(), isTrue);
      final reopened = Store.open('${dir.path}/log.db');
      addTearDown(reopened.close);
      expect(reopened.audio(hash), first);
    });

    test('a hash that is no hash never reaches outside the directory', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final opened = Store.open('${dir.path}/log.db');
      addTearDown(opened.close);
      expect(opened.audio('../log.db'), isNull);
    });
  });

  group('voice Notes', () {
    test('a Note captured with a recording names it by hash', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final note = notes.capture('', recordings: [first]);
      expect(note.text, '');
      expect(note.recordings, [hashOf(first)]);
      expect(note.photos, isEmpty);
      expect(notes.recordingBytes(note.recordings.single), first);
      expect(note.isUnsorted, isTrue);
    });

    test('typed text, photos and a recording go together in one Note', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final png = Uint8List.fromList([0x89, 0x50, 0x4E, 0x47, 9]);
      final note = notes.capture('sauce', photos: [png], recordings: [first]);
      expect(note.text, 'sauce');
      expect(note.photos, [hashOf(png)]);
      expect(note.recordings, [hashOf(first)]);
    });

    test('a recording added later comes after the ones before', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      final notes = Notes(store);
      final note = notes.capture('milk');
      expect(notes.addRecording(note.id, first), hashOf(first));
      notes.addRecording(note.id, second);
      final read = notes.byId(note.id);
      expect(read?.text, 'milk');
      expect(read?.recordings, [hashOf(first), hashOf(second)]);
    });

    test('recordings go only to Notes', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(
        () => Notes(store).addRecording('note:none', first),
        throwsArgumentError,
      );
    });

    test('recordings are in the log and survive a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final opened = Store.open('${dir.path}/log.db');
      final id = Notes(opened).capture('', recordings: [first]).id;
      opened.close();
      final reopened = Store.open('${dir.path}/log.db');
      addTearDown(reopened.close);
      final note = Notes(reopened).byId(id);
      expect(note?.recordings, [hashOf(first)]);
      expect(Notes(reopened).recordingBytes(hashOf(first)), first);
    });

    test('a merged Note brings its recordings as a Section', () {
      var now = DateTime.utc(2026, 10, 24, 8);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      final notes = Notes(store);
      final into = notes.capture('sauce', recordings: [first]);
      now = now.add(const Duration(minutes: 5));
      final merged = notes.capture('', recordings: [second]);
      notes.merge(merged.id, into.id);
      final section = notes.sections(into.id).single;
      expect(section.id, merged.id);
      expect(section.recordings, [hashOf(second)]);
      expect(notes.byId(into.id)?.recordings, [hashOf(first)]);
      expect(notes.recordingBytes(section.recordings.single), second);
    });

    test('a Note without recordings has none', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(Notes(store).capture('milk').recordings, isEmpty);
    });
  });
}

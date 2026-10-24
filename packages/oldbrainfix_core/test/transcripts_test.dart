import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Stand-ins for voice recordings: the store keeps any bytes as they are.
Uint8List recording(int n) =>
    Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, n]);

String hashOf(Uint8List bytes) => sha256.convert(bytes).toString();

void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  /// Captures a voice Note with [recordings] a minute after the last one.
  Note capture(List<Uint8List> recordings, [String text = '']) {
    now = now.add(const Duration(minutes: 1));
    return notes.capture(text, recordings: recordings);
  }

  group('Transcripts', () {
    test('a Transcript is kept next to its recording, which stays', () {
      final note = capture([recording(1)]);
      final hash = note.recordings.single;
      notes.setTranscript(note.id, hash, '  buy more sauce ');
      final read = notes.byId(note.id);
      expect(read?.transcripts, {hash: 'buy more sauce'});
      expect(read?.recordings, [hash]);
      expect(read?.text, '');
      expect(notes.recordingBytes(hash), recording(1));
    });

    test('a Note without Transcripts has none', () {
      expect(capture([recording(1)]).transcripts, isEmpty);
    });

    test('each recording of a Note has its own Transcript', () {
      final note = capture([recording(1), recording(2)]);
      notes.setTranscript(note.id, hashOf(recording(2)), 'second');
      notes.setTranscript(note.id, hashOf(recording(1)), 'first');
      expect(notes.byId(note.id)?.transcripts, {
        hashOf(recording(1)): 'first',
        hashOf(recording(2)): 'second',
      });
    });

    test('a Transcript goes only to a recording of that Note', () {
      final note = capture([recording(1)]);
      final other = capture([recording(2)]);
      expect(
        () => notes.setTranscript(note.id, hashOf(recording(2)), 'x'),
        throwsArgumentError,
      );
      expect(
        () => notes.setTranscript('note:none', hashOf(recording(1)), 'x'),
        throwsArgumentError,
      );
      expect(notes.byId(other.id)?.transcripts, isEmpty);
    });

    test('Transcripts are in the log and survive a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final opened = Store.open('${dir.path}/log.db');
      final note = Notes(opened).capture('', recordings: [recording(1)]);
      Notes(opened).setTranscript(note.id, note.recordings.single, 'milk');
      opened.close();
      final reopened = Store.open('${dir.path}/log.db');
      addTearDown(reopened.close);
      expect(Notes(reopened).byId(note.id)?.transcripts, {
        hashOf(recording(1)): 'milk',
      });
    });

    test('a merged Note brings its Transcripts as a Section', () {
      final into = capture([recording(1)], 'sauce');
      final merged = capture([recording(2)]);
      notes.setTranscript(merged.id, hashOf(recording(2)), 'and basil');
      notes.merge(merged.id, into.id);
      final section = notes.sections(into.id).single;
      expect(section.transcripts, {hashOf(recording(2)): 'and basil'});
      // Written after the Merge, it still lands next to its recording.
      notes.setTranscript(into.id, hashOf(recording(1)), 'sauce please');
      expect(notes.byId(into.id)?.transcripts, {
        hashOf(recording(1)): 'sauce please',
      });
    });
  });

  group('recordings without a Transcript', () {
    test('are listed newest capture first, merged Notes included', () {
      final first = capture([recording(1)]);
      final second = capture([recording(2), recording(3)]);
      final target = capture([], 'shopping');
      notes.merge(first.id, target.id);
      expect(notes.untranscribed(), [
        (noteId: second.id, hash: hashOf(recording(2))),
        (noteId: second.id, hash: hashOf(recording(3))),
        (noteId: first.id, hash: hashOf(recording(1))),
      ]);
    });

    test('drop out once transcribed, also when nothing was heard', () {
      final note = capture([recording(1), recording(2)]);
      notes.setTranscript(note.id, hashOf(recording(1)), '');
      expect(notes.untranscribed(), [
        (noteId: note.id, hash: hashOf(recording(2))),
      ]);
      notes.setTranscript(note.id, hashOf(recording(2)), 'words');
      expect(notes.untranscribed(), isEmpty);
    });

    test('leave out Notes in the trash until they are restored', () {
      final note = capture([recording(1)]);
      capture([], 'text only');
      notes.trash(note.id);
      expect(notes.untranscribed(), isEmpty);
      notes.restore(note.id);
      expect(notes.untranscribed(), [
        (noteId: note.id, hash: hashOf(recording(1))),
      ]);
    });
  });

  group('search', () {
    test('finds a Note by its Transcript, ignoring case', () {
      final voice = capture([recording(1)]);
      capture([], 'call the garage');
      notes.setTranscript(voice.id, hashOf(recording(1)), 'Buy more SAUCE');
      expect(Search(store).notes('sauce').map((n) => n.id), [voice.id]);
    });

    test('finds a merged Note\'s Transcript under its target, once', () {
      final target = capture([], 'shopping');
      final merged = capture([recording(1)]);
      notes.setTranscript(merged.id, hashOf(recording(1)), 'fresh basil');
      notes.merge(merged.id, target.id);
      expect(Search(store).notes('basil').map((n) => n.id), [target.id]);
    });

    test('a trashed Note\'s Transcript is not found', () {
      final note = capture([recording(1)]);
      notes.setTranscript(note.id, hashOf(recording(1)), 'sauce');
      notes.trash(note.id);
      expect(Search(store).notes('sauce'), isEmpty);
    });
  });
}

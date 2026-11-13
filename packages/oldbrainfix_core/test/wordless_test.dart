import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'alarms_test.dart' show europe;

/// Stand-ins for photos and recordings: a JPEG start, so the store keeps
/// the photo as it is, then [n].
Uint8List photo(int n) => Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xD9, n]);
Uint8List voice(int n) => Uint8List.fromList([0x52, 0x49, 0x46, 0x46, n]);

void main() {
  late Store store;
  late Notes notes;

  setUp(() {
    // 09:00 on Saturday, 24 October 2026, in Europe.
    store = Store.inMemory(clock: () => DateTime.utc(2026, 10, 24, 7));
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Wordless? wordless(String id) => notes.byId(id)?.wordless;

  group('a Note without words is titled', () {
    test('as a voice recording when it holds only recordings', () {
      final untranscribed = notes.capture('', recordings: [voice(1)]);
      final silent = notes.capture('', recordings: [voice(2)]);
      notes.setTranscript(silent.id, silent.recordings.single, '');
      expect(wordless(untranscribed.id), Wordless.voiceRecording);
      expect(wordless(silent.id), Wordless.voiceRecording);
    });

    test('as a photo when it holds a photo, recordings or not', () {
      final receipt = notes.capture('', photos: [photo(1)]);
      final both = notes.capture(
        '',
        photos: [photo(2)],
        recordings: [voice(1)],
      );
      expect(wordless(receipt.id), Wordless.photo);
      expect(wordless(both.id), Wordless.photo);
    });

    test('by what it held when the file went and its text stayed', () {
      final memo = notes.capture('', recordings: [voice(1)]);
      notes.setTranscript(memo.id, memo.recordings.single, 'call the bank');
      notes.removeRecording(memo.id, memo.recordings.single);
      final both = notes.capture(
        '',
        photos: [photo(1)],
        recordings: [voice(2)],
      );
      notes.setPhotoText(both.id, both.photos.single, 'RECEIPT');
      notes.removePhoto(both.id, both.photos.single);
      expect(notes.byId(memo.id)?.recordings, isEmpty);
      expect(wordless(memo.id), Wordless.voiceRecording);
      expect(notes.byId(both.id)?.photos, isEmpty);
      expect(wordless(both.id), Wordless.photo);
    });

    test('as a file by its name when it holds files and no photo or '
        'recording, removed ones included', () {
      NewFile file(String name, int n) =>
          NewFile(name: name, bytes: Uint8List.fromList([n]));
      final lease = notes.capture(
        '',
        files: [file('Lease 2026.pdf', 1), file('Plan.zip', 2)],
      );
      final nameless = notes.capture('', files: [file('', 3)]);
      final withPhoto = notes.capture(
        '',
        photos: [photo(1)],
        files: [file('a.pdf', 4)],
      );
      final withVoice = notes.capture(
        '',
        recordings: [voice(1)],
        files: [file('b.pdf', 5)],
      );
      expect(wordless(lease.id), Wordless.file);
      expect(notes.byId(lease.id)?.wordlessName, 'Lease 2026.pdf');
      expect(wordless(nameless.id), Wordless.file);
      expect(notes.byId(nameless.id)?.wordlessName, '');
      expect(wordless(withPhoto.id), Wordless.photo);
      expect(notes.byId(withPhoto.id)?.wordlessName, '');
      expect(wordless(withVoice.id), Wordless.voiceRecording);
      expect(notes.byId(withVoice.id)?.wordlessName, '');
      // The file went, its text stayed: still titled by it.
      final rent = notes.capture('', files: [file('Rent.txt', 6)]);
      final hash = notes.byId(rent.id)!.attachments.single.hash;
      notes.setAttachmentText(rent.id, hash, '500 EUR');
      notes.removeAttachment(rent.id, hash);
      expect(notes.byId(rent.id)?.attachments, isEmpty);
      expect(wordless(rent.id), Wordless.file);
      expect(notes.byId(rent.id)?.wordlessName, 'Rent.txt');
      expect(wordless(notes.capture('Words').id), Wordless.photo);
    });

    test('in its alarms as in its rows', () {
      final routines = Routines(store);
      final memo = notes.capture('', recordings: [voice(1)]);
      final receipt = notes.capture('', photos: [photo(1)]);
      final lease = notes.capture(
        '',
        files: [
          NewFile(name: 'Lease.pdf', bytes: Uint8List.fromList([1])),
        ],
      );
      for (final note in [memo, receipt, lease]) {
        routines.spawn(
          note.id,
          Schedule(start: Day(2026, 10, 25), times: [ClockTime(10, 0)]),
        );
      }
      final titled = {
        for (final alarm in Alarms(store).plan(europe))
          routines.byId(alarm.routineId)?.noteId: (
            alarm.wordless,
            alarm.wordlessName,
          ),
      };
      expect(titled, {
        memo.id: (Wordless.voiceRecording, ''),
        receipt.id: (Wordless.photo, ''),
        lease.id: (Wordless.file, 'Lease.pdf'),
      });
    });
  });
}

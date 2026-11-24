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

    test('as the list it is when a list has no Name (#309), by what it '
        'holds first', () {
      final socks = notes.capture('Socks\nTent').id;
      Checklists(store).makeFromText(socks);
      expect(wordless(socks), Wordless.checklist);
      final milk = notes.capture('Milk').id;
      ShoppingLists(store).makeFromText(milk);
      expect(wordless(milk), Wordless.shoppingList);
      // What it holds wins.
      final receipt = notes.capture('Bread', photos: [photo(1)]).id;
      ShoppingLists(store).makeFromText(receipt);
      expect(wordless(receipt), Wordless.photo);
      // Back to a plain Note, and a kind no app writes, nothing but a
      // photo.
      Checklists(store).toPlain(socks);
      expect(wordless(socks), Wordless.photo);
      store.append(milk, ChecklistKeys.kind, 'tally:x');
      expect(wordless(milk), Wordless.photo);
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

  group('a Note without text of its own is named (obf-1049)', () {
    String? name(String id) => notes.byId(id)?.shownName;
    String? calendar(String id) => notes.byId(id)?.calendarText;

    test('by the first line of its Transcript, the whole of it for the '
        'calendar, nothing copied into it', () {
      final memo = notes.capture('', recordings: [voice(1)]);
      notes.setTranscript(
        memo.id,
        memo.recordings.single,
        '\n  Call the plumber  \r\nabout the leak\n',
      );
      expect(name(memo.id), 'Call the plumber');
      expect(calendar(memo.id), 'Call the plumber  \r\nabout the leak');
      expect(notes.byId(memo.id)?.text, '');
      // Its recording gone, the Transcript stays and still names it.
      notes.removeRecording(memo.id, memo.recordings.single);
      expect(name(memo.id), 'Call the plumber');
    });

    test('by its own text while it has any', () {
      final groceries = notes.capture('Groceries', recordings: [voice(1)]);
      notes.setTranscript(groceries.id, groceries.recordings.single, 'Milk');
      expect(name(groceries.id), 'Groceries');
      expect(calendar(groceries.id), 'Groceries');
    });

    test('by its first Transcript with words, in the order added, before '
        'any Photo Text', () {
      final both = notes.capture(
        '',
        photos: [photo(1)],
        recordings: [voice(1), voice(2)],
      );
      notes.setPhotoText(both.id, both.photos.single, 'RECEIPT');
      expect(name(both.id), 'RECEIPT');
      notes.setTranscript(both.id, both.recordings.first, '');
      notes.setTranscript(both.id, both.recordings.last, 'Second words');
      expect(name(both.id), 'Second words');
      // The order added counts, not the order of the hashes.
      final ab = notes.capture('', recordings: [voice(3), voice(4)]);
      final ba = notes.capture('', recordings: [voice(4), voice(3)]);
      final [three, four] = ab.recordings;
      expect(ba.recordings, [four, three]);
      for (final note in [ab, ba]) {
        notes.setTranscript(note.id, three, 'Three');
        notes.setTranscript(note.id, four, 'Four');
      }
      expect(name(ab.id), 'Three');
      expect(name(ba.id), 'Four');
    });

    test('by its first Photo Text while no Transcript has words', () {
      final receipt = notes.capture('', photos: [photo(1)]);
      notes.setPhotoText(
        receipt.id,
        receipt.photos.single,
        '  RECEIPT\nTotal 12,50  ',
      );
      expect(name(receipt.id), 'RECEIPT');
      expect(calendar(receipt.id), 'RECEIPT\nTotal 12,50');
    });

    test('by nothing while it has neither, titled by what it holds', () {
      final untranscribed = notes.capture('', recordings: [voice(1)]);
      expect(name(untranscribed.id), '');
      expect(calendar(untranscribed.id), '');
      expect(notes.byId(untranscribed.id)?.wordless, Wordless.voiceRecording);
    });

    test('in its alarms and Heads-ups', () {
      final routines = Routines(store);
      final memo = notes.capture('', recordings: [voice(1)]);
      notes.setTranscript(memo.id, memo.recordings.single, 'Pills\nblue');
      final silent = notes.capture('', recordings: [voice(2)]);
      for (final note in [memo, silent]) {
        final id = routines
            .spawn(
              note.id,
              Schedule(start: Day(2026, 10, 25), times: [ClockTime(10, 0)]),
            )
            .id;
        routines.setHeadsUps(id, [20]);
      }
      String? noteOf(String routineId) => routines.byId(routineId)?.noteId;
      final alarms = Alarms(store);
      expect(
        {for (final a in alarms.plan(europe)) noteOf(a.routineId): a.text},
        {memo.id: 'Pills', silent.id: ''},
      );
      expect(
        {for (final h in alarms.headsUps(europe)) noteOf(h.routineId): h.text},
        {memo.id: 'Pills', silent.id: ''},
      );
    });

    test('in the calendar, the whole Transcript its description', () {
      final memo = notes.capture('', recordings: [voice(1)]);
      notes.setTranscript(memo.id, memo.recordings.single, 'Pills\nthe blue');
      final routine = Routines(store).spawn(
        memo.id,
        Schedule(start: Day(2026, 10, 25), times: [ClockTime(10, 0)]),
      );
      final text = notes.byId(memo.id)?.calendarText ?? '';
      final event = routineEvents(routine, text).single;
      expect(event.summary, 'Pills');
      expect(event.description, 'Pills\nthe blue');
    });
  });
}

import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Stand-ins for photos: the store keeps any bytes as they are.
Uint8List photo(int n) => Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xD9, n]);

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

  /// Captures a photo Note with [photos] a minute after the last one.
  Note capture(List<Uint8List> photos, [String text = '']) {
    now = now.add(const Duration(minutes: 1));
    return notes.capture(text, photos: photos);
  }

  group('Photo Texts', () {
    test('a Photo Text is kept next to its photo, which stays', () {
      final note = capture([photo(1)]);
      final hash = note.photos.single;
      notes.setPhotoText(note.id, hash, '  Dr. Weber\nMon 9-12 ');
      final read = notes.byId(note.id);
      expect(read?.photoTexts, {hash: 'Dr. Weber\nMon 9-12'});
      expect(read?.photos, [hash]);
      expect(read?.text, '');
      expect(notes.photoBytes(hash), photo(1));
    });

    test('a Note without Photo Texts has none', () {
      expect(capture([photo(1)]).photoTexts, isEmpty);
    });

    test('each photo of a Note has its own Photo Text', () {
      final note = capture([photo(1), photo(2)]);
      notes.setPhotoText(note.id, hashOf(photo(2)), 'second');
      notes.setPhotoText(note.id, hashOf(photo(1)), 'first');
      expect(notes.byId(note.id)?.photoTexts, {
        hashOf(photo(1)): 'first',
        hashOf(photo(2)): 'second',
      });
    });

    test('a photo added later gets its own Photo Text', () {
      final note = capture([], 'parking');
      final hash = notes.addPhoto(note.id, photo(1));
      notes.setPhotoText(note.id, hash, 'Level 3');
      expect(notes.byId(note.id)?.photoTexts, {hash: 'Level 3'});
    });

    test('a Photo Text goes only to a photo of that Note', () {
      final note = capture([photo(1)]);
      final other = capture([photo(2)]);
      expect(
        () => notes.setPhotoText(note.id, hashOf(photo(2)), 'x'),
        throwsArgumentError,
      );
      expect(
        () => notes.setPhotoText('note:none', hashOf(photo(1)), 'x'),
        throwsArgumentError,
      );
      expect(notes.byId(other.id)?.photoTexts, isEmpty);
    });

    test('Photo Texts and Transcripts do not mix', () {
      final note = notes.capture(
        '',
        photos: [photo(1)],
        recordings: [photo(2)],
      );
      notes.setPhotoText(note.id, hashOf(photo(1)), 'on the photo');
      expect(
        () => notes.setPhotoText(note.id, hashOf(photo(2)), 'x'),
        throwsArgumentError,
      );
      final read = notes.byId(note.id);
      expect(read?.transcripts, isEmpty);
      expect(read?.photoTexts, {hashOf(photo(1)): 'on the photo'});
    });

    test('Photo Texts are in the log and survive a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final opened = Store.open('${dir.path}/log.db');
      final note = Notes(opened).capture('', photos: [photo(1)]);
      Notes(opened).setPhotoText(note.id, note.photos.single, 'receipt');
      opened.close();
      final reopened = Store.open('${dir.path}/log.db');
      addTearDown(reopened.close);
      expect(Notes(reopened).byId(note.id)?.photoTexts, {
        hashOf(photo(1)): 'receipt',
      });
    });

    test('a merged Note brings its Photo Texts as a Section', () {
      final into = capture([photo(1)], 'appointment');
      final merged = capture([photo(2)]);
      notes.setPhotoText(merged.id, hashOf(photo(2)), 'Room 12');
      notes.merge(merged.id, into.id);
      final section = notes.sections(into.id).single;
      expect(section.photos, [hashOf(photo(2))]);
      expect(section.photoTexts, {hashOf(photo(2)): 'Room 12'});
      // Read after the Merge, it still lands next to its photo.
      notes.setPhotoText(merged.id, hashOf(photo(2)), 'Room 14');
      expect(notes.sections(into.id).single.photoTexts, {
        hashOf(photo(2)): 'Room 14',
      });
    });
  });

  group('photos without a Photo Text', () {
    test('are listed newest capture first, merged Notes included', () {
      final first = capture([photo(1)]);
      final second = capture([photo(2), photo(3)]);
      final target = capture([], 'shopping');
      notes.merge(first.id, target.id);
      expect(notes.unreadPhotos(), [
        (noteId: second.id, hash: hashOf(photo(2))),
        (noteId: second.id, hash: hashOf(photo(3))),
        (noteId: first.id, hash: hashOf(photo(1))),
      ]);
    });

    test('drop out once read, also when no text was found', () {
      final note = capture([photo(1), photo(2)]);
      notes.setPhotoText(note.id, hashOf(photo(1)), '');
      expect(notes.unreadPhotos(), [(noteId: note.id, hash: hashOf(photo(2)))]);
      expect(notes.byId(note.id)?.photoTexts, {hashOf(photo(1)): ''});
      notes.setPhotoText(note.id, hashOf(photo(2)), 'words');
      expect(notes.unreadPhotos(), isEmpty);
    });

    test('leave out Notes in the trash until they are restored', () {
      final note = capture([photo(1)]);
      capture([], 'text only');
      notes.trash(note.id);
      expect(notes.unreadPhotos(), isEmpty);
      notes.restore(note.id);
      expect(notes.unreadPhotos(), [(noteId: note.id, hash: hashOf(photo(1)))]);
    });
  });

  group('search', () {
    test('finds a Note by its Photo Text, ignoring case', () {
      final shot = capture([photo(1)]);
      capture([], 'call the garage');
      notes.setPhotoText(shot.id, hashOf(photo(1)), 'Invoice\nTOTAL 42 EUR');
      expect(Search(store).notes('total').map((n) => n.id), [shot.id]);
    });

    test('finds a merged Note\'s Photo Text under its target, once', () {
      final target = capture([], 'doctor');
      final merged = capture([photo(1)]);
      notes.setPhotoText(merged.id, hashOf(photo(1)), 'Room 12');
      notes.merge(merged.id, target.id);
      expect(Search(store).notes('room 12').map((n) => n.id), [target.id]);
    });

    test('finds it within the Area the Note is filed in only', () {
      final area = Places(store).addArea('Health');
      final other = Places(store).addArea('Car');
      final shot = capture([photo(1)]);
      notes.file(shot.id, area.id);
      notes.setPhotoText(shot.id, hashOf(photo(1)), 'Ibuprofen 400');
      expect(Search(store).notes('ibuprofen', within: area.id), hasLength(1));
      expect(Search(store).notes('ibuprofen', within: other.id), isEmpty);
    });

    test('"no text found" finds nothing', () {
      final shot = capture([photo(1)]);
      notes.setPhotoText(shot.id, hashOf(photo(1)), '');
      expect(Search(store).notes('x'), isEmpty);
    });

    test('a trashed Note\'s Photo Text is not found', () {
      final note = capture([photo(1)]);
      notes.setPhotoText(note.id, hashOf(photo(1)), 'receipt');
      notes.trash(note.id);
      expect(Search(store).notes('receipt'), isEmpty);
    });
  });
}

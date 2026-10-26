import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'shared_areas_test.dart' show exchange, ticking;

/// Stand-ins for photos and recordings: a JPEG start, so the store keeps
/// the photo as it is, then [n].
Uint8List photo(int n) => Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xD9, n]);
Uint8List voice(int n) => Uint8List.fromList([0x52, 0x49, 0x46, 0x46, n]);

/// The blobs [to] lacks, carried over from [from], as a LAN session
/// fetches them after the entries.
void carryBlobs(Store from, Store to) {
  for (final ref in to.missingBlobs()) {
    final (kind, hash) = splitBlobRef(ref);
    if (from.blob(kind, hash) case final bytes?) to.putBlob(kind, hash, bytes);
  }
}

/// [exchange] with the blobs after it, both ways.
void syncAll(Store joiner, Store host) {
  exchange(joiner, host);
  carryBlobs(host, joiner);
  carryBlobs(joiner, host);
}

List<String> found(Store store, String query) => [
  for (final n in Search(store).notes(query)) n.id,
];

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
  });
  tearDown(() => ann.close());

  /// A Note with a photo and a recording, both with their text, filed
  /// into [place] when given.
  ({String note, String photo, String recording}) withTexts(
    Store store, {
    String? place,
    int n = 1,
  }) {
    final notes = Notes(store);
    final note = notes.capture(
      'Dentist',
      place: place,
      photos: [photo(n)],
      recordings: [voice(n)],
    );
    final p = note.photos.single;
    final r = note.recordings.single;
    notes.setPhotoText(note.id, p, 'Dr. Weber, Mon 9-12');
    notes.setTranscript(note.id, r, 'Bring the old X-ray');
    return (note: note.id, photo: p, recording: r);
  }

  group('removing a photo or recording', () {
    test('the file goes, its text stays and search finds it', () {
      final m = withTexts(ann);
      expect(notes.mayRemovePhoto(m.note, m.photo), isTrue);
      expect(notes.mayRemoveRecording(m.note, m.recording), isTrue);
      notes.removePhoto(m.note, m.photo);
      notes.removeRecording(m.note, m.recording);
      final note = notes.byId(m.note)!;
      expect(note.photos, isEmpty);
      expect(note.recordings, isEmpty);
      expect(note.removedPhotos, [m.photo]);
      expect(note.removedRecordings, [m.recording]);
      expect(note.photoTexts, {m.photo: 'Dr. Weber, Mon 9-12'});
      expect(note.transcripts, {m.recording: 'Bring the old X-ray'});
      expect(notes.photoBytes(m.photo), isNull);
      expect(notes.recordingBytes(m.recording), isNull);
      expect(ann.missingBlobs(), isEmpty, reason: 'not wanted back');
      expect(ann.knowsBlob('photo/${m.photo}'), isFalse);
      expect(found(ann, 'weber'), [m.note]);
      expect(found(ann, 'x-ray'), [m.note]);
      expect(notes.mayRemovePhoto(m.note, m.photo), isFalse);
      // An edit like any other: the null is in the log.
      expect(ann.current(m.note, NoteKeys.photo(m.photo)), isNull);
      expect(ann.history(m.note).last.field, NoteKeys.audio(m.recording));
    });

    test('a photo or recording without a text, or with an empty one, is '
        'refused', () {
      final note = notes.capture(
        '',
        photos: [photo(1), photo(2)],
        recordings: [voice(1)],
      );
      final [p1, p2] = note.photos;
      final r = note.recordings.single;
      notes.setPhotoText(note.id, p2, '   ');
      for (final hash in [p1, p2]) {
        expect(notes.mayRemovePhoto(note.id, hash), isFalse);
        expect(() => notes.removePhoto(note.id, hash), throwsStateError);
      }
      expect(notes.mayRemoveRecording(note.id, r), isFalse);
      expect(() => notes.removeRecording(note.id, r), throwsStateError);
      expect(
        () => notes.removePhoto(note.id, r),
        throwsArgumentError,
        reason: 'a recording is no photo',
      );
      expect(notes.mayRemovePhoto('note:none', p1), isFalse);
      expect(notes.byId(note.id)!.photos, [p1, p2]);
      expect(notes.photoBytes(p1), isNotNull);
    });

    test('the bytes stay while another Note shows the same photo', () {
      final m = withTexts(ann);
      final other = notes.capture('Same card', photos: [photo(1)]);
      expect(other.photos, [m.photo]);
      notes.removePhoto(m.note, m.photo);
      expect(notes.photoBytes(m.photo), photo(1));
      expect(ann.removedBlobs(), isEmpty);
      expect(notes.byId(other.id)!.photos, [m.photo]);
    });

    test('a merged Note keeps the text in its Section', () {
      final m = withTexts(ann);
      final target = notes.capture('Health');
      notes.merge(m.note, target.id);
      expect(notes.mayRemovePhoto(m.note, m.photo), isTrue);
      notes.removePhoto(m.note, m.photo);
      final section = notes.sections(target.id).single;
      expect(section.photos, isEmpty);
      expect(section.removedPhotos, [m.photo]);
      expect(section.photoTexts[m.photo], 'Dr. Weber, Mon 9-12');
      expect(notes.photoBytes(m.photo), isNull);
      expect(found(ann, 'weber'), [target.id]);
    });

    test('the removal reaches the user\'s other Device, which lets the '
        'bytes go', () {
      final desk = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(desk.close);
      final m = withTexts(ann);
      syncAll(desk, ann);
      expect(desk.photo(m.photo), isNotNull);
      expect(desk.audio(m.recording), isNotNull);
      notes.removePhoto(m.note, m.photo);
      Notes(desk).removeRecording(m.note, m.recording);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        final note = Notes(store).byId(m.note)!;
        expect(note.photos, isEmpty);
        expect(note.recordings, isEmpty);
        expect(note.photoTexts[m.photo], 'Dr. Weber, Mon 9-12');
        expect(note.transcripts[m.recording], 'Bring the old X-ray');
        expect(store.photo(m.photo), isNull);
        expect(store.audio(m.recording), isNull);
        expect(store.missingBlobs(), isEmpty);
        expect(found(store, 'weber'), [m.note]);
      }
    });

    test('a removed photo leaves the shared folder', () async {
      final desk = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(desk.close);
      exchange(desk, ann);
      final folder = MemorySyncFolder();
      final m = withTexts(ann);
      await folderSync(ann, folder);
      final names = {...folder.dirs['blobs']!.keys};
      expect(names, hasLength(2));
      await folderSync(desk, folder);
      expect(desk.photo(m.photo), isNotNull);
      notes.removePhoto(m.note, m.photo);
      await folderSync(ann, folder);
      expect(folder.dirs['blobs']!.keys, hasLength(1));
      await folderSync(desk, folder);
      expect(desk.photo(m.photo), isNull);
      expect(desk.audio(m.recording), isNotNull);
      expect(Notes(desk).byId(m.note)!.removedPhotos, [m.photo]);
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;
    late String private;

    setUp(() {
      ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
      final places = Places(ann);
      area = ann.share(places.addArea('Household').id);
      private = places.addArea('Health').id;
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    test('a Manager\'s removal reaches a Member, who lets the bytes go', () {
      final m = withTexts(ann, place: area);
      syncAll(ben, ann);
      expect(ben.photo(m.photo), isNotNull);
      expect(Notes(ben).mayRemovePhoto(m.note, m.photo), isFalse);
      notes.removePhoto(m.note, m.photo);
      syncAll(ben, ann);
      final note = Notes(ben).byId(m.note)!;
      expect(note.photos, isEmpty);
      expect(note.removedPhotos, [m.photo]);
      expect(ben.photo(m.photo), isNull);
      expect(ben.missingBlobs(), isEmpty);
      expect(found(ben, 'weber'), [m.note]);
    });

    test('a Member removes from their own Note, a Manager from anyone\'s', () {
      final own = withTexts(ben, place: area, n: 2);
      syncAll(ben, ann);
      expect(Notes(ben).mayRemovePhoto(own.note, own.photo), isTrue);
      Notes(ben).removePhoto(own.note, own.photo);
      expect(notes.mayRemoveRecording(own.note, own.recording), isTrue);
      notes.removeRecording(own.note, own.recording);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final note = Notes(store).byId(own.note)!;
        expect(note.removedPhotos, [own.photo]);
        expect(note.removedRecordings, [own.recording]);
        expect(store.photo(own.photo), isNull);
        expect(store.audio(own.recording), isNull);
      }
    });

    test('a Member\'s removal on another\'s Note is refused and, written '
        'all the same, ignored everywhere; the bytes stay', () {
      final m = withTexts(ann, place: area);
      syncAll(ben, ann);
      expect(Notes(ben).mayRemovePhoto(m.note, m.photo), isFalse);
      expect(() => Notes(ben).removePhoto(m.note, m.photo), throwsStateError);
      // As a modified app would write it.
      ben.append(m.note, NoteKeys.photo(m.photo), null);
      ben.dropRemovedBlobs();
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        expect(Notes(store).byId(m.note)!.photos, [m.photo]);
        expect(store.photo(m.photo), isNotNull);
      }
    });

    test('another person\'s removal outside a Shared Area hides the file '
        'but keeps its bytes', () {
      final m = withTexts(ann, place: area);
      syncAll(ben, ann);
      // Ann takes the Note out of the Shared Area; Ben has not seen it
      // and writes a removal as a modified app would.
      notes.file(m.note, private);
      ben.append(m.note, NoteKeys.photo(m.photo), null);
      exchange(ben, ann);
      expect(ann.current(m.note, NoteKeys.photo(m.photo)), isNull);
      expect(ann.removedBlobs(), isEmpty);
      expect(ann.photo(m.photo), photo(1));
    });
  });
}

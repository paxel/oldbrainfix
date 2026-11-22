import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show photo, syncAll, voice;
import 'shared_areas_test.dart' show exchange, ticking;

typedef _Waiting = List<({String noteId, String hash})>;

/// What [Notes.unreadPhotos], or [Notes.untranscribed] when [photos] is
/// false, listed before obf-1094: each Note made on its own by
/// [Notes.byId].
_Waiting _oneByOne(Store store, {required bool photos}) {
  final notes = Notes(store);
  List<String> blobs(Note note) => photos ? note.photos : note.recordings;
  bool waits(Note note, String hash) => photos
      ? !note.photoTexts.containsKey(hash) &&
            !note.removedPhotoTexts.contains(hash)
      : !note.transcripts.containsKey(hash) &&
            !note.removedTranscripts.contains(hash);
  final listed = [
    for (final id in store.everOfType(NoteKeys.typeNote))
      if (notes.byId(id) case final note? when !note.isTrashed)
        if (blobs(note).any((h) => waits(note, h))) note,
  ];
  listed.sort((a, b) => b.captured.compareTo(a.captured));
  return [
    for (final note in listed)
      for (final hash in blobs(note))
        if (waits(note, hash)) (noteId: note.id, hash: hash),
  ];
}

/// Notes of every kind the readers tell apart, their photos and
/// recordings made from [n] up to [n] + 10; the photos left waiting.
List<String> _seed(Notes notes, int n) {
  notes.capture('plain $n');
  final several = notes.capture(
    'several',
    photos: [photo(n), photo(n + 1), photo(n + 2)],
    recordings: [voice(n), voice(n + 1)],
  );
  notes.setPhotoText(several.id, several.photos[1], 'Read');
  notes.setPhotoText(several.id, several.photos[2], '');
  notes.setTranscript(several.id, several.recordings[0], 'Heard');
  for (final (i, kind) in ['read', 'removed text', 'removed'].indexed) {
    final note = notes.capture(
      kind,
      photos: [photo(n + 3 + i)],
      recordings: [voice(n + 2 + i)],
    );
    final (p, r) = (note.photos.single, note.recordings.single);
    notes.setPhotoText(note.id, p, 'Text');
    notes.setTranscript(note.id, r, 'Words');
    if (kind == 'removed text') {
      notes.removePhotoText(note.id, p);
      notes.removeTranscript(note.id, r);
    } else if (kind == 'removed') {
      notes.removePhoto(note.id, p);
      notes.removeRecording(note.id, r);
    }
  }
  final trashed = notes.capture(
    'trashed',
    photos: [photo(n + 6)],
    recordings: [voice(n + 5)],
  );
  notes.trash(trashed.id);
  final restored = notes.capture(
    'restored',
    photos: [photo(n + 7)],
    recordings: [voice(n + 6)],
  );
  notes.trash(restored.id);
  notes.restore(restored.id);
  final merged = notes.capture(
    'merged',
    photos: [photo(n + 8)],
    recordings: [voice(n + 7)],
  );
  notes.merge(merged.id, notes.capture('target').id);
  notes.discard(
    notes
        .capture(
          'discarded',
          photos: [photo(n + 9)],
          recordings: [voice(n + 8)],
        )
        .id,
  );
  final twin = notes.capture(
    'twin',
    photos: [photo(n + 10)],
    recordings: [voice(n + 9)],
  );
  notes.store.append(
    twin.id,
    NoteKeys.captured,
    notes.store.current(several.id, NoteKeys.captured),
  );
  return [
    several.photos[0],
    restored.photos.single,
    merged.photos.single,
    twin.photos.single,
  ];
}

void main() {
  late Store store;
  late Notes notes;

  setUp(() {
    store = Store.inMemory(clock: ticking());
    addTearDown(store.close);
    notes = Notes(store);
  });

  test('the photos and recordings waiting are those each Note made on its '
      'own leaves, in its order', () {
    final waiting = [..._seed(notes, 0), ..._seed(notes, 20)];
    final photos = notes.unreadPhotos();
    expect(photos, _oneByOne(store, photos: true));
    expect(notes.untranscribed(), _oneByOne(store, photos: false));
    expect([for (final p in photos) p.hash], unorderedEquals(waiting));
    expect(notes.untranscribed(), hasLength(8));
  });

  test('the reads do not grow with the Notes, plain or holding a photo or '
      'recording', () {
    int reads(_Waiting Function() read) {
      // What the store works out once per change is not counted.
      read();
      final before = store.historyReads + store.loggedReads;
      read();
      return store.historyReads + store.loggedReads - before;
    }

    _seed(notes, 0);
    final photos = reads(notes.unreadPhotos);
    final recordings = reads(notes.untranscribed);
    for (var i = 1; i < 6; i++) {
      _seed(notes, 20 * i);
    }
    for (var i = 0; i < 40; i++) {
      notes.capture('plain $i');
    }
    expect(reads(notes.unreadPhotos), photos);
    expect(reads(notes.untranscribed), recordings);
    expect(photos, lessThanOrEqualTo(2));
  });

  test('a removal of a text not read yet, written by one who may not, '
      'leaves the photo and recording waiting', () {
    final clock = ticking();
    final ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    final ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
    addTearDown(ann.close);
    addTearDown(ben.close);
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    final note = Notes(
      ann,
    ).capture('Shed', place: area, photos: [photo(1)], recordings: [voice(1)]);
    syncAll(ben, ann);
    // As a modified app would write it: a Member removes a text only on
    // their own Note.
    ben.append(note.id, NoteKeys.ocr(note.photos.single), null);
    ben.append(note.id, NoteKeys.transcript(note.recordings.single), null);
    syncAll(ben, ann);
    for (final store in [ann, ben]) {
      expect(Notes(store).unreadPhotos(), [
        (noteId: note.id, hash: note.photos.single),
      ]);
      expect(Notes(store).untranscribed(), [
        (noteId: note.id, hash: note.recordings.single),
      ]);
    }
  });
}

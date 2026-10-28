import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show found, photo, syncAll, voice;
import 'shared_areas_test.dart' show exchange, ticking;

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

  group('removing a Transcript or Photo Text', () {
    test('the text goes, the file stays, search no longer finds it and '
        'nothing is read again', () {
      final m = withTexts(ann);
      expect(notes.mayRemovePhotoText(m.note, m.photo), isTrue);
      expect(notes.mayRemoveTranscript(m.note, m.recording), isTrue);
      notes.removePhotoText(m.note, m.photo);
      notes.removeTranscript(m.note, m.recording);
      final note = notes.byId(m.note)!;
      expect(note.photos, [m.photo]);
      expect(note.recordings, [m.recording]);
      expect(note.photoTexts, isEmpty);
      expect(note.transcripts, isEmpty);
      expect(note.removedPhotoTexts, [m.photo]);
      expect(note.removedTranscripts, [m.recording]);
      expect(note.removedPhotos, isEmpty);
      expect(note.removedRecordings, isEmpty);
      expect(notes.photoBytes(m.photo), isNotNull);
      expect(notes.recordingBytes(m.recording), isNotNull);
      expect(found(ann, 'weber'), isEmpty);
      expect(found(ann, 'x-ray'), isEmpty);
      expect(notes.unreadPhotos(), isEmpty);
      expect(notes.untranscribed(), isEmpty);
      // Without a text the file may not go either.
      expect(notes.mayRemovePhoto(m.note, m.photo), isFalse);
      expect(notes.mayRemovePhotoText(m.note, m.photo), isFalse);
      // An edit like any other: the null is in the log.
      expect(ann.history(m.note).last.field, NoteKeys.transcript(m.recording));
      expect(ann.history(m.note).last.value, isNull);
    });

    test('transcribing or reading again brings a text back', () {
      final m = withTexts(ann);
      notes.removePhotoText(m.note, m.photo);
      notes.removeTranscript(m.note, m.recording);
      notes.setPhotoText(m.note, m.photo, 'Dr. Weber, Tue 9-12');
      notes.setTranscript(m.note, m.recording, 'Bring the new X-ray');
      final note = notes.byId(m.note)!;
      expect(note.photoTexts, {m.photo: 'Dr. Weber, Tue 9-12'});
      expect(note.transcripts, {m.recording: 'Bring the new X-ray'});
      expect(note.removedPhotoTexts, isEmpty);
      expect(note.removedTranscripts, isEmpty);
      expect(found(ann, 'tue'), [m.note]);
    });

    test('a missing or empty text, or one whose file is gone, is refused', () {
      final note = notes.capture(
        '',
        photos: [photo(1), photo(2)],
        recordings: [voice(1)],
      );
      final [p1, p2] = note.photos;
      final r = note.recordings.single;
      notes.setPhotoText(note.id, p2, '');
      for (final hash in [p1, p2]) {
        expect(notes.mayRemovePhotoText(note.id, hash), isFalse);
        expect(() => notes.removePhotoText(note.id, hash), throwsStateError);
      }
      expect(notes.mayRemoveTranscript(note.id, r), isFalse);
      expect(() => notes.removeTranscript(note.id, r), throwsStateError);
      expect(
        () => notes.removePhotoText(note.id, r),
        throwsArgumentError,
        reason: 'a recording is no photo',
      );
      expect(notes.mayRemoveTranscript('note:none', r), isFalse);

      final m = withTexts(ann, n: 3);
      notes.removePhoto(m.note, m.photo);
      expect(notes.mayRemovePhotoText(m.note, m.photo), isFalse);
      expect(
        () => notes.removePhotoText(m.note, m.photo),
        throwsArgumentError,
        reason: 'the text of a removed photo is all that is left of it',
      );
    });

    test('a merged Note shows the removal in its Section', () {
      final m = withTexts(ann);
      final target = notes.capture('Health');
      notes.merge(m.note, target.id);
      expect(notes.mayRemoveTranscript(m.note, m.recording), isTrue);
      notes.removeTranscript(m.note, m.recording);
      final section = notes.sections(target.id).single;
      expect(section.recordings, [m.recording]);
      expect(section.transcripts, isEmpty);
      expect(section.removedTranscripts, [m.recording]);
      expect(section.removedPhotoTexts, isEmpty);
      expect(found(ann, 'x-ray'), isEmpty);
      expect(found(ann, 'weber'), [target.id]);
    });

    test('the removal reaches the user\'s other Device, which does not '
        'transcribe again', () {
      final desk = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(desk.close);
      final m = withTexts(ann);
      syncAll(desk, ann);
      notes.removeTranscript(m.note, m.recording);
      Notes(desk).removePhotoText(m.note, m.photo);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        final note = Notes(store).byId(m.note)!;
        expect(note.removedTranscripts, [m.recording]);
        expect(note.removedPhotoTexts, [m.photo]);
        expect(store.audio(m.recording), isNotNull);
        expect(store.photo(m.photo), isNotNull);
        expect(Notes(store).untranscribed(), isEmpty);
        expect(Notes(store).unreadPhotos(), isEmpty);
        expect(found(store, 'weber'), isEmpty);
      }
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
      area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    test('a Member removes from their own Note, a Manager from anyone\'s', () {
      final own = withTexts(ben, place: area, n: 2);
      syncAll(ben, ann);
      expect(Notes(ben).mayRemovePhotoText(own.note, own.photo), isTrue);
      Notes(ben).removePhotoText(own.note, own.photo);
      expect(notes.mayRemoveTranscript(own.note, own.recording), isTrue);
      notes.removeTranscript(own.note, own.recording);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final note = Notes(store).byId(own.note)!;
        expect(note.removedPhotoTexts, [own.photo]);
        expect(note.removedTranscripts, [own.recording]);
      }
    });

    test('a Member\'s removal on another\'s Note is refused and, written '
        'all the same, ignored everywhere', () {
      final m = withTexts(ann, place: area);
      syncAll(ben, ann);
      expect(Notes(ben).mayRemoveTranscript(m.note, m.recording), isFalse);
      expect(
        () => Notes(ben).removeTranscript(m.note, m.recording),
        throwsStateError,
      );
      // As a modified app would write it.
      ben.append(m.note, NoteKeys.transcript(m.recording), null);
      ben.append(m.note, NoteKeys.ocr(m.photo), null);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final note = Notes(store).byId(m.note)!;
        expect(note.transcripts[m.recording], 'Bring the old X-ray');
        expect(note.photoTexts[m.photo], 'Dr. Weber, Mon 9-12');
        expect(note.removedTranscripts, isEmpty);
        expect(found(store, 'x-ray'), [m.note]);
      }
    });

    test('a Member\'s machine still gives another\'s Note its texts', () {
      final note = notes.capture(
        'Shed',
        place: area,
        photos: [photo(4)],
        recordings: [voice(4)],
      );
      syncAll(ben, ann);
      Notes(ben).setPhotoText(note.id, note.photos.single, 'Spare key');
      Notes(ben).setTranscript(note.id, note.recordings.single, 'Oil the door');
      syncAll(ben, ann);
      final read = notes.byId(note.id)!;
      expect(read.photoTexts, {note.photos.single: 'Spare key'});
      expect(read.transcripts, {note.recordings.single: 'Oil the door'});
    });
  });
}

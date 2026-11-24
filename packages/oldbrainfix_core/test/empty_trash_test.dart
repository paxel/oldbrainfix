import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show photo, syncAll, voice;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;
  late Routines routines;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
    routines = Routines(ann);
  });
  tearDown(() => ann.close());

  final daily = Schedule(
    start: Day(2026, 3, 1),
    times: [ClockTime(8, 0)],
    rule: Recurrence(frequency: Frequency.daily),
  );
  final lease = NewFile(
    name: 'lease.txt',
    bytes: Uint8List.fromList('The lease'.codeUnits),
  );

  test('what is in the trash goes at once, however long it has been '
      'there; the count names what goes, the log keeps it all', () {
    final kept = notes.capture('Kept');
    final recent = notes.capture('Recent');
    final amended = notes.capture('Amended');
    notes.amend(amended.id, 'later');
    final merged = notes.capture('Merged');
    notes.merge(merged.id, amended.id);
    final dentist = notes.capture('Dentist');
    final reminder = routines.spawn(dentist.id, daily);
    final pills = notes.capture('Pills');
    final alone = routines.spawn(pills.id, daily);
    final old = notes.capture('Old');
    notes.trash(recent.id);
    notes.trash(amended.id);
    routines.trashNote(dentist.id);
    routines.trash(alone.id);
    // Past the 30 days, as a Device that has not purged yet holds it.
    ann.append(
      old.id,
      NoteKeys.trashed,
      DateTime.utc(2025, 11, 1).toIso8601String(),
    );

    final trash = Trash(ann);
    final asked = trash.toEmpty();
    expect(asked.notes.toSet(), {recent.id, amended.id, dentist.id, old.id});
    expect(asked.routines.toSet(), {reminder.id, alone.id});
    expect(asked.blobs, isEmpty);

    final removed = trash.empty();
    expect(removed.notes, asked.notes);
    expect(removed.routines, asked.routines);
    for (final id in [recent.id, amended.id, merged.id, dentist.id, old.id]) {
      expect(notes.byId(id), isNull);
      expect(ann.history(id), isNotEmpty, reason: 'the log keeps it');
    }
    expect(notes.sections(amended.id), isEmpty);
    expect(ann.entitiesOfType(SectionKeys.typeSection), isEmpty);
    expect(routines.byId(reminder.id), isNull);
    expect(routines.byId(alone.id), isNull);
    expect(notes.trashed(), isEmpty);
    expect(routines.trashed(), isEmpty);
    expect(notes.all().map((n) => n.id), [pills.id, kept.id]);
    expect(routines.all(), isEmpty);

    expect(trash.toEmpty().isEmpty, isTrue);
    final mark = ann.currentSeq();
    expect(trash.empty().isEmpty, isTrue);
    expect(ann.entriesAfter(mark), isEmpty, reason: 'nothing written');
  });

  test('a Note and a Routine an AI Task only drafted into the trash stay; '
      'rejected, they are back', () {
    final drafted = notes.capture('Drafted');
    final routine = routines.spawn(drafted.id, daily);
    final alone = routines.spawn(notes.capture('Alone').id, daily);
    final set = ann.openPending('ai-task', shown: true);
    final at = clock().toIso8601String();
    ann.draftPending(set, drafted.id, NoteKeys.trashed, at);
    ann.draftPending(set, routine.id, RoutineKeys.trashed, at);
    ann.draftPending(set, alone.id, RoutineKeys.trashed, at);
    // Shown as the screens show it, waiting to be accepted.
    expect(notes.trashed().map((n) => n.id), [drafted.id]);
    expect(routines.trashed().map((r) => r.id), contains(alone.id));

    final trash = Trash(ann);
    expect(trash.toEmpty().isEmpty, isTrue);
    final mark = ann.currentSeq();
    expect(trash.empty().isEmpty, isTrue);
    expect(ann.entriesAfter(mark), isEmpty, reason: 'nothing written');
    ann.declinePending(set);
    expect(notes.byId(drafted.id)?.isTrashed, isFalse);
    expect(routines.byId(routine.id)?.isTrashed, isFalse);
    expect(routines.byId(alone.id)?.isTrashed, isFalse);
  });

  test('the files of the Notes leave this Device and are not wanted '
      'back; one another Note shows stays', () {
    final card = notes.capture(
      'Dentist',
      photos: [photo(1), photo(2)],
      recordings: [voice(1)],
      files: [lease],
    );
    final [shared, own] = card.photos;
    final recording = card.recordings.single;
    final file = card.attachments.single.hash;
    final target = notes.capture('Health', recordings: [voice(2)]);
    final into = notes.capture('Into it', photos: [photo(3)]);
    notes.merge(into.id, target.id);
    final other = notes.capture('Same card', photos: [photo(1)]);
    notes.trash(card.id);
    notes.trash(target.id);

    final removed = Trash(ann).empty();
    expect(notes.byId(into.id), isNull);
    final fromTarget = target.recordings.single;
    final fromMerged = into.photos.single;
    expect(
      removed.blobs,
      [
        'audio/$recording',
        'audio/$fromTarget',
        'file/$file',
        'photo/$own',
        'photo/$fromMerged',
      ]..sort(),
    );
    expect(ann.photo(shared), photo(1), reason: 'another Note shows it');
    expect(notes.byId(other.id)?.photos, [shared]);
    expect(ann.photo(own), isNull);
    expect(ann.audio(recording), isNull);
    expect(ann.audio(fromTarget), isNull);
    expect(ann.file(file), isNull);
    expect(ann.photo(fromMerged), isNull);
    expect(ann.missingBlobs(), isEmpty);
    expect(ann.knowsBlob('photo/$own'), isFalse);
    expect(ann.knowsBlob('photo/$shared'), isTrue);
    // Pushed back by another Device, they are not kept.
    expect(ann.knowsBlob('file/$file'), isFalse);
  });

  test('a sync with the user\'s other Device brings nothing back and '
      'leaves its bytes there', () {
    final desk = Store.inMemory(clock: clock)
      ..applyProfileDefaults(name: 'Ann');
    addTearDown(desk.close);
    final card = notes.capture('Dentist', photos: [photo(1)], files: [lease]);
    final p = card.photos.single;
    final file = card.attachments.single.hash;
    syncAll(desk, ann);
    expect(desk.photo(p), isNotNull);
    notes.trash(card.id);
    syncAll(desk, ann);
    Trash(ann).empty();
    syncAll(desk, ann);
    syncAll(ann, desk);
    expect(ann.photo(p), isNull);
    expect(ann.file(file), isNull);
    expect(ann.missingBlobs(), isEmpty);
    expect(Notes(desk).byId(card.id), isNull, reason: 'the purge synced');
    expect(desk.photo(p), photo(1), reason: 'only this Device lets go');
    expect(desk.missingBlobs(), isEmpty);
  });

  test('in a Shared Area what this Device may not change stays in the '
      'trash with its files and is not counted', () {
    final ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
    addTearDown(ben.close);
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    final hers = notes.capture('Hers', place: area, photos: [photo(1)]);
    final routine = routines.spawn(hers.id, daily);
    final his = Notes(ben).capture('His', place: area, photos: [photo(2)]);
    syncAll(ben, ann);
    routines.trashNote(hers.id);
    Notes(ben).trash(his.id);
    syncAll(ben, ann);
    expect(Notes(ben).trashed().map((n) => n.id).toSet(), {hers.id, his.id});

    final trash = Trash(ben);
    final asked = trash.toEmpty();
    expect(asked.notes, [his.id]);
    expect(asked.routines, isEmpty, reason: 'a Member may not manage it');
    final removed = trash.empty();
    expect(removed.notes, [his.id]);
    expect(removed.routines, isEmpty);
    expect(removed.blobs, ['photo/${his.photos.single}']);
    expect(Notes(ben).trashed().map((n) => n.id), [hers.id]);
    expect(Routines(ben).byId(routine.id)?.isTrashed, isTrue);
    expect(ben.photo(hers.photos.single), photo(1));
    expect(ben.photo(his.photos.single), isNull);
    expect(ben.missingBlobs(), isEmpty);
  });
}

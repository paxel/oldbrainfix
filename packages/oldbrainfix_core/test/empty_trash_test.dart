import 'dart:io';
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

  test('it purges Trash.perBatch Notes at a time: another connection '
      'writes between two batches, and one it brings back stays', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    // One clock for both, as one Device: what the app writes is later.
    final shared = ticking();
    final emptying = Store.open(path, clock: shared);
    final app = Store.open(path, clock: shared);
    addTearDown(() {
      emptying.close();
      app.close();
    });
    final writing = Notes(emptying);
    final (card, reminder) = emptying.batch(() {
      final card = writing.capture('Card', photos: [photo(1)]);
      writing.trash(card.id);
      for (var i = 0; i < 2 * Trash.perBatch; i++) {
        writing.trash(writing.capture('Note $i').id);
      }
      final dentist = writing.capture('Dentist');
      final reminder = Routines(emptying).spawn(dentist.id, daily);
      Routines(emptying).trashNote(dentist.id);
      return (card, reminder);
    });
    final trash = Trash(emptying);
    final asked = trash.toEmpty();
    final of = 2 * Trash.perBatch + 2;
    expect(asked.notes, hasLength(of));
    expect(asked.routines, [reminder.id]);
    // In the second batch.
    final back = asked.notes[Trash.perBatch + 1];
    final mark = emptying.currentSeq();
    final told = <(int, int)>[];
    final captured = <String>[];
    final watch = Stopwatch()..start();
    final removed = trash.empty(
      progress: (done, of) {
        told.add((done, of));
        // A write here waits for no lock the purge holds.
        if (done == Trash.perBatch) Routines(app).restoreNote(back);
        captured.add(Notes(app).capture('Meanwhile $done').id);
      },
    );
    expect(watch.elapsed, lessThan(const Duration(seconds: 4)));
    expect(told, [
      (0, of),
      (Trash.perBatch, of),
      (2 * Trash.perBatch, of),
      (of, of),
    ]);
    expect(removed.notes, [...asked.notes]..remove(back));
    expect(removed.routines, [reminder.id]);
    expect(removed.blobs, ['photo/${card.photos.single}']);
    expect(emptying.photo(card.photos.single), isNull);
    expect(writing.byId(back)?.isTrashed, isFalse);
    expect(captured, hasLength(4));
    for (final id in captured) {
      expect(writing.byId(id), isNotNull);
    }
    // The purge's entries in the order one batch wrote them.
    expect(
      [
        for (final e in emptying.entriesAfter(mark))
          if (e.field == NoteKeys.type && e.value == null) e.entity,
      ],
      [...removed.notes, ...removed.routines],
    );
    final again = emptying.currentSeq();
    expect(trash.empty().isEmpty, isTrue);
    expect(emptying.entriesAfter(again), isEmpty, reason: 'nothing written');
  });

  test('stopped, what it purged stays purged and its files go; the rest '
      'stays in the trash', () {
    final card = notes.capture('Card', photos: [photo(1)]);
    notes.trash(card.id);
    for (var i = 0; i < Trash.perBatch; i++) {
      notes.trash(notes.capture('Note $i').id);
    }
    final alone = routines.spawn(notes.capture('Pills').id, daily);
    routines.trash(alone.id);
    final trash = Trash(ann);
    final asked = trash.toEmpty();
    var batches = 0;
    final removed = trash.empty(stop: () => batches++ > 0);
    expect(removed.notes, asked.notes.take(Trash.perBatch));
    expect(removed.routines, isEmpty);
    expect(removed.blobs, ['photo/${card.photos.single}']);
    expect(ann.photo(card.photos.single), isNull);
    expect(notes.trashed().map((n) => n.id), [asked.notes.last]);
    expect(routines.byId(alone.id)?.isTrashed, isTrue);

    final rest = trash.empty();
    expect(rest.notes, [asked.notes.last]);
    expect(rest.routines, [alone.id]);
  });
}

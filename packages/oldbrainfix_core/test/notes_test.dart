import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  test('a captured Note carries its text and capture time', () {
    final at = DateTime.utc(2026, 10, 24, 19, 40);
    final store = Store.inMemory(clock: () => at);
    addTearDown(store.close);
    final note = Notes(store).capture('nees mor sauce');
    expect(note.text, 'nees mor sauce');
    expect(note.captured, at);
    expect(note.id, startsWith('note:'));
  });

  test('Notes are listed newest capture first', () {
    var now = DateTime.utc(2026, 10, 24, 8);
    final store = Store.inMemory(clock: () => now);
    addTearDown(store.close);
    final notes = Notes(store);
    notes.capture('first');
    now = now.add(const Duration(hours: 1));
    notes.capture('second');
    expect(notes.all().map((n) => n.text), ['second', 'first']);
  });

  test('Notes survive a reopen', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    final first = Store.open(path);
    Notes(first).capture('milk');
    first.close();
    final second = Store.open(path);
    addTearDown(second.close);
    expect(Notes(second).all().single.text, 'milk');
  });

  test('other entities are not Notes', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('area:x', r'$type', 'area');
    expect(Notes(store).all(), isEmpty);
    expect(Notes(store).byId('area:x'), isNull);
  });

  test('a Note captured into a place is filed there at once', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    final places = Places(store);
    final kitchen = places.addTopic(places.addArea('Household').id, 'Kitchen');
    final note = Notes(store).capture('nees mor sauce', place: kitchen.id);
    expect(note.place, kitchen.id);
    expect(Notes(store).byId(note.id)?.isUnsorted, isFalse);
  });

  test('capturing into something that is no place captures nothing', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(
      () => Notes(store).capture('x', place: 'area:gone'),
      throwsArgumentError,
    );
    expect(Notes(store).all(), isEmpty);
  });

  test('an Area holds its own Notes and its Topics\' ones, newest first', () {
    var now = DateTime.utc(2026, 10, 24, 8);
    final store = Store.inMemory(clock: () => now);
    addTearDown(store.close);
    final places = Places(store);
    final notes = Notes(store);
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    final work = places.addArea('Work');
    Note capture(String text, String? place) {
      now = now.add(const Duration(minutes: 1));
      return notes.capture(text, place: place);
    }

    capture('in the kitchen', kitchen.id);
    capture('at home', home.id);
    capture('at work', work.id);
    capture('unsorted', null);
    expect(notes.inside(home.id).map((n) => n.text), [
      'at home',
      'in the kitchen',
    ]);
    expect(notes.inside(kitchen.id).map((n) => n.text), ['in the kitchen']);
    expect(places.notesInside(home.id), 2);
  });

  group('editing', () {
    late Store store;
    late Notes notes;
    var now = DateTime.utc(2026, 10, 24, 8);

    setUp(() {
      now = DateTime.utc(2026, 10, 24, 8);
      store = Store.inMemory(clock: () => now);
      notes = Notes(store);
    });
    tearDown(() => store.close());

    void tick([Duration by = const Duration(minutes: 1)]) => now = now.add(by);

    test('an edit changes the text; the old one stays only in the log', () {
      final note = notes.capture('nees mor sauce');
      tick();
      notes.edit(note.id, '  need more sauce ');
      expect(notes.byId(note.id)?.text, 'need more sauce');
      expect(notes.all().single.text, 'need more sauce');
      expect(
        [
          for (final e in store.history(note.id))
            if (e.field == NoteKeys.text) StoredText.read(e.value).text,
        ],
        ['nees mor sauce', 'need more sauce'],
      );
      expect(notes.byId(note.id)?.captured, DateTime.utc(2026, 10, 24, 8));
    });

    test('the time the text was last set is known', () {
      final note = notes.capture('milk');
      expect(notes.textSetAt(note.id), DateTime.utc(2026, 10, 24, 8));
      tick();
      notes.setImportance(note.id, Importance.important);
      expect(notes.textSetAt(note.id), DateTime.utc(2026, 10, 24, 8));
      notes.edit(note.id, 'oat milk');
      expect(notes.textSetAt(note.id), DateTime.utc(2026, 10, 24, 8, 1));
      expect(notes.textSetAt('note:gone'), isNull);
    });

    test('an edit never empties a Note', () {
      final note = notes.capture('milk');
      expect(() => notes.edit(note.id, '  '), throwsArgumentError);
      expect(() => notes.edit('note:gone', 'x'), throwsArgumentError);
      expect(notes.byId(note.id)?.text, 'milk');
    });

    test('Amend adds dated Sections, oldest first, the text untouched', () {
      final note = notes.capture('nees mor sauce');
      tick(const Duration(days: 1));
      final first = notes.amend(note.id, ' Update: sauce bought ');
      tick();
      notes.amend(note.id, 'and pasta');
      final other = notes.capture('other');
      notes.amend(other.id, 'not mine');

      expect(first.id, startsWith('section:'));
      expect(first.noteId, note.id);
      expect(first.text, 'Update: sauce bought');
      expect(first.context.captured, DateTime.utc(2026, 10, 25, 8));
      final sections = notes.sections(note.id);
      expect(sections.map((s) => s.text), [
        'Update: sauce bought',
        'and pasta',
      ]);
      expect(sections.last.context.captured, DateTime.utc(2026, 10, 25, 8, 1));
      expect(notes.byId(note.id)?.text, 'nees mor sauce');
      expect(() => notes.amend(note.id, ''), throwsArgumentError);
      expect(() => notes.amend('note:gone', 'x'), throwsArgumentError);
    });

    test('Importance is normal until set', () {
      final note = notes.capture('pills');
      expect(note.importance, Importance.normal);
      tick();
      notes.setImportance(note.id, Importance.important);
      expect(notes.byId(note.id)?.importance, Importance.important);
      tick();
      notes.setImportance(note.id, Importance.unimportant);
      expect(notes.byId(note.id)?.importance, Importance.unimportant);
    });
  });

  group('trash', () {
    late Store store;
    late Notes notes;
    var now = DateTime.utc(2026, 10, 24, 8);

    setUp(() {
      now = DateTime.utc(2026, 10, 24, 8);
      store = Store.inMemory(clock: () => now);
      notes = Notes(store);
    });
    tearDown(() => store.close());

    test('a deleted Note leaves every list and Unsorted for the trash', () {
      final places = Places(store);
      final home = places.addArea('Household');
      final unsorted = notes.capture('nees mor sauce');
      final filed = notes.capture('sauce', place: home.id);
      now = now.add(const Duration(minutes: 1));
      notes.trash(unsorted.id);
      notes.trash(filed.id);

      final gone = notes.byId(unsorted.id);
      expect(gone?.isTrashed, isTrue);
      expect(gone?.isUnsorted, isFalse);
      expect(notes.all(), isEmpty);
      expect(notes.inside(home.id), isEmpty);
      expect(places.notesInside(home.id), 0);
      expect(Search(store).notes('sauce'), isEmpty);
      expect(notes.trashed().map((n) => n.id).toSet(), {unsorted.id, filed.id});
    });

    test('a restored Note comes back where it was', () {
      final home = Places(store).addArea('Household');
      final note = notes.capture('sauce', place: home.id);
      now = now.add(const Duration(days: 29));
      notes.trash(note.id);
      now = now.add(const Duration(days: 29));
      notes.restore(note.id);
      expect(notes.byId(note.id)?.isTrashed, isFalse);
      expect(notes.inside(home.id).map((n) => n.id), [note.id]);
      expect(notes.trashed(), isEmpty);
    });

    test('the trash lists the latest deletion first', () {
      final a = notes.capture('a');
      final b = notes.capture('b');
      now = now.add(const Duration(minutes: 1));
      notes.trash(b.id);
      now = now.add(const Duration(minutes: 1));
      notes.trash(a.id);
      expect(notes.trashed().map((n) => n.text), ['a', 'b']);
    });

    test('after 30 days in the trash a Note is purged with its Sections', () {
      final note = notes.capture('sauce');
      notes.amend(note.id, 'bought');
      final kept = notes.capture('kept');
      final trashedAt = now.add(const Duration(minutes: 1));
      now = trashedAt;
      notes.trash(note.id);
      expect(
        notes.purgeAt(notes.trashed().single),
        trashedAt.add(Notes.trashKeep),
      );

      now = trashedAt.add(
        const Duration(days: 30) - const Duration(seconds: 1),
      );
      notes.purgeTrash();
      expect(notes.trashed().map((n) => n.id), [note.id]);
      expect(notes.byId(note.id), isNotNull);

      now = trashedAt.add(const Duration(days: 30));
      expect(notes.trashed(), isEmpty);
      notes.purgeTrash();
      expect(notes.byId(note.id), isNull);
      expect(notes.sections(note.id), isEmpty);
      expect(notes.all().map((n) => n.id), [kept.id]);
      // The log still holds what the Note said.
      expect(store.history(note.id), isNotEmpty);
    });
  });

  group('merge', () {
    late Store store;
    late Places places;
    late Notes notes;
    var now = DateTime.utc(2026, 10, 24, 8);

    setUp(() {
      now = DateTime.utc(2026, 10, 24, 8);
      store = Store.inMemory(clock: () => now);
      places = Places(store);
      notes = Notes(store);
    });
    tearDown(() => store.close());

    void tick([Duration by = const Duration(hours: 1)]) => now = now.add(by);

    test('a merged Note becomes a Section with its own Capture Context', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final target = notes.capture('sauce', place: home.id);
      notes.setImportance(target.id, Importance.important);
      tick();
      final merged = notes.capture('nees mor sauce', place: kitchen.id);
      tick();
      notes.amend(merged.id, 'Update: sauce bought');
      tick();
      notes.merge(merged.id, target.id);

      final sections = notes.sections(target.id);
      expect(sections.map((s) => s.text), [
        'nees mor sauce',
        'Update: sauce bought',
      ]);
      expect(sections.map((s) => s.context.captured), [
        DateTime.utc(2026, 10, 24, 9),
        DateTime.utc(2026, 10, 24, 10),
      ]);
      expect(sections.first.id, merged.id);
      expect(sections.map((s) => s.noteId), everyElement(target.id));
      final kept = notes.byId(target.id);
      expect(kept?.text, 'sauce');
      expect(kept?.place, home.id);
      expect(kept?.importance, Importance.important);
      expect(notes.byId(merged.id)?.mergedInto, target.id);
    });

    test('a merged Note leaves every list and Unsorted', () {
      final home = places.addArea('Household');
      final target = notes.capture('sauce', place: home.id);
      final filed = notes.capture('more sauce', place: home.id);
      final unsorted = notes.capture('sauce bought');
      notes.merge(filed.id, target.id);
      notes.merge(unsorted.id, target.id);

      expect(notes.all().map((n) => n.id), [target.id]);
      expect(notes.inside(home.id).map((n) => n.id), [target.id]);
      expect(places.notesInside(home.id), 1);
      expect(Search(store).notes('sauce').map((n) => n.id), [target.id]);
      expect(notes.trashed(), isEmpty);
      expect(notes.byId(unsorted.id)?.isUnsorted, isFalse);
    });

    test('what reaches a merged Note later still shows in its target', () {
      final target = notes.capture('sauce');
      final merged = notes.capture('nees mor sauce');
      notes.merge(merged.id, target.id);
      // Another Device, not yet synced, fixed and amended the merged Note.
      tick();
      notes.edit(merged.id, 'need more sauce');
      notes.amend(merged.id, 'and pasta');
      expect(notes.sections(target.id).map((s) => s.text), [
        'need more sauce',
        'and pasta',
      ]);
    });

    test('a Note merged into a merged Note ends in the last target', () {
      final a = notes.capture('a');
      tick();
      final b = notes.capture('b');
      tick();
      final c = notes.capture('c');
      notes.merge(a.id, b.id);
      notes.merge(b.id, c.id);
      expect(notes.all().map((n) => n.id), [c.id]);
      expect(notes.byId(a.id)?.mergedInto, c.id);
      expect(notes.sections(c.id).map((s) => s.text), ['a', 'b']);
    });

    test('a Note cannot be merged into itself or a gone Note', () {
      final a = notes.capture('a');
      final b = notes.capture('b');
      final trashed = notes.capture('trashed');
      notes.trash(trashed.id);
      final merged = notes.capture('merged');
      notes.merge(merged.id, b.id);
      final before = store.allEntries().length;
      for (final (from, into) in [
        (a.id, a.id),
        (a.id, 'note:gone'),
        ('note:gone', a.id),
        (a.id, trashed.id),
        (trashed.id, a.id),
        (a.id, merged.id),
        (merged.id, a.id),
      ]) {
        expect(() => notes.merge(from, into), throwsArgumentError);
      }
      expect(store.allEntries(), hasLength(before));
    });

    test('Devices merging two Notes into each other keep the lower id', () {
      final a = notes.capture('a');
      final b = notes.capture('b');
      // Both merges arrive, as after a sync of two Devices.
      store.append(a.id, NoteKeys.merged, b.id);
      store.append(b.id, NoteKeys.merged, a.id);
      final kept = a.id.compareTo(b.id) < 0 ? a : b;
      final folded = kept == a ? b : a;
      expect(notes.all().map((n) => n.id), [kept.id]);
      expect(notes.sections(kept.id).map((s) => s.id), [folded.id]);
      expect(notes.sections(folded.id), isEmpty);
    });

    test('a merge into a Note that is gone keeps the Note', () {
      final a = notes.capture('a');
      store.append(a.id, NoteKeys.merged, 'note:never-synced');
      expect(notes.all().map((n) => n.id), [a.id]);
      expect(notes.byId(a.id)?.mergedInto, isNull);
    });

    test('merged Notes go to the trash, back and away with the target', () {
      final target = notes.capture('sauce');
      final merged = notes.capture('more sauce');
      notes.amend(merged.id, 'bought');
      notes.merge(merged.id, target.id);
      tick();
      notes.trash(target.id);
      expect(notes.trashed().map((n) => n.id), [target.id]);
      notes.restore(target.id);
      expect(notes.sections(target.id), hasLength(2));

      notes.trash(target.id);
      now = now.add(Notes.trashKeep);
      notes.purgeTrash();
      expect(notes.byId(target.id), isNull);
      expect(notes.byId(merged.id), isNull);
      expect(notes.sections(target.id), isEmpty);
      expect(notes.sections(merged.id), isEmpty);
      expect(store.entitiesOfType(SectionKeys.typeSection), isEmpty);
    });

    test('the Sections of many Notes read at once are those of each', () {
      final plain = notes.capture('plain');
      tick();
      final amended = notes.capture('amended');
      tick();
      notes.amend(amended.id, 'amend');
      // A chain a → b → c, with Amends on a and b, read for all three.
      final a = notes.capture('a');
      tick();
      notes.amend(a.id, 'a amend');
      tick();
      final b = notes.capture('b');
      tick();
      notes.amend(b.id, 'b amend');
      tick();
      final c = notes.capture('c');
      notes.merge(a.id, b.id);
      notes.merge(b.id, c.id);
      // Two Notes merged into each other, and a Merge into a Note gone.
      final x = notes.capture('x');
      final y = notes.capture('y');
      notes.amend(y.id, 'y amend');
      store.append(x.id, NoteKeys.merged, y.id);
      store.append(y.id, NoteKeys.merged, x.id);
      final lost = notes.capture('lost');
      store.append(lost.id, NoteKeys.merged, 'note:never-synced');
      tick();
      notes.amend(lost.id, 'lost amend');
      // A Section whose Note never arrived.
      store.append('section:orphan', SectionKeys.type, SectionKeys.typeSection);
      store.append('section:orphan', SectionKeys.note, 'note:never-synced');
      store.append('section:orphan', SectionKeys.text, 'orphan');
      store.append(
        'section:orphan',
        SectionKeys.captured,
        now.toIso8601String(),
      );

      String shown(Section s) => [
        s.id,
        s.noteId,
        s.text,
        s.context.captured.toIso8601String(),
      ].join('|');
      final ids = [
        for (final note in [plain, amended, a, b, c, x, y, lost]) note.id,
        'note:never-synced',
        'note:unknown',
      ];
      final read = notes.sectionsOf(ids);
      expect(read.keys, unorderedEquals(ids));
      for (final id in ids) {
        expect(read[id]?.map(shown), notes.sections(id).map(shown), reason: id);
        expect(notes.sectionsOf([id])[id]?.map(shown), read[id]?.map(shown));
      }
      expect(read[b.id]?.map((s) => s.text), ['b amend']);
      expect(read[c.id]?.map((s) => s.text), ['a', 'a amend', 'b', 'b amend']);
      expect(read[c.id]?.map((s) => s.noteId), everyElement(c.id));
      expect(read['note:never-synced']?.map((s) => s.text), ['orphan']);
      expect(read['note:unknown'], isEmpty);
      expect(notes.sectionsOf(const []), isEmpty);
    });

    test('a merge is in the log and survives a reopen', () {
      final dir = Directory.systemTemp.createTempSync('obf');
      addTearDown(() => dir.deleteSync(recursive: true));
      final path = '${dir.path}/log.db';
      final first = Store.open(path);
      final target = Notes(first).capture('sauce');
      final merged = Notes(first).capture('more sauce');
      Notes(first).merge(merged.id, target.id);
      expect(
        first.history(merged.id).last,
        isA<Entry>()
            .having((e) => e.field, 'field', NoteKeys.merged)
            .having((e) => e.value, 'value', target.id),
      );
      first.close();
      final second = Store.open(path);
      addTearDown(second.close);
      expect(Notes(second).all().map((n) => n.id), [target.id]);
      expect(Notes(second).sections(target.id).single.text, 'more sauce');
    });
  });

  group('diary', () {
    Note note(String text, int hour, Importance importance) => Note(
      id: 'note:$text',
      text: text,
      context: CaptureContext(captured: DateTime.utc(2026, 10, 24, hour)),
      importance: importance,
    );

    final notes = [
      note('normal old', 8, Importance.normal),
      note('important old', 9, Importance.important),
      note('unimportant', 10, Importance.unimportant),
      note('normal new', 11, Importance.normal),
      note('important new', 12, Importance.important),
    ];

    test('by time is newest first', () {
      expect(diary(notes).map((n) => n.text), [
        'important new',
        'normal new',
        'unimportant',
        'important old',
        'normal old',
      ]);
    });

    test('by Importance is important first, newest first within', () {
      expect(diary(notes, order: DiaryOrder.importance).map((n) => n.text), [
        'important new',
        'important old',
        'normal new',
        'normal old',
        'unimportant',
      ]);
    });

    test('unimportant Notes can be hidden', () {
      expect(
        diary(notes, hideUnimportant: true).map((n) => n.text),
        isNot(contains('unimportant')),
      );
      expect(diary(notes, hideUnimportant: true), hasLength(4));
    });
  });
}

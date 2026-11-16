import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show syncAll;
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

  List<String> ids(List<Note> list) => [for (final n in list) n.id];

  group('merging several Notes into one (#273)', () {
    test('writes one pointer each, in one batch heard once', () {
      final home = Places(ann).addArea('Home').id;
      final keys = notes.capture('Spare keys at the neighbours', place: home);
      final plants = notes.capture('Water the plants');
      final post = notes.capture('Empty the letter box');
      final into = notes.capture('Neighbours on holiday');
      var heard = 0;
      void hear() => heard++;
      ann.addListener(hear);
      addTearDown(() => ann.removeListener(hear));
      final mark = ann.currentSeq();
      final merged = notes.mergeAll([keys.id, plants.id, post.id], into.id);
      expect(merged, [keys.id, plants.id, post.id]);
      expect(heard, 1);
      expect(ann.entriesAfter(mark).map((e) => (e.entity, e.field, e.value)), [
        (keys.id, NoteKeys.merged, into.id),
        (plants.id, NoteKeys.merged, into.id),
        (post.id, NoteKeys.merged, into.id),
      ]);
      expect(ids(notes.all()), [into.id]);
      expect(notes.sections(into.id).map((s) => s.id), [
        keys.id,
        plants.id,
        post.id,
      ]);
    });

    test('merges a Note named twice once', () {
      final a = notes.capture('a');
      final into = notes.capture('into');
      final mark = ann.currentSeq();
      expect(notes.mergeAll([a.id, a.id], into.id), [a.id]);
      expect(ann.entriesAfter(mark), hasLength(1));
    });

    test('writes nothing when one Note or the target is refused', () {
      final a = notes.capture('a');
      final b = notes.capture('b');
      final into = notes.capture('into');
      final trashed = notes.capture('trashed');
      notes.trash(trashed.id);
      final merged = notes.capture('merged');
      notes.merge(merged.id, into.id);
      final mark = ann.currentSeq();
      for (final (from, target) in [
        ([a.id, trashed.id], into.id),
        ([a.id, merged.id], into.id),
        ([a.id, 'note:none'], into.id),
        ([a.id, b.id], trashed.id),
        ([a.id, b.id], merged.id),
        ([a.id, into.id], into.id),
      ]) {
        expect(
          () => notes.mergeAll(from, target),
          throwsArgumentError,
          reason: '$from into $target',
        );
      }
      expect(ann.entriesAfter(mark), isEmpty);
      expect(notes.mergeAll(const [], into.id), isEmpty);
      expect(ann.entriesAfter(mark), isEmpty);
    });

    test('undoing it brings each back, in one batch, past what is gone', () {
      final home = Places(ann).addArea('Home').id;
      final keys = notes.capture('Spare keys', place: home);
      final plants = notes.capture('Water the plants');
      final into = notes.capture('Neighbours on holiday');
      final merged = notes.mergeAll([keys.id, plants.id], into.id);
      // Another step got to one of them first.
      notes.undoMerge(plants.id);
      var heard = 0;
      void hear() => heard++;
      ann.addListener(hear);
      addTearDown(() => ann.removeListener(hear));
      final mark = ann.currentSeq();
      expect(notes.undoMergeAll([...merged, 'note:none']), [keys.id]);
      expect(heard, 1);
      expect(ann.entriesAfter(mark).map((e) => (e.entity, e.field, e.value)), [
        (keys.id, NoteKeys.merged, null),
      ]);
      expect(notes.byId(keys.id)?.place, home);
      expect(notes.sections(into.id), isEmpty);
      expect(ids(notes.unsorted()), unorderedEquals([plants.id, into.id]));
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
      area = ann.share(Places(ann).addArea('Office').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    test('a Member merges their own Notes and none of another\'s', () {
      final into = notes.capture('Toner', place: area);
      final anns = notes.capture('Order more', place: area);
      final bens = Notes(ben);
      final own = bens.capture('Ben\'s toner', place: area);
      final also = bens.capture('Ben\'s paper', place: area);
      syncAll(ben, ann);
      final mark = ben.currentSeq();
      expect(() => bens.mergeAll([own.id, anns.id], into.id), throwsStateError);
      expect(ben.entriesAfter(mark), isEmpty);
      expect(bens.mergeAll([own.id, also.id], into.id), [own.id, also.id]);
      syncAll(ben, ann);
      expect(ids(notes.inside(area)), unorderedEquals([into.id, anns.id]));
      expect(bens.undoMergeAll([own.id, also.id]), [own.id, also.id]);
    });
  });
}

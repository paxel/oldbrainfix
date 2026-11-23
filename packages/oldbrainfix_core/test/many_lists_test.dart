import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// A list of many Notes reads their lists, Sections and what may be done
/// to them in a few queries however many there are (obf-1069), with the
/// same results as reading each Note on its own.
void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;
  late String area;
  late String kitchen;
  late String cidsNote;
  DateTime wall(DateTime utc) => utc;

  String idOf(Store store) => store.ownProfile!.id;
  String keyOf(Store store) => base64.encode(store.signingKey.publicKey);
  String link(String key, String person) =>
      jsonEncode({'from': key, 'id': 'x', 'profile': person, 'version': 1});

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    ben = device(clock, 'Ben');
    area = ann.share(Places(ann).addArea('Household').id);
    kitchen = Places(ann).addTopic(area, 'Kitchen').id;
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    final cid = device(clock, 'Cid');
    cidsNote = acceptSentItem(
      ann,
      openSentItem(ann, sendNote(cid, Notes(cid).capture('Hi').id)).item,
    )!;
    // One Checklist on a Routine, the same for any number of Notes.
    final daily = Notes(ann).capture('Daily\nStretch\nWalk').id;
    Checklists(ann).make(daily);
    Routines(ann).spawn(
      daily,
      Schedule(
        start: Day(2026, 3, 1),
        times: [ClockTime(8, 0)],
        rule: Recurrence(frequency: Frequency.daily),
      ),
    );
  });

  /// Adds [rounds] times one Note of every kind the lists tell apart, on
  /// Ann's Device and on Ben's in the Shared Area, then syncs them.
  void seed(int rounds) {
    final notes = Notes(ann);
    final lists = Checklists(ann);
    final shopping = ShoppingLists(ann);
    for (var i = 0; i < rounds; i++) {
      final check = notes.capture('Pack $i\nSocks\nShirt', place: area).id;
      lists.make(check);
      final shirt = lists.byNote(check)!.lines.last.id;
      lists.delete(shirt);
      lists.check(lists.byNote(check)!.lines.first.id);
      final shop = notes.capture('Shop $i\nMilk\nBread', place: kitchen).id;
      shopping.make(shop);
      shopping.check(shopping.byNote(shop, wall)!.lines.first.id, wall);
      final plain = notes.capture('Plain $i').id;
      final amend = notes.amend(plain, 'Amend $i');
      ann.append(
        amend.id,
        SectionKeys.text,
        StoredText.of(TextFormats.plain, 'Amend $i, fixed').encode(),
      );
      final merged = notes.capture('Merged $i', place: area).id;
      notes.amend(merged, 'Amend of merged $i');
      notes.merge(merged, plain);
      notes.trash(notes.capture('Trashed $i').id);
      // A list made back into a Note keeps its Lines out of sight.
      final undone = notes.capture('Undone $i\nA\nB').id;
      lists.make(undone);
      ann.append(undone, ChecklistKeys.kind, null);
      // Ben's own Notes in the Area, one of them marked as Ann's by a
      // `received` link his Device wrote, and a change to Ann's Note no
      // role of his allows.
      final bens = Notes(ben).capture('Ben $i\nTea', place: area).id;
      Checklists(ben).make(bens);
      final forged = Notes(ben).capture('Forged $i', place: area).id;
      ben.append(forged, 'received', link(keyOf(ann), idOf(ann)));
      Notes(ben).amend(forged, 'Ben amends $i');
      exchange(ben, ann);
      ben.append(check, NoteKeys.text, 'Ben was here $i');
      exchange(ben, ann);
    }
  }

  List<String> noteIds(Store store) => [
    for (final id in store.entitiesOfType(NoteKeys.typeNote)) id,
    'note:none',
  ];

  int readsOf(Store store, void Function() read) {
    final before = store.historyReads + store.loggedReads;
    read();
    return store.historyReads + store.loggedReads - before;
  }

  String lineShown(Line l) => '${l.id} ${l.text} ${l.checked} ${l.parent}';
  String shoppingShown(ShoppingLine l) => '${l.id} ${l.text} ${l.struck}';
  String sectionShown(Section s) => [
    s.id,
    s.noteId,
    s.text,
    s.context.captured,
    s.modified,
    s.editedBy?.id,
    s.boxes.length,
  ].join(' | ');

  group('reads as one by one', () {
    setUp(() => seed(2));

    test('the histories of many entities', () {
      final ids = [
        ...noteIds(ann),
        ...ann.entitiesOfType(SectionKeys.typeSection),
      ];
      final histories = ann.historiesOf(ids);
      final logged = ann.loggedHistoriesOf(ids);
      for (final id in ids) {
        expect(
          [for (final e in histories[id] ?? const <Entry>[]) e.id],
          [for (final e in ann.history(id)) e.id],
        );
        expect(
          [for (final e in logged[id] ?? const <Entry>[]) e.id],
          [for (final e in ann.loggedHistory(id)) e.id],
        );
      }
      // Ben's change to Ann's Checklist is left out of both.
      expect(ann.judged.ignored, isNotEmpty);
    });

    test('the Checklists of many Notes', () {
      final lists = Checklists(ann);
      final ids = noteIds(ann);
      final many = lists.ofNotes(ids);
      expect(many.keys, [
        for (final id in ids)
          if (lists.isChecklist(id)) id,
      ]);
      for (final id in ids) {
        final one = lists.byNote(id);
        expect(many[id]?.making, one?.making);
        expect(many[id]?.lines.map(lineShown), one?.lines.map(lineShown));
        // The Lines that count, read one by one from the log.
        if (one != null) {
          expect(many[id]!.lines.map((l) => l.id).toSet(), {
            for (final line in ann.entitiesOfType(ChecklistKeys.typeLine))
              if (ann.currentFields(line) case final f
                  when f[ChecklistKeys.note] == id &&
                      f[ChecklistKeys.made] == one.making &&
                      f[ChecklistKeys.trashed] == null)
                line,
          });
        }
      }
      expect(many.length, greaterThan(4));
      final now = lists.ofNotesNow(ids, wall);
      for (final id in ids) {
        final one = lists.shownNow(id, wall);
        expect(now[id]?.on?.routineId, one?.on?.routineId);
        expect(now[id]?.lines.map(lineShown), one?.lines.map(lineShown));
      }
      expect(now.values.where((l) => l.on != null), hasLength(1));
    });

    test('the Shopping Lists of many Notes', () {
      final shopping = ShoppingLists(ann);
      final ids = noteIds(ann);
      final many = shopping.ofNotes(ids, wall);
      expect(many.keys, [
        for (final id in ids)
          if (shopping.isShoppingList(id)) id,
      ]);
      for (final id in ids) {
        final one = shopping.byNote(id, wall);
        expect(
          many[id]?.lines.map(shoppingShown),
          one?.lines.map(shoppingShown),
        );
      }
      // The checked Lines, one of each list.
      expect(many.values.expand((l) => [...l.struck, ...l.gone]), hasLength(2));
    });

    test('the Sections of many Notes', () {
      final notes = Notes(ann);
      final ids = noteIds(ann);
      final many = notes.sectionsOf(ids);
      expect(many.keys, ids);
      for (final id in ids) {
        expect(
          many[id]!.map(sectionShown),
          notes.sections(id).map(sectionShown),
        );
      }
      final shown = many.values.expand((s) => s).toList();
      expect(shown.where((s) => s.modified != null), isNotEmpty);
      expect(shown.where((s) => s.id.startsWith('note:')), hasLength(2));
    });

    /// [Store.accessOf] on [store] as each Note's own reads tell it.
    void accessAsOneByOne(Store store) {
      final ids = noteIds(store);
      final many = store.accessOf(ids);
      expect(many.keys, ids);
      for (final id in ids) {
        final area = store.areaOf(id);
        expect(many[id]!.area, area, reason: id);
        expect(many[id]!.changes, store.mayChange(id), reason: id);
        expect(many[id]!.schedules, store.mayManage(area), reason: id);
        expect(many[id]!.amends, store.mayWriteIn(id), reason: id);
        expect(many[id]!.author?.id, store.authorOf(id)?.id, reason: id);
        expect(many[id]!.author?.name, store.authorOf(id)?.name);
      }
    }

    test('what may be done to many Notes, and whose they are', () {
      for (final store in [ann, ben]) {
        accessAsOneByOne(store);
      }
      final notes = {for (final n in Notes(ann).all()) n.text: n.id};
      final onAnn = ann.accessOf(notes.values);
      final onBen = ben.accessOf(notes.values);
      // A Member changes only their own Notes and adds to every one.
      expect(onBen[notes['Ben 0']]!.changes, isTrue);
      expect(onBen[notes['Pack 0']]!.changes, isFalse);
      expect(onBen[notes['Pack 0']]!.amends, isTrue);
      expect(onBen[notes['Pack 0']]!.schedules, isFalse);
      expect(onBen[notes['Pack 0']]!.author?.id, idOf(ann));
      expect(onAnn[notes['Pack 0']]!.area, area);
      expect(onAnn[notes['Shop 0']]!.area, area);
      expect(onAnn[notes['Plain 0']]!.area, isNull);
      // A `received` link counts only when one's own Device wrote it.
      expect(onAnn[notes['Forged 0']]!.author?.id, idOf(ben));
      expect(onAnn[cidsNote]!.author?.name, 'Cid');
      expect(onAnn[notes['Plain 0']]!.author, isNull);
    });

    test('a search of one or two letters', () {
      for (final query in ['so', 'br', 't', 'a']) {
        expect(
          Search(ann).notes(query).map((n) => n.id),
          Search.scanning(ann).notes(query).map((n) => n.id),
          reason: query,
        );
      }
      final hits = Search(ann).notes('so').map((n) => n.text);
      // The Line "Socks", but not "Shirt" deleted nor those of a Note
      // that is no list any more.
      expect(hits, contains(startsWith('Pack 0')));
      expect(Search(ann).notes('shirt'), isEmpty);
      expect(Search(ann).notes('br').map((n) => n.text), contains('Shop 0'));
    });
  });

  group('with a pending change shown', () {
    late Map<String, String> notes;

    setUp(() {
      seed(2);
      notes = {for (final n in Notes(ann).all()) n.text: n.id};
      // Shown, but trusted for nothing (ADR 0022): it moves no Note into
      // an Area and marks none as somebody else's.
      final pending = ann.openPending('ai-task', shown: true);
      ann.draftPending(pending, notes['Plain 0']!, NoteKeys.place, area);
      ann.draftPending(
        pending,
        notes['Pack 0']!,
        'received',
        link(keyOf(ben), idOf(ben)),
      );
      ann.draftPending(pending, notes['Shop 1']!, NoteKeys.text, 'Shop now');
    });

    test('reads the histories as one by one', () {
      expect(Notes(ann).byId(notes['Plain 0']!)!.place, area);
      final ids = [
        ...noteIds(ann),
        ...ann.entitiesOfType(SectionKeys.typeSection),
      ];
      final histories = ann.historiesOf(ids);
      final logged = ann.loggedHistoriesOf(ids);
      for (final id in ids) {
        expect(
          [for (final e in histories[id] ?? const <Entry>[]) e.id],
          [for (final e in ann.history(id)) e.id],
        );
        expect(
          [for (final e in logged[id] ?? const <Entry>[]) e.id],
          [for (final e in ann.loggedHistory(id)) e.id],
        );
      }
      expect(
        ann.currentFieldsOf(ids)[notes['Shop 1']]?[NoteKeys.text],
        'Shop now',
      );
    });

    test('tells what may be done to them from the log alone', () {
      final onAnn = ann.accessOf(notes.values);
      expect(onAnn[notes['Plain 0']]!.area, isNull);
      expect(onAnn[notes['Pack 0']]!.author, isNull);
      // A Member who left the Area may write nothing in it any more.
      ben.leaveArea(area);
      for (final store in [ann, ben]) {
        final ids = noteIds(store);
        final many = store.accessOf(ids);
        for (final id in ids) {
          final area = store.areaOf(id);
          expect(many[id]!.area, area, reason: id);
          expect(many[id]!.changes, store.mayChange(id), reason: id);
          expect(many[id]!.schedules, store.mayManage(area), reason: id);
          expect(many[id]!.amends, store.mayWriteIn(id), reason: id);
          expect(many[id]!.author?.id, store.authorOf(id)?.id, reason: id);
        }
      }
      final onBen = ben.accessOf(notes.values);
      expect(onBen[notes['Pack 0']]!.amends, isFalse);
      expect(onBen[notes['Ben 0']]!.changes, isFalse);
    });
  });

  test('reads no more for many Notes than for a few', () {
    // The one Routine stays one: the Routines are read one by one yet.
    seed(1);
    Map<String, int> reads() {
      final ids = noteIds(ann);
      final bens = noteIds(ben);
      return {
        'checklists': readsOf(ann, () => Checklists(ann).ofNotes(ids)),
        'now': readsOf(ann, () => Checklists(ann).ofNotesNow(ids, wall)),
        'shopping': readsOf(ann, () => ShoppingLists(ann).ofNotes(ids, wall)),
        'sections': readsOf(ann, () => Notes(ann).sectionsOf(ids)),
        'access': readsOf(ann, () => ann.accessOf(ids)),
        'on ben': readsOf(ben, () => ben.accessOf(bens)),
      };
    }

    final few = reads();
    seed(4);
    expect(reads(), few);
  });
}

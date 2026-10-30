import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'conflicts_test.dart' as own show device, exchange;
import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange;

/// A clock that moves one second per read from where it is set.
class _Clock {
  DateTime now;

  _Clock(this.now);

  DateTime call() => now = now.add(const Duration(seconds: 1));

  /// The next read is [at].
  void set(DateTime at) => now = at.subtract(const Duration(seconds: 1));
}

/// The wall clock an hour ahead of UTC, as in Central Europe's winter.
DateTime _wall(DateTime utc) => utc.add(const Duration(hours: 1));

void main() {
  late _Clock clock;
  late Store ann;

  setUp(() {
    // A Thursday.
    clock = _Clock(DateTime.utc(2026, 3, 5, 10));
    ann = device(clock.call, 'Ann');
  });

  /// A Shopping List made from a new Note with [text]; returns the Note.
  String shopping(Store store, String text, {String? place}) {
    final id = Notes(store).capture(text, place: place).id;
    ShoppingLists(store).make(id);
    return id;
  }

  /// The Lines of the Shopping List of [note] as they show: `~text` for
  /// a struck one.
  List<String> shown(Store store, String note) => [
    for (final l
        in ShoppingLists(store).byNote(note, _wall)?.lines ??
            const <ShoppingLine>[])
      '${l.struck ? '~' : ''}${l.text}',
  ];

  ShoppingLine lineOf(Store store, String note, String text) =>
      ShoppingLists(store)
          .byNote(note, _wall)!
          .lines
          .firstWhere((l) => l.text == text);

  String idOf(Store store, String note, String text) =>
      lineOf(store, note, text).id;

  group('making one', () {
    test('the first line stays, the others become Lines in their order, '
        'indented or not, a marked one struck', () {
      final note = Notes(ann)
          .capture('Groceries\r\nMilk\n  Eggs\n\tTea\n\n✓ Bread\r   \nApples');
      final lists = ShoppingLists(ann);
      expect(lists.mayMake(note.id), isTrue);
      lists.make(note.id);
      expect(Notes(ann).byId(note.id)!.text, 'Groceries');
      expect(Notes(ann).byId(note.id)!.isUnsorted, isFalse);
      expect(shown(ann, note.id), ['Milk', 'Eggs', 'Tea', 'Apples', '~Bread']);
      final list = lists.byNote(note.id, _wall)!;
      expect(list.count, 5);
      expect(list.openCount, 4);
      expect(list.struckCount, 1);
      expect(list.lines.map((l) => l.reappear).toSet(), {Reappear.week1});
      for (final l in list.lines) {
        expect(ann.current(l.id, ShoppingKeys.reappear), '1w');
        expect(ann.current(l.id, ChecklistKeys.indent), isNull);
      }
      expect(lists.isShoppingList(note.id), isTrue);
      expect(Checklists(ann).byNote(note.id), isNull);
      expect(lists.mayMake(note.id), isFalse, reason: 'one already');
      expect(() => lists.make(note.id), throwsStateError);
    });

    test('a Note is at most one of a Tally, a Checklist and a Shopping '
        'List', () {
      final lists = ShoppingLists(ann);
      final gym = Notes(ann).capture('Gym\nPush-ups').id;
      Tallies(ann).addField(gym, name: 'Push-ups');
      expect(lists.mayMake(gym), isFalse);
      expect(() => lists.make(gym), throwsStateError);
      final packing = Notes(ann).capture('Packing\nSocks').id;
      Checklists(ann).make(packing);
      expect(lists.mayMake(packing), isFalse);
      expect(lists.byNote(packing, _wall), isNull);
      final shop = shopping(ann, 'Shop\nMilk');
      expect(Checklists(ann).mayMake(shop), isFalse);
      expect(() => Checklists(ann).make(shop), throwsStateError);
      expect(Checklists(ann).byNote(shop), isNull);
      expect(() => Checklists(ann).add(shop, 'Tea'), throwsArgumentError);
      expect(() => Tallies(ann).addField(shop, name: 'Milk'), throwsStateError);
      // Made one on two Devices apart: the Tally wins.
      final both = shopping(ann, 'Both\nMilk');
      ann.append(both, TallyKeys.field('f1', TallyKeys.name), 'Milk');
      ann.append(both, TallyKeys.field('f1', TallyKeys.unit), 'l');
      expect(Tallies(ann).isTally(both), isTrue);
      expect(lists.byNote(both, _wall), isNull);
      expect(lists.isShoppingList(both), isFalse);
    });

    test('kinds and reappear times no app writes are read without fail', () {
      final note = Notes(ann).capture('Odd\nOne').id;
      for (final kind in ['shopping', 'shopping:', 'shopping:a:b']) {
        ann.append(note, ChecklistKeys.kind, kind);
        expect(ShoppingLists(ann).byNote(note, _wall), isNull, reason: kind);
      }
      ann.append(note, ChecklistKeys.kind, null);
      ShoppingLists(ann).make(note);
      final one = idOf(ann, note, 'One');
      for (final code in ['0', '5', 'x', '', '1W', null]) {
        ann.append(one, ShoppingKeys.reappear, code);
        expect(lineOf(ann, note, 'One').reappear, Reappear.week1, reason: code);
      }
      expect(Reappear.read('once'), Reappear.once);
      expect(Reappear.read('4w'), Reappear.week4);
    });
  });

  group('checking', () {
    test('a checked Line goes to the end struck with its day back, and is '
        'open at the top from the start of that day, nothing written', () {
      final note = shopping(ann, 'Shop\nMilk\nEggs\nTea');
      final lists = ShoppingLists(ann);
      lists.check(idOf(ann, note, 'Milk'), _wall);
      expect(shown(ann, note), ['Eggs', 'Tea', '~Milk']);
      final milk = lineOf(ann, note, 'Milk');
      expect(milk.struck, isTrue);
      expect(milk.backOn, Day(2026, 3, 12), reason: 'Thursday a week on');
      expect(milk.checkedAt, isNotNull);
      final before = ann.allEntries().length;
      // 23:59:59 on the wall on Wednesday the 11th.
      clock.set(DateTime.utc(2026, 3, 11, 22, 59, 59));
      expect(shown(ann, note), ['Eggs', 'Tea', '~Milk']);
      // Midnight on the wall: Thursday the 12th begins.
      clock.set(DateTime.utc(2026, 3, 11, 23));
      expect(shown(ann, note), ['Milk', 'Eggs', 'Tea']);
      expect(lineOf(ann, note, 'Milk').backOn, isNull);
      expect(ann.allEntries().length, before, reason: 'reading writes none');
    });

    test('the day back follows the wall clock, not UTC', () {
      final note = shopping(ann, 'Shop\nMilk');
      // 22:30 UTC on Wednesday is 00:30 on Thursday two hours ahead.
      DateTime twoAhead(DateTime utc) => utc.add(const Duration(hours: 2));
      clock.set(DateTime.utc(2026, 3, 4, 22, 30));
      final lists = ShoppingLists(ann);
      lists.check(idOf(ann, note, 'Milk'), twoAhead);
      final milk = lists.byNote(note, twoAhead)!.lines.single;
      expect(milk.backOn, Day(2026, 3, 12));
      // An hour ahead, it was Wednesday still.
      expect(lists.byNote(note, _wall)!.lines.single.backOn, Day(2026, 3, 11));
      clock.set(DateTime.utc(2026, 3, 11, 21, 59, 59));
      expect(lists.byNote(note, twoAhead)!.lines.single.struck, isTrue);
      clock.set(DateTime.utc(2026, 3, 11, 22));
      expect(lists.byNote(note, twoAhead)!.lines.single.struck, isFalse);
    });

    test('each reappear time, and Once is gone', () {
      final note = shopping(ann, 'Shop\nOnce\nOne\nTwo\nThree\nFour');
      final lists = ShoppingLists(ann);
      lists.setReappear(idOf(ann, note, 'Once'), Reappear.once);
      lists.setReappear(idOf(ann, note, 'Two'), Reappear.week2);
      lists.setReappear(idOf(ann, note, 'Three'), Reappear.week3);
      lists.setReappear(idOf(ann, note, 'Four'), Reappear.week4);
      for (final text in ['Once', 'One', 'Two', 'Three', 'Four']) {
        lists.check(idOf(ann, note, text), _wall);
      }
      expect(shown(ann, note), ['~One', '~Two', '~Three', '~Four']);
      expect(
        [for (final l in lists.byNote(note, _wall)!.lines) l.backOn],
        [Day(2026, 3, 12), Day(2026, 3, 19), Day(2026, 3, 26), Day(2026, 4, 2)],
      );
      final once = ann
          .entitiesOfType(ChecklistKeys.typeLine)
          .firstWhere(
            (id) =>
                StoredText.read(ann.current(id, ChecklistKeys.text)).text ==
                'Once',
          );
      expect(ann.current(once, ChecklistKeys.trashed), isNull);
      expect(() => lists.check(once, _wall), throwsArgumentError);
      clock.set(DateTime.utc(2026, 3, 25, 23));
      expect(shown(ann, note), ['Three', 'Two', 'One', '~Four']);
    });

    test('the open Line that came last is at the top, the struck one '
        'checked last at the end; unchecking brings one back now', () {
      final note = shopping(ann, 'Shop\nMilk\nEggs\nTea\nRice');
      final lists = ShoppingLists(ann);
      lists.check(idOf(ann, note, 'Eggs'), _wall);
      lists.check(idOf(ann, note, 'Milk'), _wall);
      lists.check(idOf(ann, note, 'Rice'), _wall);
      expect(shown(ann, note), ['Tea', '~Eggs', '~Milk', '~Rice']);
      lists.uncheck(idOf(ann, note, 'Milk'), _wall);
      expect(shown(ann, note), ['Milk', 'Tea', '~Eggs', '~Rice']);
      // Checking again, or unchecking an open one, writes nothing.
      final before = ann.allEntries().length;
      lists.check(idOf(ann, note, 'Eggs'), _wall);
      lists.uncheck(idOf(ann, note, 'Tea'), _wall);
      expect(ann.allEntries().length, before);
      // Eggs and Rice come back on the same day: as checked.
      clock.set(DateTime.utc(2026, 3, 11, 23));
      expect(shown(ann, note), ['Eggs', 'Rice', 'Milk', 'Tea']);
    });

    test('two Devices show the same order after a Line came back', () {
      final phone = own.device(clock.call);
      final desk = own.device(clock.call);
      addTearDown(phone.close);
      addTearDown(desk.close);
      final note = shopping(phone, 'Shop\nMilk\nEggs\nTea');
      ShoppingLists(phone).check(idOf(phone, note, 'Tea'), _wall);
      own.exchange(phone, desk);
      ShoppingLists(desk).check(idOf(desk, note, 'Milk'), _wall);
      ShoppingLists(desk).add(note, 'Bread', _wall);
      own.exchange(phone, desk);
      clock.set(DateTime.utc(2026, 3, 11, 23, 30));
      expect(shown(phone, note), ['Milk', 'Tea', 'Bread', 'Eggs']);
      expect(shown(desk, note), shown(phone, note));
    });
  });

  group('adding', () {
    test('a new name comes in at the top with a week', () {
      final note = shopping(ann, 'Shop\nMilk');
      final lists = ShoppingLists(ann);
      final tea = lists.add(note, '  Tea\n', _wall);
      expect(shown(ann, note), ['Tea', 'Milk']);
      expect(lineOf(ann, note, 'Tea').id, tea);
      expect(lineOf(ann, note, 'Tea').reappear, Reappear.week1);
      expect(() => lists.add(note, '  ', _wall), throwsArgumentError);
    });

    test('a name it has adds nothing: a struck or gone one comes back at '
        'the top, an open one stays', () {
      final note = shopping(ann, 'Shop\nMilk\nEggs\nÄpfel\nTea');
      final lists = ShoppingLists(ann);
      final milk = idOf(ann, note, 'Milk');
      final tea = idOf(ann, note, 'Tea');
      lists.setReappear(tea, Reappear.once);
      lists.check(milk, _wall);
      lists.check(tea, _wall);
      expect(shown(ann, note), ['Eggs', 'Äpfel', '~Milk']);
      var before = ann.allEntries().length;
      expect(lists.add(note, ' eggs ', _wall), idOf(ann, note, 'Eggs'));
      expect(lists.add(note, 'äPFEL', _wall), idOf(ann, note, 'Äpfel'));
      expect(ann.allEntries().length, before, reason: 'open ones stay');
      expect(lists.add(note, 'MILK', _wall), milk);
      expect(shown(ann, note), ['Milk', 'Eggs', 'Äpfel']);
      expect(lists.add(note, 'tea', _wall), tea);
      expect(shown(ann, note), ['Tea', 'Milk', 'Eggs', 'Äpfel']);
      expect(lineOf(ann, note, 'Tea').reappear, Reappear.once);
      before = ann.allEntries().length;
      expect(lists.add(note, 'Milk', _wall), milk);
      expect(ann.allEntries().length, before);
      expect(ann.entitiesOfType(ChecklistKeys.typeLine), hasLength(4));
    });

    test('a deleted Line is not brought back by its name', () {
      final note = shopping(ann, 'Shop\nMilk');
      final lists = ShoppingLists(ann);
      final milk = idOf(ann, note, 'Milk');
      lists.delete(milk);
      expect(lists.add(note, 'milk', _wall), isNot(milk));
      expect(shown(ann, note), ['milk']);
    });
  });

  group('editing', () {
    test('Lines are renamed, deleted and restored, and their reappear time '
        'set', () {
      final note = shopping(ann, 'Shop\nMilk\nEggs');
      final lists = ShoppingLists(ann);
      final milk = idOf(ann, note, 'Milk');
      lists.rename(milk, 'Oat milk');
      expect(shown(ann, note), ['Oat milk', 'Eggs']);
      final before = ann.allEntries().length;
      lists.rename(milk, ' Oat milk ');
      lists.setReappear(milk, Reappear.week1);
      expect(ann.allEntries().length, before);
      lists.setReappear(milk, Reappear.week3);
      expect(lineOf(ann, note, 'Oat milk').reappear, Reappear.week3);
      lists.delete(milk);
      expect(shown(ann, note), ['Eggs']);
      expect(() => lists.rename(milk, 'x'), throwsArgumentError);
      lists.restore(milk);
      expect(shown(ann, note), ['Oat milk', 'Eggs']);
      expect(() => lists.restore(milk), throwsArgumentError);
    });

    test('back to a plain Note, struck ones marked, and made again afresh', () {
      final note = shopping(ann, 'Shop\nMilk\nEggs\nTea');
      final lists = ShoppingLists(ann);
      lists.setReappear(idOf(ann, note, 'Tea'), Reappear.once);
      lists.check(idOf(ann, note, 'Tea'), _wall);
      lists.check(idOf(ann, note, 'Milk'), _wall);
      lists.toPlain(note, _wall);
      expect(lists.isShoppingList(note), isFalse);
      expect(Notes(ann).byId(note)!.text, 'Shop\nEggs\n✓ Milk');
      lists.make(note);
      expect(shown(ann, note), ['Eggs', '~Milk']);
      expect(lineOf(ann, note, 'Milk').reappear, Reappear.week1);
    });

    test('search finds a Note by its Lines, not by those gone', () {
      final note = shopping(ann, 'Shop\nOat milk\nEggs');
      expect(Search(ann).notes('MILK').map((n) => n.id), [note]);
      final lists = ShoppingLists(ann);
      lists.check(idOf(ann, note, 'Oat milk'), _wall);
      expect(Search(ann).notes('milk').map((n) => n.id), [note]);
      lists.setReappear(idOf(ann, note, 'Eggs'), Reappear.once);
      lists.check(idOf(ann, note, 'Eggs'), _wall);
      expect(Search(ann).notes('eggs'), isEmpty);
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = device(clock.call, 'Ben');
      area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });

    test('every Member checks, brings back and adds Lines; the rest stays '
        'with the author and the Managers', () {
      final note = shopping(ann, 'Shop\nMilk\nEggs', place: area);
      exchange(ben, ann);
      final lists = ShoppingLists(ben);
      final milk = idOf(ben, note, 'Milk');
      final eggs = idOf(ben, note, 'Eggs');
      expect(lists.mayCheck(note), isTrue);
      lists.check(milk, _wall);
      lists.check(eggs, _wall);
      lists.add(note, 'milk', _wall);
      final bread = lists.add(note, 'Bread', _wall);
      exchange(ben, ann);
      expect(shown(ann, note), ['Bread', 'Milk', '~Eggs']);
      expect(lists.mayEdit(bread), isTrue);
      expect(lists.mayEdit(milk), isFalse);
      expect(() => lists.rename(milk, 'Beer'), throwsStateError);
      expect(() => lists.delete(milk), throwsStateError);
      expect(() => lists.setReappear(milk, Reappear.once), throwsStateError);
      expect(() => lists.toPlain(note, _wall), throwsStateError);
      // As a modified app would.
      final forged = [
        ben.append(milk, ShoppingKeys.reappear, Reappear.once.code),
        ben.append(eggs, ChecklistKeys.trashed, '2026-03-05T11:00:00.000Z'),
      ];
      exchange(ben, ann);
      final held = ann.allEntries().map((e) => e.id).toSet();
      expect(held.containsAll([for (final e in forged) e.id]), isTrue);
      expect(lineOf(ann, note, 'Milk').reappear, Reappear.week1);
      expect(shown(ann, note), ['Bread', 'Milk', '~Eggs']);
      ann.setRole(
        area,
        ann.grantsOf(area)!.personOf(ben.deviceId)!,
        AreaRole.manager,
      );
      exchange(ben, ann);
      lists.setReappear(milk, Reappear.week2);
      exchange(ben, ann);
      expect(lineOf(ann, note, 'Milk').reappear, Reappear.week2);
    });
  });

  group('apart on two Devices', () {
    late Store phone;
    late Store desk;

    setUp(() {
      phone = own.device(clock.call);
      desk = own.device(clock.call);
      addTearDown(phone.close);
      addTearDown(desk.close);
    });

    test('two renames of one Line are a Conflict; a check against a '
        'bringing back: the later one wins', () {
      final note = shopping(phone, 'Shop\nMilk\nEggs');
      own.exchange(phone, desk);
      final milk = idOf(phone, note, 'Milk');
      final eggs = idOf(phone, note, 'Eggs');
      ShoppingLists(phone).rename(milk, 'Oat milk');
      ShoppingLists(desk).rename(milk, 'Soy milk');
      ShoppingLists(desk).check(eggs, _wall);
      ShoppingLists(phone).check(eggs, _wall);
      ShoppingLists(phone).uncheck(eggs, _wall);
      own.exchange(phone, desk);
      final conflict = desk.conflicts().single;
      expect(conflict.entity, milk);
      expect(conflict.type, ChecklistKeys.typeLine);
      expect(desk.mayResolve(milk, conflict.type), isTrue);
      expect(shown(desk, note).first, 'Eggs', reason: 'back at the top');
      expect(shown(phone, note), shown(desk, note));
    });
  });

  test('a Routine spawned from a Shopping List is Done only when checked '
      'off', () {
    final note = shopping(ann, 'Shop\nMilk');
    final routine = Routines(ann)
        .spawn(note, Schedule(start: Day(2026, 3, 5)))
        .id;
    final day = Occurrence(Day(2026, 3, 5));
    ShoppingLists(ann).check(idOf(ann, note, 'Milk'), _wall);
    // As a modified app would.
    ann.append(
      routine,
      ChecklistKeys.checkFor(day, idOf(ann, note, 'Milk')),
      '2026-03-05T10:30:00.000Z',
    );
    expect(Routines(ann).byId(routine)!.doneAt(day), isNull);
    expect(
      Checklists(ann).forOccurrence((routineId: routine, occurrence: day)),
      isNull,
    );
    Routines(ann).markDone(routine, day);
    expect(Routines(ann).byId(routine)!.doneAt(day), isNotNull);
  });

  test('a Shopping List made of a received Checklist stays one when the '
      'sender\'s changes', () {
    final ben = device(clock.call, 'Ben');
    final note = Notes(ann).capture('Shop\nMilk').id;
    Checklists(ann).make(note);
    final got = acceptSentItem(
      ben,
      openSentItem(ben, sendNote(ann, note)).item,
    )!;
    Checklists(ben).toPlain(got);
    ShoppingLists(ben).make(got);
    Checklists(ann).add(note, 'Tea');
    final opened = openSentItem(ben, sendNote(ann, note));
    expect(opened.changes!.makesChecklist, isFalse);
    expect(opened.changes!.linesAdded, isEmpty);
    acceptSentItem(ben, opened.item);
    expect(shown(ben, got), ['Milk']);
    expect(Checklists(ben).byNote(got), isNull);
  });
}

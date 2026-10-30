import 'dart:convert';
import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'backup_test.dart' show cheap, passphrase;
import 'conflicts_test.dart' as own show device, exchange;
import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  /// A Checklist made from a new Note with [text]; returns the Note.
  String checklist(Store store, String text, {String? place}) {
    final id = Notes(store).capture(text, place: place).id;
    Checklists(store).make(id);
    return id;
  }

  /// The Lines of the Checklist of [note] as `text`, `  text` for one of
  /// a group, `[x]` for one shown checked.
  List<String> shown(Store store, String note) => [
    for (final l in Checklists(store).byNote(note)?.lines ?? const <Line>[])
      '${l.inGroup ? '  ' : ''}${l.checked ? '[x] ' : ''}${l.text}',
  ];

  String idOf(Store store, String note, String text) =>
      Checklists(store)
          .byNote(note)!
          .lines
          .firstWhere((l) => l.text == text)
          .id;

  group('making one', () {
    test('the first line stays, the others become Lines; an indented one '
        'goes under the line above', () {
      final note = Notes(ann).capture(
        'Packing\r\nSocks\n  Blue\n\tRed\n\n✓ Passport\r   \nTent\n  ✓ Pegs',
      );
      expect(note.isUnsorted, isTrue);
      final lists = Checklists(ann);
      expect(lists.mayMake(note.id), isTrue);
      lists.make(note.id);
      expect(Notes(ann).byId(note.id)!.text, 'Packing');
      expect(Notes(ann).byId(note.id)!.isUnsorted, isFalse);
      expect(shown(ann, note.id), [
        'Socks',
        '  Blue',
        '  Red',
        '[x] Passport',
        '[x] Tent',
        '  [x] Pegs',
      ]);
      final list = lists.byNote(note.id)!;
      expect(list.count, 6);
      expect(list.checkedCount, 3, reason: 'Tent by its group');
      expect(list.groupOf(idOf(ann, note.id, 'Socks')).map((l) => l.text), [
        'Blue',
        'Red',
      ]);
      expect(lists.mayMake(note.id), isFalse, reason: 'one already');
      expect(() => lists.make(note.id), throwsStateError);
    });

    test('two levels only; a first Line indented stays at the top', () {
      final note = checklist(ann, 'List\n  a\n    b\n      c\nd');
      expect(shown(ann, note), ['a', '  b', '  c', 'd']);
      final lone = checklist(ann, 'Title only');
      expect(shown(ann, lone), isEmpty);
      expect(Checklists(ann).byNote(lone)!.allChecked, isTrue);
    });

    test('from a formatted text, the marks of items and quotes go; one '
        'indented goes under the line above', () {
      final note = Notes(ann)
          .capture(
            '# Packing\n- Socks\n  - Blue\n* Tent\n\t+ ✓ Pegs\n1. Map\n'
            '> Torch\n- \n**Rope**\n## Later\n-Knife',
            format: TextFormats.markdown,
          )
          .id;
      Checklists(ann).make(note);
      expect(Notes(ann).byId(note)!.text, '# Packing');
      expect(shown(ann, note), [
        'Socks',
        '  Blue',
        '[x] Tent',
        '  [x] Pegs',
        'Map',
        'Torch',
        '**Rope**',
        '## Later',
        '-Knife',
      ]);
      // A plain text keeps them as written.
      final plain = checklist(ann, 'Packing\n- Socks\n> Torch');
      expect(shown(ann, plain), ['- Socks', '> Torch']);
    });

    test('a Note is at most one of a Tally and a Checklist', () {
      final gym = Notes(ann).capture('Gym\nPush-ups').id;
      Tallies(ann).addField(gym, name: 'Push-ups');
      expect(Checklists(ann).mayMake(gym), isFalse);
      expect(() => Checklists(ann).make(gym), throwsStateError);
      final list = checklist(ann, 'Packing\nSocks');
      expect(
        () => Tallies(ann).addField(list, name: 'Socks'),
        throwsStateError,
      );
      expect(Tallies(ann).isTally(list), isFalse);
    });

    test('kinds and places no app writes are read without fail', () {
      final note = Notes(ann).capture('Odd\nOne').id;
      for (final kind in [
        'checklist',
        'checklist:',
        ':x',
        'checklist:a:b',
        'shoppingList:x',
      ]) {
        ann.append(note, ChecklistKeys.kind, kind);
        expect(Checklists(ann).byNote(note), isNull, reason: kind);
      }
      ann.append(note, ChecklistKeys.kind, null);
      Checklists(ann).make(note);
      final making = Checklists(ann).byNote(note)!.making;
      for (final (i, order) in ['V0', '', 'é', null, 'V'].indexed) {
        final id = 'line:odd$i';
        ann.append(id, ChecklistKeys.type, ChecklistKeys.typeLine);
        ann.append(id, ChecklistKeys.note, note);
        ann.append(id, ChecklistKeys.made, making);
        ann.append(id, ChecklistKeys.text, 'Odd $i');
        if (order != null) ann.append(id, ChecklistKeys.order, order);
      }
      // A Line of another making, and one of no Note, show nowhere.
      ann.append('line:other', ChecklistKeys.type, ChecklistKeys.typeLine);
      ann.append('line:other', ChecklistKeys.note, note);
      ann.append('line:other', ChecklistKeys.made, 'other');
      // The keys no app writes come after the others.
      expect(shown(ann, note), [
        'One',
        'Odd 4',
        'Odd 1',
        'Odd 3',
        'Odd 0',
        'Odd 2',
      ]);
      final lists = Checklists(ann);
      final added = lists.add(note, 'Added', at: 3);
      // Before a Line of the same key goes after it, past any tie.
      lists.move('line:odd2', before: 'line:odd4');
      lists.move(added);
      expect(shown(ann, note), [
        'One',
        'Odd 4',
        'Odd 2',
        'Added',
        'Odd 1',
        'Odd 3',
        'Odd 0',
      ]);
    });
  });

  group('checking', () {
    test('each check is an entry of its own; the Note\'s text never '
        'changes', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs');
      final text = ann.current(note, NoteKeys.text);
      final milk = idOf(ann, note, 'Milk');
      final mark = ann.currentSeq();
      Checklists(ann).check(milk);
      final written = ann.entriesAfter(mark);
      expect(written.single.entity, milk);
      expect(written.single.field, ChecklistKeys.checked);
      expect(shown(ann, note), ['[x] Milk', 'Eggs']);
      Checklists(ann).check(milk);
      expect(ann.entriesAfter(mark), hasLength(1), reason: 'checked already');
      Checklists(ann).uncheck(milk);
      expect(shown(ann, note), ['Milk', 'Eggs']);
      expect(ann.current(note, NoteKeys.text), text);
    });

    test('a parent and its group check each other', () {
      final note = checklist(ann, 'Trip\nBath\n  Soap\n  Towel\nTent');
      final lists = Checklists(ann);
      lists.check(idOf(ann, note, 'Soap'));
      expect(shown(ann, note), ['Bath', '  [x] Soap', '  Towel', 'Tent']);
      final mark = ann.currentSeq();
      lists.check(idOf(ann, note, 'Towel'));
      expect(shown(ann, note), [
        '[x] Bath',
        '  [x] Soap',
        '  [x] Towel',
        'Tent',
      ]);
      expect(
        [for (final e in ann.entriesAfter(mark)) e.entity],
        [idOf(ann, note, 'Towel'), idOf(ann, note, 'Bath')],
      );
      lists.uncheck(idOf(ann, note, 'Soap'));
      expect(shown(ann, note), ['Bath', '  Soap', '  [x] Towel', 'Tent']);
      lists.check(idOf(ann, note, 'Bath'));
      expect(shown(ann, note), [
        '[x] Bath',
        '  [x] Soap',
        '  [x] Towel',
        'Tent',
      ]);
      lists.uncheck(idOf(ann, note, 'Bath'));
      expect(shown(ann, note), ['Bath', '  Soap', '  Towel', 'Tent']);
    });

    test('the parent shows what its group says, whatever came in', () {
      final note = checklist(ann, 'Trip\nBath\n  Soap');
      final bath = idOf(ann, note, 'Bath');
      ann.append(bath, ChecklistKeys.checked, '2026-03-01T09:00:00.000Z');
      expect(shown(ann, note), ['Bath', '  Soap']);
      expect(Checklists(ann).byNote(note)!.line(bath)!.checkedAt, isNotNull);
    });

    test('"Uncheck all" readies it for the next use', () {
      final note = checklist(ann, 'Shop\n✓ Milk\n✓ Eggs\nBread\n  ✓ Rye');
      expect(Checklists(ann).byNote(note)!.checkedCount, 4);
      Checklists(ann).uncheckAll(note);
      expect(shown(ann, note), ['Milk', 'Eggs', 'Bread', '  Rye']);
    });
  });

  group('editing', () {
    test('Lines are added, renamed, deleted and restored', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs');
      final lists = Checklists(ann);
      lists.add(note, '  Bread  ');
      lists.add(note, 'Butter', at: 0);
      lists.add(note, 'Oat', at: 2, indent: true);
      lists.add(note, 'Top', at: 0, indent: true);
      expect(shown(ann, note), [
        'Top',
        'Butter',
        'Milk',
        '  Oat',
        'Eggs',
        'Bread',
      ]);
      expect(() => lists.add(note, ' \n '), throwsArgumentError);
      final milk = idOf(ann, note, 'Milk');
      lists.rename(milk, 'Whole\nmilk');
      expect(shown(ann, note)[2], 'Whole milk');
      lists.delete(milk);
      expect(shown(ann, note), ['Top', 'Butter', 'Oat', 'Eggs', 'Bread']);
      lists.restore(milk);
      expect(shown(ann, note), [
        'Top',
        'Butter',
        'Whole milk',
        'Oat',
        'Eggs',
        'Bread',
      ]);
      expect(() => lists.restore(milk), throwsArgumentError);
    });

    test('Lines move, a group along with its parent, and indent', () {
      final note = checklist(ann, 'Trip\nBath\n  Soap\n  Towel\nTent\nMap');
      final lists = Checklists(ann);
      // A single Line moved into a group joins it.
      lists.move(idOf(ann, note, 'Map'), before: idOf(ann, note, 'Towel'));
      expect(shown(ann, note), ['Bath', '  Soap', '  Map', '  Towel', 'Tent']);
      // Out of it again, before a Line at the top.
      lists.move(idOf(ann, note, 'Map'), before: idOf(ann, note, 'Bath'));
      expect(shown(ann, note), ['Map', 'Bath', '  Soap', '  Towel', 'Tent']);
      // A parent takes its group along and never goes into another.
      lists.move(idOf(ann, note, 'Bath'));
      expect(shown(ann, note), ['Map', 'Tent', 'Bath', '  Soap', '  Towel']);
      lists.indent(idOf(ann, note, 'Tent'));
      lists.move(idOf(ann, note, 'Bath'), before: idOf(ann, note, 'Tent'));
      expect(shown(ann, note), ['Map', '  Tent', 'Bath', '  Soap', '  Towel']);
      // Out of a group, the Lines below it follow it.
      lists.indent(idOf(ann, note, 'Soap'), under: false);
      expect(shown(ann, note), ['Map', '  Tent', 'Bath', 'Soap', '  Towel']);
      // Under the Line above, a parent's group goes along.
      lists.indent(idOf(ann, note, 'Soap'));
      expect(shown(ann, note), ['Map', '  Tent', 'Bath', '  Soap', '  Towel']);
      expect(
        () => lists.indent(idOf(ann, note, 'Map')),
        throwsArgumentError,
        reason: 'nothing above it',
      );
      // Deleting a parent brings its group up to the top.
      lists.delete(idOf(ann, note, 'Bath'));
      expect(shown(ann, note), ['Map', '  Tent', 'Soap', 'Towel']);
    });

    test('keys fit between any two and stay short at the end', () {
      expect(LineOrder.place(null, null), 'V');
      expect(LineOrder.after('V'), 'W');
      expect(LineOrder.after('Vz'), 'W');
      expect(LineOrder.after('z'), 'zV');
      expect(LineOrder.after('zzz'), 'zzzV');
      expect(LineOrder.between('', 'V'), 'F');
      expect(LineOrder.between('', '1'), '0V');
      expect(LineOrder.between('', '0V'), '0F');
      expect(LineOrder.between('V', 'W'), 'VV');
      expect(LineOrder.between('V', 'V05'), 'V02');
      expect(LineOrder.between('V', 'X'), 'W');
      expect(LineOrder.between('VV', 'W'), 'VW');
      expect(LineOrder.between('Vz', 'W'), 'VzV');
      var low = 'V';
      var high = 'W';
      for (var i = 0; i < 200; i++) {
        final mid = LineOrder.between(low, high);
        expect(LineOrder.isValid(mid), isTrue, reason: mid);
        expect(low.compareTo(mid) < 0 && mid.compareTo(high) < 0, isTrue);
        if (i.isEven) {
          low = mid;
        } else {
          high = mid;
        }
      }
      var key = '';
      for (var i = 0; i < 100; i++) {
        key = LineOrder.after(key);
      }
      expect(key.length, lessThanOrEqualTo(4));
    });
  });

  group('back to a plain Note', () {
    test('every Line goes into the text, and making it again starts '
        'afresh', () {
      final note = checklist(ann, 'Trip\nBath\n  Soap\n  ✓ Towel\n✓ Tent');
      final lists = Checklists(ann);
      final soap = idOf(ann, note, 'Soap');
      lists.check(soap);
      lists.add(note, 'Map');
      lists.toPlain(note);
      expect(lists.byNote(note), isNull);
      final text = 'Trip\n✓ Bath\n  ✓ Soap\n  ✓ Towel\n✓ Tent\nMap';
      expect(Notes(ann).byId(note)!.text, text);
      expect(ann.current(soap, ChecklistKeys.checked), isNotNull);
      lists.make(note);
      expect(shown(ann, note), [
        '[x] Bath',
        '  [x] Soap',
        '  [x] Towel',
        '[x] Tent',
        'Map',
      ]);
      expect(lists.byNote(note)!.line(soap), isNull, reason: 'a new making');
      final empty = Notes(ann).capture('').id;
      lists.make(empty);
      lists.add(empty, 'Only');
      lists.toPlain(empty);
      expect(Notes(ann).byId(empty)!.text, 'Only');
    });

    test('search finds a Note by its Lines', () {
      final note = checklist(ann, 'Shop\nOat milk\nEggs');
      expect(Search(ann).notes('MILK').map((n) => n.id), [note]);
      Checklists(ann).delete(idOf(ann, note, 'Oat milk'));
      expect(Search(ann).notes('milk'), isEmpty);
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = device(clock, 'Ben');
      area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });

    test('every Member checks, unchecks and adds Lines', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs', place: area);
      exchange(ben, ann);
      final lists = Checklists(ben);
      expect(lists.mayCheck(note), isTrue);
      lists.check(idOf(ben, note, 'Milk'));
      lists.check(idOf(ben, note, 'Eggs'));
      lists.uncheck(idOf(ben, note, 'Eggs'));
      final bread = lists.add(note, 'Bread');
      exchange(ben, ann);
      expect(shown(ann, note), ['[x] Milk', 'Eggs', 'Bread']);
      expect(ann.authorOf(bread)?.name, 'Ben');
      // His own Line is his to change; Ann's are hers, and his are hers
      // too, as the Checklist's author.
      expect(lists.mayEdit(bread), isTrue);
      expect(lists.mayEdit(idOf(ben, note, 'Milk')), isFalse);
      lists.rename(bread, 'Rye bread');
      exchange(ben, ann);
      Checklists(ann).rename(bread, 'Brown bread');
      exchange(ben, ann);
      expect(shown(ben, note), ['[x] Milk', 'Eggs', 'Brown bread']);
    });

    test('renaming, deleting, moving and the kind stay with the author and '
        'the Managers', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs\nTea', place: area);
      exchange(ben, ann);
      final lists = Checklists(ben);
      final milk = idOf(ben, note, 'Milk');
      final eggs = idOf(ben, note, 'Eggs');
      expect(() => lists.rename(milk, 'Beer'), throwsStateError);
      expect(() => lists.delete(milk), throwsStateError);
      expect(() => lists.move(milk), throwsStateError);
      expect(() => lists.indent(eggs), throwsStateError);
      expect(() => lists.toPlain(note), throwsStateError);
      expect(lists.mayMake(Notes(ben).capture('Mine', place: area).id), isTrue);
      // As a modified app would.
      final forged = [
        ben.append(milk, ChecklistKeys.text, StoredText.plain('Beer').encode()),
        ben.append(eggs, ChecklistKeys.trashed, '2026-03-01T09:00:00.000Z'),
        ben.append(milk, ChecklistKeys.order, 'z'),
        ben.append(idOf(ben, note, 'Tea'), ChecklistKeys.indent, 'true'),
        ben.append(note, ChecklistKeys.kind, null),
      ];
      exchange(ben, ann);
      final held = ann.allEntries().map((e) => e.id).toSet();
      expect(held.containsAll([for (final e in forged) e.id]), isTrue);
      expect(shown(ann, note), ['Milk', 'Eggs', 'Tea']);
      // A Manager may.
      ann.setRole(
        area,
        ann.grantsOf(area)!.personOf(ben.deviceId)!,
        AreaRole.manager,
      );
      exchange(ben, ann);
      lists.rename(milk, 'Oat milk');
      exchange(ben, ann);
      expect(shown(ann, note).first, 'Oat milk');
    });

    test('a Member who made the Checklist changes every Line of it', () {
      final cleo = device(clock, 'Cleo');
      exchange(cleo, ann, inviting: Invite(area, role: AreaRole.member));
      final note = checklist(ben, 'Shop\nMilk', place: area);
      exchange(ben, ann);
      exchange(cleo, ann);
      final tea = Checklists(cleo).add(note, 'Tea');
      exchange(cleo, ann);
      exchange(ben, ann);
      expect(Checklists(ben).mayEdit(tea), isTrue);
      Checklists(ben).rename(tea, 'Green tea');
      Checklists(ben).move(tea, before: idOf(ben, note, 'Milk'));
      exchange(ben, ann);
      exchange(cleo, ann);
      expect(shown(ann, note), ['Green tea', 'Milk']);
      expect(shown(cleo, note), ['Green tea', 'Milk']);
      expect(Checklists(cleo).mayEdit(idOf(cleo, note, 'Milk')), isFalse);
    });

    test('a Checklist made before its Note was filed in goes along', () {
      final note = checklist(ann, 'Shop\nMilk');
      // Written in the Area since, so Ben's vector passes the Lines.
      Notes(ann).capture('Bread', place: area);
      exchange(ben, ann);
      Notes(ann).file(note, area);
      exchange(ben, ann);
      expect(shown(ben, note), ['Milk']);
    });
  });

  group('apart on two Devices', () {
    late Store phone;
    late Store desk;

    setUp(() {
      phone = own.device(clock);
      desk = own.device(clock);
      addTearDown(phone.close);
      addTearDown(desk.close);
    });

    test('two renames of one Line are a Conflict, picked like a text\'s', () {
      final note = checklist(phone, 'Shop\nMilk');
      own.exchange(phone, desk);
      final milk = idOf(phone, note, 'Milk');
      Checklists(phone).rename(milk, 'Oat milk');
      Checklists(desk).rename(milk, 'Soy milk');
      own.exchange(phone, desk);
      final conflict = desk.conflicts().single;
      expect(conflict.entity, milk);
      expect(conflict.unit, NoteKeys.text);
      expect(conflict.type, ChecklistKeys.typeLine);
      expect(conflict.resolvable, isTrue);
      expect(
        [for (final v in conflict.versions) v.text.text],
        ['Soy milk', 'Oat milk'],
      );
      expect(desk.mayResolve(milk, conflict.type), isTrue);
      desk.resolveConflict(conflict, conflict.versions.last);
      own.exchange(phone, desk);
      expect(phone.conflicts(), isEmpty);
      expect(shown(phone, note), ['Oat milk']);
    });

    test('a check against an uncheck: the later one wins, no Conflict', () {
      final note = checklist(phone, 'Shop\nMilk\n✓ Eggs');
      own.exchange(phone, desk);
      final milk = idOf(phone, note, 'Milk');
      final eggs = idOf(phone, note, 'Eggs');
      Checklists(desk).uncheck(eggs);
      Checklists(phone).uncheck(eggs);
      Checklists(phone).check(eggs);
      Checklists(phone).check(milk);
      Checklists(desk).check(milk);
      Checklists(desk).uncheck(milk);
      own.exchange(phone, desk);
      expect(phone.conflicts(), isEmpty);
      expect(desk.conflicts(), isEmpty);
      expect(shown(phone, note), ['Milk', '[x] Eggs']);
      expect(shown(desk, note), shown(phone, note));
    });
  });

  group('Sent Items and Backups', () {
    late Store ben;

    setUp(() {
      ben = device(clock, 'Ben');
    });

    test('a Checklist goes out in format 2 and arrives as one', () {
      final note = checklist(ann, 'Trip\nBath\n  Soap\n✓ Tent');
      final file = sendNote(ann, note);
      expect(jsonDecode(utf8.decode(file))['format'], sentItemListFormat);
      final opened = openSentItem(ben, file);
      expect(
        [for (final l in opened.item.lines!) l.text],
        ['Bath', 'Soap', 'Tent'],
      );
      final got = acceptSentItem(ben, opened.item)!;
      expect(Notes(ben).byId(got)!.isUnsorted, isTrue);
      expect(shown(ben, got), ['Bath', '  Soap', '[x] Tent']);
      // Encrypted to a known person, it says so outside as well.
      final sealed = sendNote(
        ann,
        note,
        to: Recipient(id: 'ben', name: 'Ben', keys: [ben.ownKeyRecord.boxKey!]),
      );
      expect(jsonDecode(utf8.decode(sealed))['format'], sentItemListFormat);
      final read = openSentItem(ben, sealed);
      expect(read.encrypted, isTrue);
      expect(read.item.lines!.map((l) => l.checked), [false, false, true]);
      final plain = Notes(ann).capture('Plain').id;
      expect(
        jsonDecode(utf8.decode(sendNote(ann, plain))),
        isNot(contains('format')),
      );
    });

    test('an update adds, renames and takes away the sender\'s Lines and '
        'keeps the receiver\'s', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs\nTea');
      final got = acceptSentItem(
        ben,
        openSentItem(ben, sendNote(ann, note)).item,
      )!;
      final lists = Checklists(ben);
      lists.check(idOf(ben, got, 'Milk'));
      lists.rename(idOf(ben, got, 'Tea'), 'Green tea');
      lists.add(got, 'Bread');
      Checklists(ann).rename(idOf(ann, note, 'Milk'), 'Oat milk');
      Checklists(ann).delete(idOf(ann, note, 'Eggs'));
      Checklists(ann).add(note, 'Rice', indent: true);
      final opened = openSentItem(ben, sendNote(ann, note));
      final changes = opened.changes!;
      expect(changes.linesAdded.map((l) => l.text), ['Rice']);
      expect(
        [for (final r in changes.linesRenamed) '${r.before} > ${r.after}'],
        ['Milk > Oat milk'],
      );
      expect(changes.linesRemoved.map((l) => l.text), ['Eggs']);
      acceptSentItem(ben, opened.item);
      expect(shown(ben, got), ['[x] Oat milk', 'Green tea', 'Bread', '  Rice']);
      // Back to a plain Note on Ann's side: Ben keeps his own Line.
      Checklists(ann).toPlain(note);
      final plain = openSentItem(ben, sendNote(ann, note));
      expect(plain.changes!.makesPlain, isFalse);
      acceptSentItem(ben, plain.item);
      expect(shown(ben, got), ['Bread']);
      expect(Notes(ben).byId(got)!.text, 'Shop\nOat milk\nTea\n  Rice');
    });

    test('Lines come in format 2 only, and a file in format 2 carries '
        'them', () {
      final note = checklist(ann, 'Trip\nBath');
      // The signature covers the item, not the format beside it.
      Map<String, Object?> outer(List<int> file) =>
          jsonDecode(utf8.decode(file)) as Map<String, Object?>;
      List<int> encode(Map<String, Object?> json) =>
          utf8.encode(jsonEncode(json));
      final listed = outer(sendNote(ann, note))..remove('format');
      expect(
        () => openSentItem(ben, encode(listed)),
        throwsA(isA<NotASentItem>()),
      );
      final plain = Notes(ann).capture('Plain').id;
      final unlisted = outer(sendNote(ann, plain))
        ..['format'] = sentItemListFormat;
      expect(
        () => openSentItem(ben, encode(unlisted)),
        throwsA(isA<NotASentItem>()),
      );
      // Encrypted, the outside says it too.
      final sealed = outer(
        sendNote(
          ann,
          note,
          to: Recipient(
            id: 'ben',
            name: 'Ben',
            keys: [ben.ownKeyRecord.boxKey!],
          ),
        ),
      )..remove('format');
      expect(
        () => openSentItem(ben, encode(sealed)),
        throwsA(isA<NotASentItem>()),
      );
    });

    test('of two Lines with one id, the first counts', () {
      final note = checklist(ann, 'Trip\nBath\nTent');
      final sent = sentItemOf(ann, note);
      final [bath, tent] = sent.lines!;
      final item = SentItem(
        id: sent.id,
        version: sent.version,
        from: sent.from,
        profile: sent.profile,
        text: sent.text,
        lines: [bath, SentLine(bath.id, 'Soap', checked: true), tent],
      );
      final opened = openSentItem(ben, encodeSentItem(item, ann.signingKey));
      expect([for (final l in opened.item.lines!) l.text], ['Bath', 'Tent']);
      final got = acceptSentItem(ben, opened.item)!;
      expect(shown(ben, got), ['Bath', 'Tent']);
      // An update the same: the Line is there once.
      final again = SentItem(
        id: sent.id,
        version: sent.version + 1,
        from: sent.from,
        profile: sent.profile,
        text: sent.text,
        lines: [
          bath,
          tent,
          const SentLine('line:new', 'Map'),
          const SentLine('line:new', 'Rope'),
        ],
      );
      final update = openSentItem(ben, encodeSentItem(again, ann.signingKey));
      expect(update.changes!.linesAdded.map((l) => l.text), ['Map']);
      acceptSentItem(ben, update.item);
      expect(shown(ben, got), ['Bath', 'Tent', 'Map']);
    });

    test('the receiver\'s Note goes plain with the sender\'s', () {
      final note = checklist(ann, 'Shop\nMilk');
      final got = acceptSentItem(
        ben,
        openSentItem(ben, sendNote(ann, note)).item,
      )!;
      Checklists(ann).toPlain(note);
      final opened = openSentItem(ben, sendNote(ann, note));
      expect(opened.changes!.makesPlain, isTrue);
      acceptSentItem(ben, opened.item);
      expect(Checklists(ben).byNote(got), isNull);
      expect(Notes(ben).byId(got)!.text, 'Shop\nMilk');
    });

    test('a Backup carries a Checklist', () async {
      final dir = Directory.systemTemp.createTempSync('checklist');
      addTearDown(() => dir.deleteSync(recursive: true));
      final old = Store.open('${dir.path}/old.db', clock: clock);
      addTearDown(old.close);
      final note = checklist(old, 'Trip\nBath\n  ✓ Soap');
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final path = '${dir.path}/b.$backupExtension';
      await writeBackup(old, path);
      final key = await unlockBackup(
        readBackupHeader(path)!,
        recoveryCode: code,
      );
      final restored = Store.open('${dir.path}/new.db', clock: clock);
      addTearDown(restored.close);
      await restoreBackup(restored, path, key!);
      expect(shown(restored, note), ['[x] Bath', '  [x] Soap']);
    });
  });
}

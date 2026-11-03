import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store annDesk;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
    annDesk = device(clock, 'Ann at the desk');
    ben = device(clock, 'Ben');
  });

  /// A Checklist made from a new Note with [text]; returns the Note.
  String checklist(Store store, String text, {String? place}) {
    final id = Notes(store).capture(text, place: place).id;
    Checklists(store).make(id);
    return id;
  }

  String idOf(Store store, String note, String text) =>
      Checklists(store)
          .byNote(note)!
          .lines
          .firstWhere((l) => l.text == text)
          .id;

  /// Today's Agenda rows of the Checklists of [store], as "title: open of
  /// all", with " as count" for one that shows only its count (#219).
  List<String> rows(Store store) => [
    for (final (:list, :shows) in Checklists(store).agendaRows())
      '${Notes(store).byId(list.noteId)!.text}: '
          '${list.openCount} of ${list.count}'
          '${shows == ListOnAgenda.count ? ' as count' : ''}',
  ];

  /// Every entry of a "Show on the Agenda" choice [store] holds.
  List<Entry> choices(Store store) => [
    for (final e in store.allEntries())
      if (e.entity.startsWith(ChecklistKeys.agendaPrefix)) e,
  ];

  /// Ann's Household shared with Ben as a Member, Ann's desk one of her
  /// own Devices.
  String household() {
    exchange(annDesk, ann);
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(annDesk, ann);
    return area;
  }

  group('on the Agenda', () {
    test('a Checklist shows with its open Lines until switched off or '
        'every Line is checked', () {
      final trip = checklist(ann, 'Before the trip\nTent\nTorch\n  Batteries');
      final lists = Checklists(ann);
      expect(lists.onAgenda(trip), ListOnAgenda.lines);
      expect(lists.mayShowOnAgenda(trip), isTrue);
      expect(rows(ann), ['Before the trip: 3 of 3']);
      lists.check(idOf(ann, trip, 'Batteries'));
      expect(rows(ann), ['Before the trip: 1 of 3']);
      lists.setOnAgenda(trip, ListOnAgenda.off);
      expect(lists.onAgenda(trip), ListOnAgenda.off);
      expect(rows(ann), isEmpty);
      final count = ann.allEntries().length;
      lists.setOnAgenda(trip, ListOnAgenda.off);
      expect(ann.allEntries(), hasLength(count), reason: 'the same again');
      lists.setOnAgenda(trip, ListOnAgenda.count);
      expect(lists.onAgenda(trip), ListOnAgenda.count);
      expect(rows(ann), ['Before the trip: 1 of 3 as count']);
      lists.setOnAgenda(trip, ListOnAgenda.lines);
      expect(rows(ann), ['Before the trip: 1 of 3']);
      expect(choices(ann).map((e) => e.value), ['off', 'count', 'lines']);
      lists.check(idOf(ann, trip, 'Tent'));
      expect(rows(ann), isEmpty, reason: 'every Line checked');
      lists.uncheck(idOf(ann, trip, 'Tent'));
      expect(rows(ann), ['Before the trip: 1 of 3']);
    });

    test('one shown as its count never shows without an open Line', () {
      final trip = checklist(ann, 'Trip\nTent');
      final lists = Checklists(ann)..setOnAgenda(trip, ListOnAgenda.count);
      expect(rows(ann), ['Trip: 1 of 1 as count']);
      lists.check(idOf(ann, trip, 'Tent'));
      expect(rows(ann), isEmpty);
    });

    test('a list nobody chose for shows every open Line, writing nothing', () {
      final trip = checklist(ann, 'Trip\nTent');
      final count = ann.allEntries().length;
      Checklists(ann).setOnAgenda(trip, ListOnAgenda.lines);
      expect(ann.allEntries(), hasLength(count));
      expect(rows(ann), ['Trip: 1 of 1']);
    });

    test('a Checklist with a Routine shows only as its Occurrence', () {
      final gym = checklist(ann, 'Gym bag\nTowel');
      final routine = Routines(
        ann,
      ).spawn(gym, Schedule(start: Day(2026, 3, 1), times: [ClockTime(7, 0)]));
      final lists = Checklists(ann);
      expect(lists.mayShowOnAgenda(gym), isFalse);
      expect(rows(ann), isEmpty);
      Routines(ann).trash(routine.id);
      expect(lists.mayShowOnAgenda(gym), isTrue);
      expect(rows(ann), ['Gym bag: 1 of 1']);
    });

    test('a Routine of a later version counts as none (ADR 0034)', () {
      final trip = checklist(ann, 'Trip\nTent');
      ann.batch(() {
        ann.append('routine:later', RoutineKeys.type, RoutineKeys.typeRoutine);
        ann.append('routine:later', RoutineKeys.note, trip);
        ann.append('routine:later', RoutineKeys.start, '2026-03-01');
        ann.append('routine:later', RoutineKeys.scheduleVersion, '2');
      });
      expect(Routines(ann).newerOfNote(trip), hasLength(1));
      expect(Checklists(ann).mayShowOnAgenda(trip), isTrue);
      expect(rows(ann), ['Trip: 1 of 1']);
    });

    test('a plain Note, a Shopping List, a Checklist without '
        'Lines, in the trash or merged never shows; oldest capture first', () {
      final notes = Notes(ann);
      final lists = Checklists(ann);
      final first = checklist(ann, 'First\nOne');
      notes.capture('Plain\nline');
      final shop = notes.capture('Groceries\nMilk').id;
      ShoppingLists(ann).make(shop);
      checklist(ann, 'Empty');
      final trashed = checklist(ann, 'Trashed\nOne');
      notes.trash(trashed);
      final merged = checklist(ann, 'Merged\nOne');
      final into = notes.capture('Into').id;
      notes.merge(merged, into);
      final second = checklist(ann, 'Second\nOne\nTwo');
      expect(rows(ann), ['First: 1 of 1', 'Second: 2 of 2']);
      for (final id in [shop, trashed, merged, into]) {
        expect(lists.mayShowOnAgenda(id), isFalse, reason: id);
      }
      expect(lists.mayShowOnAgenda(first), isTrue);
      expect(lists.mayShowOnAgenda(second), isTrue);
      expect(
        () => lists.setOnAgenda(shop, ListOnAgenda.off),
        throwsArgumentError,
      );
      expect(
        () => lists.setOnAgenda(into, ListOnAgenda.off),
        throwsArgumentError,
      );
      expect(choices(ann), isEmpty);
    });

    test('a value no app writes, or none, reads as every open Line', () {
      final trip = checklist(ann, 'Trip\nTent');
      for (final value in ['OFF', 'off ', 'false', '', 'Count', 'on', null]) {
        ann.append(
          '${ChecklistKeys.agendaPrefix}$trip',
          ChecklistKeys.agenda,
          value,
        );
        expect(
          Checklists(ann).onAgenda(trip),
          ListOnAgenda.lines,
          reason: value,
        );
      }
      expect(rows(ann), ['Trip: 1 of 1']);
    });

    test('reading writes nothing', () {
      checklist(ann, 'Trip\nTent');
      final count = ann.allEntries().length;
      Checklists(ann).agendaRows();
      expect(ann.allEntries(), hasLength(count));
    });
  });

  group('per person', () {
    test('in a Shared Area each person decides for their own Agenda', () {
      final area = household();
      final trip = checklist(ann, 'Trip\nTent\nTorch', place: area);
      exchange(ben, ann);
      expect(rows(ben), ['Trip: 2 of 2']);
      // Ben, a Member, may switch off a list he did not write.
      Checklists(ben).setOnAgenda(trip, ListOnAgenda.off);
      expect(rows(ben), isEmpty);
      exchange(ben, ann);
      expect(rows(ann), ['Trip: 2 of 2']);
      expect(choices(ann), isEmpty);
      // And Ann hers: Ben's shows its count.
      Checklists(ann).setOnAgenda(trip, ListOnAgenda.off);
      Checklists(ben).setOnAgenda(trip, ListOnAgenda.count);
      final joined = exchange(ben, ann);
      final hosted = exchange(ann, ben);
      for (final wire in [
        joined.request,
        joined.answer,
        hosted.request,
        hosted.answer,
      ]) {
        expect(jsonEncode(wire), isNot(contains(ChecklistKeys.agendaPrefix)));
      }
      expect(rows(ann), isEmpty);
      expect(rows(ben), ['Trip: 2 of 2 as count']);
      expect(choices(ann).map((e) => e.device).toSet(), {ann.deviceId});
      expect(choices(ben).map((e) => e.device).toSet(), {ben.deviceId});
    });

    test('one\'s own Devices carry it', () {
      household();
      final trip = checklist(ann, 'Trip\nTent');
      Checklists(ann).setOnAgenda(trip, ListOnAgenda.off);
      exchange(annDesk, ann);
      expect(rows(annDesk), isEmpty);
      Checklists(annDesk).setOnAgenda(trip, ListOnAgenda.count);
      exchange(annDesk, ann);
      expect(rows(ann), ['Trip: 1 of 1 as count']);
    });

    test('the shared folder holds none another person can open', () async {
      final area = household();
      final trip = checklist(ann, 'Trip\nTent', place: area);
      Checklists(ann).setOnAgenda(trip, ListOnAgenda.off);
      final folder = MemorySyncFolder();
      await folderSync(ann, folder);
      await folderSync(ben, folder);
      await folderSync(annDesk, folder);
      expect(Notes(ben).byId(trip), isNotNull);
      expect(choices(ben), isEmpty);
      expect(rows(ben), ['Trip: 1 of 1']);
      expect(rows(annDesk), isEmpty);
    });

    test('a choice another person\'s Device wrote counts for nothing', () {
      final area = household();
      final trip = checklist(ann, 'Trip\nTent', place: area);
      exchange(ben, ann);
      // As a modified app would: Ben takes the list off Ann's Agenda.
      final forged = ben.append(
        '${ChecklistKeys.agendaPrefix}$trip',
        ChecklistKeys.agenda,
        ChecklistKeys.agendaOff,
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(Checklists(ann).onAgenda(trip), ListOnAgenda.lines);
      expect(rows(ann), ['Trip: 1 of 1']);
      // Nor does it stop her switching it off and on herself.
      Checklists(ann).setOnAgenda(trip, ListOnAgenda.off);
      expect(rows(ann), isEmpty);
    });
  });
}

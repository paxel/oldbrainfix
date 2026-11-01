import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

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
  late Store annDesk;
  late Store ben;

  setUp(() {
    // A Thursday.
    clock = _Clock(DateTime.utc(2026, 3, 5, 10));
    ann = device(clock.call, 'Ann');
    annDesk = device(clock.call, 'Ann at the desk');
    ben = device(clock.call, 'Ben');
  });

  /// A Shopping List made from a new Note with [text]; returns the Note.
  String shopping(Store store, String text, {String? place}) {
    final id = Notes(store).capture(text, place: place).id;
    ShoppingLists(store).make(id);
    return id;
  }

  String idOf(Store store, String note, String text) =>
      ShoppingLists(store)
          .byNote(note, _wall)!
          .lines
          .firstWhere((l) => l.text == text)
          .id;

  /// Today's Agenda rows of the Shopping Lists of [store]: "title: n
  /// open" for one row with the count, "title: Line" for each open Line.
  List<String> rows(Store store) {
    final out = <String>[];
    for (final (:list, :shows, :lines) in ShoppingLists(
      store,
    ).agendaRows(_wall)) {
      final title = Notes(store).byId(list.noteId)!.text;
      switch (shows) {
        case ShoppingOnAgenda.count:
          out.add('$title: ${lines.length} open');
        case ShoppingOnAgenda.lines:
          out.addAll([for (final l in lines) '$title: ${l.text}']);
        case ShoppingOnAgenda.off:
          throw StateError('an Off row');
      }
    }
    return out;
  }

  /// Every entry of a Shopping List's Agenda choice [store] holds.
  List<Entry> choices(Store store) => [
    for (final e in store.allEntries())
      if (e.entity.startsWith(ChecklistKeys.agendaPrefix) &&
          e.field == ShoppingKeys.agenda)
        e,
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
    test('each list shows as chosen for it', () {
      final lists = ShoppingLists(ann);
      final week = shopping(ann, 'Groceries\nMilk\nEggs\nTea');
      final hardware = shopping(ann, 'Hardware\nNails\nGlue');
      final pharmacy = shopping(ann, 'Pharmacy\nPlasters');
      lists.check(idOf(ann, week, 'Tea'), _wall);
      expect(lists.onAgenda(week), ShoppingOnAgenda.count);
      expect(lists.mayShowOnAgenda(week), isTrue);
      expect(rows(ann), [
        'Groceries: 2 open',
        'Hardware: 2 open',
        'Pharmacy: 1 open',
      ]);
      lists.setOnAgenda(hardware, ShoppingOnAgenda.lines);
      lists.setOnAgenda(pharmacy, ShoppingOnAgenda.off);
      expect(lists.onAgenda(pharmacy), ShoppingOnAgenda.off);
      expect(rows(ann), [
        'Groceries: 2 open',
        'Hardware: Nails',
        'Hardware: Glue',
      ]);
      final count = ann.allEntries().length;
      lists.setOnAgenda(pharmacy, ShoppingOnAgenda.off);
      expect(ann.allEntries(), hasLength(count), reason: 'the same again');
      // Chosen as the default shows it, it is written all the same.
      lists.setOnAgenda(week, ShoppingOnAgenda.count);
      expect(choices(ann).map((e) => e.value), ['lines', 'off', 'count']);
    });

    test('a list with no open Line never shows', () {
      final lists = ShoppingLists(ann);
      final week = shopping(ann, 'Groceries\nMilk\nEggs');
      final hardware = shopping(ann, 'Hardware\nNails');
      shopping(ann, 'Title only');
      lists.setOnAgenda(week, ShoppingOnAgenda.lines);
      lists.setReappear(idOf(ann, week, 'Eggs'), Reappear.once);
      for (final text in ['Milk', 'Eggs']) {
        lists.check(idOf(ann, week, text), _wall);
      }
      lists.check(idOf(ann, hardware, 'Nails'), _wall);
      expect(rows(ann), isEmpty);
      // A week on, Milk and Nails are back, Eggs gone Once.
      clock.set(DateTime.utc(2026, 3, 12, 9));
      expect(rows(ann), ['Groceries: Milk', 'Hardware: 1 open']);
    });

    test('a list nobody chose for shows as the Device showed Shopping '
        'Lists before', () {
      final lists = ShoppingLists(ann);
      final week = shopping(ann, 'Groceries\nMilk');
      final chosen = shopping(ann, 'Hardware\nNails');
      lists.setOnAgenda(chosen, ShoppingOnAgenda.count);
      for (final (old, shows) in [
        (ShoppingAgenda.off, ShoppingOnAgenda.off),
        (ShoppingAgenda.lines, ShoppingOnAgenda.lines),
        (ShoppingAgenda.back, ShoppingOnAgenda.count),
        (ShoppingAgenda.open, ShoppingOnAgenda.count),
      ]) {
        ann.setLocalSetting('shoppingAgenda', old.name);
        expect(lists.onAgenda(week), shows, reason: old.name);
        expect(lists.onAgenda(chosen), ShoppingOnAgenda.count);
      }
      ann.setLocalSetting('shoppingAgenda', ShoppingAgenda.off.name);
      expect(rows(ann), ['Hardware: 1 open']);
      ann.setLocalSetting('shoppingAgenda', 'sideways');
      expect(lists.onAgenda(week), ShoppingOnAgenda.count);
    });

    test('a value no app writes reads as one row with the count', () {
      ann.setLocalSetting('shoppingAgenda', ShoppingAgenda.off.name);
      final week = shopping(ann, 'Groceries\nMilk');
      final entity = '${ChecklistKeys.agendaPrefix}$week';
      for (final value in ['OFF', 'off ', 'later', '']) {
        ann.append(entity, ShoppingKeys.agenda, value);
        expect(
          ShoppingLists(ann).onAgenda(week),
          ShoppingOnAgenda.count,
          reason: value,
        );
      }
      // Null is no choice: the default again.
      ann.append(entity, ShoppingKeys.agenda, null);
      expect(ShoppingLists(ann).onAgenda(week), ShoppingOnAgenda.off);
    });

    test('a Checklist\'s switch is not a Shopping List\'s choice', () {
      final note = Notes(ann).capture('Groceries\nMilk').id;
      final checklists = Checklists(ann);
      checklists
        ..make(note)
        ..setShowOnAgenda(note, false)
        ..toPlain(note);
      final lists = ShoppingLists(ann)..make(note);
      expect(lists.onAgenda(note), ShoppingOnAgenda.count);
      expect(rows(ann), ['Groceries: 1 open']);
      lists
        ..setOnAgenda(note, ShoppingOnAgenda.off)
        ..toPlain(note, _wall);
      checklists
        ..make(note)
        ..setShowOnAgenda(note, true);
      expect(checklists.agenda(), hasLength(1));
    });

    test('only Shopping Lists not in the trash nor merged take a choice; '
        'one with a Routine shows too', () {
      final notes = Notes(ann);
      final lists = ShoppingLists(ann);
      final plain = notes.capture('Plain\nline').id;
      final trip = notes.capture('Trip\nTent').id;
      Checklists(ann).make(trip);
      final trashed = shopping(ann, 'Trashed\nOne');
      notes.trash(trashed);
      final merged = shopping(ann, 'Merged\nOne');
      final into = notes.capture('Into').id;
      notes.merge(merged, into);
      final errands = shopping(ann, 'Errands\nStamps');
      Routines(ann).spawn(errands, Schedule(start: Day(2026, 3, 5)));
      for (final id in [plain, trip, trashed, merged, into]) {
        expect(lists.mayShowOnAgenda(id), isFalse, reason: id);
      }
      expect(lists.mayShowOnAgenda(errands), isTrue);
      expect(rows(ann), ['Errands: 1 open']);
      expect(
        () => lists.setOnAgenda(plain, ShoppingOnAgenda.off),
        throwsArgumentError,
      );
      expect(
        () => lists.setOnAgenda(trip, ShoppingOnAgenda.off),
        throwsArgumentError,
      );
      expect(choices(ann), isEmpty);
    });

    test('reading writes nothing', () {
      final week = shopping(ann, 'Groceries\nMilk');
      final count = ann.allEntries().length;
      ShoppingLists(ann)
        ..agendaRows(_wall)
        ..onAgenda(week);
      expect(ann.allEntries(), hasLength(count));
    });
  });

  group('per person', () {
    test('in a Shared Area each person chooses for their own Agenda', () {
      final area = household();
      final note = shopping(ann, 'Shop\nMilk\nEggs', place: area);
      exchange(ben, ann);
      // Ben, a Member, may choose for a list he did not write.
      ShoppingLists(ben).setOnAgenda(note, ShoppingOnAgenda.lines);
      ShoppingLists(ann).setOnAgenda(note, ShoppingOnAgenda.off);
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
      expect(rows(ben), ['Shop: Milk', 'Shop: Eggs']);
      expect(choices(ann).map((e) => (e.device, e.value)), [
        (ann.deviceId, 'off'),
      ]);
      expect(choices(ben).map((e) => (e.device, e.value)), [
        (ben.deviceId, 'lines'),
      ]);
    });

    test('one\'s own Devices carry it', () {
      household();
      final note = shopping(ann, 'Shop\nMilk');
      exchange(annDesk, ann);
      // The desk's old setting showed nothing; her choice shows it there.
      annDesk.setLocalSetting('shoppingAgenda', ShoppingAgenda.off.name);
      expect(rows(annDesk), isEmpty);
      ShoppingLists(ann).setOnAgenda(note, ShoppingOnAgenda.lines);
      exchange(annDesk, ann);
      expect(rows(annDesk), ['Shop: Milk']);
      ShoppingLists(annDesk).setOnAgenda(note, ShoppingOnAgenda.count);
      exchange(annDesk, ann);
      expect(rows(ann), ['Shop: 1 open']);
    });

    test('a choice another person\'s Device wrote counts for nothing', () {
      final area = household();
      final note = shopping(ann, 'Shop\nMilk', place: area);
      exchange(ben, ann);
      // As a modified app would: Ben takes the list off Ann's Agenda.
      final forged = ben.append(
        '${ChecklistKeys.agendaPrefix}$note',
        ShoppingKeys.agenda,
        ShoppingOnAgenda.off.code,
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      final lists = ShoppingLists(ann);
      expect(lists.onAgenda(note), ShoppingOnAgenda.count);
      expect(rows(ann), ['Shop: 1 open']);
      // Nor does it count as her own choice made already.
      lists.setOnAgenda(note, ShoppingOnAgenda.off);
      expect(rows(ann), isEmpty);
    });
  });
}

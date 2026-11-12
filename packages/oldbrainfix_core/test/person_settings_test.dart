import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  group('"Move a Note to the Trash when its one-time Reminder is done"', () {
    test('off until switched on; the same again writes nothing', () {
      expect(ann.trashesDoneReminder, isFalse);
      ann.trashesDoneReminder = true;
      expect(ann.trashesDoneReminder, isTrue);
      final written = ann.allEntries().length;
      ann.trashesDoneReminder = true;
      expect(ann.allEntries().length, written);
      ann.trashesDoneReminder = false;
      expect(ann.trashesDoneReminder, isFalse);
      expect(
        ann.current(
          PersonSettingKeys.entity,
          PersonSettingKeys.trashDoneReminder,
        ),
        isNull,
      );
    });

    test('a value no app writes reads as off', () {
      ann.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.trashDoneReminder,
        'yes',
      );
      expect(ann.trashesDoneReminder, isFalse);
    });

    test("it travels between one's own Devices", () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      ann.trashesDoneReminder = true;
      exchange(desk, ann);
      expect(desk.trashesDoneReminder, isTrue);
      desk.trashesDoneReminder = false;
      exchange(desk, ann);
      expect(ann.trashesDoneReminder, isFalse);
    });
  });

  group('Simple or Advanced', () {
    test('Advanced until Simple is chosen; the same again writes nothing', () {
      expect(ann.advanced, isTrue);
      ann.advanced = false;
      expect(ann.advanced, isFalse);
      expect(
        ann.current(PersonSettingKeys.entity, PersonSettingKeys.features),
        PersonSettingKeys.simple,
      );
      final written = ann.allEntries().length;
      ann.advanced = false;
      expect(ann.allEntries().length, written);
      ann.advanced = true;
      expect(ann.advanced, isTrue);
      expect(
        ann.current(PersonSettingKeys.entity, PersonSettingKeys.features),
        PersonSettingKeys.advanced,
      );
    });

    test('Advanced chosen while none was stored is written', () {
      final written = ann.allEntries().length;
      ann.advanced = true;
      expect(ann.allEntries().length, written + 1);
      ann.advanced = true;
      expect(ann.allEntries().length, written + 1);
    });

    test('a value no app writes reads as Advanced', () {
      ann.append(PersonSettingKeys.entity, PersonSettingKeys.features, 'easy');
      expect(ann.advanced, isTrue);
    });

    test("it travels between one's own Devices", () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      ann.advanced = false;
      exchange(desk, ann);
      expect(desk.advanced, isFalse);
      desk.advanced = true;
      exchange(desk, ann);
      expect(ann.advanced, isTrue);
    });
  });

  group("the person's own order of the Areas", () {
    late Places places;
    List<String> names(Store store) => [
      for (final area in store.areasInOrder()) area.name,
    ];
    String id(String name) =>
        places.areas().firstWhere((a) => a.name == name).id;
    void order(Store store, Object? value, {DateTime? date}) => store.append(
      PersonSettingKeys.entity,
      PersonSettingKeys.areaOrder,
      value is String ? value : jsonEncode(value),
      date: date,
    );

    setUp(() {
      places = Places(ann);
      for (final name in ['Work', 'Home', 'Car']) {
        places.addArea(name);
      }
    });

    test('by name until moved; the same place again writes nothing', () {
      expect(names(ann), ['Car', 'Home', 'Work']);
      ann.moveArea(id('Work'), 0);
      expect(names(ann), ['Work', 'Car', 'Home']);
      expect(
        jsonDecode(
          ann.current(PersonSettingKeys.entity, PersonSettingKeys.areaOrder) ??
              'null',
        ),
        [id('Work'), id('Car'), id('Home')],
      );
      final written = ann.allEntries().length;
      ann.moveArea(id('Work'), 0);
      ann.setAreaOrder([id('Work'), id('Car')]);
      expect(ann.allEntries().length, written);
      ann.moveArea(id('Car'), 99);
      expect(names(ann), ['Work', 'Home', 'Car']);
      ann.moveArea(id('Car'), -1);
      expect(names(ann), ['Car', 'Work', 'Home']);
      expect(places.areas().map((a) => a.name), ['Car', 'Home', 'Work']);
    });

    test('nothing is stored while the order stays by name', () {
      ann.moveArea(id('Car'), 0);
      ann.setAreaOrder([id('Car'), id('Home')]);
      expect(ann.loggedHistory(PersonSettingKeys.entity), isEmpty);
    });

    test('new Areas go to the end, those missing from the order by name', () {
      ann.moveArea(id('Work'), 0);
      places.addArea('Garden');
      expect(names(ann), ['Work', 'Car', 'Home', 'Garden']);
      places.addArea('Attic');
      expect(names(ann), ['Work', 'Car', 'Home', 'Attic', 'Garden']);
      ann.moveArea(id('Attic'), 1);
      places.deleteArea(id('Car'), NotesGo.unsorted);
      expect(names(ann), ['Work', 'Attic', 'Home', 'Garden']);
    });

    test('the Areas left out of a new order keep the order they had', () {
      ann.moveArea(id('Work'), 0);
      ann.setAreaOrder([id('Home'), 'area:gone', id('Home')]);
      expect(names(ann), ['Home', 'Work', 'Car']);
    });

    test('moving what is no Area is refused', () {
      expect(() => ann.moveArea('area:gone', 0), throwsArgumentError);
      final topic = places.addTopic(id('Home'), 'Garden');
      expect(() => ann.moveArea(topic.id, 0), throwsArgumentError);
      expect(ann.loggedHistory(PersonSettingKeys.entity), isEmpty);
    });

    test('ids that are no Area, repeats and other values are skipped', () {
      final topic = places.addTopic(id('Home'), 'Garden').id;
      order(ann, [id('Work'), 'area:gone', topic, 7, null, id('Work')]);
      expect(names(ann), ['Work', 'Car', 'Home']);
      for (final value in ['not json', '{"a":1}', '"area:x"', '', 'null']) {
        order(ann, value);
        expect(names(ann), ['Car', 'Home', 'Work'], reason: value);
      }
    });

    test('an order dated back loses to the newer one', () {
      ann.moveArea(id('Work'), 0);
      order(ann, [id('Home')], date: DateTime.utc(2020));
      expect(names(ann), ['Work', 'Car', 'Home']);
    });

    test("it travels between one's own Devices", () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      ann.moveArea(id('Work'), 0);
      exchange(desk, ann);
      expect(names(desk), ['Work', 'Car', 'Home']);
      desk.moveArea(id('Home'), 0);
      exchange(desk, ann);
      expect(names(ann), ['Home', 'Work', 'Car']);
    });

    test('a starter Area keeps its place when it is shared', () {
      places.plantStarterSet({'household'});
      final car = places.areas().firstWhere((a) => a.starter == 'household');
      ann.moveArea(car.id, 0);
      final before = names(ann);
      final shared = ann.share(car.id);
      expect(shared, isNot(car.id));
      expect(names(ann), before);
      expect(ann.areasInOrder().first.id, shared);
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

    test('it never reaches another person', () {
      ann.trashesDoneReminder = true;
      Notes(ann).capture('Milk', place: area);
      exchange(ben, ann);
      expect(ben.loggedHistory(PersonSettingKeys.entity), isEmpty);
      expect(ben.trashesDoneReminder, isFalse);
    });

    test('Simple never reaches another person', () {
      ann.advanced = false;
      Notes(ann).capture('Milk', place: area);
      exchange(ben, ann);
      expect(ben.loggedHistory(PersonSettingKeys.entity), isEmpty);
      expect(ben.advanced, isTrue);
    });

    test("Simple another person's Device wrote counts for nothing", () {
      exchange(ben, ann);
      final forged = ben.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.features,
        PersonSettingKeys.simple,
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.advanced, isTrue);
    });

    test("what another person's Device wrote counts for nothing", () {
      exchange(ben, ann);
      // As a modified app would: Ben slips the setting into Ann's log.
      final forged = ben.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.trashDoneReminder,
        PersonSettingKeys.on,
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(ann.trashesDoneReminder, isFalse);
    });

    test("the order of the Areas never reaches another person", () {
      final attic = Places(ann).addArea('Attic').id;
      ann.moveArea(attic, 99);
      exchange(ben, ann);
      expect(ben.loggedHistory(PersonSettingKeys.entity), isEmpty);
      final bens = Places(ben).addArea('Bike').id;
      expect([for (final a in ben.areasInOrder()) a.id], [bens, area]);
      ben.moveArea(bens, 99);
      exchange(ben, ann);
      expect([for (final a in ben.areasInOrder()) a.id], [area, bens]);
      expect([for (final a in ann.areasInOrder()) a.id], [area, attic]);
    });

    test("an order another person's Device wrote counts for nothing", () {
      final attic = Places(ann).addArea('Attic').id;
      exchange(ben, ann);
      final forged = ben.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.areaOrder,
        jsonEncode([area, attic]),
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect([for (final a in ann.areasInOrder()) a.id], [attic, area]);
    });

    test("a Member's Done moves nothing of another person's Note", () {
      final dentist = Notes(ann).capture('Dentist', place: area);
      final reminder = Routines(ann)
          .spawn(dentist.id, Schedule(start: Day.today(clock(), (utc) => utc)));
      exchange(ben, ann);
      ben.trashesDoneReminder = true;
      final occurrence = reminder.schedule.on(reminder.schedule.start).single;
      expect(Routines(ben).checkOff(reminder.id, occurrence), isFalse);
      expect(Notes(ben).byId(dentist.id)?.isTrashed, isFalse);
      expect(Routines(ben).byId(reminder.id)?.doneAt(occurrence), isNotNull);
    });
  });
}

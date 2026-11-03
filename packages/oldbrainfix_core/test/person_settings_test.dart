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

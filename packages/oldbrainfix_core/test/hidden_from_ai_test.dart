import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Places places;
  late Notes notes;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    places = Places(ann);
    notes = Notes(ann);
  });

  Matcher refusedFor(List<String> marks) =>
      isA<HiddenFromAiRefused>().having((e) => e.marks, 'marks', marks);

  group('the mark', () {
    test('set and taken off; the same again writes nothing', () {
      final note = notes.capture('Diary').id;
      expect(ann.isMarkedHiddenFromAi(note), isFalse);
      ann.setHiddenFromAi(note, true);
      expect(ann.isMarkedHiddenFromAi(note), isTrue);
      expect(ann.hiddenFromAiMarks(), {note});
      final written = ann.allEntries().length;
      ann.setHiddenFromAi(note, true);
      expect(ann.allEntries().length, written);
      ann.setHiddenFromAi(note, false);
      expect(ann.isMarkedHiddenFromAi(note), isFalse);
      expect(
        ann.current(HiddenFromAiKeys.of(note), HiddenFromAiKeys.hidden),
        isNull,
      );
      ann.setHiddenFromAi(note, false);
      expect(ann.allEntries().length, written + 1);
    });

    test('a value no app writes is no mark', () {
      final note = notes.capture('Diary').id;
      ann.append(HiddenFromAiKeys.of(note), HiddenFromAiKeys.hidden, 'yes');
      expect(ann.isMarkedHiddenFromAi(note), isFalse);
      expect(ann.isHiddenFromAi(note), isFalse);
    });

    test('only on a Note or an Area', () {
      final area = places.addArea('Home').id;
      final topic = places.addTopic(area, 'Kitchen').id;
      expect(ann.mayHideFromAi(topic), isFalse);
      expect(() => ann.setHiddenFromAi(topic, true), throwsArgumentError);
      expect(() => ann.setHiddenFromAi('note:gone', true), throwsArgumentError);
      ann.setHiddenFromAi(area, true);
      expect(ann.isMarkedHiddenFromAi(area), isTrue);
    });

    test('a Note, a Note in a hidden Area and all that is on them are '
        'hidden; a Note filed out of the Area is not', () {
      final area = places.addArea('Health').id;
      final topic = places.addTopic(area, 'Back').id;
      final inTopic = notes.capture('Physio', place: topic).id;
      final inArea = notes.capture('Doctor', place: area).id;
      final alone = notes.capture('Diary').id;
      final open = notes.capture('Shopping').id;
      final section = notes.amend(inTopic, 'Exercises').id;
      final routine = Routines(ann)
          .spawn(
            alone,
            Schedule(start: Day(2026, 11, 12), times: [ClockTime(9, 0)]),
          )
          .id;
      final frog = Frogs(ann).spawn(inArea).id;
      final list = notes.capture('Bag\nShoes').id;
      Checklists(ann).make(list);
      final line = Checklists(ann).add(list, 'Towel');
      final merged = notes.capture('Notes on the diary').id;
      notes.merge(merged, alone);
      ann.setHiddenFromAi(area, true);
      ann.setHiddenFromAi(alone, true);
      ann.setHiddenFromAi(list, true);
      expect(ann.hiddenFromAiBy(inTopic), [area]);
      expect(ann.hiddenFromAiBy(section), [area]);
      expect(ann.hiddenFromAiBy(frog), [area]);
      expect(ann.hiddenFromAiBy(topic), [area]);
      expect(ann.hiddenFromAiBy(routine), [alone]);
      expect(ann.hiddenFromAiBy(merged), [alone]);
      expect(ann.hiddenFromAiBy(line), [list]);
      expect(ann.hiddenFromAiBy('${ChecklistKeys.agendaPrefix}$list'), [list]);
      expect(ann.isHiddenFromAi(open), isFalse);
      notes.file(inArea, places.addArea('Work').id);
      expect(ann.isHiddenFromAi(inArea), isFalse);
      expect(ann.isHiddenFromAi(frog), isFalse);
      ann.setHiddenFromAi(area, false);
      expect(ann.isHiddenFromAi(inTopic), isFalse);
    });

    test("it travels between one's own Devices", () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      final note = notes.capture('Diary').id;
      ann.setHiddenFromAi(note, true);
      exchange(desk, ann);
      expect(desk.isHiddenFromAi(note), isTrue);
      desk.setHiddenFromAi(note, false);
      exchange(desk, ann);
      expect(ann.isHiddenFromAi(note), isFalse);
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = device(clock, 'Ben');
      area = ann.share(places.addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });

    test('the mark is not set there, but always taken off', () {
      final note = notes.capture('Milk', place: area).id;
      expect(ann.mayHideFromAi(area), isFalse);
      expect(ann.mayHideFromAi(note), isFalse);
      expect(() => ann.setHiddenFromAi(area, true), throwsStateError);
      expect(() => ann.setHiddenFromAi(note, true), throwsStateError);
      expect(ann.hiddenFromAiMarks(), isEmpty);
      // Marked before it was shared, as two own Devices apart could.
      ann.append(
        HiddenFromAiKeys.of(note),
        HiddenFromAiKeys.hidden,
        HiddenFromAiKeys.on,
      );
      expect(ann.isHiddenFromAi(note), isTrue);
      ann.setHiddenFromAi(note, false);
      expect(ann.isHiddenFromAi(note), isFalse);
    });

    test('it never reaches another person', () {
      final diary = notes.capture('Diary').id;
      ann.setHiddenFromAi(diary, true);
      notes.capture('Milk', place: area);
      exchange(ben, ann);
      expect(ben.entitiesWithPrefix(HiddenFromAiKeys.prefix), isEmpty);
    });

    test("a mark another person's Device wrote counts for nothing", () {
      final diary = notes.capture('Diary').id;
      exchange(ben, ann);
      // As a modified app would: Ben slips a mark into Ann's log.
      final forged = ben.append(
        HiddenFromAiKeys.of(diary),
        HiddenFromAiKeys.hidden,
        HiddenFromAiKeys.on,
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(ann.isMarkedHiddenFromAi(diary), isFalse);
      expect(ann.isHiddenFromAi(diary), isFalse);
    });

    test('a hidden Note is not filed there, one of a hidden Area neither', () {
      final diary = notes.capture('Diary').id;
      ann.setHiddenFromAi(diary, true);
      final topic = places.addTopic(area, 'Kitchen').id;
      final health = places.addArea('Health').id;
      ann.setHiddenFromAi(health, true);
      final doctor = notes.capture('Doctor', place: health).id;
      final written = ann.allEntries().length;
      expect(() => notes.file(diary, area), throwsA(refusedFor([diary])));
      expect(() => notes.file(diary, topic), throwsA(refusedFor([diary])));
      expect(() => notes.file(doctor, area), throwsA(refusedFor([health])));
      expect(ann.allEntries().length, written);
      expect(notes.byId(diary)?.place, isNull);
      notes.file(diary, places.addArea('Private').id);
      final work = places.addArea('Work').id;
      notes.file(doctor, work);
      notes.file(doctor, area);
      expect(notes.byId(doctor)?.place, area);
      expect(ann.isHiddenFromAi(doctor), isFalse);
    });

    test("a link another person's Device wrote shows nothing hidden", () {
      final health = places.addArea('Health').id;
      final doctor = notes.capture('Doctor', place: health).id;
      final diary = notes.capture('Diary').id;
      final later = notes.capture('Diary, later').id;
      notes.merge(later, diary);
      ann.setHiddenFromAi(health, true);
      ann.setHiddenFromAi(diary, true);
      exchange(ben, ann);
      // As a modified app would: Ben slips entries into Ann's log that
      // take her Notes out of the hidden Area and the hidden Note.
      final forged = [
        ben.append(doctor, NoteKeys.place, null),
        ben.append(later, NoteKeys.merged, null),
      ];
      ann.applyEntries(forged, verifiedDevice: ben.deviceId);
      expect(ann.logged(doctor, NoteKeys.place), isNull);
      expect(ann.hiddenFromAiBy(doctor), [health]);
      expect(ann.hiddenFromAiBy(later), [diary]);
    });
  });

  group('sharing is refused', () {
    test('for a hidden Area and one holding a hidden Note, and nothing '
        'changes', () {
      final health = places.addArea('Health').id;
      notes.capture('Doctor', place: health);
      ann.setHiddenFromAi(health, true);
      final written = ann.allEntries().length;
      expect(() => ann.share(health), throwsA(refusedFor([health])));
      expect(ann.allEntries().length, written);
      expect(ann.isShared(health), isFalse);
      final home = places.addArea('Home').id;
      final kitchen = places.addTopic(home, 'Kitchen').id;
      final recipe = notes.capture('Recipe', place: kitchen).id;
      final diary = notes.capture('Diary', place: home).id;
      final trashed = notes.capture('Old', place: home).id;
      ann.setHiddenFromAi(recipe, true);
      ann.setHiddenFromAi(trashed, true);
      Routines(ann).trashNote(trashed);
      final before = ann.allEntries().length;
      expect(
        () => ann.share(home),
        throwsA(refusedFor([recipe, trashed]..sort())),
      );
      expect(ann.allEntries().length, before);
      expect(ann.isShared(home), isFalse);
      ann.setHiddenFromAi(recipe, false);
      ann.setHiddenFromAi(trashed, false);
      expect(ann.share(home), home);
      expect(ann.isShared(home), isTrue);
      expect(ann.isHiddenFromAi(diary), isFalse);
    });

    test('for a starter Area before it moves to fresh ids', () {
      places.plantStarterSet({'household'});
      final household = Places.starterId(PlaceKeys.typeArea, 'household');
      ann.setHiddenFromAi(household, true);
      final written = ann.allEntries().length;
      expect(() => ann.share(household), throwsA(refusedFor([household])));
      expect(ann.allEntries().length, written);
    });

    test('for a hidden Note as a Sent Item, its cancellation too', () {
      final health = places.addArea('Health').id;
      final doctor = notes.capture('Doctor', place: health).id;
      final diary = notes.capture('Diary').id;
      ann.setHiddenFromAi(health, true);
      ann.setHiddenFromAi(diary, true);
      final written = ann.allEntries().length;
      expect(() => sentItemOf(ann, doctor), throwsA(refusedFor([health])));
      expect(() => sentItemOf(ann, diary), throwsA(refusedFor([diary])));
      expect(
        () => sentItemOf(ann, diary, cancel: true),
        throwsA(refusedFor([diary])),
      );
      expect(ann.allEntries().length, written);
      ann.setHiddenFromAi(diary, false);
      expect(sentItemOf(ann, diary).text, 'Diary');
    });

    test('for a Pack of a hidden Note, Area, or one holding them', () {
      final health = places.addArea('Health').id;
      final back = places.addTopic(health, 'Back').id;
      final physio = notes.capture('Physio', place: back).id;
      notes.capture('Doctor', place: health);
      final diary = notes.capture('Diary').id;
      ann.setHiddenFromAi(physio, true);
      ann.setHiddenFromAi(diary, true);
      expect(() => ann.packOf(health), throwsA(refusedFor([physio])));
      expect(() => ann.packOf(back), throwsA(refusedFor([physio])));
      expect(() => ann.packOf(diary), throwsA(refusedFor([diary])));
      ann.setHiddenFromAi(physio, false);
      expect(ann.packOf(health).notes, hasLength(2));
      ann.setHiddenFromAi(health, true);
      expect(() => ann.packOf(back), throwsA(refusedFor([health])));
    });

    test('for a Pack whose Session picks from a hidden Tally', () {
      final laps = notes.capture('Laps').id;
      Tallies(ann).addField(laps, name: 'Laps', unit: '');
      final training = notes.capture('Training').id;
      Sessions(ann)
          .make(training, pool: [laps], count: 1, rule: SessionRule.rolling);
      ann.setHiddenFromAi(laps, true);
      expect(() => ann.packOf(training), throwsA(refusedFor([laps])));
    });

    test('a Pack of everything leaves what is hidden out; a Backup opened '
        'keeps it', () {
      final health = places.addArea('Health').id;
      notes.capture('Doctor', place: health);
      final diary = notes.capture('Diary').id;
      notes.capture('Shopping');
      final laps = notes.capture('Laps').id;
      Tallies(ann).addField(laps, name: 'Laps', unit: '');
      final training = notes.capture('Training').id;
      Sessions(ann)
          .make(training, pool: [laps], count: 1, rule: SessionRule.rolling);
      ann.setHiddenFromAi(health, true);
      ann.setHiddenFromAi(diary, true);
      ann.setHiddenFromAi(laps, true);
      final texts = [for (final n in ann.packOfAll().notes) n.text];
      expect(texts, ['Training', 'Shopping']);
      final everything = ann.packOfEverything().pack;
      expect(everything.notes, hasLength(5));
    });
  });
}

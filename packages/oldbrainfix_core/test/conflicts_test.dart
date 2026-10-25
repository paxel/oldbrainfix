import 'dart:convert';
import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:test/test.dart';

/// A clock shared by every store of a test, one second per read, so
/// Devices write in a known order.
DateTime Function() ticking() {
  var now = DateTime.utc(2026, 3, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

Store device(DateTime Function() clock) => Store.inMemory(clock: clock);

/// One exchange as the LAN session runs it, without the network; the
/// sessions of both sides.
(SyncSession host, SyncSession joiner) exchange(Store joiner, Store host) {
  final request = syncRequest(joiner, host.versionVector(), name: 'Phone');
  final done = answerSync(
    host,
    jsonDecode(jsonEncode(request)) as Map<String, Object?>,
    name: 'Desk',
  )!;
  final back = applyAnswer(
    joiner,
    jsonDecode(jsonEncode(done.answer)) as Map<String, Object?>,
  );
  return (done.session, back);
}

/// The values of every version of [conflict], newest first.
List<String?> values(Conflict conflict) => [
  for (final v in conflict.versions) v.value,
];

void main() {
  late DateTime Function() clock;
  late Store phone;
  late Store desk;

  setUp(() {
    clock = ticking();
    phone = device(clock);
    desk = device(clock);
  });

  tearDown(() {
    phone.close();
    desk.close();
  });

  group('detecting', () {
    test('two Devices editing the same text offline are a Conflict', () {
      final note = Notes(phone).capture('nees mor sauce');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Needs more sauce');
      Notes(desk).edit(note.id, 'Needs more hot sauce');
      final (hostSide, joinerSide) = exchange(phone, desk);

      for (final (store, session) in [(desk, hostSide), (phone, joinerSide)]) {
        final conflict = store.conflicts().single;
        expect(conflict.entity, note.id);
        expect(conflict.unit, NoteKeys.text);
        expect(conflict.type, NoteKeys.typeNote);
        expect(values(conflict), ['Needs more hot sauce', 'Needs more sauce']);
        expect(conflict.versions.first.entry.device, desk.deviceId);
        expect(conflict.versions.last.entry.device, phone.deviceId);
        expect(session.conflicts.single.entity, note.id);
        // Latest wins keeps showing one of them meanwhile.
        expect(Notes(store).byId(note.id)?.text, 'Needs more hot sauce');
      }
    });

    test('found whatever the Devices\' clocks say', () {
      // The desk's clock is far behind: its edit is dated before the
      // Note was even captured.
      final behind = Store.inMemory(
        clock: () => clock().subtract(const Duration(days: 1)),
      );
      addTearDown(behind.close);
      final note = Notes(phone).capture('nees mor sauce');
      exchange(phone, behind);
      Notes(behind).edit(note.id, 'From yesterday');
      Notes(phone).edit(note.id, 'From today');
      exchange(phone, behind);
      for (final store in [phone, behind]) {
        expect(values(store.conflicts().single), [
          'From today',
          'From yesterday',
        ]);
      }
    });

    test('an edit made after seeing the other one is no Conflict', () {
      final note = Notes(phone).capture('First');
      exchange(phone, desk);
      Notes(desk).edit(note.id, 'Second');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Third');
      final (hostSide, joinerSide) = exchange(phone, desk);
      expect(hostSide.conflicts, isEmpty);
      expect(joinerSide.conflicts, isEmpty);
      expect(desk.conflicts(), isEmpty);
      expect(phone.conflicts(), isEmpty);
    });

    test('the same value written on both sides is no Conflict', () {
      final note = Notes(phone).capture('nees mor sauce');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Needs more sauce');
      Notes(desk).edit(note.id, 'Needs more sauce');
      exchange(phone, desk);
      expect(phone.conflicts(), isEmpty);
      expect(desk.conflicts(), isEmpty);
    });

    test('Area and Topic names, Section text, a Named Place radius', () {
      final places = Places(phone);
      final area = places.addArea('Household');
      final topic = places.addTopic(area.id, 'Kitchen');
      final note = Notes(phone).capture('Dentist');
      final section = Notes(phone).amend(note.id, 'on Tuesday');
      final home = NamedPlaces(phone)
          .add('Home', Coordinates(52.5, 13.4), radius: 100);
      exchange(phone, desk);
      places.rename(area.id, 'House');
      Places(desk).rename(area.id, 'Home');
      places.rename(topic.id, 'Cooking');
      Places(desk).rename(topic.id, 'Food');
      phone.append(section.id, SectionKeys.text, 'on Wednesday');
      desk.append(section.id, SectionKeys.text, 'on Thursday');
      NamedPlaces(phone).rename(home.id, 'Flat');
      NamedPlaces(desk).setRadius(home.id, 250);
      NamedPlaces(phone).setRadius(home.id, 50);
      exchange(phone, desk);
      final units = {
        for (final c in desk.conflicts()) (c.entity, c.unit): c.type,
      };
      expect(units, {
        (area.id, PlaceKeys.name): PlaceKeys.typeArea,
        (topic.id, PlaceKeys.name): PlaceKeys.typeTopic,
        (section.id, SectionKeys.text): SectionKeys.typeSection,
        (home.id, NamedPlaceKeys.radius): NamedPlaceKeys.typeNamedPlace,
      });
      expect(
        [for (final c in phone.conflicts()) (c.entity, c.unit)],
        [for (final c in desk.conflicts()) (c.entity, c.unit)],
      );
    });

    test('a schedule conflicts as a whole', () {
      final routine = Routines(phone).create(
        'Pills',
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      exchange(phone, desk);
      Routines(phone).reschedule(
        routine.id,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(9, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      Routines(desk).reschedule(
        routine.id,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
        ),
      );
      exchange(phone, desk);
      for (final store in [phone, desk]) {
        final conflict = store.conflicts().single;
        expect(conflict.unit, scheduleUnit);
        expect(conflict.type, RoutineKeys.typeRoutine);
        expect(
          [for (final v in conflict.versions) v.values],
          [
            {
              'start': '2026-03-01',
              'times': '08:00',
              'rule': 'FREQ=WEEKLY;BYDAY=MO',
            },
            {'start': '2026-03-01', 'times': '09:00', 'rule': 'FREQ=DAILY'},
          ],
        );
      }
      // Latest wins mixes them meanwhile, the same on both.
      expect(
        Routines(phone).byId(routine.id)?.schedule.times,
        Routines(desk).byId(routine.id)?.schedule.times,
      );
    });

    test('Done marks, trash, filing and Importance go by latest wins', () {
      final routine = Routines(phone).create(
        'Pills',
        Schedule(start: Day(2026, 3, 1), times: [ClockTime(8, 0)]),
      );
      final note = Notes(phone).capture('Milk');
      final area = Places(phone).addArea('Household');
      exchange(phone, desk);
      final occurrence = Occurrence(Day(2026, 3, 1), ClockTime(8, 0));
      Routines(phone).markDone(routine.id, occurrence);
      Routines(desk).markDone(routine.id, occurrence);
      Notes(phone).setImportance(note.id, Importance.important);
      Notes(desk).setImportance(note.id, Importance.unimportant);
      Notes(phone).trash(note.id);
      Notes(desk).file(note.id, area.id);
      Places(phone).setColour(area.id, 'red');
      Places(desk).setColour(area.id, 'blue');
      exchange(phone, desk);
      expect(phone.conflicts(), isEmpty);
      expect(desk.conflicts(), isEmpty);
    });

    test('a deleted Area keeps no Conflict', () {
      final area = Places(phone).addArea('Household');
      exchange(phone, desk);
      Places(phone).rename(area.id, 'House');
      Places(desk).rename(area.id, 'Home');
      exchange(phone, desk);
      expect(phone.conflicts(), hasLength(1));
      Places(phone).deleteArea(area.id, NotesGo.unsorted);
      expect(phone.conflicts(), isEmpty);
    });

    test(
      'a Starter Set rename on one side wins over planting on the other',
      () {
        Places(phone).plantStarterSet({'work'});
        Places(desk).plantStarterSet({'work'});
        final work = Places.starterId(PlaceKeys.typeArea, 'work');
        Places(phone).rename(work, 'Office');
        exchange(phone, desk);
        expect(phone.conflicts(), isEmpty);
        expect(desk.conflicts(), isEmpty);
        expect(Places(desk).areaById(work)?.name, 'Office');
      },
    );

    test('a log written before contexts were kept has no Conflicts', () {
      final note = Notes(phone).capture('First');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Phone');
      Notes(desk).edit(note.id, 'Desk');
      final old = device(clock);
      addTearDown(old.close);
      for (final e in [...phone.allEntries(), ...desk.allEntries()]) {
        old.insert(
          Entry(
            seq: -1,
            device: e.device,
            dseq: e.dseq,
            entity: e.entity,
            field: e.field,
            value: e.value,
            date: e.date,
            author: e.author,
            recorded: e.recorded,
            sig: e.sig,
          ),
        );
      }
      expect(old.conflicts(), isEmpty);
      expect(Notes(old).byId(note.id)?.text, 'Desk');
    });
  });

  group('resolving', () {
    late String noteId;

    setUp(() {
      noteId = Notes(phone).capture('nees mor sauce').id;
      exchange(phone, desk);
      Notes(phone).edit(noteId, 'Needs more sauce');
      Notes(desk).edit(noteId, 'Needs more hot sauce');
      exchange(phone, desk);
    });

    test('keeping the losing version ends it on both Devices', () {
      final conflict = desk.conflicts().single;
      desk.resolveConflict(conflict, conflict.versions.last);
      expect(desk.conflicts(), isEmpty);
      expect(Notes(desk).byId(noteId)?.text, 'Needs more sauce');
      final (_, joinerSide) = exchange(phone, desk);
      expect(joinerSide.conflicts, isEmpty);
      expect(phone.conflicts(), isEmpty);
      expect(Notes(phone).byId(noteId)?.text, 'Needs more sauce');
    });

    test('keeping the version that shows still writes and ends it', () {
      final conflict = phone.conflicts().single;
      final before = phone.allEntries().length;
      phone.resolveConflict(conflict, conflict.versions.first);
      expect(phone.allEntries(), hasLength(before + 1));
      expect(phone.conflicts(), isEmpty);
      exchange(phone, desk);
      expect(desk.conflicts(), isEmpty);
      expect(Notes(desk).byId(noteId)?.text, 'Needs more hot sauce');
    });

    test('a third Device holding only the loser takes the pick quietly', () {
      // The third Device met the phone after the phone's edit, before
      // the phone met the desk again: it holds the losing version only.
      final third = device(clock);
      addTearDown(third.close);
      final note = Notes(phone).capture('Call mum');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Call mum at six');
      exchange(third, phone);
      Notes(desk).edit(note.id, 'Call mum at seven');
      exchange(phone, desk);
      final conflict = desk.conflicts().firstWhere((c) => c.entity == note.id);
      expect(conflict.versions.first.value, 'Call mum at seven');
      desk.resolveConflict(conflict, conflict.versions.first);
      // The winner and the pick arrive at the third Device in one go.
      final (_, joinerSide) = exchange(third, desk);
      expect(joinerSide.conflicts.where((c) => c.entity == note.id), isEmpty);
      expect(third.conflictOn(note.id, NoteKeys.text), isNull);
      expect(Notes(third).byId(note.id)?.text, 'Call mum at seven');
    });

    test('a pick is dated after a version from a clock running ahead', () {
      final ahead = Store.inMemory(
        clock: () => clock().add(const Duration(hours: 1)),
      );
      addTearDown(ahead.close);
      exchange(ahead, phone);
      Notes(ahead).edit(noteId, 'From the future');
      Notes(phone).edit(noteId, 'From now');
      exchange(ahead, phone);
      final conflict = phone.conflicts().single;
      final now = conflict.versions.firstWhere((v) => v.value == 'From now');
      phone.resolveConflict(conflict, now);
      expect(phone.conflicts(), isEmpty);
      expect(Notes(phone).byId(noteId)?.text, 'From now');
    });

    test('picking two different versions on two Devices is a new one', () {
      final onDesk = desk.conflicts().single;
      desk.resolveConflict(onDesk, onDesk.versions.first);
      final onPhone = phone.conflicts().single;
      phone.resolveConflict(onPhone, onPhone.versions.last);
      exchange(phone, desk);
      expect(values(desk.conflicts().single), [
        'Needs more sauce',
        'Needs more hot sauce',
      ]);
    });

    test('a schedule is kept whole', () {
      final routine = Routines(phone).create(
        'Pills',
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      exchange(phone, desk);
      Routines(phone).reschedule(
        routine.id,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(9, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      Routines(desk).reschedule(
        routine.id,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
        ),
      );
      exchange(phone, desk);
      final conflict = phone.conflictOn(routine.id, scheduleUnit)!;
      final daily = conflict.versions.firstWhere(
        (v) => v.values[RoutineKeys.rule] == 'FREQ=DAILY',
      );
      phone.resolveConflict(conflict, daily);
      exchange(phone, desk);
      for (final store in [phone, desk]) {
        expect(store.conflictOn(routine.id, scheduleUnit), isNull);
        final schedule = Routines(store).byId(routine.id)?.schedule;
        expect(schedule?.times, [ClockTime(9, 0)]);
        expect(schedule?.rule?.format(), 'FREQ=DAILY');
      }
    });
  });

  group('contexts', () {
    test('travel signed: a changed context is refused', () {
      final note = Notes(phone).capture('First');
      exchange(phone, desk);
      Notes(desk).edit(note.id, 'Second');
      final edit = desk.allEntries().last;
      expect(
        edit.ctx,
        '${phone.deviceId}:${phone.versionVector()[phone.deviceId]}',
      );
      final json = edit.toJson()..['ctx'] = '';
      final report = ImportReport();
      phone.applyEntries(
        [Entry.fromJson(json)],
        keys: desk.keyRecords(),
        report: report,
      );
      expect(report.refusedCount, 1);
    });

    test('only edits of conflicting fields carry one', () {
      final note = Notes(phone).capture('First');
      Notes(phone).edit(note.id, 'Second');
      Notes(phone).setImportance(note.id, Importance.important);
      final byField = {for (final e in phone.history(note.id)) e.field: e.ctx};
      expect(byField[NoteKeys.text], '');
      expect(byField[NoteKeys.importance], isNull);
      expect(phone.history(note.id).first.ctx, isNull);
      expect(phone.history(note.id).first.toJson().containsKey('ctx'), isFalse);
    });

    test('are written and read by device', () {
      expect(encodeContext({'b': 2, 'a': 10}), 'a:10 b:2');
      expect(encodeContext({}), '');
      expect(decodeContext('a:10 b:2'), {'a': 10, 'b': 2});
      expect(decodeContext(''), isEmpty);
      expect(decodeContext('junk :3 c:x'), isEmpty);
    });

    test('a store from before contexts gains the column', () {
      final dir = Directory.systemTemp.createTempSync('oldbrainfix_ctx_');
      addTearDown(() => dir.deleteSync(recursive: true));
      final path = '${dir.path}/oldbrainfix.db';
      final key = SigningKey.generate();
      Store.open(path, newKey: () => key).close();
      final db = sqlite3.open(path);
      db.execute('DROP TABLE entries');
      db.execute('''
        CREATE TABLE entries (
          seq      INTEGER PRIMARY KEY AUTOINCREMENT,
          device   TEXT NOT NULL,
          dseq     INTEGER NOT NULL,
          entity   TEXT NOT NULL,
          field    TEXT NOT NULL,
          value    TEXT,
          date     TEXT NOT NULL,
          author   TEXT NOT NULL,
          recorded TEXT NOT NULL,
          sig      TEXT
        )''');
      db.execute(
        "INSERT INTO entries (device, dseq, entity, field, value, date, author, recorded) "
        "VALUES ('d', 1, 'note:a', 'text', 'kept', '2026-01-01T00:00:00.000Z', '', '2026-01-01T00:00:00.000Z')",
      );
      db.close();
      final store = Store.open(path, newKey: () => key);
      addTearDown(store.close);
      expect(store.current('note:a', 'text'), 'kept');
      expect(store.history('note:a').single.ctx, isNull);
      store.append('note:a', 'text', 'edited');
      expect(store.history('note:a').last.ctx, 'd:1');
    });
  });
}

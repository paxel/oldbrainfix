import 'dart:convert';
import 'dart:typed_data';

import 'package:axt_protocol/axt_protocol.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:oldbrainfix_core/src/axt_types.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

DateTime _utc(DateTime utc) => utc;

/// A file of another app, signed by a key of its own: an Appointment at
/// 10:00 in Berlin's winter time with two reminders before it and one
/// after, a Task done on its due day, and a Memo, a type Old Brain Fix
/// does not take.
Future<Uint8List> _catalogFile() async {
  final appointment = TypeDescription.fromJson(
    jsonDecode(axtAppointmentDescription),
  );
  final task = TypeDescription.fromJson(jsonDecode(axtTaskDescription));
  final memo = TypeDescription.fromJson({
    'name': 'memo',
    'doc': 'A memo.',
    'texts': <String, Object?>{},
    'kind': {
      'axt.ObjectField': {
        'name': 'org.example.Memo',
        'version': 1,
        'fields': [
          {
            'name': 'text',
            'doc': 'The text.',
            'texts': <String, Object?>{},
            'required': true,
            'kind': {'axt.StringField': <String, Object?>{}},
          },
        ],
      },
    },
  });
  final node = await AxtNode.open(
    store: _Memory(),
    signingSeed: Uint8List.fromList(List.generate(32, (i) => 200 - i)),
    manifest: {
      'app_name': 'cat-a-log',
      'app_url': null,
      'formats': [1],
      'takes': <Object?>[],
      'any_type': null,
      'sends': null,
      'encryption_key': null,
      'relay': null,
    },
    clock: () => DateTime.utc(2026, 11, 20),
  );
  Map<String, Object?> duration(int days, int millis) => {
    'months': 0,
    'days': days,
    'millis': millis,
  };
  return node.writeFile([
    OutgoingRecord('0b9a8f1e-3c2d-4e5f-8a7b-6c5d4e3f2a10', appointment, {
      'title': 'Vet: Mia',
      'start': {
        'year': 2026,
        'month': 12,
        'day': 3,
        'hour': 10,
        'minute': 0,
        'second': 0,
        'millisecond': 0,
        'zone': 'Europe/Berlin',
        'offset_seconds': 3600,
        'instant_ms': DateTime.utc(2026, 12, 3, 9).millisecondsSinceEpoch,
      },
      'end': null,
      'all_day': false,
      'location': 'Main Street 4',
      'notes': 'Bring the booklet',
      'reminders': [
        duration(-1, 0),
        duration(0, -1200000),
        duration(0, 600000),
      ],
      'relations': <Object?>[],
      'extensions': <Object?>[],
    }),
    OutgoingRecord('1c2d3e4f-5a6b-4c7d-8e9f-a0b1c2d3e4f5', task, {
      'title': 'Order food',
      'notes': '',
      'due': {
        'year': 2026,
        'month': 11,
        'day': 18,
        'hour': 0,
        'minute': 0,
        'second': 0,
        'millisecond': 0,
        'zone': null,
        'offset_seconds': null,
        'instant_ms': null,
      },
      'due_all_day': true,
      'repeat': 'FREQ=MONTHLY',
      'done': true,
      'done_at': null,
      'priority': 'NONE',
      'relations': <Object?>[],
      'extensions': <Object?>[],
    }),
    OutgoingRecord('2d3e4f5a-6b7c-4d8e-9fa0-b1c2d3e4f5a6', memo, {
      'text': 'Not for Old Brain Fix',
    }),
  ]);
}

final class _Memory implements AxtStore {
  final _values = <String, Uint8List>{};

  @override
  Future<Uint8List?> read(String key) async => _values[key];

  @override
  Future<void> write(String key, Uint8List value) async => _values[key] = value;

  @override
  Future<void> delete(String key) async => _values.remove(key);

  @override
  Future<List<String>> list(String prefix) async => [
    for (final k in _values.keys)
      if (k.startsWith(prefix)) k,
  ];

  @override
  Future<T> transaction<T>(Future<T> Function() body) => body();
}

/// Another app that keeps what it took between files, as one does: one
/// library instance on one store, taking Notes and Tasks.
Future<Future<Map<String, String>> Function(Uint8List)> _receiver() async {
  final held = <String, Map<String, Object?>?>{};
  final node = await AxtNode.open(
    store: _Memory(),
    signingSeed: Uint8List.fromList(List.filled(32, 77)),
    manifest: {
      'app_name': 'second-brain',
      'app_url': null,
      'formats': [1],
      'takes': [
        for (final type in [AxtTypes.note, AxtTypes.task])
          {
            'type': type,
            'min_version': 1,
            'max_version': null,
            'purpose': 'It becomes a page.',
            'purpose_translations': <String, String>{},
          },
      ],
      'any_type': null,
      'sends': null,
      'encryption_key': null,
      'relay': null,
    },
    readers: [
      for (final text in [axtNoteDescription, axtTaskDescription])
        TypeDescription.fromJson(jsonDecode(text)),
    ],
  );
  return (Uint8List bytes) async {
    final result = await node.apply(
      await node.inspect(bytes),
      acceptSender: true,
      onRecord: (change) async => held[change.id.toLowerCase()] = change.value,
      currentValue: (_, id) async => held[id],
    );
    return {for (final r in result.records) r.id: r.result};
  };
}

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
    ben = device(clock, 'Ben');
  });

  ({String plain, String formatted, String dentist, String pills}) notes() {
    final notes = Notes(ann);
    final routines = Routines(ann);
    final plain = notes.capture('Shopping\nMilk and #Eggs ☐ #eggs').id;
    final formatted = notes
        .capture(
          '# Garden\nWater the **roses** #garden/roses',
          format: TextFormats.markdown,
        )
        .id;
    final dentist = notes.capture('Dentist\nAt the new place').id;
    final reminder = routines.spawn(
      dentist,
      Schedule(start: Day(2026, 12, 3), times: [ClockTime(10, 0)]),
    );
    routines.markDone(
      reminder.id,
      Occurrence(Day(2026, 12, 3), ClockTime(10, 0)),
    );
    final pills = notes.capture('Pills').id;
    routines.spawn(
      pills,
      Schedule(
        start: Day(2026, 11, 1),
        times: [ClockTime(8, 0), ClockTime(20, 0)],
        rule: Recurrence.parse('FREQ=DAILY;COUNT=10'),
      ),
    );
    return (plain: plain, formatted: formatted, dentist: dentist, pills: pills);
  }

  group('writing', () {
    test('a Note goes as a Note, one with a Reminder or Routine as a Task', () {
      final n = notes();
      final files = AxtFiles(ann);
      final (plainType, plain) = files.axtRecordOf(n.plain)!;
      expect(plainType, AxtTypes.note);
      expect(plain['title'], 'Shopping');
      expect(plain['text'], 'Shopping\nMilk and #Eggs ☐ #eggs');
      expect(plain['format'], 'PLAIN');
      expect(plain['tags'], ['Eggs']);
      final (_, formatted) = files.axtRecordOf(n.formatted)!;
      expect(formatted['title'], 'Garden');
      expect(formatted['format'], 'MARKDOWN');
      expect(formatted['tags'], ['garden/roses']);
      final (dentistType, dentist) = files.axtRecordOf(n.dentist)!;
      expect(dentistType, AxtTypes.task);
      expect(dentist['title'], 'Dentist');
      expect(dentist['notes'], 'At the new place');
      expect(dentist['repeat'], isNull);
      expect(dentist['done'], isTrue);
      expect(dentist['due'], containsPair('hour', 10));
      expect(dentist['due'], containsPair('zone', null));
      final (_, pills) = files.axtRecordOf(n.pills)!;
      // Two times of day as BYHOUR and BYMINUTE, COUNT as it counts.
      expect(pills['repeat'], 'FREQ=DAILY;COUNT=10;BYHOUR=8,20;BYMINUTE=0');
      expect(pills['done'], isFalse);
    });

    test('the record id is the Note id as a UUID of version 4, one of its '
        'own for each type', () {
      expect(
        axtRecordId('note:000000000000000000000000000000ab', AxtTypes.note),
        '00000000-0000-4000-8000-0000000000ab',
      );
      final task = axtRecordId(
        'note:000000000000000000000000000000ab',
        AxtTypes.task,
      );
      expect(task, isNot('00000000-0000-4000-8000-0000000000ab'));
      expect(task.substring(14, 15), '4');
      final other = axtRecordId('note:help-start', AxtTypes.note);
      expect(
        other,
        matches(
          RegExp(
            r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-'
            r'[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
          ),
        ),
      );
      expect(axtRecordId('note:help-start', AxtTypes.note), other);
    });
  });

  group('reading', () {
    test(
      'a file of one Device comes back as the Notes it was written from',
      () async {
        final n = notes();
        final bytes = await AxtFiles(ann)
            .write([n.plain, n.formatted, n.dentist, n.pills, 'note:gone']);
        final file = await AxtFiles(ben).read(bytes, wallClock: _utc);
        expect(file.sender, hex(ann.signingKey.publicKey));
        expect(file.appName, 'Old Brain Fix');
        expect(file.fromOwnDevice, isFalse);
        expect(file.notTaken, isEmpty);
        expect(
          [for (final r in file.records) r.id],
          [
            for (final id in [n.plain, n.formatted])
              axtRecordId(id, AxtTypes.note),
            for (final id in [n.dentist, n.pills])
              axtRecordId(id, AxtTypes.task),
          ],
        );
        final drafts = [for (final r in file.records) r.draft!];
        expect(drafts[0].text, 'Shopping\nMilk and #Eggs ☐ #eggs');
        expect(drafts[0].format, TextFormats.plain);
        expect(drafts[1].text, '# Garden\nWater the **roses** #garden/roses');
        expect(drafts[1].format, TextFormats.markdown);
        expect(drafts[2].text, 'Dentist\nAt the new place');
        expect(drafts[2].schedule?.start, Day(2026, 12, 3));
        expect(drafts[2].schedule?.times, [ClockTime(10, 0)]);
        expect(drafts[2].done, isTrue);
        expect(drafts[3].schedule?.dayTimes, [
          ClockTime(8, 0),
          ClockTime(20, 0),
        ]);
        expect(drafts[3].schedule?.rule?.count, 10);
        // Its own file is from one of its own Devices.
        final own = await AxtFiles(ann).read(bytes, wallClock: _utc);
        expect(own.fromOwnDevice, isTrue);
      },
    );

    test('reading takes nothing and keeps nothing: the same file reads the '
        'same again', () async {
      final n = notes();
      final bytes = await AxtFiles(ann).write([n.plain]);
      final entries = ben.allEntries().length;
      final settings = ben.localSettingsWithPrefix('').length;
      final first = await AxtFiles(ben).read(bytes, wallClock: _utc);
      final second = await AxtFiles(ben).read(bytes, wallClock: _utc);
      expect(ben.allEntries().length, entries);
      expect(ben.localSettingsWithPrefix('').length, settings);
      expect(second.taken.single.id, first.taken.single.id);
      expect(second.taken.single.version, first.taken.single.version);
    });

    test('a file changed after it was signed is no AXT file', () async {
      final n = notes();
      final bytes = await AxtFiles(ann).write([n.plain]);
      for (final at in [bytes.length ~/ 2, bytes.length - 20]) {
        final changed = Uint8List.fromList(bytes);
        changed[at] ^= 0x01;
        await expectLater(
          AxtFiles(ben).read(changed, wallClock: _utc),
          throwsA(isA<NotAnAxtFile>()),
        );
      }
      await expectLater(
        AxtFiles(ben)
            .read(Uint8List.fromList(utf8.encode('{}')), wallClock: _utc),
        throwsA(isA<NotAnAxtFile>()),
      );
    });

    test('a second file of the same Note is a later version of the same '
        'record', () async {
      final n = notes();
      final first = await AxtFiles(ben)
          .read(await AxtFiles(ann).write([n.plain]), wallClock: _utc);
      Notes(ann).edit(n.plain, 'Shopping\nMilk');
      final second = await AxtFiles(ben)
          .read(await AxtFiles(ann).write([n.plain]), wallClock: _utc);
      expect(second.taken.single.id, first.taken.single.id);
      expect(
        second.taken.single.version,
        greaterThan(first.taken.single.version),
      );
    });

    test("another app's file: an Appointment on the wall clock with its "
        'Heads-ups, a Task done, and a type not taken', () async {
      final file = await AxtFiles(ben)
          .read(await _catalogFile(), wallClock: _utc);
      expect(file.appName, 'cat-a-log');
      expect(file.fromOwnDevice, isFalse);
      final [vet, food] = file.taken;
      expect(vet.type, AxtTypes.appointment);
      expect(vet.draft!.text, 'Vet: Mia\nMain Street 4\nBring the booklet');
      // 10:00 in Berlin is 09:00 on this UTC wall clock.
      expect(vet.draft!.schedule?.start, Day(2026, 12, 3));
      expect(vet.draft!.schedule?.times, [ClockTime(9, 0)]);
      expect(vet.draft!.headsUps, [20, 1440]);
      expect(food.type, AxtTypes.task);
      expect(food.draft!.schedule?.times, isEmpty);
      expect(food.draft!.schedule?.rule?.format(), 'FREQ=MONTHLY');
      expect(food.draft!.done, isTrue);
      final memo = file.notTaken.single;
      expect(memo.type, 'org.example.Memo');
      expect(memo.result, 'APPLIED');
    });
  });

  group('another app that keeps what it took', () {
    test(
      'takes every later file of a Note as an update, whatever changed',
      () async {
        final n = notes();
        final files = AxtFiles(ann);
        final take = await _receiver();
        Future<String?> send(String note, String type) async =>
            (await take(await files.write([note])))[axtRecordId(note, type)];
        final first = await take(
          await files.write([n.plain, n.formatted, n.dentist, n.pills]),
        );
        expect(first.values.toSet(), {'APPLIED'});
        final updated = ['APPLIED', 'MERGED'];
        // Its text edited, a Box ticked, an Occurrence Done.
        Notes(ann).edit(n.plain, 'Shopping\nMilk and #Eggs ☐ #eggs and bread');
        expect(await send(n.plain, AxtTypes.note), isIn(updated));
        final box = Notes(ann).byId(n.plain)!.boxes.single;
        Boxes(ann).tick(n.plain, box.id, BoxState.done);
        expect(await send(n.plain, AxtTypes.note), isIn(updated));
        final pills = Routines(ann).ofNote(n.pills).single;
        Routines(ann)
            .markDone(pills.id, Occurrence(Day(2026, 11, 1), ClockTime(8, 0)));
        expect(await send(n.pills, AxtTypes.task), isIn(updated));
        // It gains a Reminder: a Task of its own; losing it, the Note again.
        final reminder = Routines(ann).spawn(
          n.plain,
          Schedule(start: Day(2026, 12, 1), times: [ClockTime(9, 0)]),
        );
        expect(await send(n.plain, AxtTypes.task), 'APPLIED');
        Routines(ann).trash(reminder.id);
        expect(await send(n.plain, AxtTypes.note), isIn(updated));
        // Sent beside a Note changed later, then alone.
        Notes(ann).edit(n.formatted, '# Garden\nWater the roses');
        await take(await files.write([n.plain, n.formatted]));
        expect(await send(n.plain, AxtTypes.note), isIn(updated));
      },
    );
  });

  group('taking', () {
    test('the chosen records become Notes where they are filed, with where '
        'they came from', () async {
      final file = await AxtFiles(ben)
          .read(await _catalogFile(), wallClock: _utc);
      final vet = Places(ben).addArea('Vet');
      final [vetRecord, food] = file.taken;
      final made = AxtFiles(ben).take(file, [vetRecord.id], place: vet.id);
      final note = Notes(ben).byId(made.single)!;
      expect(note.place, vet.id);
      expect(note.text, 'Vet: Mia\nMain Street 4\nBring the booklet');
      final routine = Routines(ben).ofNote(note.id).single;
      expect(routine.schedule.times, [ClockTime(9, 0)]);
      expect(Routines(ben).headsUps(routine.id), [20, 1440]);
      final from = AxtFiles(ben).from(note.id)!;
      expect(from.sender, file.sender);
      expect(from.id, vetRecord.id);
      expect(from.version, vetRecord.version);
      expect(from.type, AxtTypes.appointment);
      // Into Unsorted, Done on its first Occurrence.
      final unsorted = AxtFiles(ben).take(file, [food.id]).single;
      expect(Notes(ben).byId(unsorted)!.place, isNull);
      final task = Routines(ben).ofNote(unsorted).single;
      expect(task.doneAt(Occurrence(Day(2026, 11, 18))), isNotNull);
      // A record not taken is never made.
      expect(AxtFiles(ben).take(file, [file.notTaken.single.id]), isEmpty);
    });

    test('a place that is no Area or Topic is refused before anything is '
        'written', () async {
      final file = await AxtFiles(ben)
          .read(await _catalogFile(), wallClock: _utc);
      final before = ben.allEntries().length;
      expect(
        () => AxtFiles(ben).take(file, [file.taken.first.id], place: 'area:x'),
        throwsArgumentError,
      );
      expect(ben.allEntries().length, before);
    });

    test('a Member takes no Reminder into a Shared Area, and a Device that '
        'left it takes nothing there', () async {
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      final file = await AxtFiles(ben)
          .read(await _catalogFile(), wallClock: _utc);
      final before = ben.allEntries().length;
      expect(
        () => AxtFiles(ben).take(file, [file.taken.first.id], place: area),
        throwsStateError,
      );
      expect(ben.allEntries().length, before);
      final milk = Notes(ann).capture('Milk').id;
      final plain = await AxtFiles(ben)
          .read(await AxtFiles(ann).write([milk]), wallClock: _utc);
      final made = AxtFiles(ben)
          .take(plain, [plain.taken.single.id], place: area);
      expect(Notes(ben).byId(made.single)!.place, area);
      ben.leaveArea(area);
      final after = ben.allEntries().length;
      expect(
        () => AxtFiles(ben).take(plain, [plain.taken.single.id], place: area),
        throwsStateError,
      );
      expect(ben.allEntries().length, after);
    });

    test("what another person's Device wrote counts for nothing", () async {
      final file = await AxtFiles(ben)
          .read(await _catalogFile(), wallClock: _utc);
      final taken = AxtFiles(ben).take(file, [file.taken.first.id]).single;
      final own = Notes(ben).capture('Milk').id;
      final kept = AxtFiles(ben).from(taken)!;
      // As a modified app in a Shared Area would: Cy's Device writes the
      // field of Ben's Notes.
      final cy = device(clock, 'Cy');
      final forged = AxtFrom(
        sender: 'ab' * 32,
        id: '3e4f5a6b-7c8d-4e9f-a0b1-c2d3e4f5a6b7',
        type: AxtTypes.note,
        version: 1 << 60,
      ).encode();
      ben
        ..insert(cy.append(taken, AxtKeys.from, forged))
        ..insert(cy.append(own, AxtKeys.from, forged));
      expect(AxtFiles(ben).from(taken)?.encode(), kept.encode());
      expect(AxtFiles(ben).from(own), isNull);
    });
  });
}

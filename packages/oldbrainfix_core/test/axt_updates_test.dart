import 'dart:convert';
import 'dart:typed_data';

import 'package:axt_protocol/axt_protocol.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:oldbrainfix_core/src/axt_types.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

DateTime _utc(DateTime utc) => utc;

final _types = {
  for (final text in [
    axtNoteDescription,
    axtTaskDescription,
    axtAppointmentDescription,
  ])
    TypeDescription.fromJson(jsonDecode(text)),
};

TypeDescription _type(String name) => _types.firstWhere((t) => t.name == name);

/// A file signed with the key of [sender] holding [records], written as
/// the library writes any file, at [sender]'s now: what Old Brain Fix
/// does not write itself, such as a deletion.
Future<Uint8List> _file(Store sender, List<OutgoingRecord> records) async {
  final node = await AxtNode.open(
    store: _Memory(),
    signingSeed: sender.signingKey.seed,
    manifest: axtManifest,
    readers: _types.toList(),
    clock: sender.clock,
  );
  return node.writeFile(records);
}

/// An AXT `Note` of [text], plain.
Map<String, Object?> _note(String text) => {
  'title': text.split('\n').first,
  'text': text,
  'format': 'PLAIN',
  'created': null,
  'updated': null,
  'tags': <Object?>[],
  'relations': <Object?>[],
  'extensions': <Object?>[],
};

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

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
    ben = device(clock, 'Ben');
  });

  Future<AxtFile> read(Uint8List bytes) =>
      AxtFiles(ben).read(bytes, wallClock: _utc);

  /// Ann's Note Dentist with a Reminder on 3 December at 10:00, and the
  /// Note Ben took of it.
  Future<({String ann, String ben})> takenDentist() async {
    final id = Notes(ann).capture('Dentist\nAt the new place').id;
    Routines(
      ann,
    ).spawn(id, Schedule(start: Day(2026, 12, 3), times: [ClockTime(10, 0)]));
    final file = await read(await AxtFiles(ann).write([id]));
    final taken = AxtFiles(ben).take(file, [file.taken.single.id]).single;
    return (ann: id, ben: taken);
  }

  Future<({String ann, String ben})> takenShopping() async {
    final id = Notes(ann).capture('Shopping\nMilk').id;
    final file = await read(await AxtFiles(ann).write([id]));
    final taken = AxtFiles(ben).take(file, [file.taken.single.id]).single;
    return (ann: id, ben: taken);
  }

  test('a later file of the same sender updates the Note taken, its text '
      'and its Reminder, and keeps the version it took', () async {
    final dentist = await takenDentist();
    final first = AxtFiles(ben).from(dentist.ben)!;
    Notes(ann).edit(dentist.ann, 'Dentist\nAt the old place');
    final reminder = Routines(ann).ofNote(dentist.ann).single;
    Routines(ann).reschedule(
      reminder.id,
      Schedule(start: Day(2026, 12, 4), times: [ClockTime(11, 30)]),
    );
    final later = await AxtFiles(ann).write([dentist.ann]);
    final file = await read(later);
    final record = file.records.single;
    expect(record.kind, AxtRecordKind.update);
    expect(record.kind.code, 'update');
    expect(record.noteId, dentist.ben);
    expect(record.kept, isFalse);
    expect(file.changes.single.id, record.id);
    expect(file.taken, isEmpty);
    expect(record.version, greaterThan(first.version));
    expect(AxtFiles(ben).take(file, [record.id]), isEmpty);
    expect(Notes(ben).byId(dentist.ben)!.text, 'Dentist\nAt the old place');
    final routine = Routines(ben).ofNote(dentist.ben).single;
    expect(routine.schedule.start, Day(2026, 12, 4));
    expect(routine.schedule.times, [ClockTime(11, 30)]);
    expect(AxtFiles(ben).from(dentist.ben)!.version, record.version);
    // The same file again, and the one before it, change nothing more.
    final before = ben.allEntries().length;
    expect((await read(later)).records.single.kind, AxtRecordKind.stale);
    final again = await read(later);
    AxtFiles(ben).take(again, [record.id]);
    expect(ben.allEntries().length, before);
  });

  test('taking the same file twice makes one Note', () async {
    final id = Notes(ann).capture('Milk').id;
    final bytes = await AxtFiles(ann).write([id]);
    final file = await read(bytes);
    final ids = [for (final r in file.records) r.id];
    expect(AxtFiles(ben).take(file, ids), hasLength(1));
    expect(AxtFiles(ben).take(file, ids), isEmpty);
    expect((await read(bytes)).records.single.kind, AxtRecordKind.stale);
    expect(Notes(ben).unsorted(), hasLength(1));
  });

  test('a Note the user changed since keeps the change, and stays the '
      "user's for every later update", () async {
    final shopping = await takenShopping();
    Notes(ben).edit(shopping.ben, 'Shopping\nMilk and bread');
    final version = AxtFiles(ben).from(shopping.ben)!.version;
    Notes(ann).edit(shopping.ann, 'Shopping\nMilk and eggs');
    final file = await read(await AxtFiles(ann).write([shopping.ann]));
    final record = file.records.single;
    expect(record.kind, AxtRecordKind.update);
    expect(record.kept, isTrue);
    final before = ben.allEntries().length;
    AxtFiles(ben).take(file, [record.id]);
    expect(ben.allEntries().length, before);
    expect(Notes(ben).byId(shopping.ben)!.text, 'Shopping\nMilk and bread');
    expect(AxtFiles(ben).from(shopping.ben)!.version, version);
    Notes(ann).edit(shopping.ann, 'Shopping\nMilk, eggs and tea');
    final later = await read(await AxtFiles(ann).write([shopping.ann]));
    expect(later.records.single.kept, isTrue);
  });

  test('a change of its Reminder is a change of the Note', () async {
    final dentist = await takenDentist();
    final routine = Routines(ben).ofNote(dentist.ben).single;
    Routines(ben).setHeadsUps(routine.id, [30]);
    Notes(ann).edit(dentist.ann, 'Dentist\nAt the old place');
    final file = await read(await AxtFiles(ann).write([dentist.ann]));
    expect(file.records.single.kept, isTrue);
    AxtFiles(ben).take(file, [file.records.single.id]);
    expect(Notes(ben).byId(dentist.ben)!.text, 'Dentist\nAt the new place');
  });

  test('a deletion of the same sender moves the Note to the trash with its '
      'Reminder, and a later version brings it back', () async {
    final dentist = await takenDentist();
    final recordId = axtRecordId(dentist.ann, AxtTypes.task);
    final file = await read(
      await _file(ann, [OutgoingRecord.deleted(recordId)]),
    );
    final record = file.records.single;
    expect(record.kind, AxtRecordKind.deletion);
    expect(record.noteId, dentist.ben);
    expect(record.kept, isFalse);
    expect(file.notTaken, isEmpty);
    AxtFiles(ben).take(file, [recordId]);
    expect(Notes(ben).byId(dentist.ben)!.isTrashed, isTrue);
    expect(Routines(ben).ofNote(dentist.ben), isEmpty);
    expect(AxtFiles(ben).from(dentist.ben)!.version, record.version);
    Notes(ann).edit(dentist.ann, 'Dentist\nOn Friday');
    final back = await read(await AxtFiles(ann).write([dentist.ann]));
    expect(back.records.single.kind, AxtRecordKind.update);
    expect(back.records.single.kept, isFalse);
    AxtFiles(ben).take(back, [recordId]);
    final note = Notes(ben).byId(dentist.ben)!;
    expect(note.isTrashed, isFalse);
    expect(note.text, 'Dentist\nOn Friday');
    expect(Routines(ben).ofNote(dentist.ben), hasLength(1));
  });

  test('a deletion of a Note the user changed since keeps it', () async {
    final shopping = await takenShopping();
    Notes(ben).edit(shopping.ben, 'Shopping\nMilk and bread');
    final recordId = axtRecordId(shopping.ann, AxtTypes.note);
    final file = await read(
      await _file(ann, [OutgoingRecord.deleted(recordId)]),
    );
    expect(file.records.single.kind, AxtRecordKind.deletion);
    expect(file.records.single.kept, isTrue);
    AxtFiles(ben).take(file, [recordId]);
    expect(Notes(ben).byId(shopping.ben)!.isTrashed, isFalse);
  });

  test('another sender can neither change nor delete a Note taken, and a '
      'record of another type under its id changes nothing', () async {
    final shopping = await takenShopping();
    final recordId = axtRecordId(shopping.ann, AxtTypes.note);
    final cy = device(clock, 'Cy');
    final forged = await read(
      await _file(cy, [
        OutgoingRecord(recordId, _type(AxtTypes.note), _note('Not yours')),
      ]),
    );
    expect(forged.records.single.kind, AxtRecordKind.refused);
    expect(forged.records.single.noteId, shopping.ben);
    expect(forged.taken, isEmpty);
    final deleting = await read(
      await _file(cy, [OutgoingRecord.deleted(recordId)]),
    );
    expect(deleting.records.single.kind, AxtRecordKind.refused);
    final retyped = await read(
      await _file(ann, [
        OutgoingRecord(recordId, _type(AxtTypes.task), {
          'title': 'Shopping',
          'notes': '',
          'due': null,
          'due_all_day': false,
          'repeat': null,
          'done': false,
          'done_at': null,
          'priority': 'NONE',
          'relations': <Object?>[],
          'extensions': <Object?>[],
        }),
      ]),
    );
    expect(retyped.records.single.kind, AxtRecordKind.refused);
    final before = ben.allEntries().length;
    for (final file in [forged, deleting, retyped]) {
      expect(AxtFiles(ben).take(file, [recordId]), isEmpty);
    }
    expect(ben.allEntries().length, before);
    expect(Notes(ben).byId(shopping.ben)!.text, 'Shopping\nMilk');
  });

  test("another person's Device can make no Note an update target, and "
      "its edit is no change of the user's", () async {
    final shopping = await takenShopping();
    final milk = Notes(ben).capture('Milk').id;
    final cy = device(clock, 'Cy');
    final from = AxtFiles(ben).from(shopping.ben)!;
    final recordId = axtRecordId(shopping.ann, AxtTypes.note);
    final other = Notes(ann).capture('Tea').id;
    final otherId = axtRecordId(other, AxtTypes.note);
    // Cy's Device names another record of Ann's on Ben's own Note and on
    // the Note Ben took, later, and edits the Note Ben took.
    final forged = AxtFrom(
      sender: from.sender,
      id: otherId,
      type: AxtTypes.note,
      version: 0,
    ).encode();
    ben
      ..insert(cy.append(milk, AxtKeys.from, forged))
      ..insert(cy.append(shopping.ben, AxtKeys.from, forged))
      ..insert(
        cy.append(
          shopping.ben,
          NoteKeys.text,
          StoredText.of(TextFormats.plain, 'Shopping\nCy was here').encode(),
        ),
      );
    Notes(ann).edit(shopping.ann, 'Shopping\nMilk and eggs');
    final file = await read(await AxtFiles(ann).write([shopping.ann, other]));
    final byId = {for (final r in file.records) r.id: r};
    expect(byId[otherId]!.kind, AxtRecordKind.newRecord);
    expect(byId[otherId]!.noteId, isNull);
    expect(byId[recordId]!.kind, AxtRecordKind.update);
    expect(byId[recordId]!.kept, isFalse);
    AxtFiles(ben).take(file, [recordId, otherId]);
    expect(Notes(ben).byId(milk)!.text, 'Milk');
    expect(Notes(ben).byId(shopping.ben)!.text, 'Shopping\nMilk and eggs');
  });

  test('a Note this Device may no longer change is refused before '
      'anything is written', () async {
    final house = Places(ann).addArea('Household');
    final area = ann.share(house.id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    final id = Notes(ann).capture('Milk').id;
    final file = await read(await AxtFiles(ann).write([id]));
    AxtFiles(ben).take(file, [file.records.single.id], place: area);
    ben.leaveArea(area);
    Notes(ann).edit(id, 'Milk and eggs');
    final later = await read(await AxtFiles(ann).write([id]));
    expect(later.records.single.kind, AxtRecordKind.update);
    final before = ben.allEntries().length;
    expect(
      () => AxtFiles(ben).take(later, [later.records.single.id]),
      throwsStateError,
    );
    expect(ben.allEntries().length, before);
  });
}

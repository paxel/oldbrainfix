/// Writes the fixture corpus the desktop core is tested against.
///
///     dart run tool/gen_fixtures.dart ../../desktop/fixtures
///
/// Every scenario fills an in-memory store with a fixed signing key, a
/// fixed clock and counted ids, and writes its log (`log.jsonl`, one
/// entry per line in log order) and what the core reads from it
/// (`expected.json`). The desktop core loads each log into its own store
/// and must read the same. Next to them: the store's schema, a signed
/// entry with awkward text, and the question-mark scatter.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:sqlite3/sqlite3.dart';

/// The writer's fixed key seed: bytes 1 to 32.
final _seed = Uint8List.fromList(List<int>.generate(32, (i) => 1 + i));

/// A clock that starts at a fixed moment and moves one second per read,
/// unless it is held still for reading the results.
late DateTime _tick;
DateTime? _held;
DateTime _clock() => _held ?? (_tick = _tick.add(const Duration(seconds: 1)));

typedef Scenario = ({String name, String about, void Function(Store) build});

final _scenarios = <Scenario>[
  (
    name: 'notes',
    about:
        'Notes of every Importance; a text fixed in place; an edit dated '
        'back that loses to the text before it; two texts half a '
        'millisecond apart written in reverse order; text with quotes, '
        'backslashes, control characters, umlauts and an emoji.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Buy more sauce');
      notes.setImportance(a.id, Importance.important);
      final b = notes.capture('nees mor sauce');
      notes.edit(b.id, 'Needs more sauce');
      final c = notes.capture('Maybe paint the shed');
      notes.setImportance(c.id, Importance.unimportant);
      notes.setImportance(c.id, Importance.normal);
      notes.setImportance(c.id, Importance.unimportant);
      final d = notes.capture('First text');
      store.append(
        d.id,
        NoteKeys.text,
        'Dated back',
        date: DateTime.utc(2025, 12, 31),
      );
      final e = notes.capture('Sub-millisecond');
      store.append(
        e.id,
        NoteKeys.text,
        'Half a millisecond later',
        date: DateTime.utc(2026, 1, 2, 9, 0, 0, 0, 500),
      );
      store.append(
        e.id,
        NoteKeys.text,
        'Earlier, written later',
        date: DateTime.utc(2026, 1, 2, 9),
      );
      notes.capture(_awkward);
    },
  ),
  (
    name: 'sections',
    about:
        'Amends; a chain of Merges where the merged Note had an Amend of '
        'its own; two Notes merged into each other, kept under the lower '
        'id; a Merge into a Note this Device does not hold.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Dentist on Tuesday');
      notes.amend(a.id, 'Moved to Wednesday');
      final b = notes.capture('Ask about the crown');
      notes.amend(b.id, 'Bring the old X-ray');
      final c = notes.capture('Dentist number');
      notes.merge(b.id, a.id);
      notes.merge(a.id, c.id);
      notes.amend(c.id, 'Called, all set');
      final x = notes.capture('Loop one');
      final y = notes.capture('Loop two');
      store.append(y.id, NoteKeys.merged, x.id);
      store.append(x.id, NoteKeys.merged, y.id);
      final z = notes.capture('Merged into a stranger');
      store.append(z.id, NoteKeys.merged, 'note:elsewhere');
    },
  ),
  (
    name: 'places',
    about:
        'The whole Starter Set planted, one starter Area renamed; an Area '
        'of the user with a colour and an icon and a Topic; Notes filed '
        'into Areas and Topics; a Note whose Topic vanished falls back to '
        'Unsorted; a deleted Area takes its Topics with it.',
    build: (store) {
      final places = Places(store);
      final notes = Notes(store);
      places.plantStarterSet({
        for (final area in starterSet) ...{
          area.slug,
          for (final topic in area.topics) topic.slug,
        },
      });
      final car = places.areas().firstWhere((a) => a.starter == 'car');
      places.rename(car.id, 'Bike');
      final kids = places.addArea('Kids');
      places.setColour(kids.id, '6A1B9A');
      places.setIcon(kids.id, 'kids');
      final school = places.addTopic(kids.id, 'School');
      final household = places.areas().firstWhere(
        (a) => a.starter == 'household',
      );
      final kitchen = places
          .topics(household.id)
          .firstWhere((t) => t.starter == 'kitchen');
      notes.capture('Descale the kettle', place: kitchen.id);
      notes.capture('Sort the cellar', place: household.id);
      notes.capture('Parents evening', place: school.id);
      notes.capture('New tyres', place: car.id);
      final lost = notes.capture('Where did this go?', place: school.id);
      store.append(school.id, PlaceKeys.type, null);
      notes.capture('Still Unsorted');
      final gone = places.addArea('Gone');
      places.addTopic(gone.id, 'Gone too');
      places.deleteArea(gone.id, NotesGo.unsorted);
      notes.setImportance(lost.id, Importance.important);
    },
  ),
  (
    name: 'trash',
    about:
        'A Note in the trash; one trashed and restored; one trashed long '
        'enough ago to be past restoring; a merged Note whose target is '
        'in the trash.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Old shopping list');
      notes.trash(a.id);
      final b = notes.capture('Not so old after all');
      notes.trash(b.id);
      notes.restore(b.id);
      final c = notes.capture('Long gone');
      store.append(
        c.id,
        NoteKeys.trashed,
        DateTime.utc(2025, 11, 1).toIso8601String(),
      );
      final d = notes.capture('Into the bin with the other');
      final e = notes.capture('The other');
      notes.merge(d.id, e.id);
      notes.trash(e.id);
    },
  ),
  (
    name: 'edits',
    about:
        'What the desktop writes beyond the other scenarios: Areas and '
        'Topics made and renamed, an Area coloured and given an icon, '
        'another one with both cleared again; Notes captured with each '
        'Nudge time, some into a place; Notes filed and filed again; Topics '
        'deleted with their Notes sent back to Unsorted (one that spawned a '
        'Routine included), into the Area and into the trash, and an empty '
        'one; Areas deleted with their Topics, their Notes sent into the '
        'trash and back to Unsorted.',
    build: (store) {
      final notes = Notes(store);
      final places = Places(store);
      final home = places.addArea('Hom');
      places.rename(home.id, 'Home');
      places.setColour(home.id, '00838F');
      places.setIcon(home.id, 'home');
      final post = places.addTopic(home.id, 'Post');
      final bills = places.addTopic(home.id, 'Bils');
      places.rename(bills.id, 'Bills');
      final junk = places.addTopic(home.id, 'Junk');
      final empty = places.addTopic(home.id, 'Empty');
      final errands = places.addArea('Errands');
      places.setColour(errands.id, 'C62828');
      places.setIcon(errands.id, 'shopping');
      places.setColour(errands.id, null);
      places.setIcon(errands.id, null);
      final shop = places.addTopic(errands.id, 'Shop');
      final garage = places.addArea('Garage');
      final tools = places.addTopic(garage.id, 'Tools');
      final plumber = notes.capture(
        'Call the plumber',
        nudgeIn: const Duration(minutes: 10),
      );
      final plants = notes.capture(
        'Water the plants',
        nudgeIn: const Duration(minutes: 30),
      );
      final passport = notes.capture(
        'Renew the passport',
        nudgeIn: Notes.defaultNudge,
      );
      final hour = Notes.defaultNudge;
      notes.capture('Pay the rent', place: bills.id, nudgeIn: hour);
      notes.capture('Old flyer', place: junk.id, nudgeIn: hour);
      notes.capture('Buy stamps', place: shop.id, nudgeIn: hour);
      notes.capture('Return the drill', place: errands.id, nudgeIn: hour);
      notes.capture('Oil the saw', place: tools.id, nudgeIn: hour);
      notes.capture('Sweep the floor', place: garage.id, nudgeIn: hour);
      notes.file(plumber.id, post.id);
      notes.file(plumber.id, bills.id);
      notes.file(plants.id, post.id);
      notes.file(passport.id, post.id);
      // A Routine was spawned from it on another Device.
      store.append(
        passport.id,
        NoteKeys.spawned,
        DateTime.utc(2026, 1, 1, 12).toIso8601String(),
      );
      places.deleteTopic(post.id, NotesGo.unsorted);
      places.deleteTopic(bills.id, NotesGo.area);
      places.deleteTopic(junk.id, NotesGo.trash);
      places.deleteTopic(empty.id, NotesGo.unsorted);
      places.deleteArea(errands.id, NotesGo.trash);
      places.deleteArea(garage.id, NotesGo.unsorted);
    },
  ),
];

/// A value that tests how the signed bytes escape text.
const _awkward =
    'Quote " backslash \\ slash / newline \n tab \t bell \u0001 '
    'umlaut ü emoji 🧠';

Map<String, Object?> _note(Note note) => {
  'id': note.id,
  'text': note.text,
  'captured': note.captured.toUtc().toIso8601String(),
  'place': note.place,
  'importance': note.importance.name,
  'trashed': note.trashed?.toUtc().toIso8601String(),
  'mergedInto': note.mergedInto,
  'spawned': note.spawned,
};

Map<String, Object?> _section(Section section) => {
  'id': section.id,
  'noteId': section.noteId,
  'text': section.text,
  'captured': section.context.captured.toUtc().toIso8601String(),
};

/// What the core reads from [store]: the desktop core must read the same.
Map<String, Object?> _expected(Store store) {
  final notes = Notes(store);
  final places = Places(store);
  final noteIds = store.entitiesOfType(NoteKeys.typeNote)..sort();
  final all = notes.all();
  final trashed = notes.trashed();
  final placeIds = [
    for (final area in places.areas()) ...[
      area.id,
      for (final topic in places.topics(area.id)) topic.id,
    ],
  ];
  return {
    'now': store.clock().toIso8601String(),
    'device': store.deviceId,
    'areas': [
      for (final area in places.areas())
        {
          'id': area.id,
          'name': area.name,
          'starter': area.starter,
          'colour': area.colour,
          'icon': area.icon,
          'topics': [
            for (final topic in places.topics(area.id))
              {
                'id': topic.id,
                'areaId': topic.areaId,
                'name': topic.name,
                'starter': topic.starter,
              },
          ],
        },
    ],
    'notes': [
      for (final id in noteIds)
        if (notes.byId(id) case final note?) _note(note),
    ],
    'all': [for (final note in all) note.id],
    'unsorted': [for (final note in notes.unsorted()) note.id],
    'trashed': [for (final note in trashed) note.id],
    'inside': {
      for (final id in placeIds) id: [for (final n in notes.inside(id)) n.id],
    },
    'sections': {
      for (final note in [...all, ...trashed])
        note.id: [for (final s in notes.sections(note.id)) _section(s)],
    },
  };
}

Store _store() {
  _tick = DateTime.utc(2026, 1, 1, 10);
  _held = null;
  var ids = 0;
  Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
  final store = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_seed),
  );
  store.setLocalSetting('author', 'Ada');
  return store;
}

const _json = JsonEncoder.withIndent('  ');

void _write(File file, String text) {
  file.parent.createSync(recursive: true);
  file.writeAsStringSync(text);
}

/// The store's schema, as SQLite keeps it, in a stable order.
String _schema(Directory work) {
  final path = '${work.path}/schema.db';
  Store.open(path, newKey: () => SigningKey.fromSeed(_seed)).close();
  final db = sqlite3.open(path);
  try {
    final rows = db.select(
      "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name",
    );
    return '${[for (final row in rows) '${row['sql']};'].join('\n')}\n';
  } finally {
    db.close();
  }
}

/// One entry with awkward text, signed by the writer's key, with the
/// exact bytes the signature covers.
Map<String, Object?> _signing() {
  final key = SigningKey.fromSeed(_seed);
  const date = '2026-01-01T10:00:00.000Z';
  const recorded = '2026-01-01T10:00:00.123456Z';
  final device = deviceIdFromKey(key.publicKey);
  final bytes = entryBytes(
    device: device,
    dseq: 7,
    entity: 'note:0001',
    field: 'text',
    value: _awkward,
    dateIso: date,
    author: 'Ada "the writer"',
    recordedIso: recorded,
  );
  final nullBytes = entryBytes(
    device: device,
    dseq: 8,
    entity: 'note:0001',
    field: 'trashed',
    value: null,
    dateIso: date,
    author: '',
    recordedIso: recorded,
  );
  return {
    'seed': key.seedHex,
    'publicKey': key.publicBase64,
    'device': device,
    'code': key.code,
    'entries': [
      {
        'device': device,
        'dseq': 7,
        'entity': 'note:0001',
        'field': 'text',
        'value': _awkward,
        'date': date,
        'author': 'Ada "the writer"',
        'recorded': recorded,
        'bytes': base64.encode(bytes),
        'sig': key.sign(bytes),
      },
      {
        'device': device,
        'dseq': 8,
        'entity': 'note:0001',
        'field': 'trashed',
        'value': null,
        'date': date,
        'author': '',
        'recorded': recorded,
        'bytes': base64.encode(nullBytes),
        'sig': key.sign(nullBytes),
      },
    ],
  };
}

/// The marks over a region around the origin, negative cells included.
Map<String, Object?> _questionMarks() => {
  'shapes': [
    for (final shape in questionMarkShapes)
      {'strokes': shape.strokes, 'dot': shape.dot, 'weight': shape.weight},
  ],
  'seed': questionMarkSeed,
  'region': [-400, -300, 700, 500],
  'marks': [
    for (final m in questionMarksIn(-400, -300, 700, 500))
      {
        'shape': m.shape,
        'x': m.x,
        'y': m.y,
        'size': m.size,
        'rotation': m.rotation,
      },
  ],
  'otherSeed': [
    for (final m in questionMarksIn(0, 0, 300, 300, seed: 1))
      {
        'shape': m.shape,
        'x': m.x,
        'y': m.y,
        'size': m.size,
        'rotation': m.rotation,
      },
  ],
};

void main(List<String> args) {
  if (args.length != 1) {
    stderr.writeln('usage: dart run tool/gen_fixtures.dart <fixtures dir>');
    exit(2);
  }
  final root = Directory(args[0])..createSync(recursive: true);
  // A scenario dropped from the list leaves no folder behind.
  for (final old in root.listSync().whereType<Directory>()) {
    old.deleteSync(recursive: true);
  }
  final index = StringBuffer(
    '# Fixture corpus\n\n'
    'Generated by `packages/oldbrainfix_core/tool/gen_fixtures.dart` from '
    'the Dart core; the desktop core loads every `log.jsonl` and must read '
    'the same `expected.json`. Do not edit by hand.\n\n'
    '- `schema.sql`: the tables and indexes of the store.\n'
    '- `signing.json`: entries signed by the writer key, with the bytes '
    'the signatures cover.\n'
    '- `question_marks.json`: the background\'s shapes and their scatter '
    'over a region.\n',
  );
  final work = Directory.systemTemp.createTempSync('oldbrainfix_fixture_');
  try {
    _write(File('${root.path}/schema.sql'), _schema(work));
  } finally {
    work.deleteSync(recursive: true);
  }
  _write(File('${root.path}/signing.json'), '${_json.convert(_signing())}\n');
  _write(
    File('${root.path}/question_marks.json'),
    '${_json.convert(_questionMarks())}\n',
  );
  for (final scenario in _scenarios) {
    final store = _store();
    try {
      scenario.build(store);
      final out = Directory('${root.path}/${scenario.name}');
      final log = StringBuffer();
      for (final entry in store.allEntries()) {
        log.writeln(jsonEncode(entry.toJson()));
      }
      _write(File('${out.path}/log.jsonl'), log.toString());
      _held = _tick.add(const Duration(days: 1));
      _write(
        File('${out.path}/expected.json'),
        '${_json.convert(_expected(store))}\n',
      );
      index.writeln('- `${scenario.name}`: ${scenario.about}');
    } finally {
      store.close();
    }
  }
  _write(File('${root.path}/README.md'), index.toString());
  stdout.writeln('Wrote ${_scenarios.length} scenarios to ${root.path}');
}

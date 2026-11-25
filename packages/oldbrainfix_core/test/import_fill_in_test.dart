import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// Importing the same source again adds only what is missing (obf-1078,
/// ADR 0036).
void main() {
  late Store store;
  late Imports imports;
  late Notes notes;
  late Places places;
  final march = DateTime.utc(2025, 3, 1, 8);

  setUp(() {
    store = Store.inMemory(clock: ticking());
    imports = Imports(store);
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  ImportFile file(String name, String text) =>
      ImportFile(name, march, Uint8List.fromList(utf8.encode(text)));

  List<ImportFile> folder() => [
    file('Shopping.md', '# Shopping\nmilk'),
    file(r'sub\Dentist.txt', 'Dentist\nTuesday'),
    file('Ideas.txt', 'Ideas\nmore'),
  ];

  String? source(String id) => store.current(id, ImportKeys.source);

  Note withPath(String areaId, String path) => notes
      .inside(areaId)
      .firstWhere((n) => store.current(n.id, ImportKeys.source) == path);

  group('an Import keeps where its Notes came from', () {
    test('each Note its path and the Import, the Area what was picked', () {
      final s = imports.folder('From Notes', folder(), from: '/home/a/Notes');
      final area = s.areaId!;
      final inside = notes.inside(area);
      expect(
        {for (final n in inside) source(n.id)},
        {'Shopping.md', 'sub/Dentist.txt', 'Ideas.txt'},
      );
      for (final n in inside) {
        expect(store.current(n.id, ImportKeys.importArea), area);
      }
      final id = Imports.sourceId(ImportSourceKind.folder, '/home/a/Notes');
      expect(store.current(area, ImportKeys.importSource), id);
      // No path lies in the log as written.
      expect(id, hasLength(64));
      expect(id, isNot(contains('Notes')));
      expect(
        id,
        isNot(Imports.sourceId(ImportSourceKind.keep, '/home/a/Notes')),
      );
    });

    test('is read for a Note, none for one no Import made', () {
      final s = imports.folder('From Notes', folder(), from: '/n');
      final dentist = withPath(s.areaId!, 'sub/Dentist.txt').id;
      expect(imports.sourceOf(dentist), (
        path: 'sub/Dentist.txt',
        areaId: s.areaId!,
      ));
      expect(imports.sourceOf(notes.capture('Own').id), isNull);
    });

    test('without a source picked the Area keeps none', () {
      final s = imports.folder('From Notes', folder());
      expect(store.current(s.areaId!, ImportKeys.importSource), isNull);
      expect(
        imports.earlier(ImportSourceKind.folder, '', ['Shopping.md']),
        isNull,
      );
    });
  });

  group('the earlier Import', () {
    test('is found by what was picked and its paths, never by a name', () {
      final s = imports.folder('From Notes', folder(), from: '/n');
      final at = store.loggedHistory(s.areaId!).last.date;
      final found = imports.earlier(ImportSourceKind.folder, '/n', [
        'Shopping.md',
        r'sub\Dentist.txt',
        'New.md',
      ]);
      expect(found?.areaId, s.areaId);
      expect(found?.areaName, 'From Notes');
      expect(found?.at, at);
      expect(found?.held, 2);
      final all = [for (final f in folder()) f.name];
      // Another folder of the same names, the same folder read as a Keep
      // export, and paths none of its Notes came from: none of them.
      expect(imports.earlier(ImportSourceKind.folder, '/m', all), isNull);
      expect(imports.earlier(ImportSourceKind.keep, '/n', all), isNull);
      expect(
        imports.earlier(ImportSourceKind.folder, '/n', ['New.md']),
        isNull,
      );
    });

    test('is the newer one once the first was not filled in', () {
      final first = imports.folder('From Notes', folder(), from: '/n');
      final second = imports.folder('From Notes', folder(), from: '/n');
      expect(second.areaId, isNot(first.areaId));
      expect(second.notes, 3);
      expect(places.areaById(second.areaId!)?.name, 'From Notes (2)');
      final found = imports.earlier(ImportSourceKind.folder, '/n', [
        'Ideas.txt',
      ]);
      expect(found?.areaId, second.areaId);
    });

    test('is none once its Area is deleted, and an Import makes a new one', () {
      final first = imports.folder('From Notes', folder(), from: '/n');
      places.deleteArea(first.areaId!, NotesGo.unsorted);
      expect(
        imports.earlier(ImportSourceKind.folder, '/n', ['Ideas.txt']),
        isNull,
      );
      final again = imports.folder(
        'From Notes',
        folder(),
        from: '/n',
        fillIn: first.areaId,
      );
      expect(again.areaId, isNot(first.areaId));
      expect(again.notes, 3);
      expect(again.already, 0);
    });
  });

  group('filling in', () {
    test('makes only what was never made, into the first Area', () {
      final first = imports.folder('From Notes', folder(), from: '/n');
      final area = first.areaId!;
      final health = places.addArea('Health').id;
      notes.file(withPath(area, 'sub/Dentist.txt').id, health);
      notes.trash(withPath(area, 'Ideas.txt').id);
      final filled = imports.folder(
        'From Notes',
        [
          file('Shopping.md', '# Shopping\nmilk and bread'),
          file('sub/Dentist.txt', 'Dentist\nTuesday'),
          file('Ideas.txt', 'Ideas\nmore'),
          file('New.md', 'New'),
          file(r'sub\Later.txt', 'Later'),
        ],
        from: '/n',
        fillIn: area,
      );
      expect(filled.areaId, area);
      expect(filled.notes, 2);
      expect(filled.already, 3);
      expect(filled.skipped, 0);
      expect(places.areas().map((a) => a.name), ['From Notes', 'Health']);
      expect(
        {for (final n in notes.inside(area)) source(n.id)},
        {'Shopping.md', 'New.md', 'sub/Later.txt'},
      );
      // Changed since, it stays as it was made.
      expect(withPath(area, 'Shopping.md').text, '# Shopping\nmilk');
      expect(
        store.current(withPath(area, 'New.md').id, ImportKeys.importArea),
        area,
      );
    });

    test('a Google Keep export stopped is filled in to all its notes', () {
      ImportFile note(String name, String title) => ImportFile(
        'Takeout/Keep/$name.json',
        null,
        Uint8List.fromList(
          utf8.encode(
            jsonEncode({
              'title': title,
              'textContent': 'text',
              'createdTimestampUsec': 1700000000000000,
            }),
          ),
        ),
      );
      final export = [note('a', 'A'), note('b', 'B'), note('c', 'C')];
      var asked = 0;
      final first = imports.keep(
        'From Google Keep',
        export,
        from: 'content://zip',
        // Before the Area and the first Note, then stopped.
        stop: () => asked++ >= 2,
      );
      expect(first.notes, 1);
      expect(first.stopped, 3);
      final found = imports.earlier(ImportSourceKind.keep, 'content://zip', [
        for (final e in export) e.name,
      ]);
      expect(found?.areaId, first.areaId);
      expect(found?.held, 1);
      final filled = imports.keep(
        'From Google Keep',
        export,
        from: 'content://zip',
        fillIn: found?.areaId,
      );
      expect(filled.areaId, first.areaId);
      expect(filled.notes, 2);
      expect(filled.already, 1);
      expect(notes.inside(first.areaId!), hasLength(3));
    });

    test('an Area of another source gets nothing: a new Area takes all', () {
      final first = imports.folder('From Notes', folder(), from: '/n');
      for (final (from, kind) in [('/m', 'folder'), (null, 'none')]) {
        final other = imports.folder(
          'From Notes',
          folder(),
          from: from,
          fillIn: first.areaId,
        );
        expect(other.areaId, isNot(first.areaId), reason: kind);
        expect(other.notes, 3, reason: kind);
        expect(other.already, 0, reason: kind);
      }
      final keep = imports.keep(
        'From Google Keep',
        const [],
        from: '/n',
        fillIn: first.areaId,
      );
      expect(keep.areaId, isNull);
    });

    test('reads as often for a few Notes as for many', () {
      ({int logged, int history}) reads(int count) {
        final s = Store.inMemory(clock: ticking());
        addTearDown(s.close);
        final made = [
          for (var i = 0; i < count; i++) file('n$i.txt', 'Note $i'),
        ];
        final first = Imports(s).folder('From Notes', made, from: '/n');
        s.loggedReads = 0;
        s.historyReads = 0;
        final paths = [for (final f in made) f.name, 'new.txt'];
        final found = Imports(s).earlier(ImportSourceKind.folder, '/n', paths);
        expect(found?.held, count);
        final filled = Imports(s).folder(
          'From Notes',
          [...made, file('new.txt', 'New')],
          from: '/n',
          fillIn: first.areaId,
        );
        expect(filled.notes, 1);
        expect(filled.already, count);
        return (logged: s.loggedReads, history: s.historyReads);
      }

      expect(reads(60), reads(3));
    });
  });

  group('only what the user\'s own Devices wrote counts', () {
    late DateTime Function() clock;
    late Store ann;
    late Store ben;

    setUp(() {
      clock = ticking();
      ann = device(clock, 'Ann');
      ben = device(clock, 'Ben');
    });

    test("a path another person's Device wrote on a Note counts for "
        'nothing', () {
      final s = Imports(ann)
          .folder('From Notes', [file('a.md', 'a')], from: '/n');
      final area = ann.share(s.areaId!);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      // As a modified app would: Ben slips a Note of that Import, of a
      // path still to come, into the Area.
      final forged = Notes(ben).capture('b', place: area).id;
      ben.append(forged, ImportKeys.importArea, area);
      ben.append(forged, ImportKeys.source, 'b.md');
      exchange(ben, ann);
      expect(ann.current(forged, ImportKeys.source), 'b.md');
      final imports = Imports(ann);
      final found = imports.earlier(ImportSourceKind.folder, '/n', [
        'a.md',
        'b.md',
      ]);
      expect(found?.held, 1);
      final filled = imports.folder(
        'From Notes',
        [file('a.md', 'a'), file('b.md', 'b')],
        from: '/n',
        fillIn: area,
      );
      expect(filled.notes, 1);
      expect(filled.already, 1);
    });

    test("what was picked as another person's Device wrote it on an Area "
        'counts for nothing', () {
      final s = Imports(ann)
          .folder('From Notes', [file('a.md', 'a')], from: '/n');
      final area = ann.share(s.areaId!);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.manager));
      // Ben, a Manager there, says the Area holds another source of Ann's.
      final other = Imports.sourceId(ImportSourceKind.folder, '/other');
      ben.append(area, ImportKeys.importSource, other);
      exchange(ben, ann);
      expect(ann.current(area, ImportKeys.importSource), other);
      final imports = Imports(ann);
      expect(
        imports.earlier(ImportSourceKind.folder, '/other', ['a.md']),
        isNull,
      );
      expect(
        imports.earlier(ImportSourceKind.folder, '/n', ['a.md'])?.areaId,
        area,
      );
      final filled = imports.folder(
        'From Notes',
        [file('a.md', 'a')],
        from: '/other',
        fillIn: area,
      );
      expect(filled.areaId, isNot(area));
      expect(filled.notes, 1);
    });
  });
}

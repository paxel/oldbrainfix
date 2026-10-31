import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  late Store store;
  late Imports imports;
  late Notes notes;
  late Places places;
  final now = DateTime.utc(2026, 10, 31, 12);

  setUp(() {
    store = Store.inMemory(clock: () => now);
    imports = Imports(store);
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  ImportFile file(String name, String text, [DateTime? modified]) =>
      ImportFile(name, modified, Uint8List.fromList(utf8.encode(text)));

  ImportFile raw(String name, List<int> bytes, [DateTime? modified]) =>
      ImportFile(name, modified, Uint8List.fromList(bytes));

  final march = DateTime.utc(2025, 3, 1, 8);
  final may = DateTime.utc(2025, 5, 2, 9, 30, 0, 123);

  group('a folder', () {
    test('becomes Notes in a new Area, dated by their files', () {
      final summary = imports.folder('From Notes', [
        file('Later.txt', 'Later\nsecond line', may),
        file('Earlier.md', '# Earlier\n**bold**', march),
        file('photo.png', 'not text', march),
      ]);
      expect(summary.notes, 2);
      expect(summary.skipped, 1);
      expect(summary.notText, 1);
      final area = places.areaById(summary.areaId!)!;
      expect(area.name, 'From Notes');
      final inside = notes.inside(area.id);
      expect(inside.map((n) => n.text), [
        'Later\nsecond line',
        '# Earlier\n**bold**',
      ]);
      expect(inside.map((n) => n.captured), [may, march]);
      expect(inside.map((n) => n.textFormat), [
        TextFormats.plain,
        TextFormats.markdown,
      ]);
      // Written oldest first, every entry of a Note dated by its file.
      final earlier = inside.last.id;
      expect(
        store
            .allEntries()
            .where((e) => e.entity.startsWith('note:'))
            .first
            .entity,
        earlier,
      );
      expect(store.history(earlier).map((e) => e.date).toSet(), {march});
    });

    test('nothing of it is Unsorted or Nudged', () {
      final summary = imports.folder('From Notes', [
        file('a.txt', 'a', march),
        file('b.md', 'b', may),
      ]);
      expect(notes.unsorted(), isEmpty);
      for (final note in notes.inside(summary.areaId!)) {
        expect(note.place, summary.areaId);
        expect(store.current(note.id, NoteKeys.nudge), isNull);
      }
    });

    test('lands beside an Area of the same name, not in it', () {
      final mine = places.addArea('From Notes');
      final first = imports.folder(' From Notes ', [file('a.txt', 'a')]);
      final second = imports.folder('From Notes', [file('b.txt', 'b')]);
      expect(places.areaById(first.areaId!)!.name, 'From Notes (2)');
      expect(places.areaById(second.areaId!)!.name, 'From Notes (3)');
      expect(notes.inside(mine.id), isEmpty);
      expect(
        () => imports.folder('  ', [file('a.txt', 'a')]),
        throwsArgumentError,
      );
    });

    test('with nothing to import makes no Area and writes nothing', () {
      for (final files in [
        <ImportFile>[],
        [file('photo.jpg', 'x'), file('.hidden.md', 'x'), file('README', 'x')],
      ]) {
        final summary = imports.folder('From Notes', files);
        expect(summary.areaId, isNull);
        expect(summary.notes, 0);
        expect(summary.notText, files.length);
      }
      expect(store.allEntries(), isEmpty);
    });

    test('orders Notes of one date by name, as code points', () {
      final summary = imports.folder('From Notes', [
        file('😀.txt', 'emoji', march),
        file('～.txt', 'tilde', march),
        file('b.txt', 'b', march),
        file('a.txt', 'a', march),
      ]);
      final ids = [
        for (final e in store.allEntries())
          if (e.field == NoteKeys.type && e.entity.startsWith('note:'))
            e.entity,
      ];
      expect(ids.map((id) => notes.byId(id)!.text.split('\n').last), [
        'a',
        'b',
        'tilde',
        'emoji',
      ]);
      expect(summary.notes, 4);
    });

    test('dates a file of unknown, zero or later time now', () {
      final summary = imports.folder('From Notes', [
        file('none.txt', 'none'),
        file('zero.txt', 'zero', DateTime.utc(1970)),
        file('later.txt', 'later', DateTime.utc(2030)),
      ]);
      expect(notes.inside(summary.areaId!).map((n) => n.captured).toSet(), {
        now,
      });
    });
  });

  group('a file', () {
    String only(ImportFile f) {
      final summary = imports.folder('From Notes', [f]);
      expect(summary.notes, 1, reason: f.name);
      return notes.inside(summary.areaId!).single.shownText;
    }

    test('loses its byte order mark, and its line breaks become \\n', () {
      final bom = [0xef, 0xbb, 0xbf, ...utf8.encode('Shop\r\nmilk\reggs\r\n')];
      expect(only(raw('Shop.txt', bom)), 'Shop\nmilk\neggs');
      final twice = [0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf, ...utf8.encode('Two')];
      expect(only(raw('Two.txt', twice)), '\uFEFFTwo');
    });

    test('that is not UTF-8 reads as Windows-1252', () {
      final bytes = [
        ...utf8.encode('Herr M'),
        0xfc,
        ...utf8.encode('ller '),
        0x80,
        0x81,
      ];
      expect(only(raw('Bill.txt', bytes)), 'Bill\nHerr Müller €\u0081');
    });

    test('holding a zero byte, or hidden, is no text', () {
      final summary = imports.folder('From Notes', [
        raw('wide.txt', [0xff, 0xfe, 0x41, 0x00]),
        file('._mac.md', 'x'),
      ]);
      expect(summary.notText, 2);
      expect(summary.areaId, isNull);
    });

    test('is taken by its extension in any case, its folders left out', () {
      final summary = imports.folder('From Notes', [
        file('A.MD', 'alpha'),
        file('b.Markdown', 'beta'),
        file('sub/c.TXT', 'gamma'),
        file(r'win\d.txt', 'delta'),
        file('e.text', 'epsilon'),
        file('f.mar\u212Adown', 'kelvin'),
      ]);
      expect(summary.notes, 4);
      expect(summary.notText, 2);
      expect(notes.inside(summary.areaId!).map((n) => n.shownText).toSet(), {
        '# A\nalpha',
        '# b\nbeta',
        'c\ngamma',
        'd\ndelta',
      });
    });

    test('is titled by its name unless its first line says so', () {
      expect(
        only(file('Dentist.txt', 'Tuesday 9:00')),
        'Dentist\nTuesday 9:00',
      );
      expect(
        only(file('dentist.txt', '\n  \nDENTIST\nTuesday\n\n')),
        'DENTIST\nTuesday',
      );
      expect(only(file('Shop.md', '## *Shop*\n- milk')), '## *Shop*\n- milk');
      expect(
        only(file('2*3 [x].md', '- a')),
        r'# 2\*3 \[x]'
        '\n- a',
      );
      expect(only(file('Call the bank.md', '')), '# Call the bank');
      expect(only(file('  .txt', '  kept  ')), '  kept');
    });

    test('with nothing in it and its name is no text', () {
      final summary = imports.folder('From Notes', [file('  .txt', ' \n ')]);
      expect(summary.notText, 1);
      expect(summary.areaId, isNull);
    });

    test('puts in its boxes', () {
      final summary = imports.folder('From Notes', [
        file('Trip.md', '# Trip\n- ☐ passport'),
      ]);
      final note = notes.inside(summary.areaId!).single;
      expect(note.boxes, hasLength(1));
      expect(note.shownText, '# Trip\n- ☐ passport');
    });
  });

  group('sizes', () {
    test('a file or text too large is skipped', () {
      final summary = imports.folder('From Notes', [
        raw('big.txt', List.filled(Imports.maxFileBytes + 1, 0x61)),
        // Under the file's limit, but each quote stored as two bytes.
        file('quotes.txt', '"' * (Imports.maxTextBytes ~/ 2 + 1)),
        file('fits.txt', 'x' * (Imports.maxTextBytes - 100)),
      ]);
      expect(summary.tooLarge, 2);
      expect(summary.notes, 1);
    });

    test('beyond the total, the oldest files are skipped', () {
      final text = 'a' * 256000;
      final files = [
        for (var i = 0; i < 132; i++)
          file('$i.txt', text, march.add(Duration(minutes: i))),
      ];
      final summary = imports.folder('From Notes', files);
      expect(summary.notes, 131);
      expect(summary.tooLarge, 1);
      final dates = notes.inside(summary.areaId!).map((n) => n.captured);
      expect(dates, isNot(contains(march)));
      expect(dates.first, march.add(const Duration(minutes: 131)));
    });
  });

  test('takes only formats known here', () {
    expect(
      () => imports.notes('From Keep', [
        ImportedNote(name: 'a', text: 'a', format: 'x-future', captured: now),
      ]),
      throwsArgumentError,
    );
  });
}

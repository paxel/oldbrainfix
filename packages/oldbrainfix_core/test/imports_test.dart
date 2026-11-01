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

  group('Google Keep', () {
    final made = DateTime.utc(2024, 3, 1, 8, 0, 0, 0, 250);
    final changed = DateTime.utc(2024, 6, 2, 9, 30);
    final png = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2];

    /// A Keep note's `.json` at `Takeout/Keep/<name>.json`.
    ImportFile note(String name, Map<String, Object?> json) =>
        file('Takeout/Keep/$name.json', jsonEncode(json));

    Map<String, Object?> times([DateTime? created, DateTime? edited]) => {
      'createdTimestampUsec': (created ?? made).microsecondsSinceEpoch,
      'userEditedTimestampUsec': (edited ?? changed).microsecondsSinceEpoch,
    };

    Note only(ImportSummary summary) => notes.inside(summary.areaId!).single;

    test('a note keeps its title, text and date, its labels as tags', () {
      final summary = imports.keep('From Google Keep', [
        note('Dentist', {
          ...times(),
          'title': 'Dentist',
          'textContent': 'Tuesday 9:00\r\nbring the X-ray #errands',
          'textContentHtml': '<p>ignored</p>',
          'isPinned': true,
          'color': 'RED',
          'labels': [
            {'name': 'Errands'},
            {'name': 'Home Garden'},
            {'name': 'home-garden'},
            {'name': '2024'},
            {'name': '🛒'},
            {'nom': 'x'},
          ],
        }),
      ]);
      expect(summary.notes, 1);
      expect(summary.skipped, 0);
      expect(places.areaById(summary.areaId!)!.name, 'From Google Keep');
      final dentist = only(summary);
      expect(
        dentist.text,
        'Dentist\nTuesday 9:00\nbring the X-ray #errands\n#Home-Garden #2024',
      );
      expect(dentist.textFormat, TextFormats.plain);
      expect(dentist.captured, made);
      expect(store.history(dentist.id).map((e) => e.date).toSet(), {made});
      expect(notes.unsorted(), isEmpty);
      expect(store.current(dentist.id, NoteKeys.nudge), isNull);
    });

    test('a list becomes a Checklist, its Lines checked when the note '
        'last changed', () {
      final summary = imports.keep('From Google Keep', [
        note('Groceries', {
          ...times(),
          'title': 'Groceries',
          'textContent': '',
          'listContent': [
            {'text': 'Milk', 'isChecked': true},
            {'text': ' Eggs\nlarge ', 'isChecked': false},
            {'text': '  ', 'isChecked': true},
            {'text': 'Bread', 'isChecked': 'yes'},
            {'isChecked': true},
          ],
          'labels': [
            {'name': 'shopping'},
          ],
        }),
      ]);
      expect(summary.checklists, 1);
      final groceries = only(summary);
      expect(groceries.text, 'Groceries\n#shopping');
      final list = Checklists(store).byNote(groceries.id)!;
      expect(list.lines.map((l) => (l.text, l.checked)), [
        ('Milk', true),
        ('Eggs large', false),
        ('Bread', false),
      ]);
      expect(list.lines.first.checkedAt, changed);
      final milk = store.history(list.lines.first.id);
      expect({
        for (final e in milk) e.field: e.date,
      }, containsPair(ChecklistKeys.checked, changed));
      expect({for (final e in milk) e.field: e.date}[ChecklistKeys.type], made);
      expect(groceries.spawned, isFalse);
    });

    test('a list starts off the Agenda until switched on', () {
      final summary = imports.keep('From Google Keep', [
        note('Groceries', {
          ...times(),
          'title': 'Groceries',
          'listContent': [
            {'text': 'Milk', 'isChecked': false},
          ],
        }),
      ]);
      final lists = Checklists(store);
      final id = only(summary).id;
      expect(lists.showsOnAgenda(id), isFalse);
      expect(lists.agenda(), isEmpty);
      lists.setShowOnAgenda(id, true);
      expect(lists.agenda().map((l) => l.noteId), [id]);
    });

    test('a list checked before it was made is checked when it was made', () {
      final summary = imports.keep('From Google Keep', [
        note('Odd', {
          ...times(made, DateTime.utc(2020)),
          'title': 'Odd',
          'listContent': [
            {'text': 'Milk', 'isChecked': true},
          ],
        }),
      ]);
      final list = Checklists(store).byNote(only(summary).id)!;
      expect(list.lines.single.checkedAt, made);
    });

    test('a note in the trash is skipped, an archived one is made', () {
      final summary = imports.keep('From Google Keep', [
        note('Old', {...times(), 'title': 'Old', 'isTrashed': true}),
        note('Kept', {...times(), 'title': 'Kept', 'isArchived': true}),
        note('Odd', {...times(), 'title': 'Odd', 'isTrashed': 'yes'}),
      ]);
      expect(summary.trashed, 1);
      expect(summary.skipped, 1);
      expect(summary.notes, 2);
      final texts = notes.inside(summary.areaId!).map((n) => n.text);
      expect(texts, unorderedEquals(['Kept', 'Odd']));
    });

    test('photos it names are its photos; missing ones, other kinds and '
        'those too large are counted', () {
      final jpeg = [0xff, 0xd8, 0xff, 0xd9];
      final summary = imports.keep('From Google Keep', [
        note('Garden', {
          ...times(),
          'title': 'Garden',
          'attachments': [
            {'filePath': 'a1.png', 'mimetype': 'image/png'},
            {'filePath': 'b2.jpeg', 'mimetype': 'image/jpeg'},
            {'filePath': 'a1.png', 'mimetype': 'image/png'},
            {'filePath': 'gone.webp', 'mimetype': 'image/webp'},
            {'filePath': 'hum.3gp', 'mimetype': 'audio/3gpp'},
            {'filePath': 'big.GIF', 'mimetype': 'image/gif'},
            {'mimetype': 'image/png'},
          ],
        }),
        note('Shed', {
          ...times(),
          'attachments': [
            {'filePath': 'a1.png', 'mimetype': 'image/png'},
          ],
        }),
        raw('Takeout/Keep/a1.png', png),
        raw('Takeout/Keep/b2.jpg', jpeg),
        raw('Takeout/Keep/hum.3gp', [1, 2, 3]),
        raw('Takeout/Keep/big.GIF', List.filled(Imports.maxPhotoBytes + 1, 0)),
      ]);
      expect(summary.notes, 2);
      expect(summary.photos, 3);
      expect(summary.missing, 2);
      expect(summary.notPhoto, 1);
      expect(summary.tooLarge, 1);
      expect(summary.skipped, 4);
      final [shed, garden] = notes.inside(summary.areaId!)
        ..sort((a, b) => a.text.compareTo(b.text));
      expect(garden.photos, hasLength(2));
      expect(notes.photoBytes(garden.photos.first), png);
      expect(notes.photoBytes(garden.photos.last), jpeg);
      // A Note of a photo alone has no text, as at Capture.
      expect(shed.text, '');
      expect(shed.photos, [garden.photos.first]);
    });

    test('only the entries of a Keep folder count when there is one; '
        'its .html twins and Labels.txt are not read', () {
      final keep = note('Dentist', {...times(), 'title': 'Dentist'});
      final other = file(
        'Takeout/Fit/Walk.json',
        jsonEncode({...times(), 'title': 'Walk'}),
      );
      final summary = imports.keep('From Google Keep', [
        keep,
        file('Takeout/Keep/Dentist.html', '<html>Dentist</html>'),
        file('Takeout/Keep/Labels.txt', 'Errands\n'),
        other,
        file('Takeout/archive_browser.html', '<html></html>'),
      ]);
      expect((summary.notes, summary.skipped), (1, 0));
      expect(only(summary).text, 'Dentist');
      final loose = imports.keep('From Google Keep', [
        file('Dentist.json', utf8.decode(keep.bytes)),
        file(r'sub\Walk.json', utf8.decode(other.bytes)),
      ]);
      expect(loose.notes, 2);
    });

    test('what is no Keep note or holds nothing is no text', () {
      final summary = imports.keep('From Google Keep', [
        file('Takeout/Keep/Broken.json', '{"title": "Broken"'),
        file('Takeout/Keep/List.json', '[1, 2]'),
        note('Untimed', {'title': 'Untimed'}),
        note('Float', {
          'title': 'Float',
          'createdTimestampUsec': 1.7e15,
          'userEditedTimestampUsec': 1.7e15,
        }),
        note('Empty', {...times(), 'title': ' ', 'textContent': '\n'}),
        note('Labels', {
          ...times(),
          'labels': [
            {'name': 'Errands'},
          ],
        }),
        note('No lines', {...times(), 'listContent': <Object?>[]}),
        raw('Takeout/Keep/Wide.json', [0xff, 0xfe, 0x7b, 0x00]),
        raw(
          'Takeout/Keep/Big.json',
          List.filled(Imports.maxFileBytes + 1, 0x20),
        ),
      ]);
      expect(summary.areaId, isNull);
      expect(summary.notText, 8);
      expect(summary.tooLarge, 1);
      expect(store.allEntries(), isEmpty);
    });

    test('a note is captured when made, else when last changed, else now', () {
      final later = now.add(const Duration(days: 1));
      final summary = imports.keep('From Google Keep', [
        note('Zero', {...times(DateTime.utc(1970)), 'title': 'Zero'}),
        note('Later', {...times(later, later), 'title': 'Later'}),
        note('Text', {
          'createdTimestampUsec': '${made.microsecondsSinceEpoch}',
          'userEditedTimestampUsec': changed.microsecondsSinceEpoch,
          'title': 'Text',
        }),
      ]);
      final dates = {
        for (final n in notes.inside(summary.areaId!)) n.text: n.captured,
      };
      expect(dates, {'Zero': changed, 'Later': now, 'Text': changed});
    });

    test('says which files it reads, and how much of them', () {
      expect(Imports.keepReads('Takeout/Keep/a.json'), Imports.maxFileBytes);
      expect(Imports.keepReads('Takeout/Keep/A.JSON'), Imports.maxFileBytes);
      for (final photo in ['a.jpg', 'a.JPEG', 'a.png', 'a.webp', 'a.gif']) {
        expect(Imports.keepReads('Keep/$photo'), Imports.maxPhotoBytes);
      }
      for (final other in ['a.html', 'Labels.txt', 'a.3gp', '._a.json', 'a']) {
        expect(Imports.keepReads('Keep/$other'), isNull, reason: other);
      }
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

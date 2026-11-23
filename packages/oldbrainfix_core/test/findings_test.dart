import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  group('folding', () {
    test('ignores case, accents and umlauts, ß as ss', () {
      expect(Findings.fold('Müller'), 'muller');
      expect(Findings.fold('STRAßE'), 'strasse');
      expect(Findings.fold('Straße'), Findings.fold('strasse'));
      expect(Findings.fold('Müller'), 'muller');
      expect(Findings.fold('Ærø Œuvre ẞ'), 'aero oeuvre ss');
      expect(
        Findings.fold('Ελλάδα Łódź Đorđe İzmir'),
        'ελλαδα lodz dorde izmir',
      );
      expect(Findings.fold('plain ASCII 123'), 'plain ascii 123');
      expect(Findings.fold('🧠 日記'), '🧠 日記');
    });

    test('"mueller" is no "Müller", and a typo finds nothing', () {
      expect(Findings.fold('Müller').contains(Findings.fold('mueller')), false);
      expect(Findings.fold('Müller').contains(Findings.fold('muler')), false);
    });

    test('folds a text as it folds its characters one by one', () {
      const text = 'Grüße aus Österreich, ΟΔΌΣ, Ǆ, İ, Σ, 𝔸 ﬃ';
      expect(
        Findings.fold(text),
        [for (final r in text.runes) Findings.fold(String.fromCharCode(r))]
            .join(),
      );
    });
  });

  group('in a text', () {
    test('finds every place, its line and its context', () {
      final found = Findings.inText(
        'Call Herr Müller\nthen muller again, MULLER',
        TextFormats.plain,
        ' muller ',
      );
      expect(found.map((f) => (f.start, f.end, f.line)), [
        (10, 16, 0),
        (22, 28, 1),
        (36, 42, 1),
      ]);
      expect(found.first.context, 'Call Herr Müller');
      expect(
        found.first.context.substring(
          found.first.contextStart,
          found.first.contextEnd,
        ),
        'Müller',
      );
      expect(found[2].cell, isNull);
      expect(found[2].part, FindingPart.text);
    });

    test('a blank query, or one folding to nothing, finds nothing', () {
      expect(Findings.inText('a b', TextFormats.plain, '  '), isEmpty);
      expect(Findings.inText('é', TextFormats.plain, '́'), isEmpty);
    });

    test('half of a folded letter takes the whole letter once', () {
      final found = Findings.inText('Straße', TextFormats.plain, 's');
      expect(found.map((f) => (f.start, f.end)), [(0, 1), (4, 5)]);
      final marks = Findings.inText('Müller', TextFormats.plain, 'mu');
      expect(marks.single.end, 3);
    });

    test('a formatted text is found as it reads, marks inside a place', () {
      const text = '# Title\n**Mül**ler and \\*star\\* and `co de`';
      final found = Findings.inText(text, TextFormats.markdown, 'muller');
      // From its first character shown to its last, the marks between.
      expect(found.single.start, text.indexOf('Mül'));
      expect(found.single.end, text.indexOf(' and'));
      expect(found.single.line, 1);
      expect(found.single.context, 'Müller and *star* and co de');
      final star = Findings.inText(text, TextFormats.markdown, '*star*');
      expect(text.substring(star.single.start, star.single.end), r'\*star\*');
      final code = Findings.inText(text, TextFormats.markdown, 'co de');
      expect(text.substring(code.single.start, code.single.end), 'co de');
      // The heading's mark is no part of what it reads.
      expect(Findings.inText(text, TextFormats.markdown, '#'), isEmpty);
    });

    test('a place in a table names its table, row and column', () {
      const text =
          'Before\n\n| a | b |\n| --- | --- |\n| x | y |\n\n'
          '| Name | City |\n| --- | --- |\n| Ann | Köln |\n| Bo | Graz `\\|` |';
      final found = Findings.inText(text, TextFormats.markdown, 'koln');
      final cell = found.single.cell;
      expect(cell?.table, 1);
      expect(cell?.tableStart, text.indexOf('| Name'));
      expect(cell?.row, 1);
      expect(cell?.column, 1);
      expect(found.single.context, 'Ann Köln');
      expect(text.substring(found.single.start, found.single.end), 'Köln');
      // Across two cells, as search finds it; the column it begins in.
      final across = Findings.inText(text, TextFormats.markdown, 'ann kö');
      expect(across.single.cell?.column, 0);
      expect(
        text.substring(across.single.start, across.single.end),
        'Ann | Kö',
      );
      final pipe = Findings.inText(text, TextFormats.markdown, 'graz |');
      expect(text.substring(pipe.single.start, pipe.single.end), r'Graz `\|');
      expect(pipe.single.cell?.row, 2);
      // Nothing in a delimiter row.
      expect(Findings.inText(text, TextFormats.markdown, '---'), isEmpty);
    });

    test('a long line is cut around the place at whole words', () {
      final words = [for (var i = 0; i < 40; i++) 'word$i'].join(' ');
      final found = Findings.inText(words, TextFormats.plain, 'word20');
      final f = found.single;
      expect(f.context.startsWith('…'), true);
      expect(f.context.endsWith('…'), true);
      expect(f.context.substring(f.contextStart, f.contextEnd), 'word20');
      expect(f.context.runes.length, lessThanOrEqualTo(82));
      expect(f.context, contains(' word19 word20 word21 '));
    });

    test('every line reads as the text reads without its marks', () {
      // What search reads (Markdown.plain) and what is matched agree.
      final fixture = jsonDecode(
        File('../../desktop/fixtures/markdown.json').readAsStringSync(),
      ) as Map<String, Object?>;
      for (final c in fixture['cases'] as List<Object?>) {
        final text = (c as Map<String, Object?>)['text'] as String;
        final plain = Markdown.plain(text);
        for (final word in plain.split(RegExp(r'\s+'))) {
          if (word.trim().isEmpty) continue;
          expect(
            Findings.inText(text, TextFormats.markdown, word),
            isNotEmpty,
            reason: '$word in $text',
          );
        }
      }
    });
  });

  group('in a Note', () {
    late Store store;
    late Notes notes;
    var now = DateTime.utc(2026, 11, 23, 8);

    setUp(() {
      now = DateTime.utc(2026, 11, 23, 8);
      store = Store.inMemory(clock: () => now);
      notes = Notes(store);
    });
    tearDown(() => store.close());

    test('its text, Transcripts, Photo Texts, files and Sections', () {
      final note = notes.capture(
        'Müller called',
        recordings: [
          Uint8List.fromList(const [1, 2, 3]),
        ],
        files: [
          NewFile(
            name: 'muller.txt',
            mediaType: 'text/plain',
            bytes: Uint8List.fromList(utf8.encode('Dear Mr Muller')),
          ),
        ],
      );
      final recording = notes.byId(note.id)?.recordings.single ?? '';
      notes.setTranscript(note.id, recording, 'Herr Müller says hello');
      final file = notes.byId(note.id)?.attachments.single.hash ?? '';
      notes.setAttachmentText(note.id, file, 'Dear Mr Muller');
      final section = notes.amend(
        note.id,
        'and **Müller** again',
        format: TextFormats.markdown,
      );
      final now = notes.byId(note.id);
      if (now == null) fail('no Note');
      final found = Findings.ofNote(
        now,
        'MULLER',
        sections: notes.sections(note.id),
      );
      expect(found.map((f) => (f.part, f.section)), [
        (FindingPart.text, null),
        (FindingPart.transcript, null),
        (FindingPart.fileName, null),
        (FindingPart.attachmentText, null),
        (FindingPart.text, section.id),
      ]);
      expect(found[1].key, recording);
      expect(found[1].context, 'Herr Müller says hello');
    });

    test('search folds, with the index and without it', () {
      final note = notes.capture('Herr Müller, Hauptstraße 4');
      final amended = notes.capture('see below');
      notes.amend(amended.id, 'ÉCOLE');
      for (final search in [Search(store), Search.scanning(store)]) {
        expect(search.notes('muller').map((n) => n.id), [note.id]);
        expect(search.notes('HAUPTSTRASSE').map((n) => n.id), [note.id]);
        expect(search.notes('ecole').map((n) => n.id), [amended.id]);
        expect(search.notes('mueller'), isEmpty);
        // An accent alone folds to nothing and finds nothing.
        expect(search.notes('́'), isEmpty);
        expect(search.among('́', notes.all()), isEmpty);
      }
    });

    test('an index of texts only lowered is built anew folded', () {
      final note = notes.capture('Herr Müller');
      expect(Search(store).notes('muller').map((n) => n.id), [note.id]);
      // As the index of version 1 held it, up to date.
      store.database.execute('DELETE FROM search_texts');
      store.database.execute(
        'INSERT INTO search_texts (entity, text) VALUES (?, ?)',
        [note.id, 'herr müller'],
      );
      store.database.execute(
        "UPDATE search_state SET value = 1 WHERE key = 'version'",
      );
      expect(Search(store).notes('muller').map((n) => n.id), [note.id]);
    });

    test(
      'a Note search finds by its own texts has places, the others none',
      () {
        final texts = [
          'Müller',
          'Straße 4',
          '**Gr**üße',
          '| a | b |\n| - | - |\n| Köln | x |',
          'nothing here',
          'München',
        ];
        final formats = [
          TextFormats.plain,
          TextFormats.markdown,
          TextFormats.markdown,
          TextFormats.markdown,
          TextFormats.plain,
          TextFormats.plain,
        ];
        for (final (i, text) in texts.indexed) {
          now = now.add(const Duration(minutes: 1));
          notes.capture(text, format: formats[i]);
        }
        final search = Search(store);
        for (final query in [
          'muller',
          'strasse',
          'grusse',
          'koln x',
          'munchen',
          'e',
          'MÜLLER',
          'ss',
        ]) {
          final found = {for (final n in search.notes(query)) n.id};
          for (final note in notes.all()) {
            expect(
              Findings.ofNote(note, query).isNotEmpty,
              found.contains(note.id),
              reason: '$query in ${note.text}',
            );
          }
        }
      },
    );
  });
}

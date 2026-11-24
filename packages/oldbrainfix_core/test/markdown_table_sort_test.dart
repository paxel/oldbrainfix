import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A table of one column holding [cells] under the header row `h`.
String _column(List<String> cells) =>
    ['| h |', '|---|', for (final c in cells) '| $c |'].join('\n');

/// The cells of the one column of [cells] as [MarkdownTables.sortedRows]
/// orders them.
List<String> _sorted(List<String> cells, {bool descending = false}) {
  final table = MarkdownTables.all(_column(cells)).single;
  return [
    for (final i in MarkdownTables.sortedRows(table, 0, descending: descending))
      table.rows[i].cells.single.visible,
  ];
}

MarkdownSortKind _kind(List<String> cells) =>
    MarkdownTables.sortKind(MarkdownTables.all(_column(cells)).single, 0);

void main() {
  group('comparing', () {
    test('numbers with units and currency signs sort by number', () {
      final cells = ['12,50 €', '3 kg', '€ 100', '-2', '−7 %', '0,5', '+4'];
      expect(_kind(cells), MarkdownSortKind.number);
      expect(_sorted(cells), [
        '−7 %',
        '-2',
        '0,5',
        '3 kg',
        '+4',
        '12,50 €',
        '€ 100',
      ]);
    });

    test('the column decides the decimal mark', () {
      // A comma once before two digits makes it the decimal mark, so
      // "1.234" groups.
      expect(_sorted(['1.234', '12,5', '999']), ['12,5', '999', '1.234']);
      // A point once before one digit: "1,234" groups.
      expect(_sorted(['1,234', '1.5', '999']), ['1.5', '999', '1,234']);
      // A mark seen grouping makes the other the decimal one.
      expect(_sorted(['1.234.567', '2,345', '3']), ['2,345', '3', '1.234.567']);
      // Nothing decides: one mark before three digits groups.
      expect(_sorted(['1,234', '567', '2,000']), ['567', '1,234', '2,000']);
      // A 0 before it makes it the decimal mark.
      expect(_sorted(['0.500', '2']), ['0.500', '2']);
      expect(_sorted(["1'234.5", '1\u00a0234.25 €', '999']), [
        '999',
        '1\u00a0234.25 €',
        "1'234.5",
      ]);
    });

    test('cells that disagree or are no numbers make a column of text', () {
      expect(_kind(['1,5', '2.5']), MarkdownSortKind.text);
      expect(_kind(['1.2,3', '4']), MarkdownSortKind.text);
      expect(_kind(['1,23,456', '4']), MarkdownSortKind.text);
      expect(_kind(['3 x 4', '5']), MarkdownSortKind.text);
      expect(_kind(['ca. 5', '6']), MarkdownSortKind.text);
      expect(_kind(['10:30', '9:15']), MarkdownSortKind.text);
      expect(_kind(['٣', '4']), MarkdownSortKind.text);
    });

    test('numbers compare exactly, however long', () {
      expect(
        _sorted([
          '12345678901234567890123',
          '12345678901234567890122.9',
          '-0',
          '0.0',
          '-0.000001',
        ]),
        [
          '-0.000001',
          '-0',
          '0.0',
          '12345678901234567890122.9',
          '12345678901234567890123',
        ],
      );
    });

    test('dates in the app\'s formats sort by date', () {
      final cells = ['24.11.2026', '2026-01-05', '3/1/2026', '29.2.2024'];
      expect(_kind(cells), MarkdownSortKind.date);
      expect(_sorted(cells), [
        '29.2.2024',
        '2026-01-05',
        '3/1/2026',
        '24.11.2026',
      ]);
      // A day not in the calendar, a two-digit year and a year first with
      // points are no dates.
      expect(_kind(['31.02.2026', '1.1.2026']), MarkdownSortKind.text);
      expect(_kind(['29.2.2026', '1.1.2026']), MarkdownSortKind.text);
      expect(_kind(['24.11.26', '1.1.2026']), MarkdownSortKind.text);
      expect(_kind(['24/11/2026', '1/1/2026']), MarkdownSortKind.text);
      expect(_kind(['2026.11.24', '1.1.2026']), MarkdownSortKind.text);
      expect(_kind(['2026', '2025']), MarkdownSortKind.number);
    });

    test('text ignores case, accents and umlauts and keeps equal ones', () {
      final cells = ['Zebra', 'äpfel', 'Müller', 'muller', 'Apfel', 'eclair'];
      expect(_kind(cells), MarkdownSortKind.text);
      expect(_sorted(cells), [
        'äpfel',
        'Apfel',
        'eclair',
        'Müller',
        'muller',
        'Zebra',
      ]);
      expect(_sorted(cells, descending: true), [
        'Zebra',
        'Müller',
        'muller',
        'eclair',
        'äpfel',
        'Apfel',
      ]);
      // Code point by code point: an astral character after U+FF41.
      expect(_sorted(['🧠', 'ａ']), ['ａ', '🧠']);
      // Marks do not count.
      expect(_sorted(['**b**', '*a*', '`c`']), ['a', 'b', 'c']);
    });

    test('empty cells go last both ways, short rows too', () {
      const text =
          '| a | b |\n|---|---|\n| x | 2 |\n| y |\n|  | 1 |\n| z | 3 |';
      final table = MarkdownTables.all(text).single;
      expect(MarkdownTables.sortedRows(table, 1), [3, 1, 4, 2]);
      expect(MarkdownTables.sortedRows(table, 1, descending: true), [
        4,
        1,
        3,
        2,
      ]);
      expect(MarkdownTables.sortedRows(table, 0), [1, 2, 4, 3]);
      expect(MarkdownTables.sortedRows(table, 0, descending: true), [
        4,
        2,
        1,
        3,
      ]);
      // A column it does not have keeps them as written.
      expect(MarkdownTables.sortedRows(table, 2), [1, 2, 3, 4]);
      expect(MarkdownTables.sortedRows(table, -1), [1, 2, 3, 4]);
      expect(_kind(['', '']), MarkdownSortKind.text);
    });

    test('equal keys keep their order in long tables, both ways', () {
      final numbers = [for (var i = 0; i < 80; i++) '${i % 3} kg'];
      final table = MarkdownTables.all(_column(numbers)).single;
      final up = MarkdownTables.sortedRows(table, 0);
      final down = MarkdownTables.sortedRows(table, 0, descending: true);
      List<int> of(int key) => [
        for (var i = 1; i <= 80; i++)
          if ((i - 1) % 3 == key) i,
      ];
      expect(up, [...of(0), ...of(1), ...of(2)]);
      expect(down, [...of(2), ...of(1), ...of(0)]);
      final same = [for (var i = 0; i < 80; i++) i.isEven ? 'Ä' : 'a'];
      final written = [for (var i = 1; i <= 80; i++) i];
      final letters = MarkdownTables.all(_column(same)).single;
      expect(MarkdownTables.sortedRows(letters, 0), written);
      expect(MarkdownTables.sortedRows(letters, 0, descending: true), written);
    });

    test('box ids and variation selectors do not count', () {
      // Ids that would sort the rows backwards.
      final ids = ['ccc', 'bbb', 'aaa'].iterator;
      final text = TextBoxes.convert(
        _column(['☐ b', '☐\ufe0f a', '☐ c']),
        format: TextFormats.markdown,
        newId: () => (ids..moveNext()).current,
      );
      expect(TextBoxes.find(text), hasLength(3));
      final table = MarkdownTables.all(text).single;
      expect(MarkdownTables.sortedRows(table, 0), [2, 1, 3]);
    });
  });

  group('sortRows', () {
    test('moves the rows\' lines as written, the rest untouched', () {
      const text =
          'before\r\n${MarkdownTables.headingColumnLine}\r\n'
          'Item|  Price\r\n:--- | ---:\r\n'
          '  bread | 2,50 € | beyond\r\n| milk |1 €|\r\neggs | 12 €\r\n\r\nafter';
      final at = text.indexOf('Item');
      final up = MarkdownTables.sortRows(text, at, 1);
      expect(
        up.text,
        'before\r\n${MarkdownTables.headingColumnLine}\r\n'
        'Item|  Price\r\n:--- | ---:\r\n'
        '| milk |1 €|\r\n  bread | 2,50 € | beyond\r\neggs | 12 €\r\n\r\nafter',
      );
      final table = MarkdownTables.at(up.text, at);
      expect(table, isNotNull);
      expect((up.start, up.end), (table?.start, table?.end));
      final down = MarkdownTables.sortRows(text, at, 1, descending: true);
      expect(
        down.text,
        'before\r\n${MarkdownTables.headingColumnLine}\r\n'
        'Item|  Price\r\n:--- | ---:\r\n'
        'eggs | 12 €\r\n  bread | 2,50 € | beyond\r\n| milk |1 €|\r\n\r\nafter',
      );
    });

    test('rows already in order, a missing column or table change nothing', () {
      const text = 'x\n| a |\n|---|\n| 1 |\n| 2 |';
      for (final (column, descending) in [(0, false), (1, false), (-1, true)]) {
        final c = MarkdownTables.sortRows(
          text,
          4,
          column,
          descending: descending,
        );
        expect((c.text, c.start, c.end), (text, 4, 4));
      }
      expect(
        MarkdownTables.sortRows(text, 4, 0, descending: true).text,
        'x\n| a |\n|---|\n| 2 |\n| 1 |',
      );
      final none = MarkdownTables.sortRows('plain', 99, 0);
      expect((none.text, none.start, none.end), ('plain', 5, 5));
    });
  });
}

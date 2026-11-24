import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/whole_tables.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A whole table moved past the block of the text above or below it, or
/// deleted, from its ⋮ (obf-1050), every other character kept.
void main() {
  const a = '| a |\n| --- |\n| 1 |';
  const b = '| b |\n| --- |';

  /// The text with the table at [table] of [text] moved; the change
  /// selects the table moved.
  String moved(String before, String table, {required bool up}) {
    final change = WholeTables.move(before, before.indexOf(table), up: up);
    expect(change, isNotNull);
    final (:text, :start, :end) = change!;
    expect(text.substring(start, end), table);
    // No line becomes a row of a table, and no two tables one.
    List<int> rows(String text) =>
        [for (final t in MarkdownTables.all(text)) t.rows.length]..sort();
    expect(rows(text), rows(before));
    return text;
  }

  group('blocks', () {
    test('a list, a paragraph, a quote run on; a heading, a table and a '
        'code block stand alone; blank lines part them', () {
      const text =
          '# Head\nwords\nmore\n\n- one\n1. two\n  - three\n> said\n> on\n'
          '$a\n\n```\n| x |\n| --- |\n```\n  \nend';
      expect(
        [
          for (final block in WholeTables.blocks(text))
            (block.first, block.last, block.kind),
        ],
        [
          (0, 0, MarkdownBlockKind.heading1),
          (1, 2, MarkdownBlockKind.paragraph),
          (4, 6, MarkdownBlockKind.bullet),
          (7, 8, MarkdownBlockKind.quote),
          (9, 11, MarkdownBlockKind.table),
          (13, 16, MarkdownBlockKind.code),
          (18, 18, MarkdownBlockKind.paragraph),
        ],
      );
    });
  });

  group('move', () {
    test('past a paragraph, the blank lines between staying', () {
      expect(moved('Shop\n\n$a\n\nBread', a, up: true), '$a\n\nShop\n\nBread');
      expect(moved('Shop\n\n$a\n\nBread', a, up: false), 'Shop\n\nBread\n\n$a');
    });

    test('a blank line goes where it would stand right above a paragraph', () {
      expect(moved('Shop\n$a', a, up: true), '$a\n\nShop');
    });

    test('past another table, also one right under it', () {
      expect(moved('$a\n\n$b', a, up: false), '$b\n\n$a');
      const flagged = '${MarkdownTables.headingColumnLine}\n$b';
      final text = moved('$a\n$flagged', a, up: false);
      expect(text, '$flagged\n\n$a');
      expect(MarkdownTables.all(text), hasLength(2));
    });

    test('past a code block holding table lines, as one block', () {
      const code = '```\n| x |\n| --- |\n```';
      expect(moved('$code\n\n$a', a, up: true), '$a\n\n$code');
    });

    test('past a list of several items', () {
      const list = '- one\n- two\n  - three';
      expect(
        moved('$list\n\n$a\n\n# after', a, up: true),
        '$a\n\n$list\n\n# after',
      );
    });

    test('a paragraph that comes right under another table gets a blank '
        'line between', () {
      const flagged = '${MarkdownTables.headingColumnLine}\n$b';
      expect(
        moved('$a\n$flagged\n\npara', flagged, up: false),
        '$a\n\npara\n\n$flagged',
      );
    });

    test('a table it comes right under gets a blank line between', () {
      final text = moved('$a\n# H\n$b', b, up: true);
      expect(text, '$a\n\n$b\n# H');
      expect(MarkdownTables.all(text), hasLength(2));
    });

    test('keeps the kind of line break and the end of the text', () {
      const crlf = '| a |\r\n| --- |';
      expect(moved('Shop\r\n\r\n$crlf', crlf, up: true), '$crlf\r\n\r\nShop');
      expect(moved('Shop\r\n$crlf', crlf, up: true), '$crlf\r\n\r\nShop');
    });

    test('nothing to move past at the top or the bottom', () {
      const text = '$a\n\nBread';
      expect(WholeTables.canMove(text, 0, up: true), isFalse);
      expect(WholeTables.move(text, 0, up: true), isNull);
      expect(WholeTables.canMove(text, 0, up: false), isTrue);
      expect(WholeTables.canMove('Shop\n\n$a\n\n', 6, up: false), isFalse);
      expect(WholeTables.move('Shop', 0, up: false), isNull);
    });
  });

  group('delete', () {
    test('takes the table and the line break after it, as deleting its '
        'last row does', () {
      const text = 'Shop\n\n$a\n\nBread';
      expect(WholeTables.delete(text, 6)?.text, 'Shop\n\n\nBread');
      var rows = text;
      while (MarkdownTables.all(rows).isNotEmpty) {
        rows = MarkdownTables.deleteRow(rows, 6, 0).text;
      }
      expect(WholeTables.delete(text, 6)?.text, rows);
    });

    test('last in the text, the line break before it', () {
      expect(WholeTables.delete('Shop\n\n$a', 6)?.text, 'Shop\n');
      expect(WholeTables.delete('Shop\r\n| a |\r\n| --- |', 6)?.text, 'Shop');
      expect(WholeTables.delete('Shop', 0), isNull);
    });
  });
}

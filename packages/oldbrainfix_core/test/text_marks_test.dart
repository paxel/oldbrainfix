import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// The marks of [text] as (line, mark, the mark as written).
List<(int, TextMark, String)> _marks(String text) => [
  for (final m in TextMarks.find(text))
    (m.line, m.mark, text.substring(m.start, m.end)),
];

void main() {
  group('finding marks', () {
    test('a mark counts as the first character of a line', () {
      expect(_marks('Release\n✅ v2 at customer\n❌ v1\n☐ v3'), [
        (1, TextMark.checked, '✅'),
        (2, TextMark.failed, '❌'),
        (3, TextMark.open, '☐'),
      ]);
      expect(_marks('☐'), [(0, TextMark.open, '☐')]);
      expect(_marks('✅done'), [(0, TextMark.checked, '✅')]);
    });

    test('a mark elsewhere in a line is plain text', () {
      expect(_marks('v2 supported at customer: ✅'), isEmpty);
      expect(_marks('😀 ☐ after an emoji'), isEmpty);
      expect(_marks('x ❌\ny ✅ z'), isEmpty);
    });

    test('spaces, tabs and a list item\'s bullet may come before it', () {
      expect(_marks('  ☐ a\n\t✅ b\n \t ❌ c\n- ☐ d\n* ✅ e\n+\t❌ f\n-   ☐ g'), [
        (0, TextMark.open, '☐'),
        (1, TextMark.checked, '✅'),
        (2, TextMark.failed, '❌'),
        (3, TextMark.open, '☐'),
        (4, TextMark.checked, '✅'),
        (5, TextMark.failed, '❌'),
        (6, TextMark.open, '☐'),
      ]);
      // Nothing else: no bullet without a space, no second bullet, no
      // numbered item, no other kind of space.
      expect(_marks('-☐ a\n- - ☐ b\n1. ☐ c\n ☐ d\n﻿☐ e\n> ☐ f'), isEmpty);
    });

    test('a variation selector belongs to the mark, a joiner makes it '
        'another emoji', () {
      expect(_marks('✅️ a\n❌︎ b\n☐️️ c'), [
        (0, TextMark.checked, '✅️'),
        (1, TextMark.failed, '❌︎'),
        (2, TextMark.open, '☐️'),
      ]);
      expect(_marks('✅‍🔥 a\n✅️‍🔥 b\n️☐ c'), isEmpty);
    });

    test('other boxes and ticks are no marks', () {
      expect(_marks('☑ a\n☒ b\n✔ c\n✓ d\n❎ e\n✖ f\n✗ g'), isEmpty);
    });

    test('lines break at \\n, \\r\\n and a lone \\r, as splitLines does', () {
      const text = 'a\r\n☐ b\r❌ c\n\n✅';
      expect(_marks(text).map((m) => m.$1), [1, 2, 4]);
      expect(splitLines(text)[4], '✅');
      expect(_marks('☐ a\n'), [(0, TextMark.open, '☐')]);
      expect(_marks(''), isEmpty);
    });

    test('offsets are UTF-16 code units, an astral character counting '
        'two', () {
      final [mark] = TextMarks.find('🧠\n☐ x');
      expect((mark.start, mark.end), (3, 4));
    });
  });

  group('swapping a mark', () {
    test('changes the mark alone, every other character kept', () {
      const text = 'Release 🧠\r\n  - ☐ v2 at customer: ✅\n❌ v1\n';
      expect(
        TextMarks.swap(text, 1, TextMark.checked),
        'Release 🧠\r\n  - ✅ v2 at customer: ✅\n❌ v1\n',
      );
      expect(
        TextMarks.swap(text, 2, TextMark.open),
        'Release 🧠\r\n  - ☐ v2 at customer: ✅\n☐ v1\n',
      );
    });

    test('drops the variation selector of the old mark', () {
      expect(TextMarks.swap('✅️ done', 0, TextMark.failed), '❌ done');
      expect(TextMarks.swap('a\n☐︎', 1, TextMark.checked), 'a\n✅');
    });

    test('gives null when there is nothing to change', () {
      const text = 'a ✅\n☐ b';
      expect(TextMarks.swap(text, 0, TextMark.open), isNull);
      expect(TextMarks.swap(text, 1, TextMark.open), isNull);
      expect(TextMarks.swap(text, 2, TextMark.checked), isNull);
      expect(TextMarks.swap(text, -1, TextMark.checked), isNull);
      expect(TextMarks.swap('', 0, TextMark.checked), isNull);
    });
  });

  group('inserting a mark', () {
    test('puts ☐ at the start of the line, where it is found', () {
      const text = 'Title\n  - milk\n\tbread\n\n-tea\n- ';
      final lines = splitLines(text);
      final expected = [
        '☐ Title',
        '  - ☐ milk',
        '\t☐ bread',
        '☐ ',
        '☐ -tea',
        '- ☐ ',
      ];
      for (var i = 0; i < lines.length; i++) {
        final inserted = TextMarks.insertOpen(text, i);
        expect(inserted, isNotNull);
        expect(splitLines(inserted!)[i], expected[i]);
        expect(TextMarks.find(inserted).single.line, i);
        expect(inserted.length, text.length + 2);
      }
    });

    test('keeps every other character, line breaks included', () {
      expect(TextMarks.insertOpen('a\r\nb\rc', 1), 'a\r\n☐ b\rc');
      expect(TextMarks.insertOpen('', 0), '☐ ');
    });

    test('gives null on a line with a mark or no such line', () {
      expect(TextMarks.insertOpen('a\n  ✅ b', 1), isNull);
      expect(TextMarks.insertOpen('a', 1), isNull);
      expect(TextMarks.insertOpen('a', -1), isNull);
      // A mark in the middle of the line is none.
      expect(TextMarks.insertOpen('a ✅', 0), '☐ a ✅');
    });
  });

  group('the line of an offset', () {
    test('counts every line break once', () {
      const text = 'a\r\nb\rc\nd';
      expect(
        [for (var i = 0; i <= text.length; i++) TextMarks.lineAt(text, i)],
        [0, 0, 0, 1, 1, 2, 2, 3, 3],
      );
    });

    test('takes an offset outside the text as its start or end', () {
      expect(TextMarks.lineAt('a\nb', -1), 0);
      expect(TextMarks.lineAt('a\nb', 99), 1);
      expect(TextMarks.lineAt('', 0), 0);
      expect(TextMarks.lineAt('a\n', 2), 1);
    });
  });
}

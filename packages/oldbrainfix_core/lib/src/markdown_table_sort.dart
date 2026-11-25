part of 'markdown.dart';

/// How a column of a table compares its cells when the rows are sorted
/// by it (obf-1052), the same in both cores (`table/sort.rs`): by
/// [number] when every cell of it that shows something reads as a
/// number, else by [date] when every one reads as a date, else by
/// [text]. A cell is read as it shows ([MarkdownTableCell.visible]),
/// without the ids of its boxes and variation selectors and without
/// spaces, tabs, U+00A0 and U+202F at its ends.
///
/// A number is: a sign (`-`, `+` or `−`), a currency sign (`$`, `€`,
/// `£`, `¥`, `¢`, `₹`, `₽`, `₩`, `₺`, `₪`, `₫`, `฿`, `₴`, `₦`, `₱`, `₿`)
/// with spaces after it and a sign again where none came before, each
/// when it is there, then ASCII digits, grouped by `.`, `,`, `'`, `’`,
/// U+00A0 or U+202F, and the rest, a unit, holding no digit: "12,50 €",
/// "€ 3", "3 kg", "1.5 bar", "1'234.5", "−2 %". A `.` or a `,` is the
/// decimal mark when it comes once and last; the groups after a mark that
/// groups have three digits, the first one one to three. The column
/// decides which of the two is the decimal mark: a cell holding both has
/// the last one; a cell holding one twice groups by it; a cell holding
/// one once and last has it as the decimal mark when fewer or more than
/// three digits follow it, more than three or a 0 first come before it,
/// or a `'`, `’`, U+00A0 or U+202F groups it too. Where no cell says, the mark seen
/// grouping makes the other one the decimal mark; where nothing says, a
/// cell as "1,234" or "1.234" reads 1234. Cells that disagree make the
/// column one of text. Numbers compare exactly as written, their units
/// not taken into account.
///
/// A date is written as the app's languages write it: `2026-11-24`
/// (ISO), `24.11.2026` (German) or `11/24/2026` (English), day and month
/// of one or two digits, the year of four, and in the calendar.
///
/// Text compares folded as search folds it ([Findings.fold]), case,
/// accents and umlauts not counted, code point by code point.
enum MarkdownSortKind { number, date, text }

/// How the rows under the header row of a table compare by a column: its
/// [kind] and each row's [keys], null for a cell that shows nothing.
typedef _SortColumn = ({
  MarkdownSortKind kind,
  List<Object?> keys,
  int Function(Object a, Object b) compare,
});

/// The column [column] of [table] as [MarkdownTables.sortedRows] sorts
/// by it.
_SortColumn _sortColumn(MarkdownTable table, int column) {
  final shown = [
    for (final row in table.rows.skip(1))
      column >= 0 && column < row.cells.length
          ? _sortText(row.cells[column].visible)
          : '',
  ];
  if (shown.any((s) => s.isNotEmpty)) {
    final dates = [for (final s in shown) s.isEmpty ? null : _sortDate(s)];
    var all = true;
    for (var i = 0; i < shown.length && all; i++) {
      all = shown[i].isEmpty || dates[i] != null;
    }
    if (all) {
      return (
        kind: MarkdownSortKind.date,
        keys: dates,
        compare: (a, b) => (a as int).compareTo(b as int),
      );
    }
    if (_sortNumbers(shown) case final numbers?) {
      return (
        kind: MarkdownSortKind.number,
        keys: numbers,
        compare: (a, b) => (a as _SortNumber).compareTo(b as _SortNumber),
      );
    }
  }
  return (
    kind: MarkdownSortKind.text,
    keys: [
      for (final s in shown) s.isEmpty ? null : Findings.fold(s).runes.toList(),
    ],
    compare: (a, b) => _compareRunes(a as List<int>, b as List<int>),
  );
}

/// The rows under the header row of [table] by their index in it, the
/// header row 0, sorted by [column].
List<int> _sortedRows(MarkdownTable table, int column, bool descending) {
  final by = _sortColumn(table, column);
  return [for (var i = 1; i < table.rows.length; i++) i]..sort((x, y) {
    final a = by.keys[x - 1];
    final b = by.keys[y - 1];
    if (a == null || b == null) {
      if (a != null) return -1;
      if (b != null) return 1;
      return x.compareTo(y);
    }
    final c = by.compare(a, b);
    if (c != 0) return descending ? -c : c;
    return x.compareTo(y);
  });
}

/// [text] with the rows under the header row of its table at [at] in the
/// order [MarkdownTables.sortedRows] gives.
MarkdownChange _sortRows(String text, int at, int column, bool descending) {
  final offset = at.clamp(0, text.length);
  final unchanged = (text: text, start: offset, end: offset);
  final table = MarkdownTables.at(text, offset);
  if (table == null || column < 0 || column >= table.columns) {
    return unchanged;
  }
  final order = _sortedRows(table, column, descending);
  var same = true;
  for (var i = 0; i < order.length && same; i++) {
    same = order[i] == i + 1;
  }
  if (same) return unchanged;
  final body = table.rows.sublist(1);
  final out = StringBuffer();
  for (var i = 0; i < order.length; i++) {
    final row = table.rows[order[i]];
    out.write(text.substring(row.start, row.end));
    if (i + 1 < body.length) {
      out.write(text.substring(body[i].end, body[i + 1].start));
    }
  }
  return (
    text: text.replaceRange(body.first.start, body.last.end, out.toString()),
    start: table.start,
    end: table.end,
  );
}

/// [visible] as the sorting reads it: without the ids of boxes (tag
/// characters) and variation selectors, and without spaces, tabs, U+00A0
/// and U+202F at its ends.
String _sortText(String visible) {
  final runes = [
    for (final r in visible.runes)
      if (!(r >= 0xE0000 && r <= 0xE007F) && !(r >= 0xFE00 && r <= 0xFE0F)) r,
  ];
  bool blank(int r) => r == 0x20 || r == 0x09 || r == 0xA0 || r == 0x202F;
  var from = 0;
  var to = runes.length;
  while (from < to && blank(runes[from])) {
    from++;
  }
  while (to > from && blank(runes[to - 1])) {
    to--;
  }
  return String.fromCharCodes(runes, from, to);
}

int _compareRunes(List<int> a, List<int> b) {
  final n = a.length < b.length ? a.length : b.length;
  for (var i = 0; i < n; i++) {
    if (a[i] != b[i]) return a[i].compareTo(b[i]);
  }
  return a.length.compareTo(b.length);
}

bool _sortDigit(int c) => c >= 0x30 && c <= 0x39;

const _hyphen = 0x2D;
const _period = 0x2E;
const _slash = 0x2F;
const _comma = 0x2C;

/// The date [s] is, as year, month and day in one number, or null.
int? _sortDate(String s) {
  final runs = <String>[];
  int? mark;
  var i = 0;
  while (true) {
    final start = i;
    while (i < s.length && _sortDigit(s.codeUnitAt(i))) {
      i++;
    }
    if (i == start) return null;
    runs.add(s.substring(start, i));
    if (i == s.length) break;
    final c = s.codeUnitAt(i);
    if (runs.length == 3 ||
        (c != _hyphen && c != _period && c != _slash) ||
        (mark != null && c != mark)) {
      return null;
    }
    mark = c;
    i++;
  }
  if (runs.length != 3) return null;
  final String year;
  final String month;
  final String day;
  if (mark == _hyphen && runs[0].length == 4) {
    (year, month, day) = (runs[0], runs[1], runs[2]);
  } else if (mark == _period && runs[2].length == 4) {
    (year, month, day) = (runs[2], runs[1], runs[0]);
  } else if (mark == _slash && runs[2].length == 4) {
    (year, month, day) = (runs[2], runs[0], runs[1]);
  } else {
    return null;
  }
  if (month.length > 2 || day.length > 2) return null;
  final y = int.parse(year);
  final m = int.parse(month);
  final d = int.parse(day);
  final leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
  final days = switch (m) {
    2 => leap ? 29 : 28,
    4 || 6 || 9 || 11 => 30,
    _ => 31,
  };
  if (m < 1 || m > 12 || d < 1 || d > days) return null;
  return y * 10000 + m * 100 + d;
}

/// The mark that only groups digits, `'`, `’`, U+00A0 and U+202F alike.
const _groupMark = 0x27;

const _currencies = {
  0x24, // $
  0x20AC, // €
  0xA3, // £
  0xA5, // ¥
  0xA2, // ¢
  0x20B9, // ₹
  0x20BD, // ₽
  0x20A9, // ₩
  0x20BA, // ₺
  0x20AA, // ₪
  0x20AB, // ₫
  0x0E3F, // ฿
  0x20B4, // ₴
  0x20A6, // ₦
  0x20B1, // ₱
  0x20BF, // ₿
};

/// A cell as a number is written: its sign, its runs of digits and the
/// marks between them, `.`, `,` or [_groupMark].
typedef _SortWritten = ({bool negative, List<String> groups, List<int> marks});

_SortWritten? _sortWritten(String s) {
  var i = 0;
  var negative = false;
  var signed = false;
  void sign() {
    if (signed || i >= s.length) return;
    final c = s.codeUnitAt(i);
    if (c == 0x2D || c == 0x2B || c == 0x2212) {
      negative = c != 0x2B;
      signed = true;
      i++;
    }
  }

  sign();
  if (i < s.length && _currencies.contains(s.codeUnitAt(i))) {
    i++;
    while (i < s.length &&
        (s.codeUnitAt(i) == 0x20 ||
            s.codeUnitAt(i) == 0xA0 ||
            s.codeUnitAt(i) == 0x202F)) {
      i++;
    }
    sign();
  }
  String? digits() {
    final start = i;
    while (i < s.length && _sortDigit(s.codeUnitAt(i))) {
      i++;
    }
    return i == start ? null : s.substring(start, i);
  }

  final first = digits();
  if (first == null) return null;
  final groups = [first];
  final marks = <int>[];
  while (i + 1 < s.length && _sortDigit(s.codeUnitAt(i + 1))) {
    final mark = switch (s.codeUnitAt(i)) {
      _period => _period,
      _comma => _comma,
      0x27 || 0x2019 || 0xA0 || 0x202F => _groupMark,
      _ => null,
    };
    if (mark == null) break;
    i++;
    marks.add(mark);
    groups.add(digits() ?? '');
  }
  for (var j = i; j < s.length; j++) {
    if (_sortDigit(s.codeUnitAt(j))) return null;
  }
  return (negative: negative, groups: groups, marks: marks);
}

/// The cells [shown] as numbers, null for those that show nothing, or
/// null when one of them is no number or they disagree on the decimal
/// mark.
List<_SortNumber?>? _sortNumbers(List<String> shown) {
  final written = <_SortWritten?>[];
  for (final s in shown) {
    if (s.isEmpty) {
      written.add(null);
    } else if (_sortWritten(s) case final w?) {
      written.add(w);
    } else {
      return null;
    }
  }
  final decimal = <int>{};
  final grouping = <int>{};
  for (final w in written) {
    if (w == null) continue;
    final marks = [
      for (final m in w.marks)
        if (m != _groupMark) m,
    ];
    if (marks.isEmpty) continue;
    final last = marks.last;
    if (marks.contains(_period) && marks.contains(_comma)) {
      decimal.add(last);
    } else if (marks.length > 1) {
      grouping.add(last);
    } else {
      final at = w.marks.indexOf(last);
      if (at != w.marks.length - 1) {
        grouping.add(last);
      } else if (w.groups[at + 1].length != 3 ||
          w.groups[at].length > 3 ||
          w.groups[at].startsWith('0') ||
          w.marks.contains(_groupMark)) {
        decimal.add(last);
      }
    }
  }
  if (decimal.length > 1) return null;
  final int? mark;
  if (decimal.isNotEmpty) {
    mark = decimal.first;
  } else if (grouping.length == 1) {
    mark = grouping.first == _period ? _comma : _period;
  } else {
    mark = null;
  }
  if (mark != null && grouping.contains(mark)) return null;
  final numbers = <_SortNumber?>[];
  for (final w in written) {
    if (w == null) {
      numbers.add(null);
    } else if (_SortNumber.of(w, mark) case final n?) {
      numbers.add(n);
    } else {
      return null;
    }
  }
  return numbers;
}

/// A number exactly as written: its sign, its whole digits without the
/// zeros before them and its fraction's without the zeros after them.
class _SortNumber implements Comparable<_SortNumber> {
  final bool negative;
  final String whole;
  final String fraction;

  const _SortNumber(this.negative, this.whole, this.fraction);

  /// [w] with [mark] as its decimal mark, or null when it does not read
  /// so.
  static _SortNumber? of(_SortWritten w, int? mark) {
    var cut = w.marks.length;
    if (mark != null) {
      final at = w.marks.indexOf(mark);
      if (at >= 0) {
        if (at != w.marks.length - 1) return null;
        cut = at;
      }
    }
    if (cut > 0 &&
        (w.groups[0].length > 3 ||
            w.groups.sublist(1, cut + 1).any((g) => g.length != 3))) {
      return null;
    }
    var whole = w.groups.sublist(0, cut + 1).join();
    var from = 0;
    while (from < whole.length && whole.codeUnitAt(from) == 0x30) {
      from++;
    }
    whole = whole.substring(from);
    var fraction = cut < w.marks.length ? w.groups[cut + 1] : '';
    var to = fraction.length;
    while (to > 0 && fraction.codeUnitAt(to - 1) == 0x30) {
      to--;
    }
    fraction = fraction.substring(0, to);
    return _SortNumber(
      w.negative && (whole.isNotEmpty || fraction.isNotEmpty),
      whole,
      fraction,
    );
  }

  @override
  int compareTo(_SortNumber other) {
    if (negative != other.negative) return negative ? -1 : 1;
    var c = whole.length.compareTo(other.whole.length);
    if (c == 0) c = whole.compareTo(other.whole);
    if (c == 0) c = fraction.compareTo(other.fraction);
    return negative ? -c : c;
  }
}

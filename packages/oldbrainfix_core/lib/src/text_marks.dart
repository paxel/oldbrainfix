/// Status marks in user text (ADR 0031): a ☐, ✅ or ❌ that begins a
/// line of a Note's or a Section's text shows that line open, checked or
/// failed, and a tap changes it. The marks are part of the text, so
/// changing one is an edit of the text like any other; a mark elsewhere
/// in a line is plain text.
library;

/// The state a status mark shows, and the character it is written as.
enum TextMark {
  /// ☐: not done yet.
  open('☐'),

  /// ✅: done.
  checked('✅'),

  /// ❌: failed, or not to be done.
  failed('❌');

  const TextMark(this.glyph);

  /// The character the mark is written as, without a variation
  /// selector.
  final String glyph;
}

/// A status mark found in a text: the [line] it begins, counted from 0
/// at every line break as `splitLines` counts them, the [mark] it shows,
/// and where it lies in the text, from [start] to [end] in UTF-16 code
/// units, a variation selector after it included.
typedef MarkedLine = ({int line, TextMark mark, int start, int end});

/// Finding status marks in a text and changing them, the same in both
/// cores (`text_marks.rs`).
///
/// A mark counts when it is the first character of a line after any
/// spaces and tabs, and after a list item's `-`, `*` or `+` followed by
/// a space or a tab. One variation selector after it (U+FE0E or U+FE0F)
/// belongs to the mark; a zero-width joiner after it makes it part of
/// another emoji, which is no mark.
abstract final class TextMarks {
  /// Every status mark in [text], top line first.
  static List<MarkedLine> find(String text) => [
    for (final (i, (start, end)) in _lines(text).indexed)
      if (_markIn(text, start, end) case (final mark, final from, final to))
        (line: i, mark: mark, start: from, end: to),
  ];

  /// [text] with the status mark of the line [line] changed to [to],
  /// everything else as it was; a variation selector after the old mark
  /// goes with it. Null when there is nothing to change: no such line,
  /// no mark on it, or the mark is [to] already.
  static String? swap(String text, int line, TextMark to) {
    final lines = _lines(text);
    if (line < 0 || line >= lines.length) return null;
    final (start, end) = lines[line];
    final found = _markIn(text, start, end);
    if (found == null || found.$1 == to) return null;
    return text.replaceRange(found.$2, found.$3, to.glyph);
  }

  /// [text] with "☐ " put at the start of the line [line], after its
  /// spaces and tabs and a list item's `- `, where [find] then finds it.
  /// Null when the line has a mark already or there is no such line.
  /// The editor moves its cursor by the length the text grew.
  static String? insertOpen(String text, int line) {
    final lines = _lines(text);
    if (line < 0 || line >= lines.length) return null;
    final (start, end) = lines[line];
    if (_markIn(text, start, end) != null) return null;
    final at = _markStart(text, start, end);
    return text.replaceRange(at, at, '${TextMark.open.glyph} ');
  }

  /// The line the UTF-16 offset [offset] of [text] lies on, as [find]
  /// counts lines: a cursor between the `\r` and the `\n` of a line
  /// break is still on the line before. An offset outside the text is
  /// taken as its start or its end.
  static int lineAt(String text, int offset) {
    final stop = offset.clamp(0, text.length);
    var line = 0;
    for (var i = 0; i < stop; i++) {
      final c = text.codeUnitAt(i);
      if (c == _cr && i + 1 < text.length && text.codeUnitAt(i + 1) == _lf) {
        if (i + 1 < stop) line++;
        i++;
      } else if (c == _lf || c == _cr) {
        line++;
      }
    }
    return line;
  }

  static const _lf = 0x0A;
  static const _cr = 0x0D;
  static const _space = 0x20;
  static const _tab = 0x09;
  static const _textStyle = 0xFE0E;
  static const _emojiStyle = 0xFE0F;
  static const _joiner = 0x200D;

  /// The lines of [text] as offsets, from the first character to the
  /// line break: split at `\n`, `\r\n` and a lone `\r`.
  static List<(int, int)> _lines(String text) {
    final lines = <(int, int)>[];
    var start = 0;
    var i = 0;
    while (i < text.length) {
      final c = text.codeUnitAt(i);
      if (c != _lf && c != _cr) {
        i++;
        continue;
      }
      lines.add((start, i));
      final pair =
          c == _cr && i + 1 < text.length && text.codeUnitAt(i + 1) == _lf;
      i += pair ? 2 : 1;
      start = i;
    }
    lines.add((start, text.length));
    return lines;
  }

  static bool _blank(int c) => c == _space || c == _tab;

  static int _skipBlank(String text, int at, int end) {
    while (at < end && _blank(text.codeUnitAt(at))) {
      at++;
    }
    return at;
  }

  /// Where a mark of the line from [start] to [end] would lie: after
  /// its spaces and tabs and a list item's bullet.
  static int _markStart(String text, int start, int end) {
    final at = _skipBlank(text, start, end);
    if (at + 1 < end &&
        '-*+'.contains(text[at]) &&
        _blank(text.codeUnitAt(at + 1))) {
      return _skipBlank(text, at + 1, end);
    }
    return at;
  }

  /// The mark of the line from [start] to [end] with its offsets, or
  /// null.
  static (TextMark, int, int)? _markIn(String text, int start, int end) {
    final at = _markStart(text, start, end);
    if (at >= end) return null;
    final c = text.codeUnitAt(at);
    for (final mark in TextMark.values) {
      if (mark.glyph.codeUnitAt(0) != c) continue;
      var stop = at + 1;
      if (stop < end) {
        final next = text.codeUnitAt(stop);
        if (next == _textStyle || next == _emojiStyle) stop++;
      }
      if (stop < end && text.codeUnitAt(stop) == _joiner) return null;
      return (mark, at, stop);
    }
    return null;
  }
}

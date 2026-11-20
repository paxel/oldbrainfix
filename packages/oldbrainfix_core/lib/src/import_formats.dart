import 'markdown.dart' show Markdown;

/// A kind of document of another tool an Import takes besides Markdown
/// and text (#348, ADR 0036): each becomes a formatted Note in the app's
/// Markdown subset (ADR 0033).
enum ImportFormat { asciiDoc, org, restructuredText, html }

/// A document read for an Import: the [title] its page names apart from
/// its text, HTML's `<title>`, empty for none, and its [text] in the
/// Markdown subset.
class ImportedDocument {
  final String title;
  final String text;

  const ImportedDocument(this.title, this.text);
}

/// AsciiDoc, Org, reStructuredText and HTML read into the app's Markdown
/// subset (ADR 0033, ADR 0036), by hand and the same in both cores
/// (`import_formats.rs`). What the subset has becomes its Markdown: two
/// heading levels, bold, italic, strike-through, inline code, links,
/// lists one level in, quotes, code blocks and tables. What it lacks
/// stays as the characters it is, escaped so that it reads as itself; in
/// HTML, whose marks are no text, its words stay. Nothing of a document
/// is run or fetched: an include stays the line it is, a script or a
/// style goes with its content, an image is its description, and no
/// other file is read.
abstract final class ImportFormats {
  /// The format of a file whose extension is [extension], in lower case
  /// ASCII letters; null for any other.
  static ImportFormat? ofExtension(String extension) => switch (extension) {
    'adoc' || 'asciidoc' || 'asc' => ImportFormat.asciiDoc,
    'org' => ImportFormat.org,
    'rst' => ImportFormat.restructuredText,
    'html' || 'htm' => ImportFormat.html,
    _ => null,
  };

  /// [text], a document in [format] with its line breaks as `\n`, in the
  /// Markdown subset.
  static ImportedDocument read(ImportFormat format, String text) =>
      switch (format) {
        ImportFormat.asciiDoc => ImportedDocument('', _asciiDoc(text)),
        ImportFormat.org => ImportedDocument('', _org(text)),
        ImportFormat.restructuredText => ImportedDocument('', _rst(text)),
        ImportFormat.html => _Html(text).read(),
      };
}

// ------------------------------------------------------------ characters

const _tab = 0x09;
const _space = 0x20;
const _quote = 0x22;
const _hash = 0x23;
const _amp = 0x26;
const _apos = 0x27;
const _paren = 0x28;
const _parenEnd = 0x29;
const _star = 0x2a;
const _plus = 0x2b;
const _minus = 0x2d;
const _dot = 0x2e;
const _slash = 0x2f;
const _colon = 0x3a;
const _semicolon = 0x3b;
const _lt = 0x3c;
const _equals = 0x3d;
const _gt = 0x3e;
const _bracket = 0x5b;
const _backslash = 0x5c;
const _bracketEnd = 0x5d;
const _caret = 0x5e;
const _underscore = 0x5f;
const _backtick = 0x60;
const _pipe = 0x7c;
const _tilde = 0x7e;

/// A space or a tab, the only blanks the lightweight formats know.
bool _ws(int c) => c == _space || c == _tab;

bool _digit(int c) => c >= 0x30 && c <= 0x39;

bool _letter(int c) => (c | 0x20) >= 0x61 && (c | 0x20) <= 0x7a;

bool _alnum(int c) => _digit(c) || _letter(c);

/// A character of a word: an ASCII letter or digit, `_`, or any beyond
/// ASCII.
bool _word(int c) => _alnum(c) || c == _underscore || c >= 0x80;

bool _punct(int c) =>
    (c >= 0x21 && c <= 0x2f) ||
    (c >= 0x3a && c <= 0x40) ||
    (c >= 0x5b && c <= 0x60) ||
    (c >= 0x7b && c <= 0x7e);

int _lower(int c) => c >= 0x41 && c <= 0x5a ? c + 0x20 : c;

/// The code points of [text]; every format is read by them, as the Rust
/// core reads its `char`s.
List<int> _cps(String text) => text.runes.toList();

String _str(List<int> c, [int from = 0, int? to]) =>
    String.fromCharCodes(c, from, to);

/// Whether [c] holds the ASCII [s] at [i], in any case of ASCII letters
/// with [anyCase].
bool _at(List<int> c, int i, String s, {bool anyCase = false}) {
  if (i < 0 || i + s.length > c.length) return false;
  for (var j = 0; j < s.length; j++) {
    final d = anyCase ? _lower(c[i + j]) : c[i + j];
    if (d != s.codeUnitAt(j)) return false;
  }
  return true;
}

/// How many spaces and tabs [c] starts with.
int _indent(List<int> c) {
  var i = 0;
  while (i < c.length && _ws(c[i])) {
    i++;
  }
  return i;
}

String _trimStart(String s) {
  var i = 0;
  while (i < s.length && _ws(s.codeUnitAt(i))) {
    i++;
  }
  return s.substring(i);
}

String _trimEnd(String s) {
  var i = s.length;
  while (i > 0 && _ws(s.codeUnitAt(i - 1))) {
    i--;
  }
  return s.substring(0, i);
}

String _trim(String s) => _trimStart(_trimEnd(s));

bool _blank(String s) => _trim(s).isEmpty;

/// The first word of [s], up to a space or a tab.
String _firstWord(String s) {
  final t = _trimStart(s);
  var i = 0;
  while (i < t.length && !_ws(t.codeUnitAt(i))) {
    i++;
  }
  return t.substring(0, i);
}

/// The first `k` from [from] up to [to] that is [ok]; -1 for none.
int _find(int from, int to, bool Function(int k) ok) {
  for (var k = from; k < to; k++) {
    if (ok(k)) return k;
  }
  return -1;
}

/// [lines] without the spaces and tabs they all start with, a blank one
/// empty.
List<String> _dedent(List<String> lines) {
  int? least;
  for (final line in lines) {
    if (_blank(line)) continue;
    final n = _indent(_cps(line));
    if (least == null || n < least) least = n;
  }
  return [
    for (final line in lines) _blank(line) ? '' : _str(_cps(line), least ?? 0),
  ];
}

// ----------------------------------------------------------------- spans

const _bold = 1;
const _italic = 2;
const _strike = 4;
const _code = 8;

/// A run of text in one style, part of a link to [link] when set.
class _Span {
  final String text;
  final int style;
  final String? link;

  const _Span(this.text, this.style, [this.link]);
}

/// [href] as a link's destination: when the subset can write it, without
/// spaces, angle brackets or backslashes and with its parentheses
/// balanced, and the apps may open it (`Markdown.mayOpen`); else null and
/// its label is text.
String? _target(String href) {
  if (href.isEmpty) return null;
  var depth = 0;
  for (final c in href.codeUnits) {
    if (c <= _space || c == 0x7f || c == _lt || c == _gt || c == _backslash) {
      return null;
    }
    if (c == _paren) depth++;
    if (c == _parenEnd && --depth < 0) return null;
  }
  return depth == 0 && Markdown.mayOpen(href) ? href : null;
}

/// Whether [c], beside a `_`, keeps it from reading as a mark: an ASCII
/// letter or digit, or a character beyond ASCII that is no space.
bool _plainBeside(int c) =>
    _alnum(c) ||
    (c >= 0x80 &&
        c != 0x85 &&
        c != 0xa0 &&
        c != 0x1680 &&
        !(c >= 0x2000 && c <= 0x200a) &&
        c != 0x2028 &&
        c != 0x2029 &&
        c != 0x202f &&
        c != 0x205f &&
        c != 0x3000 &&
        c != 0xfeff);

/// Where the mark lies that the line [c] would start with in the subset:
/// a heading's `#`, a quote's `>`, a bulleted item's `-`, `*` or `+`, a
/// numbered item's `.`; -1 for none.
int _lineMark(List<int> c) {
  var i = 0;
  var width = 0;
  while (i < c.length && _ws(c[i])) {
    width += c[i] == _tab ? 4 : 1;
    i++;
  }
  if (i == c.length) return -1;
  bool ends(int k) => k >= c.length || _ws(c[k]);
  final mark = c[i];
  if (width <= 3 && mark == _hash) {
    if (ends(i + 1) || (c[i + 1] == _hash && ends(i + 2))) return i;
  }
  if (width <= 3 && mark == _gt) return i;
  if ((mark == _minus || mark == _star || mark == _plus) && ends(i + 1)) {
    return i;
  }
  var k = i;
  while (k < c.length && _digit(c[k])) {
    k++;
  }
  if (k > i && k - i <= 9 && k < c.length && c[k] == _dot && ends(k + 1)) {
    return k;
  }
  return -1;
}

/// [text], a part of a line, as it reads in the subset as itself, as
/// `Markdown.escape` writes it but in a time that grows with its length
/// alone: a `\` before the mark a line would start with, before every
/// `*`, `` ` ``, `[` and `|`, before a `~` next to another, before a `_`
/// not between two characters that are neither space nor punctuation,
/// and before a `\` before ASCII punctuation. A `\` also goes before
/// what a mark beside the part could join: a `~` at either end, a `!` or
/// a `\` at its end; in a link's [label] also before each `]`.
String _escape(String text, bool label) {
  final c = _cps(text);
  final mark = _lineMark(c);
  final out = StringBuffer();
  final last = c.length - 1;
  for (var i = 0; i < c.length; i++) {
    final ch = c[i];
    final escaped = switch (ch) {
      _ when i == mark => true,
      _backslash => i == last || _punct(c[i + 1]),
      _star || _backtick || _bracket || _pipe => true,
      _tilde => i == 0 || i == last || c[i - 1] == _tilde || c[i + 1] == _tilde,
      _underscore =>
        i == 0 ||
            i == last ||
            !_plainBeside(c[i - 1]) ||
            !_plainBeside(c[i + 1]),
      0x21 => i == last,
      _bracketEnd => label,
      _ => false,
    };
    if (escaped) out.writeCharCode(_backslash);
    out.writeCharCode(ch);
  }
  return out.toString();
}

/// Whether a web address starts at [i] of [c], as the apps find one: not
/// right after an ASCII letter or digit, and something after its scheme.
bool _addressAt(List<int> c, int i) {
  if (i > 0 && _alnum(c[i - 1])) return false;
  final after = _at(c, i, 'https://', anyCase: true)
      ? i + 8
      : _at(c, i, 'http://', anyCase: true)
      ? i + 7
      : -1;
  return after >= 0 &&
      after < c.length &&
      (_alnum(c[after]) || c[after] >= 0x80);
}

bool _addressEnds(int c) => c <= _space || c == 0x7f || c == _lt || c == _gt;

/// [text] as inline code, between backticks more than any run in it.
String _codeSpan(String text, bool cell) {
  final content = cell ? text.replaceAll('|', r'\|') : text;
  var longest = 0;
  var run = 0;
  for (final c in content.codeUnits) {
    run = c == _backtick ? run + 1 : 0;
    if (run > longest) longest = run;
  }
  final fence = '`' * (longest + 1);
  final pad = content.startsWith('`') || content.endsWith('`') ? ' ' : '';
  return '$fence$pad$content$pad$fence';
}

String _mark(int style) => switch (style) {
  _strike => '~~',
  _bold => '**',
  _ => '*',
};

/// [text] split into the spaces and tabs it starts with, its core and the
/// spaces and tabs it ends with.
(String, String, String) _edges(String text) {
  var a = 0;
  while (a < text.length && _ws(text.codeUnitAt(a))) {
    a++;
  }
  var b = text.length;
  while (b > a && _ws(text.codeUnitAt(b - 1))) {
    b--;
  }
  return (text.substring(0, a), text.substring(a, b), text.substring(b));
}

/// [spans] of no link, or of one link's label, with their marks: each
/// mark opened where its style starts and closed where it ends, in the
/// order they nest, and the blanks at a span's ends outside them.
String _styled(List<_Span> spans, {required bool cell, required bool label}) {
  final out = StringBuffer();
  final open = <int>[];
  var pending = '';
  for (final span in spans) {
    final (lead, core, trail) = _edges(span.text);
    if (core.isEmpty) {
      pending += span.text;
      continue;
    }
    final want = span.style & (_strike | _bold | _italic);
    var keep = 0;
    while (keep < open.length && want & open[keep] != 0) {
      keep++;
    }
    for (var k = open.length - 1; k >= keep; k--) {
      out.write(_mark(open[k]));
    }
    open.removeRange(keep, open.length);
    out
      ..write(pending)
      ..write(lead);
    for (final style in const [_strike, _bold, _italic]) {
      if (want & style != 0 && !open.contains(style)) {
        out.write(_mark(style));
        open.add(style);
      }
    }
    out.write(
      span.style & _code != 0 ? _codeSpan(core, cell) : _escape(core, label),
    );
    pending = trail;
  }
  for (var k = open.length - 1; k >= 0; k--) {
    out.write(_mark(open[k]));
  }
  out.write(pending);
  return out.toString();
}

/// [spans] as one line of the subset, without the blanks at its end; in
/// a table's [cell] every `|` escaped.
String _inline(List<_Span> spans, {bool cell = false}) {
  final merged = <_Span>[];
  final texts = <StringBuffer>[];
  for (final span in spans) {
    if (span.text.isEmpty) continue;
    if (merged.isNotEmpty &&
        merged.last.style == span.style &&
        merged.last.link == span.link) {
      texts.last.write(span.text);
    } else {
      merged.add(span);
      texts.add(StringBuffer(span.text));
    }
  }
  final runs = [
    for (final (i, span) in merged.indexed)
      _Span(texts[i].toString(), span.style, span.link),
  ];
  final out = StringBuffer();
  var i = 0;
  while (i < runs.length) {
    final link = runs[i].link;
    var j = i + 1;
    while (j < runs.length && runs[j].link == link) {
      j++;
    }
    final group = runs.sublist(i, j);
    if (link == null || (cell && link.contains('|'))) {
      out.write(_styled(group, cell: cell, label: link != null));
    } else {
      final (lead, core, trail) = _edges(
        _styled(group, cell: cell, label: true),
      );
      final shown = core.isEmpty ? _escape(link, true) : core;
      out.write('$lead[$shown]($link)$trail');
    }
    i = j;
  }
  return _trimEnd(out.toString());
}

/// The lines of the Markdown a document becomes.
class _Out {
  final _lines = <String>[];

  /// A blank line, unless one is there already or nothing is.
  void blank() {
    if (_lines.isNotEmpty && _lines.last.isNotEmpty) _lines.add('');
  }

  void heading(int level, List<_Span> spans) {
    final text = _trimStart(_inline(spans));
    if (text.isNotEmpty) _lines.add('${level == 1 ? '#' : '##'} $text');
  }

  /// A bulleted item, or a numbered one with [number]; [level] 1 is one
  /// level in.
  void item(int level, int? number, List<_Span> spans) {
    final text = _trimStart(_inline(spans));
    if (text.isEmpty) return;
    final mark = number == null
        ? '-'
        : '${number > 999999999 ? 999999999 : number}.';
    _lines.add('${level == 0 ? '' : '  '}$mark $text');
  }

  void quote(List<_Span> spans) {
    final text = _trimStart(_inline(spans));
    _lines.add(text.isEmpty ? '>' : '> $text');
  }

  void paragraph(List<_Span> spans) {
    final text = _inline(spans);
    if (_trimStart(text).isEmpty) {
      blank();
    } else {
      _lines.add(text);
    }
  }

  /// [line] as it is written, escaped.
  void written(String line) => paragraph([_Span(line, 0)]);

  /// A code block of [lines], fenced by more backticks than any run in
  /// them, at least three; [language] its info string.
  void code(String language, List<String> lines) {
    var longest = 0;
    for (final line in lines) {
      var run = 0;
      for (final c in line.codeUnits) {
        run = c == _backtick ? run + 1 : 0;
        if (run > longest) longest = run;
      }
    }
    final fence = '`' * (longest < 3 ? 3 : longest + 1);
    final word = _firstWord(language);
    _lines
      ..add(word.contains('`') ? fence : '$fence$word')
      ..addAll(lines)
      ..add(fence);
  }

  /// A table of [rows], the first one its header row, as many columns as
  /// the widest; on lines of its own, a blank line before and after it.
  void table(List<List<List<_Span>>> rows) {
    if (rows.isEmpty) return;
    var width = 1;
    for (final row in rows) {
      if (row.length > width) width = row.length;
    }
    String line(List<List<_Span>> row) {
      final cells = [
        for (var i = 0; i < width; i++)
          i < row.length ? _trimStart(_inline(row[i], cell: true)) : '',
      ];
      return '| ${cells.join(' | ')} |';
    }

    blank();
    _lines
      ..add(line(rows.first))
      ..add('| ${List.filled(width, '---').join(' | ')} |');
    for (final row in rows.skip(1)) {
      _lines.add(line(row));
    }
    _lines.add('');
  }

  String text() {
    while (_lines.isNotEmpty && _lines.last.isEmpty) {
      _lines.removeLast();
    }
    return _lines.join('\n');
  }
}

/// Collects the spans an inline reader finds, its plain characters in a
/// run until a span of another kind comes.
class _Inline {
  final out = <_Span>[];
  final _run = StringBuffer();
  int _style = 0;
  String? _link;

  void char(int c, int style, String? link) {
    if (style != _style || link != _link) flush();
    _style = style;
    _link = link;
    _run.writeCharCode(c);
  }

  void text(String text, int style, String? link) {
    for (final c in text.runes) {
      char(c, style, link);
    }
  }

  void span(String text, int style, String? link) {
    flush();
    if (text.isNotEmpty) out.add(_Span(text, style, link));
  }

  void flush() {
    if (_run.isEmpty) return;
    out.add(_Span(_run.toString(), _style, _link));
    _run.clear();
  }

  bool get isEmpty => out.isEmpty && _run.isEmpty;

  List<_Span> take() {
    flush();
    final spans = List.of(out);
    out.clear();
    return spans;
  }
}

/// Where a closing mark was looked for and none found: none comes after
/// either, so it is not looked for again, and a line is read in a time
/// that grows with its length alone.
class _Missing {
  final _keys = <String>{};

  int find(String key, int from, int to, bool Function(int k) ok) {
    if (_keys.contains(key)) return -1;
    final k = _find(from, to, ok);
    if (k < 0) _keys.add(key);
    return k;
  }
}

// -------------------------------------------------------------- AsciiDoc

/// The delimiter [bare] is, a line without blanks at its end: four or
/// more `-` (listing), `.` (literal), `+` (passthrough), `/` (comment) or
/// `_` (quote), or `|===` (table), by that character; null for any
/// other line.
int? _adocDelimiter(String bare) {
  final c = _cps(bare);
  if (c.length >= 4 && c[0] == _pipe && c.skip(1).every((d) => d == _equals)) {
    return _pipe;
  }
  if (c.length < 4 || !c.every((d) => d == c[0])) return null;
  return const [_minus, _dot, _plus, _slash, _underscore].contains(c[0])
      ? c[0]
      : null;
}

/// [bare] without AsciiDoc's line break ` +` at its end.
String _adocBreak(String bare) =>
    bare.endsWith(' +') ? _trimEnd(bare.substring(0, bare.length - 2)) : bare;

String _asciiDoc(String text) {
  final out = _Out();
  final lines = text.split('\n');
  final counts = [0, 0];

  /// The delimited block opening at [at] in [lines], a listing or literal
  /// one in [language]; the index after it.
  int block(int at, String language) {
    final open = _trimEnd(lines[at]);
    final kind = _adocDelimiter(open);
    var end = at + 1;
    while (end < lines.length && _trimEnd(lines[end]) != open) {
      end++;
    }
    final inside = lines.sublist(at + 1, end);
    switch (kind) {
      case _minus || _dot:
        out.code(language, inside);
      case _underscore:
        for (final line in inside) {
          if (_blank(line)) {
            out.blank();
          } else {
            out.quote(_adocSpans(_adocBreak(_trim(line))));
          }
        }
        out.blank();
      case _pipe:
        out.table(_adocTable(inside));
      default:
        out.written(lines[at]);
        inside.forEach(out.written);
        if (end < lines.length) out.written(lines[end]);
    }
    counts[0] = counts[1] = 0;
    return end + 1;
  }

  var i = 0;
  while (i < lines.length) {
    final line = lines[i];
    final bare = _trimEnd(line);
    final c = _cps(bare);
    if (_adocDelimiter(bare) != null) {
      i = block(i, '');
      continue;
    }
    if (bare.isEmpty) {
      out.blank();
      i++;
      continue;
    }
    // A block's attributes, such as `[source,python]` over a listing,
    // which names its language.
    if (c[0] == _bracket && c.last == _bracketEnd) {
      final next = i + 1 < lines.length
          ? _adocDelimiter(_trimEnd(lines[i + 1]))
          : null;
      if (next == _minus || next == _dot) {
        final attributes = bare.substring(1, bare.length - 1).split(',');
        final language =
            attributes.length > 1 && _trim(attributes[0]) == 'source'
            ? _trim(attributes[1])
            : '';
        i = block(i + 1, language);
        continue;
      }
      out.written(line);
      i++;
      continue;
    }
    final item = _adocItem(c, counts);
    if (item != null) {
      out.item(
        item.level,
        item.number,
        _adocSpans(_adocBreak(_str(c, item.start))),
      );
      i++;
      continue;
    }
    counts[0] = counts[1] = 0;
    var level = 0;
    while (level < c.length && c[level] == _equals) {
      level++;
    }
    if (level >= 1 && level <= 6 && level < c.length && _ws(c[level])) {
      if (level > 2) {
        out.written(line);
      } else {
        var title = _trim(_str(c, level));
        // A title may close with its marks again.
        final closing = title.lastIndexOf(' =');
        if (closing >= 0 &&
            title.substring(closing + 1).codeUnits.every((d) => d == _equals)) {
          title = _trimEnd(title.substring(0, closing));
        }
        out.heading(level, _adocSpans(title));
      }
    } else if (_adocWritten(c)) {
      out.written(line);
    } else if (c[0] == _gt) {
      out.quote(_adocSpans(_adocBreak(_str(c, 1))));
    } else {
      out.paragraph(_adocSpans(_adocBreak(bare)));
    }
    i++;
  }
  return out.text();
}

/// Whether the AsciiDoc line [c] stays as it is written: a comment, an
/// attribute entry, a block macro such as `include::` or `image::`, a
/// block title, or a literal line, indented.
bool _adocWritten(List<int> c) {
  if (_ws(c[0])) return true;
  if (_at(c, 0, '//')) return true;
  if (c[0] == _colon) {
    final k = _find(1, c.length, (k) => c[k] == _colon);
    if (k > 1 &&
        c.sublist(1, k).every((d) => _word(d) || d == _minus || d == 0x21) &&
        (k + 1 == c.length || _ws(c[k + 1]))) {
      return true;
    }
  }
  if (_letter(c[0])) {
    var k = 0;
    while (k < c.length &&
        (_alnum(c[k]) || c[k] == _minus || c[k] == _underscore)) {
      k++;
    }
    if (_at(c, k, '::') && c.last == _bracketEnd) return true;
  }
  return c[0] == _dot && c.length > 1 && !_ws(c[1]) && c[1] != _dot;
}

/// The list item the AsciiDoc line [c] is: `*` to `*****` or `-` for a
/// bulleted one, `.` to `.....` or a number and `.` for a numbered one,
/// each with a blank after it, [counts] counting the numbered ones of
/// each level; null for any other line.
({int level, int? number, int start})? _adocItem(
  List<int> c,
  List<int> counts,
) {
  final at = _indent(c);
  if (at >= c.length) return null;
  final mark = c[at];
  var run = at;
  while (run < c.length && c[run] == mark) {
    run++;
  }
  final n = run - at;
  final spaced = run < c.length && _ws(c[run]);
  if (mark == _star && n <= 5 && spaced) {
    return (level: n > 1 ? 1 : 0, number: null, start: run + 1);
  }
  if (mark == _minus && n == 1 && spaced) {
    return (level: 0, number: null, start: run + 1);
  }
  if (mark == _dot && n <= 5 && spaced) {
    final level = n > 1 ? 1 : 0;
    final number = ++counts[level];
    if (level == 0) counts[1] = 0;
    return (level: level, number: number, start: run + 1);
  }
  var digits = at;
  while (digits < c.length && _digit(c[digits])) {
    digits++;
  }
  if (digits > at &&
      digits - at <= 9 &&
      _at(c, digits, '.') &&
      digits + 1 < c.length &&
      _ws(c[digits + 1])) {
    final number = int.parse(_str(c, at, digits));
    counts[0] = number;
    counts[1] = 0;
    return (level: 0, number: number, start: digits + 2);
  }
  return null;
}

/// The cells of an AsciiDoc table's [lines], in rows as many as the first
/// line holding cells has; a line that holds none goes on with the cell
/// before it.
List<List<List<_Span>>> _adocTable(List<String> lines) {
  final cells = <String>[];
  var width = 0;
  for (final line in lines) {
    final t = _trim(line);
    if (t.isEmpty) continue;
    final c = _cps(t);
    final bar = _find(0, c.length, (k) => c[k] == _pipe);
    final spec =
        bar >= 0 &&
        bar <= 6 &&
        c
            .sublist(0, bar)
            .every(
              (d) =>
                  _alnum(d) ||
                  d == _dot ||
                  d == _plus ||
                  d == _star ||
                  d == _lt ||
                  d == _gt ||
                  d == _caret,
            );
    if (!spec) {
      if (cells.isEmpty) {
        cells.add(t);
      } else {
        cells[cells.length - 1] = '${cells.last} $t';
      }
      continue;
    }
    var found = 0;
    var from = bar + 1;
    for (var k = bar + 1; k <= c.length; k++) {
      if (k < c.length && (c[k] != _pipe || c[k - 1] == _backslash)) continue;
      cells.add(_trim(_str(c, from, k).replaceAll(r'\|', '|')));
      found++;
      from = k + 1;
    }
    if (width == 0) width = found;
  }
  if (width == 0) width = 1;
  return [
    for (var r = 0; r < cells.length; r += width)
      [
        for (final cell in cells.sublist(
          r,
          r + width > cells.length ? cells.length : r + width,
        ))
          _adocSpans(cell),
      ],
  ];
}

List<_Span> _adocSpans(String text) {
  final into = _Inline();
  _adocInline(_cps(text), 0, text.runes.length, 0, null, into);
  return into.take();
}

/// The characters a `\` before them shows as themselves in AsciiDoc.
bool _adocEscapes(int c) =>
    c == _star ||
    c == _underscore ||
    c == _backtick ||
    c == _hash ||
    c == _plus ||
    c == _bracket ||
    c == _lt ||
    c == 0x7b ||
    c == _backslash ||
    c == _caret ||
    c == _tilde;

/// [text] without the `+` around it that AsciiDoc's `` `+text+` `` keeps
/// as written.
String _unplus(String text) =>
    text.length >= 2 && text.startsWith('+') && text.endsWith('+')
    ? text.substring(1, text.length - 1)
    : text;

/// Reads [c] from [from] to [to] in [style], a link's label when [link]
/// is set: `*bold*` and `**bold**`, `_italic_` and `__italic__`,
/// `` `code` `` and ``` ``code`` ```, `+text+` and `++text++` as written,
/// `[.line-through]#text#`, and web addresses, `link:` and `mailto:`
/// with a label in `[…]`; anything else as written.
void _adocInline(
  List<int> c,
  int from,
  int to,
  int style,
  String? link,
  _Inline into,
) {
  final missing = _Missing();
  bool opens(int i) =>
      (i == from || !_word(c[i - 1])) && i + 1 < to && !_ws(c[i + 1]);
  bool closes(int k) => k + 1 >= to || !_word(c[k + 1]);
  var i = from;
  while (i < to) {
    final ch = c[i];
    if (ch == _backslash && i + 1 < to && _adocEscapes(c[i + 1])) {
      into.char(c[i + 1], style, link);
      i += 2;
      continue;
    }
    if (link == null && (i == from || !_word(c[i - 1]))) {
      final address = _addressAt(c, i);
      final macro = _at(c, i, 'link:')
          ? 5
          : _at(c, i, 'mailto:')
          ? 0
          : -1;
      if (address || macro >= 0) {
        final start = i + (macro > 0 ? macro : 0);
        var end = start;
        while (end < to && !_addressEnds(c[end]) && c[end] != _bracket) {
          end++;
        }
        final close = end < to && c[end] == _bracket
            ? missing.find(']', end + 1, to, (k) => c[k] == _bracketEnd)
            : -1;
        if (close >= 0 && end > start) {
          final href = _str(c, start, end);
          final dest = _target(href);
          var labelEnd = close;
          if (labelEnd > end + 1 && c[labelEnd - 1] == _caret) labelEnd--;
          if (labelEnd > end + 1) {
            _adocInline(c, end + 1, labelEnd, style, dest, into);
          } else {
            into.span(href, style, dest);
          }
          i = close + 1;
          continue;
        }
        if (address) {
          into.text(_str(c, i, end), style, link);
          i = end;
          continue;
        }
      }
    }
    if (ch == _backtick && i + 1 < to && c[i + 1] == _backtick) {
      final k = missing.find(
        '``',
        i + 3,
        to - 1,
        (k) => c[k] == _backtick && c[k + 1] == _backtick,
      );
      if (k >= 0) {
        into.span(_unplus(_str(c, i + 2, k)), style | _code, link);
        i = k + 2;
        continue;
      }
    } else if (ch == _backtick && opens(i)) {
      final k = missing.find(
        '`',
        i + 2,
        to,
        (k) => c[k] == _backtick && !_ws(c[k - 1]) && closes(k),
      );
      if (k >= 0) {
        into.span(_unplus(_str(c, i + 1, k)), style | _code, link);
        i = k + 1;
        continue;
      }
    }
    final bit = ch == _star
        ? _bold
        : ch == _underscore
        ? _italic
        : 0;
    if (bit != 0 && style & bit == 0) {
      if (i + 1 < to && c[i + 1] == ch) {
        if (i + 2 < to && !_ws(c[i + 2])) {
          final k = missing.find(
            String.fromCharCodes([ch, ch]),
            i + 3,
            to - 1,
            (k) => c[k] == ch && c[k + 1] == ch && !_ws(c[k - 1]),
          );
          if (k >= 0) {
            _adocInline(c, i + 2, k, style | bit, link, into);
            i = k + 2;
            continue;
          }
        }
      } else if (opens(i)) {
        final k = missing.find(
          String.fromCharCode(ch),
          i + 2,
          to,
          (k) => c[k] == ch && !_ws(c[k - 1]) && closes(k),
        );
        if (k >= 0) {
          _adocInline(c, i + 1, k, style | bit, link, into);
          i = k + 1;
          continue;
        }
      }
    }
    if (ch == _plus) {
      final double = i + 1 < to && c[i + 1] == _plus;
      final k = double
          ? missing.find(
              '++',
              i + 3,
              to - 1,
              (k) => c[k] == _plus && c[k + 1] == _plus,
            )
          : opens(i)
          ? missing.find(
              '+',
              i + 2,
              to,
              (k) => c[k] == _plus && !_ws(c[k - 1]) && closes(k),
            )
          : -1;
      if (k >= 0) {
        into.text(_str(c, i + (double ? 2 : 1), k), style, link);
        i = k + (double ? 2 : 1);
        continue;
      }
    }
    if (ch == _bracket &&
        style & _strike == 0 &&
        _at(c, i, '[.line-through]#')) {
      final k = missing.find('#', i + 17, to, (k) => c[k] == _hash);
      if (k >= 0) {
        _adocInline(c, i + 16, k, style | _strike, link, into);
        i = k + 1;
        continue;
      }
    }
    into.char(ch, style, link);
    i++;
  }
}

// ------------------------------------------------------------------- Org

String _lowerStr(List<int> c, int from, int to) =>
    String.fromCharCodes([for (var i = from; i < to; i++) _lower(c[i])]);

/// [text] without Org's line break `\\` at its end.
String _orgBreak(String text) {
  final bare = _trimEnd(text);
  return bare.endsWith(r'\\')
      ? _trimEnd(bare.substring(0, bare.length - 2))
      : bare;
}

/// The list item the Org line [c], indented by [at], is: `-` or `+`, or
/// `*` when indented, for a bulleted one, a number and `.` or `)` for a
/// numbered one, each with a blank or the line's end after it; null for
/// any other line.
({int? number, int start})? _orgItem(List<int> c, int at) {
  final mark = c[at];
  if ((mark == _minus || mark == _plus || (mark == _star && at > 0)) &&
      (at + 1 == c.length || _ws(c[at + 1]))) {
    return (number: null, start: at + 1);
  }
  var digits = at;
  while (digits < c.length && _digit(c[digits])) {
    digits++;
  }
  if (digits > at &&
      digits - at <= 9 &&
      digits < c.length &&
      (c[digits] == _dot || c[digits] == _parenEnd) &&
      (digits + 1 == c.length || _ws(c[digits + 1]))) {
    return (number: int.parse(_str(c, at, digits)), start: digits + 1);
  }
  return null;
}

/// The lines of an Org source or example block: without the indent they
/// all have, a `,` that keeps a `*` or `#+` at a line's start from
/// reading as Org taken off.
List<String> _orgCode(List<String> lines) => [
  for (final line in _dedent(lines))
    if (_cps(line) case final c
        when _at(c, _indent(c), ',*') || _at(c, _indent(c), ',#+'))
      _str(c, 0, _indent(c)) + _str(c, _indent(c) + 1)
    else
      line,
];

/// The cells of the Org table row [row], a line between `|`.
List<List<_Span>> _orgRow(String row) {
  var inner = row.substring(1);
  if (inner.endsWith('|')) inner = inner.substring(0, inner.length - 1);
  return [for (final cell in inner.split('|')) _orgSpans(_trim(cell))];
}

String _org(String text) {
  final out = _Out();
  final lines = text.split('\n');
  int? base;
  var i = 0;
  while (i < lines.length) {
    final line = lines[i];
    final c = _cps(_trimEnd(line));
    final at = _indent(c);
    if (at == c.length) {
      out.blank();
      i++;
      continue;
    }
    var stars = 0;
    while (stars < c.length && c[stars] == _star) {
      stars++;
    }
    if (stars > 0 && stars < c.length && _ws(c[stars])) {
      base = null;
      if (stars > 2) {
        out.written(line);
      } else {
        out.heading(stars, _orgSpans(_trim(_str(c, stars))));
      }
      i++;
      continue;
    }
    if (_at(c, at, '#+begin_', anyCase: true)) {
      var k = at + 8;
      while (k < c.length && !_ws(c[k])) {
        k++;
      }
      final name = _lowerStr(c, at + 8, k);
      final args = _trim(_str(c, k));
      var end = i + 1;
      while (end < lines.length &&
          !_at(_cps(_trimStart(lines[end])), 0, '#+end_$name', anyCase: true)) {
        end++;
      }
      final inside = lines.sublist(i + 1, end);
      switch (name) {
        case 'src':
          out.code(_firstWord(args), _orgCode(inside));
        case 'example':
          out.code('', _orgCode(inside));
        case 'quote':
          for (final line in inside) {
            if (_blank(line)) {
              out.blank();
            } else {
              out.quote(_orgSpans(_orgBreak(_trim(line))));
            }
          }
          out.blank();
        default:
          out.written(line);
          inside.forEach(out.written);
          if (end < lines.length) out.written(lines[end]);
      }
      base = null;
      i = end + 1;
      continue;
    }
    if (_at(c, at, '#+title:', anyCase: true) && !_blank(_str(c, at + 8))) {
      out.heading(1, _orgSpans(_trim(_str(c, at + 8))));
      base = null;
      i++;
      continue;
    }
    // Keywords, comments, drawers and fixed-width lines.
    if ((c[at] == _hash &&
            (at + 1 == c.length || _ws(c[at + 1]) || c[at + 1] == _plus)) ||
        c[at] == _colon) {
      out.written(line);
      i++;
      continue;
    }
    if (c[at] == _pipe) {
      final rows = <List<List<_Span>>>[];
      while (i < lines.length) {
        final row = _trim(lines[i]);
        if (!row.startsWith('|')) break;
        if (!row.startsWith('|-')) rows.add(_orgRow(row));
        i++;
      }
      out.table(rows);
      base = null;
      continue;
    }
    final item = _orgItem(c, at);
    if (item != null) {
      final first = base ?? at;
      base = first;
      out.item(
        at > first ? 1 : 0,
        item.number,
        _orgSpans(_orgBreak(_str(c, item.start))),
      );
      i++;
      continue;
    }
    if (base != null && at <= base) base = null;
    out.paragraph(_orgSpans(_orgBreak(_str(c, at))));
    i++;
  }
  return out.text();
}

List<_Span> _orgSpans(String text) {
  final into = _Inline();
  final c = _cps(text);
  _orgInline(c, 0, c.length, 0, null, into);
  return into.take();
}

/// What may stand before an Org mark that opens.
bool _orgBefore(int c) =>
    _ws(c) ||
    c == _minus ||
    c == _paren ||
    c == 0x7b ||
    c == _apos ||
    c == _quote;

/// What may stand after an Org mark that closes.
bool _orgAfter(int c) =>
    _ws(c) ||
    c == _minus ||
    c == _dot ||
    c == 0x2c ||
    c == _semicolon ||
    c == _colon ||
    c == 0x21 ||
    c == 0x3f ||
    c == _apos ||
    c == _parenEnd ||
    c == 0x7d ||
    c == _bracket ||
    c == _quote ||
    c == _backslash;

/// Reads [c] from [from] to [to] in [style], a link's label when [link]
/// is set: `*bold*`, `/italic/`, `+strike-through+`, `=code=` and
/// `~code~`, links `[[address][label]]` and `[[address]]`, and web
/// addresses; anything else, `_underline_` among it, as written.
void _orgInline(
  List<int> c,
  int from,
  int to,
  int style,
  String? link,
  _Inline into,
) {
  final missing = _Missing();
  var i = from;
  while (i < to) {
    final ch = c[i];
    if (link == null && ch == _bracket && i + 1 < to && c[i + 1] == _bracket) {
      final k = missing.find(
        ']]',
        i + 2,
        to - 1,
        (k) => c[k] == _bracketEnd && c[k + 1] == _bracketEnd,
      );
      if (k >= 0) {
        final m = _find(
          i + 2,
          k - 1,
          (m) => c[m] == _bracketEnd && c[m + 1] == _bracket,
        );
        final href = _str(c, i + 2, m >= 0 ? m : k);
        final dest = _target(href);
        if (m >= 0 && m + 2 < k) {
          _orgInline(c, m + 2, k, style, dest, into);
        } else {
          into.span(href, style, dest);
        }
        i = k + 2;
        continue;
      }
    }
    if (link == null && _addressAt(c, i)) {
      var end = i;
      while (end < to && !_addressEnds(c[end])) {
        end++;
      }
      into.text(_str(c, i, end), style, link);
      i = end;
      continue;
    }
    final bit = switch (ch) {
      _star => _bold,
      _slash => _italic,
      _plus => _strike,
      _equals || _tilde => _code,
      _ => 0,
    };
    if (bit != 0 &&
        style & bit == 0 &&
        (i == from || _orgBefore(c[i - 1])) &&
        i + 1 < to &&
        !_ws(c[i + 1])) {
      final k = missing.find(
        String.fromCharCode(ch),
        i + 2,
        to,
        (k) =>
            c[k] == ch &&
            !_ws(c[k - 1]) &&
            (k + 1 == to || _orgAfter(c[k + 1])),
      );
      if (k >= 0) {
        if (bit == _code) {
          into.span(_str(c, i + 1, k), style | _code, link);
        } else {
          _orgInline(c, i + 1, k, style | bit, link, into);
        }
        i = k + 1;
        continue;
      }
    }
    into.char(ch, style, link);
    i++;
  }
}

// ------------------------------------------------------ reStructuredText

/// Whether [bare] is an adornment: two or more of one ASCII punctuation
/// character.
bool _adornment(String bare) {
  final c = _cps(bare);
  return c.length >= 2 && _punct(c[0]) && c.every((d) => d == c[0]);
}

/// The name, in lower case ASCII letters, and the arguments of the
/// directive `name:: arguments` in [c] after its `..` at [from]; null
/// when it is none, such as a comment or a target.
({String name, String args})? _rstDirective(List<int> c, int from) {
  var s = from;
  while (s < c.length && _ws(c[s])) {
    s++;
  }
  var k = s;
  while (k < c.length && !_ws(c[k]) && !_at(c, k, '::')) {
    k++;
  }
  if (k == s || !_at(c, k, '::')) return null;
  return (name: _lowerStr(c, s, k), args: _trim(_str(c, k + 2)));
}

/// The list item the reStructuredText line [c], indented by [at], is:
/// `-`, `*` or `+` for a bulleted one; a number or `#` with `.` or `)`
/// after it, or between `(` and `)`, for a numbered one, `#` counting on
/// ([auto]); each with a blank or the line's end after it. Null for any
/// other line.
({int? number, bool auto, int start})? _rstItem(List<int> c, int at) {
  bool spaced(int k) => k == c.length || (k < c.length && _ws(c[k]));
  final mark = c[at];
  if ((mark == _minus || mark == _star || mark == _plus) && spaced(at + 1)) {
    return (number: null, auto: false, start: at + 1);
  }
  final paren = mark == _paren;
  var k = paren ? at + 1 : at;
  int? number;
  var auto = false;
  if (k < c.length && c[k] == _hash) {
    auto = true;
    k++;
  } else {
    final digits = k;
    while (k < c.length && _digit(c[k])) {
      k++;
    }
    if (k == digits || k - digits > 9) return null;
    number = int.parse(_str(c, digits, k));
  }
  if (k >= c.length) return null;
  final close = c[k];
  if (paren ? close != _parenEnd : close != _dot && close != _parenEnd) {
    return null;
  }
  if (!spaced(k + 1)) return null;
  return (number: number, auto: auto, start: k + 1);
}

/// The rows of a grid table's [lines], each row's lines joined cell by
/// cell, the first row its header.
List<List<List<_Span>>> _gridRows(List<String> lines) {
  final rows = <List<List<_Span>>>[];
  var row = <String>[];
  void end() {
    if (row.any((cell) => cell.isNotEmpty)) {
      rows.add([for (final cell in row) _rstSpans(cell)]);
    }
    row = <String>[];
  }

  for (final line in lines) {
    final t = _trim(line);
    if (t.startsWith('+')) {
      end();
      continue;
    }
    var inner = t.substring(1);
    if (inner.endsWith('|')) inner = inner.substring(0, inner.length - 1);
    final parts = inner.split('|');
    for (var k = 0; k < parts.length; k++) {
      final part = _trim(parts[k]);
      while (row.length <= k) {
        row.add('');
      }
      if (part.isNotEmpty) row[k] = row[k].isEmpty ? part : '${row[k]} $part';
    }
  }
  end();
  return rows;
}

String _rst(String text) {
  final out = _Out();
  final lines = text.split('\n');
  final styles = <String>[];
  final counts = [0, 0];
  int? base;

  /// A section title in [style], as the subset's heading for the first
  /// two styles met and as [written] for those under them.
  void title(String style, String title, List<String> written) {
    if (!styles.contains(style)) styles.add(style);
    final level = styles.indexOf(style) + 1;
    if (level <= 2) {
      out.heading(level, _rstSpans(title));
    } else {
      written.forEach(out.written);
    }
    base = null;
  }

  /// The index after the lines from [from] on that are blank or indented
  /// more than [indent], without the blank ones at their end.
  int blockEnd(int from, int indent) {
    var end = from;
    while (end < lines.length &&
        (_blank(lines[end]) || _indent(_cps(lines[end])) > indent)) {
      end++;
    }
    while (end > from && _blank(lines[end - 1])) {
      end--;
    }
    return end;
  }

  var i = 0;
  while (i < lines.length) {
    final line = lines[i];
    final bare = _trimEnd(line);
    final c = _cps(bare);
    final at = _indent(c);
    if (at == c.length) {
      out.blank();
      i++;
      continue;
    }
    // A directive, a comment or a target, with what is indented under it.
    if (_at(c, at, '..') && (at + 2 == c.length || _ws(c[at + 2]))) {
      final end = blockEnd(i + 1, at);
      final directive = _rstDirective(c, at + 2);
      if (directive != null &&
          const {'code', 'code-block', 'sourcecode'}.contains(directive.name)) {
        final body = lines.sublist(i + 1, end);
        var options = 0;
        while (options < body.length &&
            _trimStart(body[options]).startsWith(':')) {
          options++;
        }
        final code = _dedent(body.sublist(options));
        var first = 0;
        while (first < code.length && code[first].isEmpty) {
          first++;
        }
        out.code(_firstWord(directive.args), code.sublist(first));
      } else {
        for (var k = i; k < end; k++) {
          out.written(lines[k]);
        }
      }
      base = null;
      i = end;
      continue;
    }
    if (at == 0 && _adornment(bare)) {
      if (i + 2 < lines.length &&
          !_blank(lines[i + 1]) &&
          _trimEnd(lines[i + 2]) == bare) {
        title('o${bare[0]}', _trim(lines[i + 1]), lines.sublist(i, i + 3));
        i += 3;
      } else {
        // A transition.
        out.written(line);
        i++;
      }
      continue;
    }
    if (at == 0 && i + 1 < lines.length) {
      final under = _trimEnd(lines[i + 1]);
      if (_adornment(under) && under.runes.length >= c.length) {
        title('u${under[0]}', bare, lines.sublist(i, i + 2));
        i += 2;
        continue;
      }
    }
    if (_at(c, at, '+-') || _at(c, at, '+=')) {
      var end = i;
      while (end < lines.length) {
        final t = _trimStart(lines[end]);
        if (!t.startsWith('+') && !t.startsWith('|')) break;
        end++;
      }
      out.table(_gridRows(lines.sublist(i, end)));
      base = null;
      i = end;
      continue;
    }
    // A field, such as `:Author: Ada`.
    if (c[at] == _colon) {
      out.written(line);
      i++;
      continue;
    }
    final item = _rstItem(c, at);
    if (item != null) {
      final first = base ?? at;
      base = first;
      final level = at > first ? 1 : 0;
      final number = item.number ?? (item.auto ? counts[level] + 1 : null);
      if (number != null) {
        counts[level] = number;
        if (level == 0) counts[1] = 0;
      }
      out.item(level, number, _rstSpans(_str(c, item.start)));
      i++;
      continue;
    }
    if (base case final b? when at <= b) {
      base = null;
      counts[0] = counts[1] = 0;
    }
    var content = _str(c, at);
    if (content.endsWith('::')) {
      // A literal block follows; its `::` shows as one `:`, or as
      // nothing after a blank or alone.
      content = content == '::'
          ? ''
          : _ws(content.codeUnitAt(content.length - 3))
          ? _trimEnd(content.substring(0, content.length - 2))
          : content.substring(0, content.length - 1);
      if (content.isNotEmpty) out.paragraph(_rstSpans(content));
      var from = i + 1;
      while (from < lines.length && _blank(lines[from])) {
        from++;
      }
      final end = blockEnd(from, at);
      if (end > from) {
        out.code('', _dedent(lines.sublist(from, end)));
        i = end;
      } else {
        i++;
      }
      continue;
    }
    out.paragraph(_rstSpans(content));
    i++;
  }
  return out.text();
}

List<_Span> _rstSpans(String text) {
  final into = _Inline();
  final c = _cps(text);
  _rstInline(c, 0, c.length, 0, null, into);
  return into.take();
}

/// What may stand before reStructuredText's inline markup.
bool _rstBefore(int c) =>
    _ws(c) ||
    c == _minus ||
    c == _colon ||
    c == _slash ||
    c == _apos ||
    c == _quote ||
    c == _lt ||
    c == _paren ||
    c == _bracket ||
    c == 0x7b;

/// What may stand after reStructuredText's inline markup.
bool _rstAfter(int c) =>
    _ws(c) ||
    c == _minus ||
    c == _dot ||
    c == 0x2c ||
    c == _colon ||
    c == _semicolon ||
    c == 0x21 ||
    c == 0x3f ||
    c == _backslash ||
    c == _slash ||
    c == _apos ||
    c == _quote ||
    c == _parenEnd ||
    c == _bracketEnd ||
    c == 0x7d ||
    c == _gt;

/// The index after the one or two `_` of a reference from [from] on.
int _underscores(List<int> c, int from, int to) {
  var k = from;
  while (k < to && k - from < 2 && c[k] == _underscore) {
    k++;
  }
  return k;
}

/// The label and the address of the reference [c] holds from [from] to
/// [to], `label <address>`; the address null when it names none.
(String, String?) _rstReference(List<int> c, int from, int to) {
  if (to - from >= 2 && c[to - 1] == _gt) {
    var lt = to - 2;
    while (lt >= from && c[lt] != _lt) {
      lt--;
    }
    if (lt >= from && (lt == from || _ws(c[lt - 1]))) {
      return (_trim(_str(c, from, lt)), _str(c, lt + 1, to - 1));
    }
  }
  return (_str(c, from, to), null);
}

/// Reads [c] from [from] to [to] in [style]: `**bold**`, `*italic*`,
/// ``` ``code`` ```, the reference `` `label <address>`_ ``, a `\` before
/// a character, and web addresses; anything else, interpreted text and
/// roles among it, as written.
void _rstInline(
  List<int> c,
  int from,
  int to,
  int style,
  String? link,
  _Inline into,
) {
  final missing = _Missing();
  bool after(int p) => p >= to || _rstAfter(c[p]);
  var i = from;
  while (i < to) {
    final ch = c[i];
    if (ch == _backslash) {
      if (i + 1 < to && !_ws(c[i + 1])) into.char(c[i + 1], style, link);
      i += 2;
      continue;
    }
    if (link == null && _addressAt(c, i)) {
      var end = i;
      while (end < to && !_addressEnds(c[end])) {
        end++;
      }
      into.text(_str(c, i, end), style, link);
      i = end;
      continue;
    }
    final opens = i == from || _rstBefore(c[i - 1]);
    if (opens &&
        ch == _backtick &&
        i + 2 < to &&
        c[i + 1] == _backtick &&
        !_ws(c[i + 2])) {
      final k = missing.find(
        '``',
        i + 3,
        to - 1,
        (k) =>
            c[k] == _backtick &&
            c[k + 1] == _backtick &&
            !_ws(c[k - 1]) &&
            after(k + 2),
      );
      if (k >= 0) {
        into.span(_str(c, i + 2, k), style | _code, link);
        i = k + 2;
        continue;
      }
    } else if (opens && ch == _backtick && i + 1 < to && !_ws(c[i + 1])) {
      final k = missing.find(
        '`',
        i + 2,
        to,
        (k) =>
            c[k] == _backtick &&
            !_ws(c[k - 1]) &&
            after(_underscores(c, k + 1, to)),
      );
      if (k >= 0) {
        final end = _underscores(c, k + 1, to);
        if (end > k + 1) {
          final (label, href) = _rstReference(c, i + 1, k);
          final dest = href == null ? null : _target(href);
          into.span(label.isEmpty ? href ?? '' : label, style, dest);
        } else {
          into.text(_str(c, i, end), style, link);
        }
        i = end;
        continue;
      }
    }
    if (opens &&
        ch == _star &&
        style & _bold == 0 &&
        i + 2 < to &&
        c[i + 1] == _star &&
        !_ws(c[i + 2])) {
      final k = missing.find(
        '**',
        i + 3,
        to - 1,
        (k) =>
            c[k] == _star &&
            c[k + 1] == _star &&
            !_ws(c[k - 1]) &&
            after(k + 2),
      );
      if (k >= 0) {
        _rstInline(c, i + 2, k, style | _bold, link, into);
        i = k + 2;
        continue;
      }
    } else if (opens &&
        ch == _star &&
        style & _italic == 0 &&
        i + 1 < to &&
        c[i + 1] != _star &&
        !_ws(c[i + 1])) {
      final k = missing.find(
        '*',
        i + 2,
        to,
        (k) => c[k] == _star && !_ws(c[k - 1]) && after(k + 1),
      );
      if (k >= 0) {
        _rstInline(c, i + 1, k, style | _italic, link, into);
        i = k + 1;
        continue;
      }
    }
    into.char(ch, style, link);
    i++;
  }
}

// ------------------------------------------------------------------ HTML

bool _htmlSpace(int c) =>
    c == _space || c == _tab || c == 0x0a || c == 0x0c || c == 0x0d;

bool _tagChar(int c) => _alnum(c) || c == _minus || c == _colon;

/// [s] without the HTML blanks around it, those inside it one space.
String _collapse(String s) {
  final out = StringBuffer();
  var space = false;
  for (final c in s.runes) {
    if (_htmlSpace(c)) {
      space = true;
      continue;
    }
    if (space && out.isNotEmpty) out.write(' ');
    space = false;
    out.writeCharCode(c);
  }
  return out.toString();
}

/// The elements that end a line of text where they start and end.
const _htmlBlocks = {
  'address', 'article', 'aside', 'body', 'caption', 'center', 'dd', //
  'details', 'dialog', 'div', 'dl', 'dt', 'fieldset', 'figcaption', //
  'figure', 'footer', 'form', 'header', 'hgroup', 'html', 'legend', //
  'main', 'nav', 'section', 'summary',
};

/// The elements whose content is never text: a script, a style, a
/// template.
const _htmlDropped = {'script', 'style', 'template'};

/// The named character references read, the Latin-1 ones by their place
/// from U+00A0 on.
const _latin1 = [
  'nbsp', 'iexcl', 'cent', 'pound', 'curren', 'yen', 'brvbar', 'sect', //
  'uml', 'copy', 'ordf', 'laquo', 'not', 'shy', 'reg', 'macr', //
  'deg', 'plusmn', 'sup2', 'sup3', 'acute', 'micro', 'para', 'middot', //
  'cedil', 'sup1', 'ordm', 'raquo', 'frac14', 'frac12', 'frac34', 'iquest', //
  'Agrave', 'Aacute', 'Acirc', 'Atilde', 'Auml', 'Aring', 'AElig', 'Ccedil',
  'Egrave', 'Eacute', 'Ecirc', 'Euml', 'Igrave', 'Iacute', 'Icirc', 'Iuml',
  'ETH', 'Ntilde', 'Ograve', 'Oacute', 'Ocirc', 'Otilde', 'Ouml', 'times',
  'Oslash', 'Ugrave', 'Uacute', 'Ucirc', 'Uuml', 'Yacute', 'THORN', 'szlig',
  'agrave', 'aacute', 'acirc', 'atilde', 'auml', 'aring', 'aelig', 'ccedil',
  'egrave', 'eacute', 'ecirc', 'euml', 'igrave', 'iacute', 'icirc', 'iuml',
  'eth', 'ntilde', 'ograve', 'oacute', 'ocirc', 'otilde', 'ouml', 'divide',
  'oslash', 'ugrave', 'uacute', 'ucirc', 'uuml', 'yacute', 'thorn', 'yuml',
];

const _entities = {
  'quot': 0x22, 'amp': 0x26, 'apos': 0x27, 'lt': 0x3c, 'gt': 0x3e, //
  'OElig': 0x152, 'oelig': 0x153, 'Scaron': 0x160, 'scaron': 0x161, //
  'Yuml': 0x178, 'fnof': 0x192, 'circ': 0x2c6, 'tilde': 0x2dc, //
  'ensp': 0x2002, 'emsp': 0x2003, 'thinsp': 0x2009, 'zwnj': 0x200c, //
  'zwj': 0x200d, 'lrm': 0x200e, 'rlm': 0x200f, 'ndash': 0x2013, //
  'mdash': 0x2014, 'lsquo': 0x2018, 'rsquo': 0x2019, 'sbquo': 0x201a, //
  'ldquo': 0x201c, 'rdquo': 0x201d, 'bdquo': 0x201e, 'dagger': 0x2020, //
  'Dagger': 0x2021, 'bull': 0x2022, 'hellip': 0x2026, 'permil': 0x2030, //
  'prime': 0x2032, 'Prime': 0x2033, 'lsaquo': 0x2039, 'rsaquo': 0x203a, //
  'euro': 0x20ac, 'trade': 0x2122, 'larr': 0x2190, 'uarr': 0x2191, //
  'rarr': 0x2192, 'darr': 0x2193, 'harr': 0x2194, 'minus': 0x2212, //
};

bool _hex(int c) => _digit(c) || ((c | 0x20) >= 0x61 && (c | 0x20) <= 0x66);

/// [s] with its character references read: `&#…;` and `&#x…;` of a
/// character, and the names above; any other `&` as written.
String _decodeEntities(String s) {
  if (!s.contains('&')) return s;
  final c = _cps(s);
  final out = StringBuffer();
  var i = 0;
  while (i < c.length) {
    if (c[i] != _amp) {
      out.writeCharCode(c[i]);
      i++;
      continue;
    }
    if (i + 1 < c.length && c[i + 1] == _hash) {
      final hex = i + 2 < c.length && (c[i + 2] | 0x20) == 0x78;
      final start = i + (hex ? 3 : 2);
      var j = start;
      while (j < c.length &&
          j - start < (hex ? 6 : 7) &&
          (hex ? _hex(c[j]) : _digit(c[j]))) {
        j++;
      }
      if (j > start && j < c.length && c[j] == _semicolon) {
        final value = int.parse(_str(c, start, j), radix: hex ? 16 : 10);
        if (value > 0 &&
            value <= 0x10ffff &&
            (value < 0xd800 || value > 0xdfff)) {
          out.writeCharCode(value);
          i = j + 1;
          continue;
        }
      }
    } else {
      var j = i + 1;
      while (j < c.length && j - i - 1 < 8 && _alnum(c[j])) {
        j++;
      }
      if (j > i + 1 && j < c.length && c[j] == _semicolon) {
        final name = _str(c, i + 1, j);
        final latin = _latin1.indexOf(name);
        final value = latin >= 0 ? 0xa0 + latin : _entities[name];
        if (value != null) {
          out.writeCharCode(value);
          i = j + 1;
          continue;
        }
      }
    }
    out.writeCharCode(_amp);
    i++;
  }
  return out.toString();
}

/// The number an `<ol>`'s `start` says, up to nine digits; 1 for none.
int _start(String? value) {
  final s = _collapse(value ?? '');
  if (s.isEmpty || s.length > 9 || !s.codeUnits.every(_digit)) return 1;
  return int.parse(s);
}

/// The language a `class` of `language-…` or `lang-…` names; empty for
/// none.
String _languageOf(Map<String, String> attributes) {
  for (final name in _collapse(attributes['class'] ?? '').split(' ')) {
    for (final prefix in const ['language-', 'lang-']) {
      if (name.startsWith(prefix) && name.length > prefix.length) {
        return name.substring(prefix.length);
      }
    }
  }
  return '';
}

/// HTML read as a browser shows it, into the subset: the text of its
/// elements, a run of blanks one space; `<h1>` and `<h2>` its headings,
/// the others a line of text; `<b>`, `<strong>`, `<i>`, `<em>`, `<s>`,
/// `<del>` and `<code>` their formats; `<a href>` a link; lists, quotes,
/// `<pre>` and tables as the subset has them; an image its `alt` text.
/// A script, a style and a template go with their content, and so do
/// comments; nothing is fetched.
class _Html {
  _Html(String text) : _c = _cps(text);

  final List<int> _c;
  final _out = _Out();
  var _title = '';
  final _line = _Inline();
  _Inline? _cell;
  var _blankDue = false;
  var _bolds = 0;
  var _italics = 0;
  var _strikes = 0;
  var _codes = 0;
  final _links = <String?>[];
  var _heading = 0;

  /// The lists open, each its next number, null for a bulleted one.
  final _lists = <int?>[];
  ({int level, int? number})? _item;
  var _quotes = 0;
  var _pre = 0;
  final _preText = StringBuffer();
  var _language = '';
  var _table = 0;
  final _rows = <List<List<_Span>>>[];
  List<List<_Span>>? _row;

  int get _style =>
      (_bolds > 0 ? _bold : 0) |
      (_italics > 0 ? _italic : 0) |
      (_strikes > 0 ? _strike : 0) |
      (_codes > 0 ? _code : 0);

  ImportedDocument read() {
    final c = _c;
    final n = c.length;
    var text = 0;
    var i = 0;
    while (i < n) {
      if (c[i] != _lt || i + 1 >= n) {
        i++;
        continue;
      }
      final next = c[i + 1];
      if (_at(c, i, '<!--')) {
        _textRun(text, i);
        final k = _find(
          i + 4,
          n - 2,
          (k) => c[k] == _minus && c[k + 1] == _minus && c[k + 2] == _gt,
        );
        i = text = k < 0 ? n : k + 3;
      } else if (next == 0x21 ||
          next == 0x3f ||
          (next == _slash && !(i + 2 < n && _letter(c[i + 2])))) {
        _textRun(text, i);
        final k = _find(i + 2, n, (k) => c[k] == _gt);
        i = text = k < 0 ? n : k + 1;
      } else if (next == _slash) {
        _textRun(text, i);
        var k = i + 2;
        while (k < n && _tagChar(c[k])) {
          k++;
        }
        _close(_lowerStr(c, i + 2, k));
        final gt = _find(k, n, (k) => c[k] == _gt);
        i = text = gt < 0 ? n : gt + 1;
      } else if (_letter(next)) {
        _textRun(text, i);
        final tag = _tag(i);
        if (tag == null) {
          i = text = n;
          continue;
        }
        i = text = tag.end;
        final name = tag.name;
        if (_htmlDropped.contains(name) ||
            name == 'title' ||
            name == 'textarea') {
          final close = _find(
            i,
            n,
            (k) => c[k] == _lt && _at(c, k + 1, '/$name', anyCase: true),
          );
          final inner = _str(c, i, close < 0 ? n : close);
          if (name == 'title') {
            if (_title.isEmpty) _title = _collapse(_decodeEntities(inner));
          } else if (name == 'textarea') {
            _text(_decodeEntities(inner));
          }
          final gt = close < 0 ? -1 : _find(close, n, (k) => c[k] == _gt);
          i = text = gt < 0 ? n : gt + 1;
          continue;
        }
        _open(name, tag.attributes);
        if (tag.selfClosing) _close(name);
      } else {
        i++;
      }
    }
    _textRun(text, n);
    if (_pre > 0) {
      _pre = 1;
      _close('pre');
    }
    if (_table > 0) {
      _table = 1;
      _close('table');
    }
    _flush();
    return ImportedDocument(_title, _out.text());
  }

  /// The start tag at [i]: its name in lower case ASCII letters, its
  /// attributes, whether it closes itself and the index after it; null
  /// when the text ends inside it.
  ({String name, Map<String, String> attributes, bool selfClosing, int end})?
  _tag(int i) {
    final c = _c;
    final n = c.length;
    var j = i + 1;
    while (j < n && _tagChar(c[j])) {
      j++;
    }
    final name = _lowerStr(c, i + 1, j);
    final attributes = <String, String>{};
    var selfClosing = false;
    while (true) {
      while (j < n && (_htmlSpace(c[j]) || c[j] == _slash)) {
        selfClosing = c[j] == _slash;
        j++;
      }
      if (j >= n) return null;
      if (c[j] == _gt) {
        return (
          name: name,
          attributes: attributes,
          selfClosing: selfClosing,
          end: j + 1,
        );
      }
      selfClosing = false;
      final from = j;
      while (j < n &&
          !_htmlSpace(c[j]) &&
          c[j] != _gt &&
          c[j] != _slash &&
          (c[j] != _equals || j == from)) {
        j++;
      }
      final key = _lowerStr(c, from, j);
      while (j < n && _htmlSpace(c[j])) {
        j++;
      }
      var value = '';
      if (j < n && c[j] == _equals) {
        j++;
        while (j < n && _htmlSpace(c[j])) {
          j++;
        }
        if (j < n && (c[j] == _quote || c[j] == _apos)) {
          final q = c[j];
          final k = _find(j + 1, n, (k) => c[k] == q);
          if (k < 0) return null;
          value = _str(c, j + 1, k);
          j = k + 1;
        } else {
          final start = j;
          while (j < n && !_htmlSpace(c[j]) && c[j] != _gt) {
            j++;
          }
          value = _str(c, start, j);
        }
      }
      attributes.putIfAbsent(key, () => _decodeEntities(value));
    }
  }

  void _textRun(int from, int to) {
    if (to > from) _text(_decodeEntities(_str(_c, from, to)));
  }

  /// Text as a browser shows it: a run of blanks one space, none at the
  /// start of a line; in `<pre>` as it is.
  void _text(String s) {
    if (_pre > 0) {
      _preText.write(s);
      return;
    }
    final into = _cell ?? _line;
    final link = _links.isEmpty ? null : _links.last;
    for (final ch in s.runes) {
      if (_htmlSpace(ch)) {
        _blankDue = true;
        continue;
      }
      if (_blankDue && !into.isEmpty) into.char(0x20, _style, link);
      _blankDue = false;
      into.char(ch, _style, link);
    }
  }

  /// Ends the line, or in a table's cell puts a space.
  void _boundary() {
    if (_cell != null) {
      _blankDue = true;
    } else {
      _flush();
    }
  }

  /// A blank line, but in a list or a table's cell.
  void _gap() {
    if (_cell == null && _lists.isEmpty) _out.blank();
  }

  void _flush() {
    final spans = _line.take();
    _blankDue = false;
    if (spans.isEmpty) return;
    if (_heading == 1 || _heading == 2) {
      _out.heading(_heading, spans);
    } else if (_item case final item?) {
      _out.item(item.level, item.number, spans);
      _item = null;
    } else if (_quotes > 0) {
      _out.quote(spans);
    } else {
      _out.paragraph(spans);
    }
  }

  void _endCell() {
    if (_cell case final cell?) {
      (_row ??= []).add(cell.take());
      _cell = null;
      _blankDue = false;
    }
  }

  void _endRow() {
    if (_row case final row?) {
      if (row.isNotEmpty) _rows.add(row);
      _row = null;
    }
  }

  void _open(String name, Map<String, String> attributes) {
    switch (name) {
      case 'br':
        if (_pre > 0) {
          _preText.write('\n');
        } else {
          _boundary();
        }
      case 'p' || 'hr':
        _boundary();
        _gap();
      case 'h1' || 'h2' || 'h3' || 'h4' || 'h5' || 'h6':
        _boundary();
        _gap();
        if (_cell == null) {
          _heading = switch (name) {
            'h1' => 1,
            'h2' => 2,
            _ => 3,
          };
        }
      case 'ul' || 'ol':
        _boundary();
        _gap();
        _lists.add(name == 'ol' ? _start(attributes['start']) : null);
      case 'li':
        _boundary();
        if (_cell == null) {
          final next = _lists.isEmpty ? null : _lists.last;
          if (next != null) _lists[_lists.length - 1] = next + 1;
          _item = (level: _lists.length > 1 ? 1 : 0, number: next);
        }
      case 'blockquote':
        _boundary();
        _gap();
        _quotes++;
      case 'pre':
        _boundary();
        if (_table == 0) {
          if (_pre == 0) {
            _gap();
            _preText.clear();
            _language = _languageOf(attributes);
          }
          _pre++;
        }
      case 'code' || 'kbd' || 'samp' || 'tt':
        if (_pre > 0) {
          if (_language.isEmpty) _language = _languageOf(attributes);
        } else {
          _codes++;
        }
      case 'b' || 'strong':
        _bolds++;
      case 'i' || 'em':
        _italics++;
      case 's' || 'strike' || 'del':
        _strikes++;
      case 'a':
        _links.add(_target(_collapse(attributes['href'] ?? '')));
      case 'img':
        final alt = attributes['alt'] ?? '';
        if (alt.isNotEmpty) _text(' $alt ');
      case 'table':
        if (_table == 0) {
          _boundary();
          _gap();
          _rows.clear();
          _row = null;
        } else {
          _boundary();
        }
        _table++;
      case 'tr':
        if (_table == 1) {
          _endCell();
          _endRow();
          _row = [];
        } else {
          _boundary();
        }
      case 'td' || 'th':
        if (_table == 1) {
          _endCell();
          _row ??= [];
          _cell = _Inline();
          _blankDue = false;
        } else {
          _boundary();
        }
      default:
        if (_htmlBlocks.contains(name)) _boundary();
    }
  }

  void _close(String name) {
    switch (name) {
      case 'p':
        _boundary();
        _gap();
      case 'h1' || 'h2' || 'h3' || 'h4' || 'h5' || 'h6':
        _boundary();
        if (_cell == null) _heading = 0;
        _gap();
      case 'ul' || 'ol':
        _boundary();
        _item = null;
        if (_lists.isNotEmpty) _lists.removeLast();
        _gap();
      case 'li':
        _boundary();
        _item = null;
      case 'blockquote':
        _boundary();
        if (_quotes > 0) _quotes--;
        _gap();
      case 'pre':
        if (_table > 0 || _pre == 0) {
          _boundary();
        } else if (--_pre == 0) {
          var text = _preText.toString();
          if (text.startsWith('\n')) text = text.substring(1);
          if (text.endsWith('\n')) text = text.substring(0, text.length - 1);
          _out.code(_language, text.split('\n'));
          _gap();
        }
      case 'code' || 'kbd' || 'samp' || 'tt':
        if (_pre == 0 && _codes > 0) _codes--;
      case 'b' || 'strong':
        if (_bolds > 0) _bolds--;
      case 'i' || 'em':
        if (_italics > 0) _italics--;
      case 's' || 'strike' || 'del':
        if (_strikes > 0) _strikes--;
      case 'a':
        if (_links.isNotEmpty) _links.removeLast();
      case 'table':
        if (_table > 0 && --_table == 0) {
          _endCell();
          _endRow();
          _flush();
          _out.table(List.of(_rows));
          _rows.clear();
        }
      case 'tr':
        if (_table == 1) {
          _endCell();
          _endRow();
        }
      case 'td' || 'th':
        if (_table == 1) _endCell();
      default:
        if (_htmlBlocks.contains(name)) _boundary();
    }
  }
}

/// Code coloured by its language (obf-1036, ADR 0054): a grammar is a
/// standard Sublime syntax definition (`.sublime-syntax`), read as data;
/// nothing of it runs. Both cores read the same grammars and colour the
/// same text into the same spans, so they read only a subset of what
/// Sublime Text reads, and refuse a grammar that goes beyond it:
///
/// - the keys `name`, `scope`, `file_extensions`, `first_line_match`,
///   `variables` and `contexts` (`hidden` and `version` are ignored);
/// - in a context, `match` with `scope`, `captures` and one of `push`,
///   `set` and `pop: true`, `include` of a context of the same grammar,
///   `meta_scope`, `meta_content_scope` and `meta_include_prototype`;
///   never `embed`, `escape`, `branch_point`, `fail`, `extends`,
///   `with_prototype` or `clear_scopes`;
/// - regular expressions of literals, `.`, `^`, `$`, `|`, groups,
///   lookaround, greedy and lazy quantifiers, classes and the escapes
///   `\w \W \d \D \s \S \b \B \t \n \r \f \v \e \a \xHH \x{H…} \uHHHH`,
///   with a leading `(?x)` or `(?i)`; never possessive quantifiers,
///   atomic or named groups, backreferences, other flags, `\G \A \z \Z
///   \h \K \p{…}`, POSIX classes or `&&`, a capture inside a quantified
///   group or a lookaround, or a quantifier on an assertion.
///
/// Their meaning is the same in both cores: `\w`, `\d`, `\s` and `\b` are
/// ASCII, `.` is any character but a line break, `$` is before the
/// line's break, `(?i)` folds ASCII letters only, and a character is one
/// whatever its UTF-16 or UTF-8 length.
library;

import 'dart:typed_data';

import 'package:yaml/yaml.dart';

import 'built_in_grammars.dart';

/// What a coloured part of code is, which each look draws in a colour
/// of its own.
enum CodeColour {
  comment,
  string,
  number,
  constant,
  keyword,
  operator,
  type,
  function,
  tag,
  attribute,
  variable,
}

/// A coloured part of a text, from [start] to [end] in UTF-16 code units.
class CodeSpan {
  final int start;
  final int end;
  final CodeColour colour;

  const CodeSpan(this.start, this.end, this.colour);

  @override
  bool operator ==(Object other) =>
      other is CodeSpan &&
      other.start == start &&
      other.end == end &&
      other.colour == colour;

  @override
  int get hashCode => Object.hash(start, end, colour);

  @override
  String toString() => 'CodeSpan($start, $end, ${colour.name})';
}

/// A grammar the cores do not read, with [reason].
class GrammarRefused implements Exception {
  final String reason;

  const GrammarRefused(this.reason);

  @override
  String toString() => 'GrammarRefused: $reason';
}

/// The scopes that give a colour, by their dotted prefix: a scope takes
/// the colour of its longest prefix here, and one without any gives
/// none, so that a scope further out decides.
const _colours = <String, CodeColour>{
  'comment': CodeColour.comment,
  'markup.quote': CodeColour.comment,
  'string': CodeColour.string,
  'markup.raw': CodeColour.string,
  'constant': CodeColour.constant,
  'constant.numeric': CodeColour.number,
  'variable.language': CodeColour.constant,
  'markup.underline.link': CodeColour.constant,
  'keyword': CodeColour.keyword,
  'storage.modifier': CodeColour.keyword,
  'markup.heading': CodeColour.keyword,
  'keyword.operator': CodeColour.operator,
  'storage.type': CodeColour.type,
  'entity.name.type': CodeColour.type,
  'entity.name.class': CodeColour.type,
  'entity.name.struct': CodeColour.type,
  'entity.name.enum': CodeColour.type,
  'entity.name.trait': CodeColour.type,
  'entity.name.interface': CodeColour.type,
  'entity.other.inherited-class': CodeColour.type,
  'support.type': CodeColour.type,
  'support.class': CodeColour.type,
  'entity.name.function': CodeColour.function,
  'support.function': CodeColour.function,
  'variable.function': CodeColour.function,
  'entity.name.tag': CodeColour.tag,
  'entity.other.attribute-name': CodeColour.attribute,
  'variable.parameter': CodeColour.variable,
};

/// The colour of one [scope], by its longest prefix in [_colours].
CodeColour? scopeColour(String scope) {
  var name = scope;
  while (true) {
    final colour = _colours[name];
    if (colour != null) return colour;
    final dot = name.lastIndexOf('.');
    if (dot < 0) return null;
    name = name.substring(0, dot);
  }
}

/// The colour of [scopes], outermost first: the innermost that has one.
CodeColour? _colourOf(Iterable<List<String>> scopes) {
  CodeColour? found;
  for (final list in scopes) {
    for (final s in list) {
      found = scopeColour(s) ?? found;
    }
  }
  return found;
}

/// The longest line, and the longest text, that is coloured, in
/// characters; longer ones stay as they are.
const maxColouredLine = 5000;
const maxColouredText = 200000;

/// The most contexts on the stack, and the most empty matches in a row
/// on one line before the rest of it is left as it is.
const _maxStack = 64;
const _maxEmpty = 1000;

/// One language's grammar, read from a Sublime syntax definition.
class Grammar {
  /// The name it shows, "Rust".
  final String name;

  /// Its scope, `source.rust`: what a Code text stores as its language.
  final String scope;

  /// The names of its files' extensions, lower case, which are also the
  /// names a fence may give it.
  final List<String> fileExtensions;

  final _Pattern? _firstLine;
  final _Context _main;

  Grammar._(
    this.name,
    this.scope,
    this.fileExtensions,
    this._firstLine,
    this._main,
  );

  /// Reads the grammar [source]; throws [GrammarRefused] when it is no
  /// Sublime syntax definition, or one beyond what the cores read.
  factory Grammar.parse(String source) {
    final Object? doc;
    try {
      doc = loadYaml(source);
    } on YamlException catch (e) {
      throw GrammarRefused('not YAML: ${e.message}');
    }
    if (doc is! Map) throw const GrammarRefused('not a map');
    for (final key in doc.keys) {
      if (!_topKeys.contains(key)) throw GrammarRefused('the key $key');
    }
    final name = _string(doc['name'], 'name');
    final scope = _string(doc['scope'], 'scope');
    if (name.trim().isEmpty || scope.trim().isEmpty) {
      throw const GrammarRefused('no name or scope');
    }
    final extensions = <String>[];
    final ext = doc['file_extensions'];
    if (ext != null) {
      if (ext is! List) throw const GrammarRefused('file_extensions');
      for (final e in ext) {
        extensions.add(_string(e, 'file_extensions').toLowerCase());
      }
    }
    final variables = _variables(doc['variables']);
    final first = doc['first_line_match'];
    final firstLine = first == null
        ? null
        : _Pattern.of(
            _substitute(_string(first, 'first_line_match'), variables),
          );
    final contexts = doc['contexts'];
    if (contexts is! Map) throw const GrammarRefused('no contexts');
    final main = _Contexts(contexts, variables).compile();
    return Grammar._(name, scope, extensions, firstLine, main);
  }

  /// Whether [line] is a first line of this language by its
  /// `first_line_match`.
  bool matchesFirstLine(String line) => _firstLine?.search(line, 0) != null;

  /// The coloured parts of [text], in the order they come, touching
  /// parts of one colour merged; none for a text too long.
  List<CodeSpan> colour(String text) {
    if (text.runes.length > maxColouredText) return const [];
    // The colour of each code unit, 0 for none: later paint wins, so a
    // capture shows over its match.
    final painted = Uint8List(text.length);
    void paint(int start, int end, CodeColour? colour) {
      if (colour == null) return;
      painted.fillRange(start, end, colour.index + 1);
    }

    final stack = <_Context>[_main];
    var at = 0;
    while (at < text.length) {
      final nl = text.indexOf('\n', at);
      final end = nl < 0 ? text.length : nl + 1;
      final line = text.substring(at, end);
      if (line.runes.length <= maxColouredLine) {
        _line(line, at, stack, paint);
      }
      at = end;
    }
    final spans = <CodeSpan>[];
    var k = 0;
    while (k < painted.length) {
      final c = painted[k];
      var e = k + 1;
      while (e < painted.length && painted[e] == c) {
        e++;
      }
      if (c != 0) spans.add(CodeSpan(k, e, CodeColour.values[c - 1]));
      k = e;
    }
    return spans;
  }

  /// Colours one [line] that starts at [offset] of the text, with the
  /// contexts of [stack], which it leaves as the line ends.
  static void _line(
    String line,
    int offset,
    List<_Context> stack,
    void Function(int, int, CodeColour?) paint,
  ) {
    var pos = 0;
    var empties = 0;
    // The last search of each pattern of the top context on this line,
    // from where: still the first match from any later position up to
    // its start.
    _Context? cachedFor;
    var cache = <_Found?>[];
    var searchedFrom = <int>[];
    while (true) {
      final top = stack.last;
      if (!identical(cachedFor, top)) {
        cachedFor = top;
        cache = List<_Found?>.filled(top.patterns.length, null);
        searchedFrom = List<int>.filled(top.patterns.length, -1);
      }
      _Found? best;
      var bestIndex = -1;
      for (var i = 0; i < top.patterns.length; i++) {
        final p = top.patterns[i];
        var found = cache[i];
        final from = searchedFrom[i];
        if (from < 0 || from > pos || (found != null && found.start < pos)) {
          found = p.search(line, pos);
          cache[i] = found;
          searchedFrom[i] = pos;
        }
        if (found == null) continue;
        if (found.start == found.end && p.action == _Action.none) continue;
        if (best == null || found.start < best.start) {
          best = found;
          bestIndex = i;
        }
      }
      final content = _colourOf(_scopes(stack, content: true));
      if (best == null) {
        paint(offset + pos, offset + line.length, content);
        return;
      }
      paint(offset + pos, offset + best.start, content);
      if (best.end == pos) {
        if (++empties > _maxEmpty) return;
      } else {
        empties = 0;
      }
      final p = top.patterns[bestIndex];
      var action = p.action;
      final grows = switch (action) {
        _Action.push => p.targets.length,
        _Action.set => p.targets.length - 1,
        _ => 0,
      };
      if (stack.length + grows > _maxStack) action = _Action.none;
      if (action == _Action.pop && stack.length == 1) action = _Action.none;
      final below = stack.sublist(0, stack.length - 1);
      // The scopes the match's own text has.
      final base = <List<String>>[
        ...switch (action) {
          _Action.none || _Action.push => _scopes(stack, content: true),
          _Action.set => _scopes(below, content: true),
          _Action.pop => [..._scopes(below, content: true), top.metaScope],
        },
        if (action == _Action.push || action == _Action.set)
          for (final c in p.targets) c.metaScope,
        p.scope,
      ];
      paint(offset + best.start, offset + best.end, _colourOf(base));
      for (var k = 1; k < best.groups.length; k++) {
        final g = best.groups[k];
        final cap = p.captures[k];
        if (g == null || cap == null || g.$1 >= g.$2) continue;
        if (_colourOf([cap]) == null) continue;
        paint(offset + g.$1, offset + g.$2, _colourOf([...base, cap]));
      }
      switch (action) {
        case _Action.none:
          break;
        case _Action.push:
          stack.addAll(p.targets);
        case _Action.set:
          stack.removeLast();
          stack.addAll(p.targets);
        case _Action.pop:
          stack.removeLast();
      }
      pos = best.end;
    }
  }

  static Iterable<List<String>> _scopes(
    List<_Context> stack, {
    required bool content,
  }) sync* {
    for (final c in stack) {
      yield c.metaScope;
      if (content) yield c.metaContentScope;
    }
  }
}

const _topKeys = {
  'name',
  'scope',
  'file_extensions',
  'first_line_match',
  'hidden',
  'version',
  'variables',
  'contexts',
};

String _string(Object? value, String what) {
  if (value is! String) throw GrammarRefused('$what is not a text');
  return value;
}

List<String> _scopeList(Object? value, String what) {
  if (value == null) return const [];
  return _string(
    value,
    what,
  ).split(RegExp('[ \t\n]+')).where((s) => s.isNotEmpty).toList();
}

Map<String, String> _variables(Object? raw) {
  if (raw == null) return const {};
  if (raw is! Map) throw const GrammarRefused('variables');
  final source = <String, String>{
    for (final MapEntry(:key, :value) in raw.entries)
      _string(key, 'a variable'): _string(value, 'variable $key'),
  };
  final done = <String, String>{};
  String resolve(String name, int depth) {
    final known = done[name];
    if (known != null) return known;
    final text = source[name];
    if (text == null) throw GrammarRefused('no variable $name');
    if (depth > 32) throw GrammarRefused('variable $name refers to itself');
    final out = text.replaceAllMapped(
      _variable,
      (m) => resolve(m.group(1) ?? '', depth + 1),
    );
    done[name] = out;
    return out;
  }

  for (final name in source.keys) {
    resolve(name, 0);
  }
  return done;
}

final _variable = RegExp(r'\{\{([A-Za-z0-9_]+)\}\}');

String _substitute(String pattern, Map<String, String> variables) =>
    pattern.replaceAllMapped(_variable, (m) {
      final v = variables[m.group(1)];
      if (v == null) throw GrammarRefused('no variable ${m.group(1)}');
      return v;
    });

enum _Action { none, push, set, pop }

class _Context {
  List<String> metaScope = const [];
  List<String> metaContentScope = const [];
  bool includePrototype = true;

  /// Its items as written: a [_Pattern] or the name of a context to
  /// include.
  final List<Object> items = [];
  List<_Pattern> patterns = const [];
}

/// The contexts of one grammar, compiled.
class _Contexts {
  final Map<dynamic, dynamic> raw;
  final Map<String, String> variables;
  final Map<String, _Context> named = {};
  final List<_Context> all = [];

  _Contexts(this.raw, this.variables);

  _Context compile() {
    for (final key in raw.keys) {
      _string(key, 'a context name');
    }
    if (!raw.containsKey('main')) throw const GrammarRefused('no main');
    for (final key in raw.keys) {
      named[key as String] = _Context();
    }
    for (final MapEntry(:key, :value) in raw.entries) {
      _fill(named[key as String] ?? _Context(), value, key);
    }
    final prototype = named['prototype'];
    for (final c in all) {
      final out = <_Pattern>[];
      final seen = <_Context>{};
      if (prototype != null && !identical(c, prototype) && c.includePrototype) {
        _flatten(prototype, out, seen);
      }
      _flatten(c, out, seen);
      c.patterns = out;
    }
    return named['main'] ?? _Context();
  }

  void _flatten(_Context c, List<_Pattern> out, Set<_Context> seen) {
    if (!seen.add(c)) return;
    for (final item in c.items) {
      if (item is _Pattern) {
        out.add(item);
      } else if (item is _Context) {
        _flatten(item, out, seen);
      }
    }
  }

  void _fill(_Context c, Object? items, String what) {
    all.add(c);
    if (items is! List) throw GrammarRefused('context $what is no list');
    var first = true;
    for (final item in items) {
      if (item is! Map) throw GrammarRefused('an item of $what');
      if (item.containsKey('meta_scope') ||
          item.containsKey('meta_content_scope') ||
          item.containsKey('meta_include_prototype')) {
        for (final key in item.keys) {
          if (!const {
            'meta_scope',
            'meta_content_scope',
            'meta_include_prototype',
          }.contains(key)) {
            throw GrammarRefused('the key $key in $what');
          }
        }
        if (!first) throw GrammarRefused('a meta item late in $what');
        if (item.containsKey('meta_scope')) {
          c.metaScope = _scopeList(item['meta_scope'], 'meta_scope');
        }
        if (item.containsKey('meta_content_scope')) {
          c.metaContentScope = _scopeList(
            item['meta_content_scope'],
            'meta_content_scope',
          );
        }
        if (item.containsKey('meta_include_prototype')) {
          final v = item['meta_include_prototype'];
          if (v is! bool) throw const GrammarRefused('meta_include_prototype');
          c.includePrototype = v;
        }
        continue;
      }
      first = false;
      if (item.containsKey('include')) {
        if (item.length != 1) throw GrammarRefused('an include in $what');
        final name = _string(item['include'], 'include');
        final target = named[name];
        if (target == null) throw GrammarRefused('no context $name');
        c.items.add(target);
        continue;
      }
      if (!item.containsKey('match')) throw GrammarRefused('an item of $what');
      for (final key in item.keys) {
        if (!const {
          'match',
          'scope',
          'captures',
          'push',
          'set',
          'pop',
        }.contains(key)) {
          throw GrammarRefused('the key $key in $what');
        }
      }
      final pattern = _Pattern.of(
        _substitute(_string(item['match'], 'match'), variables),
      );
      pattern.scope = _scopeList(item['scope'], 'scope');
      final captures = item['captures'];
      if (captures != null) {
        if (captures is! Map) throw const GrammarRefused('captures');
        for (final MapEntry(:key, :value) in captures.entries) {
          final k = key is int ? key : int.tryParse('$key');
          if (k == null || k < 1 || k > pattern.groupCount) {
            throw GrammarRefused('the capture $key');
          }
          pattern.captures[k] = _scopeList(value, 'a capture');
        }
      }
      final actions = ['push', 'set', 'pop'].where(item.containsKey).toList();
      if (actions.length > 1) throw GrammarRefused('two actions in $what');
      if (actions.isNotEmpty) {
        final action = actions.first;
        if (action == 'pop') {
          final v = item['pop'];
          if (v != true && v != 1) throw const GrammarRefused('pop');
          pattern.action = _Action.pop;
        } else {
          pattern.action = action == 'push' ? _Action.push : _Action.set;
          pattern.targets = _targets(item[action], what);
        }
      }
      c.items.add(pattern);
    }
  }

  List<_Context> _targets(Object? value, String what) {
    if (value is String) {
      final target = named[value];
      if (target == null) throw GrammarRefused('no context $value');
      return [target];
    }
    if (value is List && value.isNotEmpty) {
      // A list of names, or one anonymous context (a list of items).
      if (value.every((v) => v is String)) {
        return [for (final v in value) ..._targets(v, what)];
      }
      if (value.every((v) => v is Map)) {
        final c = _Context();
        _fill(c, value, '$what (anonymous)');
        return [c];
      }
    }
    throw GrammarRefused('a push or set in $what');
  }
}

/// A match found: where it lies and where each capture does (null for
/// one that took no part), in UTF-16 code units of the line.
class _Found {
  final int start;
  final int end;
  final List<(int, int)?> groups;

  _Found(this.start, this.end, this.groups);
}

class _Pattern {
  final RegExp _re;
  final _Measure? _measure;
  final int groupCount;
  List<String> scope = const [];
  final Map<int, List<String>> captures = {};
  _Action action = _Action.none;
  List<_Context> targets = const [];

  _Pattern._(this._re, this._measure, this.groupCount);

  factory _Pattern.of(String source) {
    final parsed = CodeRegex.parse(source);
    final emitted = _DartEmitter().emit(parsed._root);
    final RegExp re;
    try {
      re = RegExp(emitted.source, unicode: true);
    } on FormatException catch (e) {
      throw GrammarRefused('the regex $source: ${e.message}');
    }
    return _Pattern._(re, emitted.measure, parsed.groups);
  }

  _Found? search(String line, int from) {
    var start = from;
    RegExpMatch? m;
    while (true) {
      m = _re.allMatches(line, start).firstOrNull;
      if (m == null) return null;
      // Dart's RegExp tries an empty match between the halves of a
      // surrogate pair too; a character is one, so it is passed over.
      if (!_splitsPair(line, m.start)) break;
      start = m.start + 1;
    }
    final groups = List<(int, int)?>.filled(groupCount + 1, null);
    groups[0] = (m.start, m.end);
    final measure = _measure;
    if (measure != null) measure.walk(m, m.start, groups);
    return _Found(m.start, m.end, groups);
  }
}

/// Whether [at] lies between the two halves of a surrogate pair.
bool _splitsPair(String text, int at) =>
    at > 0 &&
    at < text.length &&
    (text.codeUnitAt(at - 1) & 0xFC00) == 0xD800 &&
    (text.codeUnitAt(at) & 0xFC00) == 0xDC00;

// The regular expressions of the subset, read into a tree that each core
// writes out for its own engine.

sealed class _Node {}

class _Seq extends _Node {
  final List<_Node> terms;

  _Seq(this.terms);
}

class _Alt extends _Node {
  final List<_Seq> alts;

  _Alt(this.alts);
}

enum _GroupKind { capture, plain, ahead, notAhead, behind, notBehind }

class _Group extends _Node {
  final _GroupKind kind;
  final _Alt body;

  _Group(this.kind, this.body);
}

class _Quant extends _Node {
  final _Node atom;
  final int min;
  final int? max;
  final bool lazy;

  _Quant(this.atom, this.min, this.max, this.lazy);
}

/// A set of characters: a literal, a class, `\w` and the like, `.`.
class _Set extends _Node {
  final bool negated;

  /// Ranges of code points, both ends included.
  final List<(int, int)> ranges;

  _Set(this.negated, this.ranges);
}

enum _AnchorKind { start, end, word, notWord }

class _Anchor extends _Node {
  final _AnchorKind kind;

  _Anchor(this.kind);
}

bool _hasCapture(_Node n) => switch (n) {
  _Seq(:final terms) => terms.any(_hasCapture),
  _Alt(:final alts) => alts.any(_hasCapture),
  _Group(:final kind, :final body) =>
    kind == _GroupKind.capture || _hasCapture(body),
  _Quant(:final atom) => _hasCapture(atom),
  _Set() || _Anchor() => false,
};

const _wordRanges = [(0x30, 0x39), (0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)];
const _digitRanges = [(0x30, 0x39)];
const _spaceRanges = [(0x09, 0x0D), (0x20, 0x20)];

/// A regular expression of the subset, read: its tree and how many
/// captures it has.
class CodeRegex {
  final _Alt _root;
  final int groups;

  CodeRegex._(this._root, this.groups);

  /// Reads [source]; throws [GrammarRefused] for anything beyond the
  /// subset.
  static CodeRegex parse(String source) =>
      _RegexParser(source.runes.toList(), source).parse();
}

class _RegexParser {
  final List<int> s;
  final String source;
  var i = 0;
  var extended = false;
  var fold = false;
  var groups = 0;

  _RegexParser(this.s, this.source);

  Never _refuse(String what) =>
      throw GrammarRefused('the regex $source: $what');

  bool _at(String text) {
    final r = text.runes.toList();
    if (i + r.length > s.length) return false;
    for (var k = 0; k < r.length; k++) {
      if (s[i + k] != r[k]) return false;
    }
    return true;
  }

  CodeRegex parse() {
    // Leading flags: (?x), (?i), (?xi), (?ix), in any number.
    while (i + 2 < s.length && _at('(?')) {
      var k = i + 2;
      var x = false;
      var ic = false;
      while (k < s.length && (s[k] == 0x78 || s[k] == 0x69)) {
        if (s[k] == 0x78) x = true;
        if (s[k] == 0x69) ic = true;
        k++;
      }
      if (k == i + 2 || k >= s.length || s[k] != 0x29) break;
      extended = extended || x;
      fold = fold || ic;
      i = k + 1;
    }
    final root = _alt();
    if (i < s.length) _refuse('a ) too many');
    return CodeRegex._(root, groups);
  }

  void _skip() {
    if (!extended) return;
    while (i < s.length) {
      final c = s[i];
      if (c == 0x20 || (c >= 0x09 && c <= 0x0D)) {
        i++;
      } else if (c == 0x23) {
        while (i < s.length && s[i] != 0x0A) {
          i++;
        }
      } else {
        return;
      }
    }
  }

  _Alt _alt() {
    final alts = [_seq()];
    while (i < s.length && s[i] == 0x7C) {
      i++;
      alts.add(_seq());
    }
    return _Alt(alts);
  }

  _Seq _seq() {
    final terms = <_Node>[];
    while (true) {
      _skip();
      if (i >= s.length || s[i] == 0x7C || s[i] == 0x29) break;
      final atom = _atom();
      _skip();
      terms.add(_quantified(atom));
    }
    return _Seq(terms);
  }

  _Node _quantified(_Node atom) {
    if (i >= s.length) return atom;
    final c = s[i];
    int min;
    int? max;
    if (c == 0x2A) {
      min = 0;
      max = null;
      i++;
    } else if (c == 0x2B) {
      min = 1;
      max = null;
      i++;
    } else if (c == 0x3F) {
      min = 0;
      max = 1;
      i++;
    } else if (c == 0x7B) {
      final q = _braces();
      if (q == null) return atom;
      (min, max) = q;
    } else {
      return atom;
    }
    var lazy = false;
    if (i < s.length && s[i] == 0x3F) {
      lazy = true;
      i++;
    }
    if (i < s.length && (s[i] == 0x2B || s[i] == 0x2A || s[i] == 0x3F)) {
      _refuse('a possessive or doubled quantifier');
    }
    if (i < s.length && s[i] == 0x7B && _braceAhead()) {
      _refuse('a doubled quantifier');
    }
    if (atom is _Anchor) _refuse('a quantifier on an assertion');
    if (atom is _Group) {
      if (atom.kind != _GroupKind.capture && atom.kind != _GroupKind.plain) {
        _refuse('a quantifier on a lookaround');
      }
      if (_hasCapture(atom)) _refuse('a capture in a quantified group');
    }
    return _Quant(atom, min, max, lazy);
  }

  bool _braceAhead() {
    final keep = i;
    final q = _braces();
    i = keep;
    return q != null;
  }

  /// `{n}`, `{n,}` or `{n,m}` at [i], read; null, and [i] as it was, for
  /// a `{` that is a literal.
  (int, int?)? _braces() {
    final keep = i;
    i++;
    int? number() {
      final from = i;
      while (i < s.length && s[i] >= 0x30 && s[i] <= 0x39) {
        i++;
      }
      if (i == from || i - from > 4) return null;
      return int.parse(String.fromCharCodes(s.sublist(from, i)));
    }

    if (i < s.length && s[i] == 0x2C) _refuse('a quantifier {,n}');
    final min = number();
    if (min == null) {
      i = keep;
      return null;
    }
    int? max = min;
    if (i < s.length && s[i] == 0x2C) {
      i++;
      max = number();
    }
    if (i >= s.length || s[i] != 0x7D) {
      i = keep;
      return null;
    }
    i++;
    if (max != null && max < min) _refuse('a quantifier {n,m} with m < n');
    return (min, max);
  }

  _Node _atom() {
    final c = s[i];
    switch (c) {
      case 0x28: // (
        return _group();
      case 0x5B: // [
        return _class();
      case 0x2E: // .
        i++;
        return _Set(true, const [(0x0A, 0x0A)]);
      case 0x5E: // ^
        i++;
        return _Anchor(_AnchorKind.start);
      case 0x24: // $
        i++;
        return _Anchor(_AnchorKind.end);
      case 0x5C: // \
        return _escape();
      case 0x2A || 0x2B || 0x3F:
        _refuse('a quantifier without anything before it');
      case 0x7B:
        if (_braceAhead()) _refuse('a quantifier without anything before it');
        i++;
        return _literal(c);
      default:
        i++;
        return _literal(c);
    }
  }

  _Node _literal(int c) => _Set(false, _folded([(c, c)]));

  List<(int, int)> _folded(List<(int, int)> ranges) {
    if (!fold) return ranges;
    final out = [...ranges];
    for (final (lo, hi) in ranges) {
      // a-z to A-Z and back, the overlap of each range only.
      final a = lo > 0x61 ? lo : 0x61;
      final b = hi < 0x7A ? hi : 0x7A;
      if (a <= b) out.add((a - 0x20, b - 0x20));
      final x = lo > 0x41 ? lo : 0x41;
      final y = hi < 0x5A ? hi : 0x5A;
      if (x <= y) out.add((x + 0x20, y + 0x20));
    }
    return out;
  }

  _Node _group() {
    i++;
    var kind = _GroupKind.capture;
    if (i < s.length && s[i] == 0x3F) {
      if (_at('?:')) {
        kind = _GroupKind.plain;
        i += 2;
      } else if (_at('?=')) {
        kind = _GroupKind.ahead;
        i += 2;
      } else if (_at('?!')) {
        kind = _GroupKind.notAhead;
        i += 2;
      } else if (_at('?<=')) {
        kind = _GroupKind.behind;
        i += 3;
      } else if (_at('?<!')) {
        kind = _GroupKind.notBehind;
        i += 3;
      } else {
        _refuse('a group (? of a kind not read');
      }
    } else {
      groups++;
    }
    final body = _alt();
    if (i >= s.length || s[i] != 0x29) _refuse('a ( not closed');
    i++;
    final group = _Group(kind, body);
    if (kind != _GroupKind.capture &&
        kind != _GroupKind.plain &&
        _hasCapture(body)) {
      _refuse('a capture in a lookaround');
    }
    return group;
  }

  /// An escape outside a class.
  _Node _escape() {
    i++;
    if (i >= s.length) _refuse('a \\ at the end');
    final c = s[i];
    switch (c) {
      case 0x77: // w
        i++;
        return _Set(false, _wordRanges);
      case 0x57: // W
        i++;
        return _Set(true, _wordRanges);
      case 0x64: // d
        i++;
        return _Set(false, _digitRanges);
      case 0x44: // D
        i++;
        return _Set(true, _digitRanges);
      case 0x73: // s
        i++;
        return _Set(false, _spaceRanges);
      case 0x53: // S
        i++;
        return _Set(true, _spaceRanges);
      case 0x62: // b
        i++;
        return _Anchor(_AnchorKind.word);
      case 0x42: // B
        i++;
        return _Anchor(_AnchorKind.notWord);
    }
    return _literal(_char());
  }

  /// The character an escape at [i] (after its `\`) stands for.
  int _char() {
    final c = s[i];
    i++;
    switch (c) {
      case 0x74:
        return 0x09; // t
      case 0x6E:
        return 0x0A; // n
      case 0x72:
        return 0x0D; // r
      case 0x66:
        return 0x0C; // f
      case 0x76:
        return 0x0B; // v
      case 0x65:
        return 0x1B; // e
      case 0x61:
        return 0x07; // a
      case 0x78: // x
        if (i < s.length && s[i] == 0x7B) {
          i++;
          final from = i;
          while (i < s.length && _hex(s[i]) >= 0) {
            i++;
          }
          if (i == from || i - from > 6 || i >= s.length || s[i] != 0x7D) {
            _refuse('a \\x{…} not closed');
          }
          final v = int.parse(
            String.fromCharCodes(s.sublist(from, i)),
            radix: 16,
          );
          i++;
          return _scalar(v);
        }
        return _scalar(_hexDigits(2));
      case 0x75: // u
        return _scalar(_hexDigits(4));
    }
    final ascii = c < 0x80;
    final letterOrDigit =
        (c >= 0x30 && c <= 0x39) ||
        (c >= 0x41 && c <= 0x5A) ||
        (c >= 0x61 && c <= 0x7A);
    if (ascii && letterOrDigit) {
      _refuse('the escape \\${String.fromCharCode(c)}');
    }
    return c;
  }

  int _hexDigits(int n) {
    if (i + n > s.length) _refuse('a short hex escape');
    var v = 0;
    for (var k = 0; k < n; k++) {
      final d = _hex(s[i + k]);
      if (d < 0) _refuse('a short hex escape');
      v = v * 16 + d;
    }
    i += n;
    return v;
  }

  int _scalar(int v) {
    if (v > 0x10FFFF || (v >= 0xD800 && v <= 0xDFFF)) {
      _refuse('no character ${v.toRadixString(16)}');
    }
    return v;
  }

  static int _hex(int c) {
    if (c >= 0x30 && c <= 0x39) return c - 0x30;
    if (c >= 0x41 && c <= 0x46) return c - 0x37;
    if (c >= 0x61 && c <= 0x66) return c - 0x57;
    return -1;
  }

  _Node _class() {
    i++;
    var negated = false;
    if (i < s.length && s[i] == 0x5E) {
      negated = true;
      i++;
    }
    final ranges = <(int, int)>[];
    var first = true;
    while (true) {
      if (i >= s.length) _refuse('a [ not closed');
      final c = s[i];
      if (c == 0x5D && !first) {
        i++;
        break;
      }
      first = false;
      if (c == 0x5B) _refuse('a [ in a class');
      if (_at('&&')) _refuse('&& in a class');
      int lo;
      if (c == 0x5C) {
        i++;
        if (i >= s.length) _refuse('a \\ at the end');
        final e = s[i];
        final set = switch (e) {
          0x77 => _wordRanges,
          0x64 => _digitRanges,
          0x73 => _spaceRanges,
          _ => null,
        };
        if (set != null) {
          i++;
          ranges.addAll(set);
          continue;
        }
        if (e == 0x57 || e == 0x44 || e == 0x53 || e == 0x62 || e == 0x42) {
          _refuse('\\${String.fromCharCode(e)} in a class');
        }
        lo = _char();
      } else {
        i++;
        lo = c;
      }
      if (i + 1 < s.length && s[i] == 0x2D && s[i + 1] != 0x5D) {
        i++;
        int hi;
        if (s[i] == 0x5C) {
          i++;
          if (i >= s.length) _refuse('a \\ at the end');
          final e = s[i];
          if (const {
            0x77,
            0x64,
            0x73,
            0x57,
            0x44,
            0x53,
            0x62,
            0x42,
          }.contains(e)) {
            _refuse('a range to a class');
          }
          hi = _char();
        } else if (s[i] == 0x5B) {
          _refuse('a [ in a class');
        } else {
          hi = s[i];
          i++;
        }
        if (hi < lo) _refuse('a range backwards');
        ranges.add((lo, hi));
      } else {
        ranges.add((lo, lo));
      }
    }
    return _Set(negated, _folded(ranges));
  }
}

// Writing the tree out for Dart's RegExp (unicode), measured so that
// where each capture lies can be told: Dart gives the text of a group,
// not its place, so each run of the pattern between captures is a group
// of its own, and so is each alternative of one holding a capture.

sealed class _M {}

class _MRun extends _M {
  final int group;

  _MRun(this.group);
}

class _MCap extends _M {
  final int user;
  final int group;
  final _MAlt? body;

  _MCap(this.user, this.group, this.body);
}

class _MGroup extends _M {
  final _MAlt body;

  _MGroup(this.body);
}

class _MAlt {
  final List<List<_M>> alts;

  /// The group of each alternative, when there are more than one.
  final List<int>? groups;

  _MAlt(this.alts, this.groups);
}

class _Measure {
  final _MAlt root;

  _Measure(this.root);

  void walk(RegExpMatch m, int start, List<(int, int)?> out) =>
      _alt(root, m, start, out);

  static int _alt(_MAlt a, RegExpMatch m, int pos, List<(int, int)?> out) {
    var index = 0;
    final groups = a.groups;
    if (groups != null) {
      index = groups.indexWhere((g) => m.group(g) != null);
      if (index < 0) return pos;
    }
    return _seq(a.alts[index], m, pos, out);
  }

  static int _seq(
    List<_M> items,
    RegExpMatch m,
    int pos,
    List<(int, int)?> out,
  ) {
    for (final item in items) {
      switch (item) {
        case _MRun(:final group):
          pos += m.group(group)?.length ?? 0;
        case _MCap(:final user, :final group, :final body):
          final text = m.group(group);
          if (text == null) continue;
          out[user] = (pos, pos + text.length);
          if (body != null) _alt(body, m, pos, out);
          pos += text.length;
        case _MGroup(:final body):
          pos = _alt(body, m, pos, out);
      }
    }
    return pos;
  }
}

class _DartEmitter {
  final _out = StringBuffer();
  var _next = 0;
  var _user = 0;

  ({String source, _Measure? measure}) emit(_Alt root) {
    if (!_hasCapture(root)) {
      _plain(root);
      return (source: _out.toString(), measure: null);
    }
    final m = _measuredAlt(root);
    return (source: _out.toString(), measure: _Measure(m));
  }

  /// [alt] written with the groups that measure it.
  _MAlt _measuredAlt(_Alt alt) {
    final many = alt.alts.length > 1;
    final groups = many ? <int>[] : null;
    final alts = <List<_M>>[];
    for (var k = 0; k < alt.alts.length; k++) {
      if (k > 0) _out.write('|');
      if (many) {
        _out.write('(');
        groups?.add(++_next);
      }
      alts.add(_measuredSeq(alt.alts[k]));
      if (many) _out.write(')');
    }
    return _MAlt(alts, groups);
  }

  List<_M> _measuredSeq(_Seq seq) {
    final items = <_M>[];
    final run = <_Node>[];
    void flush() {
      if (run.isEmpty) return;
      if (run.every((t) => t is _Anchor || (t is _Group && _lookaround(t)))) {
        run.forEach(_plain);
      } else {
        _out.write('(');
        items.add(_MRun(++_next));
        run.forEach(_plain);
        _out.write(')');
      }
      run.clear();
    }

    for (final t in seq.terms) {
      if (!_hasCapture(t)) {
        run.add(t);
        continue;
      }
      flush();
      // A term with a capture is a group, never quantified.
      final g = t as _Group;
      if (g.kind == _GroupKind.capture) {
        final user = ++_user;
        _out.write('(');
        final group = ++_next;
        final body = _hasCapture(g.body) ? _measuredAlt(g.body) : null;
        if (body == null) _plain(g.body);
        _out.write(')');
        items.add(_MCap(user, group, body));
      } else {
        _out.write('(?:');
        items.add(_MGroup(_measuredAlt(g.body)));
        _out.write(')');
      }
    }
    flush();
    return items;
  }

  static bool _lookaround(_Group g) =>
      g.kind != _GroupKind.capture && g.kind != _GroupKind.plain;

  void _plain(_Node n) {
    switch (n) {
      case _Seq(:final terms):
        terms.forEach(_plain);
      case _Alt(:final alts):
        for (var k = 0; k < alts.length; k++) {
          if (k > 0) _out.write('|');
          _plain(alts[k]);
        }
      case _Group(:final kind, :final body):
        if (kind == _GroupKind.capture) {
          _user++;
          _next++;
        }
        _out.write(switch (kind) {
          _GroupKind.capture => '(',
          _GroupKind.plain => '(?:',
          _GroupKind.ahead => '(?=',
          _GroupKind.notAhead => '(?!',
          _GroupKind.behind => '(?<=',
          _GroupKind.notBehind => '(?<!',
        });
        _plain(body);
        _out.write(')');
      case _Quant(:final atom, :final min, :final max, :final lazy):
        _plain(atom);
        _out.write(switch ((min, max)) {
          (0, null) => '*',
          (1, null) => '+',
          (0, 1) => '?',
          (final a, null) => '{$a,}',
          (final a, final b?) when a == b => '{$a}',
          (final a, final b) => '{$a,$b}',
        });
        if (lazy) _out.write('?');
      case _Set(:final negated, :final ranges):
        if (!negated && ranges.length == 1 && ranges[0].$1 == ranges[0].$2) {
          _out.write(_lit(ranges[0].$1));
        } else {
          _out.write(negated ? '[^' : '[');
          for (final (lo, hi) in ranges) {
            _out.write(_lit(lo));
            if (hi != lo) _out.write('-${_lit(hi)}');
          }
          _out.write(']');
        }
      case _Anchor(:final kind):
        _out.write(switch (kind) {
          _AnchorKind.start => '^',
          _AnchorKind.end => r'(?![^\n])',
          _AnchorKind.word => _wordBoundary,
          _AnchorKind.notWord => _notWordBoundary,
        });
    }
  }

  /// A character as Dart's unicode RegExp reads it alike in a class and
  /// out of one: an ASCII letter or digit as itself, anything else as
  /// `\u{…}`.
  static String _lit(int c) {
    final alnum =
        (c >= 0x30 && c <= 0x39) ||
        (c >= 0x41 && c <= 0x5A) ||
        (c >= 0x61 && c <= 0x7A);
    return alnum
        ? String.fromCharCode(c)
        : '\\u{${c.toRadixString(16).toUpperCase()}}';
  }
}

const _w = '[A-Za-z0-9_]';
const _wordBoundary = '(?:(?<=$_w)(?!$_w)|(?<!$_w)(?=$_w))';
const _notWordBoundary = '(?:(?<=$_w)(?=$_w)|(?<!$_w)(?!$_w))';

/// The grammars this app reads: the ones it ships with (obf-1036), in
/// the order a guess tries them.
class Grammars {
  final List<Grammar> all;

  Grammars(this.all);

  /// The grammars the app ships with, read once.
  static final Grammars builtIn = Grammars([
    for (final source in builtInGrammars) ?_tryParse(source),
  ]);

  static Grammar? _tryParse(String source) {
    try {
      return Grammar.parse(source);
    } on GrammarRefused {
      return null;
    }
  }

  /// The grammar whose scope is [scope], a Code text's language; null
  /// for none and for one not known here.
  Grammar? byScope(String? scope) {
    if (scope == null || scope.isEmpty) return null;
    for (final g in all) {
      if (g.scope == scope) return g;
    }
    return null;
  }

  /// The grammar a fence's [language] names (```` ```rs ````): by one of
  /// its file extensions or its name, compared in lower case; null for
  /// none and for one not known.
  Grammar? forFence(String language) {
    final name = language.trim().toLowerCase();
    if (name.isEmpty) return null;
    for (final g in all) {
      if (g.fileExtensions.contains(name) || g.name.toLowerCase() == name) {
        return g;
      }
    }
    return null;
  }

  /// The grammar a pasted [text] is guessed to be in: the first whose
  /// `first_line_match` its first line that is not blank matches; null
  /// when none does.
  Grammar? guess(String text) {
    for (final line in text.split('\n')) {
      if (line.trim().isEmpty) continue;
      final first = line.endsWith('\r')
          ? line.substring(0, line.length - 1)
          : line;
      for (final g in all) {
        if (g.matchesFirstLine(first)) return g;
      }
      return null;
    }
    return null;
  }
}

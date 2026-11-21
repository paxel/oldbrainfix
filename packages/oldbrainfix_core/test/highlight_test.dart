import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:oldbrainfix_core/src/built_in_grammars.dart';
import 'package:test/test.dart';

import '../tool/gen_grammars.dart' show grammarFiles;
import 'sync_test.dart' show ticking;

/// The coloured parts of [text] in [g] as `text:colour`, for reading.
List<String> _parts(Grammar g, String text) => [
  for (final s in g.colour(text))
    '${text.substring(s.start, s.end)}:${s.colour.name}',
];

Grammar _rust() => Grammars.builtIn.byScope('source.rust')!;

/// A grammar of [contexts], which follow its `contexts:` key.
Grammar _grammar(String contexts) =>
    Grammar.parse('name: T\nscope: source.t\ncontexts:\n$contexts');

/// A grammar whose `main` context has one item that matches [regex]
/// with [scope].
Grammar _matching(String regex, [String scope = 'keyword']) => _grammar(
  "  main:\n    - match: '${regex.replaceAll("'", "''")}'\n"
  '      scope: $scope\n',
);

/// Refused with a reason that holds [why].
Matcher _refused(String why) => throwsA(
  isA<GrammarRefused>().having((r) => r.reason, 'reason', contains(why)),
);

void main() {
  group('built-in grammars (obf-1036)', () {
    test('the generated file holds the grammar files as they are', () {
      expect(builtInGrammars, hasLength(grammarFiles.length));
      for (var k = 0; k < grammarFiles.length; k++) {
        final file = File(
          '../../assets/grammars/${grammarFiles[k]}.sublime-syntax',
        );
        expect(builtInGrammars[k], file.readAsStringSync(), reason: file.path);
      }
    });

    test('every one is read, none refused', () {
      for (final source in builtInGrammars) {
        expect(() => Grammar.parse(source), returnsNormally);
      }
      expect(Grammars.builtIn.all, hasLength(15));
      expect(Grammars.builtIn.all.map((g) => g.scope).toSet(), hasLength(15));
    });

    test('each colours its own keywords, strings and comments', () {
      expect(
        _parts(_rust(), 'fn main() { let s = "x"; } // done'),
        containsAllInOrder([
          'fn:keyword',
          'main:function',
          'let:keyword',
          '"x":string',
          '// done:comment',
        ]),
      );
    });
  });

  group('colouring', () {
    Grammar g(String contexts) =>
        Grammar.parse('name: T\nscope: source.t\ncontexts:\n$contexts');

    test('a capture shows over its match, the innermost scope wins', () {
      final t = g(
        '  main:\n'
        "    - match: '(let)\\s+(\\w+)'\n"
        '      scope: string.x\n'
        '      captures:\n'
        '        1: keyword.let\n'
        '        2: meta.name\n',
      );
      expect(_parts(t, 'let ab'), ['let:keyword', ' ab:string']);
    });

    test('a pushed context colours until it pops, its delimiters too', () {
      final t = g(
        '  main:\n'
        "    - match: '\"'\n"
        '      push: str\n'
        '  str:\n'
        '    - meta_scope: string.quoted\n'
        "    - match: '\\\\.'\n"
        '      scope: constant.character.escape\n'
        "    - match: '\"'\n"
        '      pop: true\n',
      );
      expect(_parts(t, 'a "b\\"c\nd" e'), [
        '"b:string',
        '\\":constant',
        'c\nd":string',
      ]);
    });

    test('\\w, \\b and . are ASCII and a 😀 is one character', () {
      final t = g(
        '  main:\n'
        "    - match: '\\b\\w+\\b'\n"
        '      scope: keyword\n'
        "    - match: '<.>'\n"
        '      scope: string\n',
      );
      expect(_parts(t, 'éab <😀> <é>'), [
        'ab:keyword',
        '<😀>:string',
        '<é>:string',
      ]);
    });

    test(r'(?i) folds ASCII letters, $ is before the line break', () {
      final t = g(
        '  main:\n'
        "    - match: '(?i)select'\n"
        '      scope: keyword\n'
        "    - match: '\\d+\$'\n"
        '      scope: constant.numeric\n',
      );
      expect(_parts(t, 'SeLeCt 1 2\r\n3\n'), ['SeLeCt:keyword', '3:number']);
    });

    test('a line too long stays as it is, the lines after it do not', () {
      final t = g(
        '  main:\n'
        "    - match: 'x'\n"
        '      scope: keyword\n',
      );
      final long = 'x' * (maxColouredLine + 1);
      expect(_parts(t, '$long\nx'), ['x:keyword']);
    });

    test('offsets are UTF-16 code units of the text', () {
      final spans = _rust().colour('/* 😀 */ fn');
      expect(spans.first, const CodeSpan(0, 8, CodeColour.comment));
      expect(spans.last, const CodeSpan(9, 11, CodeColour.keyword));
    });
  });

  group('what is refused', () {
    test('Sublime features beyond the subset', () {
      for (final item in [
        '    - match: a\n      embed: scope:source.y\n      escape: b\n',
        '    - match: a\n      branch_point: p\n',
        '    - include: scope:source.y\n',
        '    - match: a\n      pop: 2\n',
        "    - match: 'a*+'\n",
        "    - match: '(?>a)'\n",
        "    - match: '(a)\\1'\n",
        "    - match: '[[:alpha:]]'\n",
        "    - match: '(?:(a))+'\n",
        "    - match: '\\Ga'\n",
      ]) {
        expect(
          () => Grammar.parse(
            'name: T\nscope: source.t\ncontexts:\n  main:\n$item',
          ),
          throwsA(isA<GrammarRefused>()),
          reason: item,
        );
      }
    });
  });

  group('a span and a refusal', () {
    test('a span is equal to one of the same place and colour', () {
      const a = CodeSpan(1, 4, CodeColour.keyword);
      expect(a, const CodeSpan(1, 4, CodeColour.keyword));
      expect(a.hashCode, const CodeSpan(1, 4, CodeColour.keyword).hashCode);
      expect(a, isNot(const CodeSpan(1, 4, CodeColour.string)));
      expect(a, isNot(const CodeSpan(0, 4, CodeColour.keyword)));
      expect(a.toString(), 'CodeSpan(1, 4, keyword)');
      expect(
        const GrammarRefused('no main').toString(),
        'GrammarRefused: no main',
      );
    });
  });

  group('contexts', () {
    test('a set replaces the context, an anonymous one too', () {
      final t = _grammar(
        '  main:\n'
        "    - match: '<'\n"
        '      set:\n'
        '        - meta_scope: entity.name.tag\n'
        "        - match: '>'\n"
        '          set: main\n'
        "    - match: 'b'\n"
        '      scope: keyword\n',
      );
      // The `>` takes the scopes of the context it sets, main's: none.
      expect(_parts(t, 'b <b> b'), ['b:keyword', '<b:tag', 'b:keyword']);
    });

    test('a push of a list of names pushes each, the last on top', () {
      final t = _grammar(
        '  main:\n'
        "    - match: '\\['\n"
        '      push: [inner, outer]\n'
        '  inner:\n'
        '    - meta_scope: string\n'
        "    - match: '\\]'\n"
        '      pop: true\n'
        '  outer:\n'
        '    - meta_scope: comment\n'
        "    - match: ';'\n"
        '      pop: true\n',
      );
      // outer pops at `;`, then inner at `]`.
      expect(_parts(t, 'a [b;c] d'), ['[b;:comment', 'c]:string']);
    });

    test(
      'the prototype comes first in each context, unless it is left out',
      () {
        final t = _grammar(
          '  prototype:\n'
          "    - match: '#.*'\n"
          '      scope: comment\n'
          '  main:\n'
          "    - match: '\"'\n"
          '      push: str\n'
          "    - match: '\\('\n"
          '      push: paren\n'
          '  str:\n'
          '    - meta_include_prototype: false\n'
          '    - meta_scope: string\n'
          "    - match: '\"'\n"
          '      pop: true\n'
          '  paren:\n'
          '    - meta_scope: keyword\n'
          "    - match: '\\)'\n"
          '      pop: true\n',
        );
        expect(_parts(t, '"a # b" # c\n(d # e\n)'), [
          '"a # b":string',
          '# c:comment',
          '(d :keyword',
          '# e:comment',
          '\n):keyword',
        ]);
      },
    );

    test('variables are put in, one inside another too', () {
      final t = Grammar.parse(
        'name: T\nscope: source.t\n'
        'variables:\n'
        "  id: '[a-z_]\\w*'\n"
        "  call: '{{id}}(?=\\()'\n"
        'contexts:\n'
        '  main:\n'
        "    - match: '{{call}}'\n"
        '      scope: variable.function\n',
      );
      expect(_parts(t, 'f(x) g (y) h_1()'), ['f:function', 'h_1:function']);
    });

    test('empty matches in a row leave the rest of the line as it is', () {
      // Each push and each pop matches nothing, so the line would never
      // end; after the most in a row it is left, and the next line read.
      final t = _grammar(
        '  main:\n'
        "    - match: '(?=x)'\n"
        '      push: other\n'
        "    - match: 'y'\n"
        '      scope: keyword\n'
        '  other:\n'
        "    - match: '(?=x)'\n"
        '      pop: true\n',
      );
      expect(t.colour('yx y\ny'), const [
        CodeSpan(0, 1, CodeColour.keyword),
        CodeSpan(5, 6, CodeColour.keyword),
      ]);
    });

    test('what a context may not hold is refused, with its reason', () {
      Grammar parse(String contexts) => _grammar(contexts);
      for (final (contexts, why) in [
        ('  main:\n    - meta_scope: a\n      match: b\n', 'the key match'),
        ('  main:\n    - match: a\n    - meta_scope: b\n', 'a meta item late'),
        ('  main:\n    - meta_include_prototype: 1\n', 'meta_include'),
        (
          '  main:\n    - match: (a)\n      captures:\n        2: keyword\n',
          'the capture 2',
        ),
        ('  main:\n    - match: a\n      push: nope\n', 'no context nope'),
        ('  main:\n    - match: a\n      push: []\n', 'a push or set'),
        ('  main:\n    - match: a\n      set: 12\n', 'a push or set'),
        (
          '  main:\n    - match: a\n      push: [main, {match: b}]\n',
          'a push or set',
        ),
        ("  main:\n    - match: '{{nope}}'\n", 'no variable nope'),
      ]) {
        expect(() => parse(contexts), _refused(why), reason: contexts);
      }
      expect(
        () => Grammar.parse(
          "name: T\nscope: source.t\nvariables:\n  a: '{{b}}x'\n"
          'contexts:\n  main: []\n',
        ),
        _refused('no variable b'),
      );
    });
  });

  group('the regex subset', () {
    test('(?x) leaves out blanks and comments', () {
      final t = _grammar(
        '  main:\n'
        '    - match: |\n'
        '        (?x)\n'
        '        a \\  b  # one\n'
        '        \\# c\n'
        '      scope: keyword\n',
      );
      expect(_parts(t, 'ab#c a b#c'), ['a b#c:keyword']);
    });

    test(r'\W, \D and \B are the ASCII sets and boundary', () {
      expect(_parts(_matching(r'\W+', 'keyword.operator'), 'a+-b é'), [
        '+-:operator',
        ' é:operator',
      ]);
      expect(_parts(_matching(r'\D+'), '12ab3'), ['ab:keyword']);
      expect(_matching(r'a\B').colour('ab a a_'), const [
        CodeSpan(0, 1, CodeColour.keyword),
        CodeSpan(5, 6, CodeColour.keyword),
      ]);
    });

    test(r'\xHH, \x{H…} and \uHHHH are the characters they name', () {
      expect(_parts(_matching(r'\x41\x{1F600}\u00e9\x{62}'), 'A😀é Ab A😀éb'), [
        'A😀éb:keyword',
      ]);
      expect(_parts(_matching(r'[\x61-\x{63}]+'), 'abcd'), ['abc:keyword']);
      expect(_parts(_matching(r'[\uFF21-\uFF3A]'), 'aＢc'), ['Ｂ:keyword']);
    });

    test('a { that starts no quantifier is a literal', () {
      expect(_parts(_matching('a{b'), 'a{b'), ['a{b:keyword']);
      expect(_parts(_matching('a{2'), 'aa a{2'), ['a{2:keyword']);
      expect(_parts(_matching('a{2,x}'), 'aa a{2,x}'), ['a{2,x}:keyword']);
      expect(_parts(_matching('{a'), '{a'), ['{a:keyword']);
    });

    test('{n,} is at least n', () {
      expect(_matching('a{2,}').colour('a aa aaa'), const [
        CodeSpan(2, 4, CodeColour.keyword),
        CodeSpan(5, 8, CodeColour.keyword),
      ]);
    });

    test(
      'captures in alternatives and inside captures lie where they match',
      () {
        final alt = _grammar(
          '  main:\n'
          "    - match: '(?:(a)|(b))c'\n"
          '      captures:\n'
          '        1: keyword\n'
          '        2: string\n',
        );
        expect(alt.colour('ac bc ab'), const [
          CodeSpan(0, 1, CodeColour.keyword),
          CodeSpan(3, 4, CodeColour.string),
        ]);
        final inner = _grammar(
          '  main:\n'
          "    - match: 'x((a)b|c)'\n"
          '      captures:\n'
          '        1: string\n'
          '        2: keyword\n',
        );
        expect(inner.colour('xab xc'), const [
          CodeSpan(1, 2, CodeColour.keyword),
          CodeSpan(2, 3, CodeColour.string),
          CodeSpan(5, 6, CodeColour.string),
        ]);
      },
    );

    test('what goes beyond it is refused, with its reason', () {
      for (final (regex, why) in [
        ('a{2}{3}', 'a doubled quantifier'),
        ('(?=a)*', 'a quantifier on a lookaround'),
        ('*a', 'a quantifier without anything before it'),
        ('{2}', 'a quantifier without anything before it'),
        ('(?=(a))', 'a capture in a lookaround'),
        (r'\x{}', r'a \x{…} not closed'),
        (r'\x{41', r'a \x{…} not closed'),
        (r'\x{1234567}', r'a \x{…} not closed'),
        (r'\x4', 'a short hex escape'),
        (r'\xG1', 'a short hex escape'),
        (r'\u12', 'a short hex escape'),
        (r'\x{110000}', 'no character 110000'),
        (r'\uD800', 'no character d800'),
        (r'[\W]', r'\W in a class'),
        (r'[\b]', r'\b in a class'),
        (r'[a-\w]', 'a range to a class'),
        (r'[a-\D]', 'a range to a class'),
        ('[a-[]', 'a [ in a class'),
        (r'[a-\', r'a \ at the end'),
        (r'[\', r'a \ at the end'),
      ]) {
        expect(() => CodeRegex.parse(regex), _refused(why), reason: regex);
      }
    });
  });

  group('a lookbehind (obf-1037)', () {
    test('of one fixed width is read, of no fixed width refused', () {
      for (final read in [
        r'(?<=a)b',
        r'(?<![\w$])b',
        r'(?<=ab|cd)e',
        r'(?<=a{3}(?:bc){2})d',
        r'(?<=\b(?=x)[a-z]$)y',
        r'(?<=(?<!q)😀)z',
        r'(?<=x{1000})y',
      ]) {
        expect(CodeRegex.parse(read).groups, 0, reason: read);
      }
      for (final refused in [
        r'(?<=a+)b',
        r'(?<=a*)b',
        r'(?<=a?)b',
        r'(?<=a{1,2})b',
        r'(?<=ab|c)d',
        r'(?<!x+)y',
        r'(?<=(?:a|bc))d',
        r'(?<=x{1001})y',
        r'(?<=(?:x{10}){101})y',
      ]) {
        expect(
          () => CodeRegex.parse(refused),
          throwsA(
            isA<GrammarRefused>().having(
              (r) => r.reason,
              'reason',
              contains('no fixed width'),
            ),
          ),
          reason: refused,
        );
      }
    });
  });

  group('finding a grammar', () {
    test('by scope, none for an empty or unknown one', () {
      final all = Grammars.builtIn;
      expect(all.byScope('source.rust')?.name, 'Rust');
      expect(all.byScope(null), isNull);
      expect(all.byScope(''), isNull);
      expect(all.byScope('source.cobol'), isNull);
    });

    test("by a fence's name, in any case; none for none or an unknown", () {
      final all = Grammars.builtIn;
      expect(all.forFence('rs')?.scope, 'source.rust');
      expect(all.forFence('RUST')?.scope, 'source.rust');
      expect(all.forFence('bash')?.scope, 'source.shell');
      expect(all.forFence('ts')?.scope, 'source.js');
      expect(all.forFence(''), isNull);
      expect(all.forFence('cobol'), isNull);
    });

    test('a guess by the first line that is not blank', () {
      final all = Grammars.builtIn;
      expect(all.guess('#!/bin/bash\necho hi')?.scope, 'source.shell');
      expect(
        all.guess('\n  \n#!/usr/bin/env python3\n')?.scope,
        'source.python',
      );
      expect(all.guess('plain words'), isNull);
    });

    test('a guess reads the first line without its \\r', () {
      final all = Grammars([
        Grammar.parse(
          "name: G\nscope: source.g\nfirst_line_match: '^#!g\$'\n"
          'contexts:\n  main: []\n',
        ),
      ]);
      expect(all.guess('\r\n#!g\r\nx')?.scope, 'source.g');
      expect(all.guess('#!g x\r\n'), isNull);
    });
  });

  group("a Code text's language", () {
    late Store store;
    late Notes notes;

    setUp(() {
      store = Store.inMemory(clock: ticking());
      notes = Notes(store);
    });
    tearDown(() => store.close());

    test('is stored with the Note and read back, empty for none', () {
      final note = notes.capture('fn main() {}', format: TextFormats.code);
      expect(notes.byId(note.id)!.codeLanguage, isNull);
      notes.setCodeLanguage(note.id, 'source.rust');
      expect(store.current(note.id, NoteKeys.language), 'source.rust');
      expect(notes.byId(note.id)!.codeLanguage, 'source.rust');
      final count = store.history(note.id).length;
      notes.setCodeLanguage(note.id, 'source.rust');
      expect(store.history(note.id), hasLength(count), reason: 'the same');
      notes.setCodeLanguage(note.id, '');
      expect(notes.byId(note.id)!.codeLanguage, isNull);
    });

    test('changes nothing of the text, so search finds it as written', () {
      final note = notes.capture('let needle = 1;', format: TextFormats.code);
      notes.setCodeLanguage(note.id, 'source.rust');
      final read = notes.byId(note.id)!;
      expect(read.text, 'let needle = 1;');
      expect(read.visibleText, 'let needle = 1;');
      expect(Search(store).notes('needle').map((n) => n.id), [note.id]);
    });

    test('a language not known here is kept and colours nothing', () {
      final note = notes.capture('x', format: TextFormats.code);
      notes.setCodeLanguage(note.id, 'source.future');
      expect(notes.byId(note.id)!.codeLanguage, 'source.future');
      expect(Grammars.builtIn.byScope('source.future'), isNull);
    });
  });
}

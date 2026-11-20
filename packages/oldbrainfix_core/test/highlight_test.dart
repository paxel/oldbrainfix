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

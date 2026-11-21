import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// A grammar of the subset for [scope], colouring `MOVE` as a keyword.
String cobol({String scope = 'source.cobol', String name = 'COBOL'}) =>
    'name: $name\nscope: $scope\nfile_extensions: [cbl, rs]\n'
    'contexts:\n  main:\n    - match: \'\\bMOVE\\b\'\n      scope: keyword\n';

List<int> bytes(String text) => utf8.encode(text);

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  group("the person's own grammars (obf-1037)", () {
    test('none until one is added; the built-in ones colour', () {
      expect(ann.grammars.added, isEmpty);
      expect(ann.grammars.all.map((g) => g.scope), [
        for (final g in Grammars.builtIn.all) g.scope,
      ]);
    });

    test('an added grammar is offered, colours its fences and comes after '
        'the built-in ones; the same again writes nothing', () {
      final added = ann.addGrammar(bytes(cobol()));
      expect(added.scope, 'source.cobol');
      final grammars = ann.grammars;
      expect(grammars.added.map((g) => g.scope), ['source.cobol']);
      expect(grammars.all.last.scope, 'source.cobol');
      expect(grammars.byScope('source.cobol')?.name, 'COBOL');
      expect(grammars.forFence('CBL')?.scope, 'source.cobol');
      // The person's own one is tried first by a fence.
      expect(grammars.forFence('rs')?.scope, 'source.cobol');
      expect(grammars.forFence('rust')?.scope, 'source.rust');
      expect(grammars.byScope('source.cobol')!.colour('MOVE A'), [
        const CodeSpan(0, 4, CodeColour.keyword),
      ]);
      final written = ann.allEntries().length;
      ann.addGrammar(bytes(cobol()));
      expect(ann.allEntries().length, written);
    });

    test('one of the scope of a built-in one replaces it in its place; '
        'Remove brings the built-in one back', () {
      final at = Grammars.builtIn.all.indexWhere(
        (g) => g.scope == 'source.rust',
      );
      ann.addGrammar(bytes(cobol(scope: 'source.rust', name: 'My Rust')));
      var grammars = ann.grammars;
      expect(grammars.all.length, Grammars.builtIn.all.length);
      expect(grammars.all[at].name, 'My Rust');
      expect(grammars.byScope('source.rust')?.name, 'My Rust');
      ann.removeGrammar('source.rust');
      grammars = ann.grammars;
      expect(grammars.added, isEmpty);
      expect(grammars.byScope('source.rust')?.name, 'Rust');
      final written = ann.allEntries().length;
      ann.removeGrammar('source.rust');
      ann.removeGrammar('source.none');
      expect(ann.allEntries().length, written);
    });

    test('a newer file of the same scope replaces the one added before', () {
      ann.addGrammar(bytes(cobol()));
      ann.addGrammar(bytes(cobol(name: 'COBOL 85')));
      expect(ann.grammars.added.map((g) => g.name), ['COBOL 85']);
    });

    test('a file that is refused says why and adds nothing', () {
      final written = ann.allEntries().length;
      void refused(List<int> file, String reason) => expect(
        () => ann.addGrammar(file),
        throwsA(
          isA<GrammarRefused>().having(
            (r) => r.reason,
            'reason',
            contains(reason),
          ),
        ),
      );
      refused(bytes('name: [x\n'), 'not YAML');
      refused(bytes('name: X\nscope: source.x\n'), 'no contexts');
      refused(
        bytes(
          'name: X\nscope: source.x\ncontexts:\n  main:\n'
          '    - match: a\n      embed: scope:source.y\n      escape: b\n',
        ),
        'embed',
      );
      refused(
        bytes(
          'name: X\nscope: source.x\ncontexts:\n  main:\n'
          '    - match: \'a++\'\n',
        ),
        'possessive',
      );
      refused(
        bytes(
          'name: X\nscope: source.x\ncontexts:\n  main:\n'
          '    - match: \'(?<=a+)b\'\n',
        ),
        'no fixed width',
      );
      refused([0x6E, 0xFF, 0xFE], 'UTF-8');
      refused(bytes('#' * (maxGrammarBytes + 1)), '256 KiB');
      expect(ann.allEntries().length, written);
      expect(ann.grammars.added, isEmpty);
    });

    test('a file of the largest size and one with a byte order mark are '
        'read', () {
      final text = cobol();
      ann.addGrammar([0xEF, 0xBB, 0xBF, ...bytes(text)]);
      expect(
        ann.current(
          PersonSettingKeys.entity,
          PersonSettingKeys.grammar('source.cobol'),
        ),
        text,
      );
      final padded = '$text${'#' * (maxGrammarBytes - text.length - 1)}\n';
      expect(utf8.encode(padded).length, maxGrammarBytes);
      ann.addGrammar(bytes(padded));
      expect(ann.grammars.added.single.scope, 'source.cobol');
    });

    test('at most $maxAddedGrammars, while one of them is still replaced', () {
      for (var n = 0; n < maxAddedGrammars; n++) {
        ann.addGrammar(bytes(cobol(scope: 'source.x$n', name: 'X$n')));
      }
      expect(
        () => ann.addGrammar(bytes(cobol(scope: 'source.more'))),
        throwsA(isA<GrammarRefused>()),
      );
      ann.addGrammar(bytes(cobol(scope: 'source.x0', name: 'X0 again')));
      expect(ann.grammars.added.length, maxAddedGrammars);
      expect(ann.grammars.byScope('source.x0')?.name, 'X0 again');
    });

    test('a text kept that does not read, or under another scope, counts '
        'for nothing', () {
      ann.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.grammar('source.cobol'),
        'name: [x\n',
      );
      ann.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.grammar('source.rust'),
        cobol(),
      );
      expect(ann.grammars.added, isEmpty);
      expect(ann.grammars.byScope('source.rust')?.name, 'Rust');
    });

    test("they travel between one's own Devices", () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      ann.addGrammar(bytes(cobol()));
      exchange(desk, ann);
      expect(desk.grammars.added.map((g) => g.scope), ['source.cobol']);
      desk.removeGrammar('source.cobol');
      exchange(desk, ann);
      expect(ann.grammars.added, isEmpty);
    });

    group('in a Shared Area', () {
      late Store ben;
      late String area;

      setUp(() {
        ben = device(clock, 'Ben');
        area = ann.share(Places(ann).addArea('Household').id);
        exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      });

      test('they never reach another person, who sees code coloured by '
          'grammars of their own', () {
        ann.addGrammar(bytes(cobol(scope: 'source.rust', name: 'My Rust')));
        Notes(ann).capture('MOVE A', place: area);
        exchange(ben, ann);
        expect(ben.loggedHistory(PersonSettingKeys.entity), isEmpty);
        expect(ben.grammars.added, isEmpty);
        expect(ben.grammars.byScope('source.rust')?.name, 'Rust');
      });

      test("one another person's Device wrote counts for nothing", () {
        exchange(ben, ann);
        // As a modified app would: Ben slips a grammar into Ann's log.
        final forged = ben.append(
          PersonSettingKeys.entity,
          PersonSettingKeys.grammar('source.cobol'),
          cobol(),
        );
        ann.applyEntries([forged], verifiedDevice: ben.deviceId);
        expect(ann.allEntries().map((e) => e.id), contains(forged.id));
        expect(ann.grammars.added, isEmpty);
      });
    });
  });
}

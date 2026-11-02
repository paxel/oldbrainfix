import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'sync_test.dart' show ticking;

/// The text of a Note as stored.
String? _stored(Store store, String id) => store.current(id, NoteKeys.text);

void main() {
  group('looks like code', () {
    test('a shebang or lines mostly of code punctuation or indented', () {
      for (final text in [
        '#!/bin/bash\necho hi',
        '#!/usr/bin/env python3',
        'int main() {\n  return 0;\n}',
        'a = 1;\nb = 2;\nc = a + b;',
        'def f(x):\n    y = x\n    return y',
        '{\n  "a": 1,\n  "b": [1, 2]\n}\n',
        'for f in *; do\n\techo "\$f"\n\tls "\$f"\ndone',
        '.card {\r\n  color: red;\r\n}',
      ]) {
        expect(TextFormats.looksLikeCode(text), isTrue, reason: text);
      }
    });

    test('prose, lists, Markdown and short texts are not', () {
      for (final text in [
        '',
        '#!',
        'echo hi;',
        'a;\nb;',
        ' #!/bin/bash\necho hi',
        'Milk\nEggs\nBread',
        '- milk\n- eggs\n    - free range\n    - brown',
        '# Shopping\n- **milk**\n- eggs',
        'Dear Ann,\nsee you at 8 (the usual place).\nBen',
        'Recipe:\n    1 cup flour\n    2 eggs\nMix well.\nBake.',
        'one;\ntwo\nthree\nfour',
        '\n\n   \n\t\n',
      ]) {
        expect(TextFormats.looksLikeCode(text), isFalse, reason: text);
      }
    });

    test('only spaces and tabs count as blank and as indentation', () {
      // A no-break space is no indentation, a line of it no blank line.
      expect(TextFormats.looksLikeCode('x\n    y\nz;'), isFalse);
      expect(TextFormats.looksLikeCode('x;\n    y\nz'), isTrue);
    });
  });

  group('the format', () {
    test('a code text reads as it is written, boxes and marks included', () {
      expect(TextFormats.known, contains(TextFormats.code));
      const text = '# not a heading\n**x** ☐ #tag';
      expect(TextFormats.visible(TextFormats.code, text), text);
      expect(TextFormats.literal(TextFormats.code, '*x*'), '*x*');
      final stored = StoredText.of(TextFormats.code, text);
      expect(stored.editable, isTrue);
      expect(
        stored.encode(),
        '{"format":"code","text":"# not a heading\\n**x** ☐ #tag"}',
      );
      expect(StoredText.read(stored.encode()), stored);
    });

    test('no glyph of a code text becomes a box', () {
      var n = 0;
      String newId() => 'id${n++}';
      expect(
        TextBoxes.convert('☐ x ✅', format: TextFormats.code, newId: newId),
        '☐ x ✅',
      );
      expect(n, 0);
    });
  });

  group('Notes', () {
    late Store store;
    late Notes notes;

    setUp(() {
      store = Store.inMemory(clock: ticking());
      notes = Notes(store);
    });
    tearDown(() => store.close());

    test('a code text keeps its glyphs and has no boxes', () {
      final note = notes.capture('☐ echo', format: TextFormats.code);
      expect(_stored(store, note.id), '{"format":"code","text":"☐ echo"}');
      final read = notes.byId(note.id)!;
      expect(read.textFormat, TextFormats.code);
      expect(read.boxes, isEmpty);
      expect(read.visibleText, '☐ echo');
    });

    test('a box with its id that came in a code text is no box', () {
      final boxed = '☐${TextBoxes.encodeId('ab12')} x';
      final note = notes.capture('x');
      store.append(
        note.id,
        NoteKeys.text,
        StoredText.of(TextFormats.code, boxed).encode(),
      );
      final read = notes.byId(note.id)!;
      expect(read.boxes, isEmpty);
      expect(read.visibleText, '☐ x');
    });

    test('an edit keeps a code text code, whatever format it asks', () {
      final note = notes.capture('ls', format: TextFormats.code);
      notes.edit(note.id, 'ls -l');
      notes.edit(note.id, '**ls**', format: TextFormats.markdown);
      notes.edit(note.id, 'ls -la', format: TextFormats.plain);
      expect(_stored(store, note.id), '{"format":"code","text":"ls -la"}');
    });

    test('setFormat puts a plain or formatted text into code and back', () {
      final note = notes.capture('# comment\necho hi');
      notes.setFormat(note.id, TextFormats.markdown);
      notes.setFormat(note.id, TextFormats.code);
      expect(
        _stored(store, note.id),
        '{"format":"code","text":"# comment\\necho hi"}',
      );
      final entries = store.history(note.id).length;
      notes.setFormat(note.id, TextFormats.code);
      expect(store.history(note.id), hasLength(entries));
      notes.setFormat(note.id, TextFormats.plain);
      expect(
        _stored(store, note.id),
        '{"format":"plain","text":"# comment\\necho hi"}',
      );
      notes.setFormat(note.id, TextFormats.code);
      notes.setFormat(note.id, TextFormats.markdown);
      expect(notes.byId(note.id)!.textFormat, TextFormats.markdown);
      // A formatted text still never goes back to plain.
      notes.setFormat(note.id, TextFormats.plain);
      expect(notes.byId(note.id)!.textFormat, TextFormats.markdown);
    });

    test('into code a text goes as it shows; back, its glyphs are boxes', () {
      final note = notes.capture('☐ milk ☐ eggs');
      final [milk, _] = note.boxes;
      Boxes(store).tick(note.id, milk.id, BoxState.done);
      notes.setFormat(note.id, TextFormats.code);
      expect(
        _stored(store, note.id),
        '{"format":"code","text":"✅ milk ☐ eggs"}',
      );
      notes.setFormat(note.id, TextFormats.plain);
      final back = notes.byId(note.id)!;
      expect(back.boxes, hasLength(2));
      expect(back.visibleText, '✅ milk ☐ eggs');
    });

    test('a code Note becomes no Checklist or Shopping List', () {
      final note = notes.capture('Script\nls\npwd', format: TextFormats.code);
      expect(Checklists(store).mayMake(note.id), isFalse);
      expect(ShoppingLists(store).mayMake(note.id), isFalse);
      notes.setFormat(note.id, TextFormats.plain);
      expect(Checklists(store).mayMake(note.id), isTrue);
      expect(ShoppingLists(store).mayMake(note.id), isTrue);
    });

    test('search finds the words of a code text', () {
      final note = notes.capture(
        'grep -r **needle** .',
        format: TextFormats.code,
      );
      expect(Search(store).notes('**needle**').map((n) => n.id), [note.id]);
    });
  });
}

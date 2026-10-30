import 'dart:math';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'conflicts_test.dart' as conflicts;
import 'sent_items_test.dart' as sent;
import 'sync_test.dart' show ticking;

/// [spans] as a short string: text in quotes, the others by name around
/// what is inside them.
String _tree(List<MarkdownSpan> spans) => [
  for (final s in spans)
    switch (s.kind) {
      MarkdownSpanKind.text => '"${s.text}"',
      MarkdownSpanKind.code => 'code"${s.text}"',
      MarkdownSpanKind.link => 'link<${s.url}>(${_tree(s.children)})',
      _ => '${s.kind.name}(${_tree(s.children)})',
    },
].join(' ');

/// The one line of [text] as [_tree] writes its parts.
String _line(String text) => _tree(Markdown.parse(text).single.spans);

void main() {
  group('reading', () {
    test('each format is a part of its line, its source where it lies', () {
      const text = '**bold** *it* ~~gone~~ `co de` [site](https://x.org)';
      expect(
        _line(text),
        'bold("bold") " " italic("it") " " strike("gone") " " code"co de" '
        '" " link<https://x.org>("site")',
      );
      final [bold, ..., link] = Markdown.parse(text).single.spans;
      expect(text.substring(bold.start, bold.end), '**bold**');
      expect(text.substring(bold.contentStart, bold.contentEnd), 'bold');
      expect(text.substring(link.contentStart, link.contentEnd), 'site');
      expect(text.substring(link.contentEnd, link.end), '](https://x.org)');
      expect(_line('__b__ _i_'), 'bold("b") " " italic("i")');
      expect(_line('***x***'), 'italic(bold("x"))');
      expect(_line('**a *b* c**'), 'bold("a " italic("b") " c")');
      expect(_line('[**a** b](u)'), 'link<u>(bold("a") " b")');
    });

    test('emphasis follows the flanking rules', () {
      expect(_line('2 * 3 * 4'), '"2 * 3 * 4"');
      expect(_line('2*3*4'), '"2" italic("3") "4"');
      expect(_line('snake_case_name'), '"snake_case_name"');
      expect(_line('**unclosed'), '"**unclosed"');
      expect(_line('a * not it*'), '"a * not it*"');
      expect(_line('~x~ ~~~x~~~'), '"~x~ ~~~x~~~"');
      expect(_line(r'\*not\* \\'), r'"*" "not" "*" " " "\"');
      expect(_line('``a`b``'), 'code"a`b"');
      expect(_line('` x `'), 'code"x"');
      expect(_line('`open'), '"`open"');
    });

    test('lines are headings, quotes, items or paragraph lines', () {
      const text =
          '# One\n## Two\n> said\n- a\n  - b\n* c\n+ d\n1. one\n1. two\n'
          '   1. sub\n3. three\n\n5. five';
      expect(
        [
          for (final b in Markdown.parse(text))
            '${b.kind.name} ${b.level} ${b.number} ${b.visible}',
        ],
        [
          'heading1 0 0 One',
          'heading2 0 0 Two',
          'quote 0 0 said',
          'bullet 0 0 a',
          'bullet 1 0 b',
          'bullet 0 0 c',
          'bullet 0 0 d',
          'numbered 0 1 one',
          'numbered 0 2 two',
          'numbered 1 1 sub',
          'numbered 0 3 three',
          'paragraph 0 0 ',
          'numbered 0 5 five',
        ],
      );
      expect(Markdown.parse('a\r\nb\rc').map((b) => b.line), [0, 1, 2]);
    });

    test('whatever the subset does not know shows as it is written', () {
      for (final text in [
        '### Three',
        '#NoSpace',
        '```',
        '| a | b |',
        '![photo](u)',
        '<https://x.org>',
        '<b>bold</b>',
        '&amp;',
        '-x',
        '1.x',
        '1234567890. ten digits',
        '---',
        '    > four spaces',
        '[no link] (u)',
        '[a](u "title")',
      ]) {
        expect(Markdown.plain(text), text, reason: text);
        expect(Markdown.uses(text), isFalse, reason: text);
      }
      // No link inside a link.
      expect(_line('[a [b](c)](d)'), 'link<d>("a [b](c)")');
    });

    test('the visible text keeps every line break as it is', () {
      expect(
        Markdown.plain('# Shop\r\n- **milk**\n\n> `x` [y](z)'),
        'Shop\r\nmilk\n\nx y',
      );
      expect(Markdown.uses('just text\n- '), isTrue);
      expect(Markdown.uses(r'just \*text\*'), isFalse);
    });

    test('a formatted text is stored as it is written, byte for byte', () {
      const source = '# Héading\r\n- **milk** 🧠\n\t* x\\y';
      final stored = StoredText.of(TextFormats.markdown, source);
      expect(
        stored.encode(),
        '{"format":"markdown","text":"# Héading\\r\\n- **milk** 🧠\\n\\t* x\\\\y"}',
      );
      final read = StoredText.read(stored.encode());
      expect(read.text, source);
      expect(read.editable, isTrue);
      expect(read.visible, 'Héading\r\nmilk 🧠\nx\\y');
    });
  });

  group('escaping', () {
    test('a plain text escaped reads as itself and uses no format', () {
      for (final text in [
        '# not a heading',
        '- not an item\n  + nor this\n1. nor this',
        '> nor a quote',
        '**not bold** *nor* ~~this~~ `or` [this](u) ![or](this)',
        r'C:\path\*x \\ end\',
        '#tag snake_case my_tag _x_ a~b ~~',
        '',
      ]) {
        final escaped = Markdown.escape(text);
        expect(Markdown.plain(escaped), text, reason: escaped);
        expect(Markdown.uses(escaped), isFalse, reason: escaped);
      }
      expect(Markdown.escape('#tag my_tag'), '#tag my_tag');
      expect(
        Markdown.escape('# x\n1. y\n* z'),
        r'\# x'
        '\n'
        r'1\. y'
        '\n'
        r'\* z',
      );
    });

    test('any text escaped reads as itself', () {
      const alphabet = [
        ' ', '\t', '\n', '\r', 'a', 'b', '1', '.', '#', '>', '-', '+', //
        '*', '_', '~', '`', '[', ']', '(', ')', '!', r'\', 'ü', '🧠',
        '\u00a0',
      ];
      final random = Random(127);
      for (var n = 0; n < 3000; n++) {
        final text = [
          for (var i = random.nextInt(14); i > 0; i--)
            alphabet[random.nextInt(alphabet.length)],
        ].join();
        final escaped = Markdown.escape(text);
        expect(Markdown.plain(escaped), text, reason: escaped);
        expect(Markdown.uses(escaped), isFalse, reason: escaped);
      }
    });
  });

  group('tags and status marks', () {
    test('a tag in code or in a link\'s destination is no tag', () {
      const text = '#a `#b` [#c](https://x.org/#d) #e\n# Heading #f';
      expect(
        [for (final t in Markdown.tags(text)) t.tag],
        ['#a', '#c', '#e', '#f'],
      );
      // As plain text all count.
      expect(TextTags.find(text), hasLength(5));
    });

    test('a status mark at the start of an item still counts', () {
      const text = '- ☐ milk\n* ✅ **eggs**';
      expect(
        [for (final m in TextMarks.find(text)) m.mark],
        [TextMark.open, TextMark.checked],
      );
      expect(Markdown.plain(text), '☐ milk\n✅ eggs');
    });
  });

  group('editing', () {
    test('a format goes on the word at the cursor and comes off again', () {
      final on = Markdown.toggle('buy milk now', 5, 5, MarkdownSpanKind.bold);
      expect(on, (text: 'buy **milk** now', start: 7, end: 7));
      expect(Markdown.toggle(on.text, 7, 7, MarkdownSpanKind.bold), (
        text: 'buy milk now',
        start: 5,
        end: 5,
      ));
      // At the word's end the word counts.
      expect(Markdown.toggle('milk', 4, 4, MarkdownSpanKind.italic), (
        text: '*milk*',
        start: 5,
        end: 5,
      ));
      // Without a word both marks go in, the cursor between them.
      expect(Markdown.toggle('a  b', 2, 2, MarkdownSpanKind.strike), (
        text: 'a ~~~~ b',
        start: 4,
        end: 4,
      ));
      // A tag's # goes with its word.
      expect(
        Markdown.toggle('see #garten', 7, 7, MarkdownSpanKind.bold).text,
        'see **#garten**',
      );
    });

    test('a selection is wrapped line by line, its spaces left out', () {
      expect(Markdown.toggle('buy milk now', 3, 9, MarkdownSpanKind.italic), (
        text: 'buy *milk* now',
        start: 3,
        end: 11,
      ));
      expect(Markdown.toggle('one\n\ntwo', 0, 8, MarkdownSpanKind.strike), (
        text: '~~one~~\n\n~~two~~',
        start: 2,
        end: 14,
      ));
      // When every piece has it, it comes off.
      expect(
        Markdown.toggle('**one**\n**two**', 0, 15, MarkdownSpanKind.bold),
        (text: 'one\ntwo', start: 0, end: 7),
      );
      // A piece without it gets it, the other keeps it.
      expect(
        Markdown.toggle('**one**\ntwo', 0, 11, MarkdownSpanKind.bold).text,
        '**one**\n**two**',
      );
      // Code gets more backticks than any inside it.
      expect(
        Markdown.toggle('x a`b y', 2, 5, MarkdownSpanKind.code).text,
        'x ``a`b`` y',
      );
      expect(Markdown.toggle('`', 0, 1, MarkdownSpanKind.code).text, '`` ` ``');
      expect(
        () => Markdown.toggle('x', 0, 1, MarkdownSpanKind.link),
        throwsArgumentError,
      );
    });

    test('a link goes on, changes its destination and comes off', () {
      final on = Markdown.link('see docs', 6, 6, ' https://x.org/a b(c) ');
      expect(on.text, r'see [docs](https://x.org/a%20b\(c\))');
      expect(
        Markdown.parse(on.text).single.spans.last.url,
        'https://x.org/a%20b(c)',
      );
      expect(
        Markdown.link(on.text, 6, 6, 'https://y.org').text,
        'see [docs](https://y.org)',
      );
      expect(Markdown.link(on.text, 6, 6, '  ').text, 'see docs');
      expect(Markdown.link('see ', 4, 4, 'https://x.org'), (
        text: 'see [https://x.org](https://x.org)',
        start: 5,
        end: 18,
      ));
      expect(Markdown.link('see', 1, 1, '').text, 'see');
      // A label made of a destination with brackets still reads as one.
      final bracketed = Markdown.link('', 0, 0, r'https://x.org/[a]\]').text;
      final [inserted] = Markdown.parse(bracketed).single.spans;
      expect(inserted.kind, MarkdownSpanKind.link);
      expect(inserted.visible, r'https://x.org/[a]\]');
      expect(inserted.url, r'https://x.org/[a]\]');
    });

    test('only a web or e-mail address is opened from a link', () {
      for (final url in [
        'https://x.org',
        'HTTP://x.org/a',
        'mailto:ann@x.org',
        'MailTo:ann@x.org',
      ]) {
        expect(Markdown.mayOpen(url), isTrue, reason: url);
      }
      for (final url in [
        'javascript:alert(1)',
        'file:///etc/passwd',
        'intent://x#Intent;end',
        'oldbrainfix://x',
        'tel:123',
        'data:text/html,x',
        'x.org',
        '//x.org',
        'https:x.org',
        ' https://x.org',
        'ʜttps://x.org',
        '',
      ]) {
        expect(Markdown.mayOpen(url), isFalse, reason: url);
      }
    });

    test('a line\'s format replaces its mark, and comes off again', () {
      expect(
        Markdown.toggleLine('one\ntwo', 0, 7, MarkdownBlockKind.numbered),
        (text: '1. one\n2. two', start: 3, end: 13),
      );
      expect(
        Markdown.toggleLine(
          '1. one\n2. two',
          0,
          13,
          MarkdownBlockKind.numbered,
        ),
        (text: 'one\ntwo', start: 0, end: 7),
      );
      expect(Markdown.toggleLine('  - one', 0, 0, MarkdownBlockKind.heading1), (
        text: '# one',
        start: 2,
        end: 2,
      ));
      expect(
        Markdown.toggleLine('  - one', 7, 7, MarkdownBlockKind.numbered).text,
        '  1. one',
      );
      expect(
        Markdown.toggleLine('1. a\nb', 5, 5, MarkdownBlockKind.numbered).text,
        '1. a\n2. b',
      );
      expect(Markdown.toggleLine('', 0, 0, MarkdownBlockKind.bullet), (
        text: '- ',
        start: 2,
        end: 2,
      ));
      // A selection ending at a line's start leaves that line out.
      expect(
        Markdown.toggleLine('a\nb', 0, 2, MarkdownBlockKind.quote).text,
        '> a\nb',
      );
      expect(
        Markdown.toggleLine('# a', 3, 3, MarkdownBlockKind.paragraph).text,
        'a',
      );
    });

    test('an item goes one level in and out', () {
      expect(Markdown.indent('- a\n- b\nc', 0, 9), (
        text: '  - a\n  - b\nc',
        start: 2,
        end: 13,
      ));
      expect(
        Markdown.indent('  - a\n\t- b', 0, 10, out: true).text,
        '- a\n- b',
      );
      expect(Markdown.indent('- a', 0, 0, out: true).text, '- a');
    });

    test('a shortcut is told when typed', () {
      for (final (text, cursor) in [
        ('# ', 2),
        ('x\n## ', 5),
        ('- ', 2),
        ('* ', 2),
        ('1. ', 3),
        ('> ', 2),
        ('**x**', 5),
        ('a *x*', 5),
        ('~~x~~', 5),
        ('`x`', 3),
        ('[a](b)', 6),
      ]) {
        expect(Markdown.shortcutAt(text, cursor), isTrue, reason: text);
      }
      for (final (text, cursor) in [
        ('#', 1),
        ('#x', 2),
        ('>', 1),
        ('**x**', 4),
        ('**x', 3),
        ('- a', 3),
        ('2 * 3 *', 7),
      ]) {
        expect(Markdown.shortcutAt(text, cursor), isFalse, reason: text);
      }
    });

    test('the word at an offset', () {
      expect(Markdown.wordAt("it's a to-do, ok", 1), (0, 4));
      expect(Markdown.wordAt('call #to-do now', 8), (5, 11));
      expect(Markdown.wordAt('a , b', 2), (2, 2));
      expect(Markdown.wordAt('🧠x Straße', 9), (4, 10));
      expect(Markdown.wordAt('x', 9), (0, 1));
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

    test('capture and Amend write a formatted text', () {
      final note = notes.capture('# Shop', format: TextFormats.markdown);
      final section = notes.amend(
        note.id,
        '~~old~~ new',
        format: TextFormats.markdown,
      );
      expect(
        store.current(note.id, NoteKeys.text),
        '{"format":"markdown","text":"# Shop"}',
      );
      expect(notes.byId(note.id)!.visibleText, 'Shop');
      expect(section.textFormat, TextFormats.markdown);
      expect(section.visibleText, 'old new');
      expect(() => notes.capture('x', format: 'x-future'), throwsArgumentError);
    });

    test('a plain text becomes formatted at its first format and stays so', () {
      final note = notes.capture('Call mum');
      notes.edit(note.id, 'Call mum later');
      expect(notes.byId(note.id)!.textFormat, TextFormats.plain);
      notes.edit(note.id, 'Call **mum**', format: TextFormats.markdown);
      expect(notes.byId(note.id)!.textFormat, TextFormats.markdown);
      // An edit without a format, or asking for plain, keeps it formatted.
      notes.edit(note.id, 'Call **mum** now');
      notes.edit(note.id, 'Call mum', format: TextFormats.plain);
      expect(
        store.current(note.id, NoteKeys.text),
        '{"format":"markdown","text":"Call mum"}',
      );
      expect(
        () => notes.edit(note.id, 'x', format: 'x-future'),
        throwsArgumentError,
      );
    });

    test('setFormat makes a plain text formatted, never back', () {
      final note = notes.capture('- milk');
      notes.setFormat(note.id, TextFormats.markdown);
      expect(
        store.current(note.id, NoteKeys.text),
        '{"format":"markdown","text":"- milk"}',
      );
      final entries = store.history(note.id).length;
      notes.setFormat(note.id, TextFormats.plain);
      notes.setFormat(note.id, TextFormats.markdown);
      expect(store.history(note.id), hasLength(entries));
    });

    test('a Transcript taken over into a formatted text reads as itself', () {
      final note = notes.capture('**Todo**', format: TextFormats.markdown);
      final written = notes.takeOver(
        note.id,
        '2*3*4 and #tag_x\n- not a list ',
        append: true,
      );
      expect(written, '**Todo**\n2\\*3\\*4 and #tag_x\n\\- not a list');
      final got = notes.byId(note.id)!;
      expect(got.text, written);
      expect(got.visibleText, 'Todo\n2*3*4 and #tag_x\n- not a list');
      expect(notes.takeOver(note.id, '*x*', append: false), r'\*x\*');
      // Into a plain text it goes as it is.
      final plain = notes.capture('Plain');
      expect(notes.takeOver(plain.id, '*x*', append: true), 'Plain\n*x*');
    });

    test('a Checklist or Shopping List keeps the text formatted', () {
      final note = notes.capture(
        '**Packing**\n- socks',
        format: TextFormats.markdown,
      );
      final checklists = Checklists(store);
      checklists.make(note.id);
      expect(
        store.current(note.id, NoteKeys.text),
        '{"format":"markdown","text":"**Packing**"}',
      );
      checklists.toPlain(note.id);
      expect(notes.byId(note.id)!.textFormat, TextFormats.markdown);
      final shopping = ShoppingLists(store);
      final list = notes.capture(
        '# Weekly\nmilk',
        format: TextFormats.markdown,
      );
      shopping.make(list.id);
      expect(notes.byId(list.id)!.textFormat, TextFormats.markdown);
      shopping.toPlain(list.id, (utc) => utc);
      expect(notes.byId(list.id)!.textFormat, TextFormats.markdown);
    });

    test('Search finds the words of a formatted text, never its marks', () {
      final bold = notes.capture(
        '**bold** `code`',
        format: TextFormats.markdown,
      );
      final plain = notes.capture('stars ** here');
      final host = notes.capture('Host');
      notes.amend(host.id, '~~struck~~', format: TextFormats.markdown);
      final merged = notes.capture(
        '[label](https://x.org)',
        format: 'markdown',
      );
      notes.merge(merged.id, host.id);
      final search = Search(store);
      expect(search.notes('bold').map((n) => n.id), [bold.id]);
      expect(search.notes('bold code').map((n) => n.id), [bold.id]);
      expect(search.notes('**').map((n) => n.id), [plain.id]);
      expect(search.notes('`'), isEmpty);
      expect(search.notes('struck').map((n) => n.id), [host.id]);
      expect(search.notes('~~'), isEmpty);
      expect(search.notes('label').map((n) => n.id), [host.id]);
      expect(search.notes('x.org'), isEmpty);
    });
  });

  test('a formatted text goes out in a Sent Item and arrives editable', () {
    final clock = ticking();
    final ann = sent.device(clock, 'Ann');
    final ben = sent.device(clock, 'Ben');
    final note = Notes(ann).capture('**Milk**', format: TextFormats.markdown);
    Notes(ann).amend(note.id, '- oat', format: TextFormats.markdown);
    final item = openSentItem(ben, sendNote(ann, note.id)).item;
    expect(item.encode(), contains('"note":{"format":"markdown","sections":['));
    expect(item.sections.single.format, TextFormats.markdown);
    final got = Notes(ben).byId(acceptSentItem(ben, item)!)!;
    expect(got.textFormat, TextFormats.markdown);
    expect(got.textEditable, isTrue);
    expect(got.text, '**Milk**');
    expect(Notes(ben).sections(got.id).single.textFormat, TextFormats.markdown);
  });

  test('a Conflict over a formatted text is resolved as over a plain one', () {
    final clock = conflicts.ticking();
    final phone = conflicts.device(clock);
    final desk = conflicts.device(clock);
    addTearDown(phone.close);
    addTearDown(desk.close);
    final note = Notes(phone).capture('Milk', format: TextFormats.markdown);
    conflicts.exchange(phone, desk);
    Notes(phone).edit(note.id, '**Milk**');
    Notes(desk).edit(note.id, '*Milk*');
    conflicts.exchange(phone, desk);
    final conflict = phone.conflicts().single;
    expect(conflict.resolvable, isTrue);
    phone.resolveConflict(conflict, conflict.versions.first);
    expect(phone.conflicts(), isEmpty);
    expect(Notes(phone).byId(note.id)!.textFormat, TextFormats.markdown);
  });
}

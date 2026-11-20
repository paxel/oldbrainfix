import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  String adoc(String text) =>
      ImportFormats.read(ImportFormat.asciiDoc, text).text;
  String org(String text) => ImportFormats.read(ImportFormat.org, text).text;
  String rst(String text) =>
      ImportFormats.read(ImportFormat.restructuredText, text).text;
  String html(String text) => ImportFormats.read(ImportFormat.html, text).text;

  test('takes the extensions of the four formats only', () {
    expect(
      {
        for (final e in [
          'adoc',
          'asciidoc',
          'asc',
          'org',
          'rst',
          'html',
          'htm',
          'md',
          'txt',
          'xhtml',
        ])
          e: ImportFormats.ofExtension(e),
      },
      {
        'adoc': ImportFormat.asciiDoc,
        'asciidoc': ImportFormat.asciiDoc,
        'asc': ImportFormat.asciiDoc,
        'org': ImportFormat.org,
        'rst': ImportFormat.restructuredText,
        'html': ImportFormat.html,
        'htm': ImportFormat.html,
        'md': null,
        'txt': null,
        'xhtml': null,
      },
    );
  });

  group('AsciiDoc', () {
    test('becomes what the subset has', () {
      expect(
        adoc(
          '= Garden\n'
          '== Planting\n'
          'Plant *tomatoes*, _basil_ and `bed 2`, **every**day +\n'
          'and [.line-through]#never# at noon, see '
          'https://example.org/plan[the plan^].\n'
          '\n'
          '* Seeds\n'
          '** Carrots\n'
          '. First\n'
          '. Second\n'
          '\n'
          '[source,python]\n'
          '----\n'
          'print("**x**")\n'
          '----\n'
          '____\n'
          'A *quote*.\n'
          '____\n'
          '|===\n'
          '|Plant |Month\n'
          '\n'
          '|Tomato\n'
          '|May\n'
          '|===',
        ),
        '# Garden\n'
        '## Planting\n'
        'Plant **tomatoes**, *basil* and `bed 2`, **every**day\n'
        'and ~~never~~ at noon, see [the plan](https://example.org/plan).\n'
        '\n'
        '- Seeds\n'
        '  - Carrots\n'
        '1. First\n'
        '2. Second\n'
        '\n'
        '```python\n'
        'print("**x**")\n'
        '```\n'
        '> A **quote**.\n'
        '\n'
        '| Plant | Month |\n'
        '| --- | --- |\n'
        '| Tomato | May |',
      );
    });

    test('keeps what the subset lacks as it is written', () {
      expect(
        adoc(
          ':author: Ada\n'
          '=== Deeper\n'
          '// a comment\n'
          'NOTE: Keep the *hose* coiled.\n'
          '[quote, Ada]\n'
          '.A title\n'
          'image::photo.png[A photo]\n'
          '<<ref,see there>> and #marked#',
        ),
        ':author: Ada\n'
        '=== Deeper\n'
        '// a comment\n'
        'NOTE: Keep the **hose** coiled.\n'
        '\\[quote, Ada]\n'
        '.A title\n'
        'image::photo.png\\[A photo]\n'
        '<<ref,see there>> and #marked#',
      );
    });

    test('reads an include as the line it is, and nothing of its file', () {
      expect(
        adoc('include::secret.adoc[]\ninclude::https://x.org/a.adoc[]'),
        'include::secret.adoc\\[]\ninclude::https://x.org/a.adoc\\[]',
      );
    });
  });

  group('Org', () {
    test('becomes what the subset has', () {
      expect(
        org(
          '#+TITLE: Week\n'
          '* Shopping\n'
          '  - milk and *butter*\n'
          '    - rye\n'
          '  1. first\n'
          '  2) second\n'
          '** Notes\n'
          'Call =Bob= about the +old+ ~car~ and /that/, '
          'see [[https://example.org/a][the page]].\n'
          '#+BEGIN_SRC shell\n'
          '  echo "hi"\n'
          '  ,* not a heading\n'
          '#+END_SRC\n'
          '#+begin_quote\n'
          'Be kind.\n'
          '#+end_quote\n'
          '| Name | Age |\n'
          '|------+-----|\n'
          '| Ann  | 3   |\n'
          'Line one\\\\',
        ),
        '# Week\n'
        '# Shopping\n'
        '- milk and **butter**\n'
        '  - rye\n'
        '1. first\n'
        '2. second\n'
        '## Notes\n'
        'Call `Bob` about the ~~old~~ `car` and *that*, '
        'see [the page](https://example.org/a).\n'
        '```shell\n'
        'echo "hi"\n'
        '* not a heading\n'
        '```\n'
        '> Be kind.\n'
        '\n'
        '| Name | Age |\n'
        '| --- | --- |\n'
        '| Ann | 3 |\n'
        '\n'
        'Line one',
      );
    });

    test('keeps what the subset lacks as it is written', () {
      expect(
        org(
          '*** Deeper\n'
          ':PROPERTIES:\n'
          ':ID: 1234\n'
          ':END:\n'
          '#+AUTHOR: Ada\n'
          '# a comment\n'
          '_underlined_ and [[file:other.org][other]]',
        ),
        '\\*\\*\\* Deeper\n'
        ':PROPERTIES:\n'
        ':ID: 1234\n'
        ':END:\n'
        '#+AUTHOR: Ada\n'
        '\\# a comment\n'
        '\\_underlined\\_ and other',
      );
    });

    test('reads an include as the line it is, and nothing of its file', () {
      expect(org('#+INCLUDE: "secret.org"'), '#+INCLUDE: "secret.org"');
    });
  });

  group('reStructuredText', () {
    test('becomes what the subset has', () {
      expect(
        rst(
          '=======\n'
          'Recipes\n'
          '=======\n'
          '\n'
          'Soup\n'
          '====\n'
          '\n'
          'Cut **onions** and *carrots*, use ``salt``, see '
          '`the site <https://example.org/soup>`_.\n'
          '\n'
          '- one\n'
          '\n'
          '  - deep\n'
          '\n'
          '#. auto\n'
          '#. again\n'
          '\n'
          'Example::\n'
          '\n'
          '    print("x")\n'
          '\n'
          '.. code-block:: python\n'
          '   :linenos:\n'
          '\n'
          '   def f():\n'
          '       return 1\n'
          '\n'
          '+------+-----+\n'
          '| Name | Qty |\n'
          '+======+=====+\n'
          '| Egg  | 2   |\n'
          '+------+-----+',
        ),
        '# Recipes\n'
        '\n'
        '## Soup\n'
        '\n'
        'Cut **onions** and *carrots*, use `salt`, see '
        '[the site](https://example.org/soup).\n'
        '\n'
        '- one\n'
        '\n'
        '  - deep\n'
        '\n'
        '1. auto\n'
        '2. again\n'
        '\n'
        'Example:\n'
        '```\n'
        'print("x")\n'
        '```\n'
        '\n'
        '```python\n'
        'def f():\n'
        '    return 1\n'
        '```\n'
        '\n'
        '| Name | Qty |\n'
        '| --- | --- |\n'
        '| Egg | 2 |',
      );
    });

    test('keeps what the subset lacks as it is written', () {
      expect(
        rst(
          'Top\n'
          '===\n'
          '\n'
          'Mid\n'
          '---\n'
          '\n'
          'Third\n'
          '~~~~~\n'
          '\n'
          '.. note:: Keep warm.\n'
          '\n'
          ':Author: Ada\n'
          'Escaped \\*star\\* and :role:`text`.',
        ),
        '# Top\n'
        '\n'
        '## Mid\n'
        '\n'
        'Third\n'
        '\\~\\~\\~\\~\\~\n'
        '\n'
        '.. note:: Keep warm.\n'
        '\n'
        ':Author: Ada\n'
        'Escaped \\*star\\* and :role:\\`text\\`.',
      );
    });

    test('reads an include as the line it is, and nothing of its file', () {
      expect(
        rst('.. include:: secret.rst\n   :literal:\n\nAfter'),
        '.. include:: secret.rst\n   :literal:\n\nAfter',
      );
    });
  });

  group('HTML', () {
    test('becomes what the subset has, its title apart', () {
      final page = ImportFormats.read(
        ImportFormat.html,
        '<!DOCTYPE html><html><head><title>My &amp; page</title></head>'
        '<body><h1>Hello   <em>world</em></h1>\n'
        '<p>Some <b>bold </b>text and <a href="https://example.org/?a=1&amp;b=2">'
        'a link</a>.<br>Next &copy; &#x1F600; &bogus;</p>'
        '<ul><li>One<ul><li>Inner</li></ul></li><li><p>Two</p></li></ul>'
        '<ol start="3"><li>Three</li><li>Four</li></ol>'
        '<blockquote><p>Quoted *stars*</p></blockquote>'
        '<pre><code class="language-js">let a = 1;\n  b &lt; 2;\n</code></pre>'
        '<table><tr><th>A</th><th>B|C</th></tr>'
        '<tr><td>1</td><td><code>x|y</code></td></tr></table>'
        '<h3>Small</h3><div>one</div><div>two <s>gone</s></div>',
      );
      expect(page.title, 'My & page');
      expect(
        page.text,
        '# Hello *world*\n'
        '\n'
        'Some **bold** text and [a link](https://example.org/?a=1&b=2).\n'
        'Next © 😀 &bogus;\n'
        '\n'
        '- One\n'
        '  - Inner\n'
        '- Two\n'
        '\n'
        '3. Three\n'
        '4. Four\n'
        '\n'
        '> Quoted \\*stars\\*\n'
        '\n'
        '```js\n'
        'let a = 1;\n'
        '  b < 2;\n'
        '```\n'
        '\n'
        '| A | B\\|C |\n'
        '| --- | --- |\n'
        '| 1 | `x\\|y` |\n'
        '\n'
        'Small\n'
        '\n'
        'one\n'
        'two ~~gone~~',
      );
    });

    test('drops scripts, styles and comments, and fetches no image', () {
      final text = html(
        '<style>p { color: red }</style>'
        '<SCRIPT type="text/javascript">steal("<p>")</SCRIPT>'
        '<p onclick="steal()">Kept<!-- hidden --></p>'
        '<img src="https://tracker.example/pixel.png" alt="A cat">'
        '<a href="javascript:steal()">bad</a> '
        '<a href="file:///etc/passwd">local</a>'
        '<script>never ends',
      );
      expect(text, 'Kept\n\nA cat bad local');
      expect(text, isNot(contains('steal')));
      expect(text, isNot(contains('tracker')));
      expect(text, isNot(contains('color')));
    });

    test('leaves broken references and markup as text', () {
      expect(
        html('a &#0; &#xD800; &#x110000; &amp b < c <b>bold'),
        'a &#0; &#xD800; &#x110000; &amp b < c **bold**',
      );
    });
  });

  test('escapes what would read as a mark', () {
    expect(
      adoc('# not a heading\n- [ ] a | b \\'),
      '\\# not a heading\n- \\[ ] a \\| b \\\\',
    );
    expect(html('<p>1. one</p><p>> two</p>'), '1\\. one\n\n\\> two');
  });

  test('reads a long line of marks in a time that grows with its length', () {
    final marks = List.filled(200000, '*_`+[=/~').join();
    final watch = Stopwatch()..start();
    for (final format in ImportFormat.values) {
      ImportFormats.read(format, marks);
      ImportFormats.read(format, '<a$marks');
    }
    expect(watch.elapsed, lessThan(const Duration(seconds: 20)));
  });

  group('an Import of a folder', () {
    late Store store;
    final now = DateTime.utc(2026, 11, 20, 12);
    final may = DateTime.utc(2025, 5, 2, 9, 30);

    setUp(() => store = Store.inMemory(clock: () => now));
    tearDown(() => store.close());

    ImportFile file(String name, String text) =>
        ImportFile(name, may, Uint8List.fromList(utf8.encode(text)));

    test('makes a formatted Note of each, mixed with Markdown and text', () {
      final summary = Imports(store).folder('From Notes', [
        file('Garden.adoc', '= Garden\n*Water* daily.'),
        file('week.ORG', '* Monday\n- call'),
        file('Soup.rst', 'Cut **onions**.'),
        file('page.htm', '<title>Trip</title><h1>Trip</h1><p>Pack</p>'),
        file('Plain.txt', 'some text'),
        file('Marked.md', '# Marked'),
        file('slides.pdf', '%PDF'),
        file('.hidden.org', '* hidden'),
      ]);
      expect(summary.notes, 6);
      expect(summary.notText, 2);
      final inside = Notes(store).inside(summary.areaId!);
      expect(
        {for (final n in inside) n.text: n.textFormat},
        {
          '# Garden\n**Water** daily.': TextFormats.markdown,
          '# week\n# Monday\n- call': TextFormats.markdown,
          '# Soup\nCut **onions**.': TextFormats.markdown,
          '# page\n# Trip\n\nPack': TextFormats.markdown,
          'Plain\nsome text': TextFormats.plain,
          '# Marked': TextFormats.markdown,
        },
      );
      expect(inside.map((n) => n.captured).toSet(), {may});
    });

    test('reads no file an include names', () {
      final summary = Imports(store).folder('From Notes', [
        file('Main.adoc', 'include::Secret.txt[]'),
        file('Main.rst', '.. include:: Secret.txt'),
        file('Main.org', '#+INCLUDE: "Secret.txt"'),
        file('Secret.txt', 'the secret'),
      ]);
      final texts = [
        for (final n in Notes(store).inside(summary.areaId!)) n.text,
      ];
      expect(texts.where((t) => t.contains('the secret')), [
        'Secret\nthe secret',
      ]);
    });
  });
}

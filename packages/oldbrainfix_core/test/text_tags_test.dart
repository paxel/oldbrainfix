import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// The tags of [text] as (as written, as searched).
List<(String, String)> _tags(String text) => [
  for (final t in TextTags.find(text)) (text.substring(t.start, t.end), t.tag),
];

/// The tags of [text] as written.
List<String> _written(String text) => [for (final t in _tags(text)) t.$1];

void main() {
  test('a tag is a # and a word, anywhere in a text', () {
    expect(_tags('#Garten water the beans\nsee #urlaub2026 and #to_do'), [
      ('#Garten', '#garten'),
      ('#urlaub2026', '#urlaub2026'),
      ('#to_do', '#to_do'),
    ]);
    expect(_tags('#a'), [('#a', '#a')]);
  });

  test('letters of every script count, umlauts among them', () {
    expect(_tags('#Müll #Straße #ÄRGER #日記 #дача #𝔸x'), [
      ('#Müll', '#müll'),
      ('#Straße', '#straße'),
      ('#ÄRGER', '#ärger'),
      ('#日記', '#日記'),
      ('#дача', '#дача'),
      ('#𝔸x', '#𝔸x'),
    ]);
  });

  test('each letter is lowered on its own, the same in both cores', () {
    expect(_tags('#İzmir #ΟΔΟΣ #ǅx'), [
      ('#İzmir', '#izmir'),
      ('#ΟΔΟΣ', '#οδοσ'),
      ('#ǅx', '#ǆx'),
    ]);
  });

  test('a tag ends at the first character that is no letter, digit, _ '
      'or -', () {
    expect(_written('#garten. #garten, (#garten) "#garten"! #garten🌱'), [
      '#garten',
      '#garten',
      '#garten',
      '#garten',
      '#garten',
    ]);
    expect(_written('#a#b #c/d #e\'s'), ['#a', '#c', '#e']);
  });

  test('a - inside counts, at its end it does not', () {
    expect(_written('#to-do #Garten- und Balkonpflanzen #a--'), [
      '#to-do',
      '#Garten',
      '#a',
    ]);
    expect(_written('#-a #- #--'), isEmpty);
  });

  test('a tag needs a letter: numbers alone are no tags', () {
    expect(_written('#1 issue #126 #2024 #_ #1_2 #²'), isEmpty);
    expect(_written('#v2 #2024plan #3d #Ⅻ'), ['#v2', '#2024plan', '#3d', '#Ⅻ']);
  });

  test('a heading, a lone # and ## are no tags', () {
    expect(_written('# Heading\n## Part\n#\n##double ###three # '), isEmpty);
  });

  test('a # after a letter, a digit, /, \\ or & is no tag', () {
    expect(
      _written(
        'C# F#m x#a 1#a _#a -#a https://example.org/#top '
        'page.html#anchor \\#escaped &#x27;',
      ),
      isEmpty,
    );
  });

  test('after spaces of any kind, punctuation and other signs it is', () {
    expect(_written('\t#a\n#b\r\n#c #d　#e (#f [#g „#h“ *#i* 🌱#j'), [
      '#a',
      '#b',
      '#c',
      '#d',
      '#e',
      '#f',
      '#g',
      '#h',
      '#i',
      '#j',
    ]);
  });

  test('a combining mark ends a tag, a letter written whole does not', () {
    expect(_written('#Müll #Müll'), ['#Mu', '#Müll']);
  });

  test('a status mark at the start of a line may come before a tag', () {
    expect(_written('☐ #call mum\n✅#done'), ['#call', '#done']);
  });

  test('offsets are UTF-16 code units, an astral character counting two', () {
    final [tag] = TextTags.find('🧠 #𝔸');
    expect((tag.start, tag.end), (3, 6));
    expect(TextTags.find(''), isEmpty);
  });
}

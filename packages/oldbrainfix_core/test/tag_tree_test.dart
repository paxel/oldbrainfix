import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// [nodes] and those under them as "tag notes", each level indented.
List<String> _lines(List<TagNode> nodes, [String indent = '']) => [
  for (final node in nodes) ...[
    '$indent${node.tag} ${node.notes}',
    ..._lines(node.children, '$indent  '),
  ],
];

void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 11, 12, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 12, 8);
    store = Store.inMemory(clock: () => now);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Note capture(String text, {String format = TextFormats.plain}) {
    now = now.add(const Duration(minutes: 1));
    return notes.capture(text, format: format);
  }

  test('nested tags come by their levels, each Note counted once', () {
    capture('#life/health/back pain, #life/work and #life/work again');
    capture('#life/health');
    capture('#garten');
    expect(_lines(TagTree(store).tree()), [
      '#garten 1',
      '#life 2',
      '  #life/health 2',
      '    #life/health/back 1',
      '  #life/work 1',
    ]);
    final [garten, life] = TagTree(store).tree();
    expect((garten.name, life.name), ('garten', 'life'));
    expect(life.children.map((n) => n.name), ['health', 'work']);
  });

  test('each part is written as written most often, ties to the first '
      'by UTF-16 code units', () {
    capture('#Life/Health');
    capture('#life/health #LIFE/x');
    capture('#Life/back');
    capture('#garten #Garten');
    expect(_lines(TagTree(store).tree()), [
      '#Garten 1',
      '#Life 3',
      '  #Life/back 1',
      '  #Life/Health 2',
      '  #Life/x 1',
    ]);
  });

  test('Sections and merged Notes count, trashed and merged ones not '
      'on their own', () {
    final host = capture('Physio');
    notes.amend(host.id, 'then #health/back');
    final merged = capture('#health/knee');
    notes.merge(merged.id, host.id);
    final gone = capture('#trash/me');
    notes.trash(gone.id);
    expect(_lines(TagTree(store).tree()), [
      '#health 1',
      '  #health/back 1',
      '  #health/knee 1',
    ]);
  });

  test('tags are read as the text draws them: none in code, none in a '
      'link or a web address', () {
    capture(
      '**#Bold/x** `#code/y` [#label](https://x.org/#dest)',
      format: TextFormats.markdown,
    );
    capture('#script/a', format: TextFormats.code);
    capture('see https://example.org/x#frag and C# #ok');
    expect(_lines(TagTree(store).tree()), [
      '#Bold 1',
      '  #Bold/x 1',
      '#label 1',
      '#ok 1',
    ]);
  });

  test('a changed text changes the tree', () {
    final note = capture('#one');
    expect(_lines(TagTree(store).tree()), ['#one 1']);
    notes.edit(note.id, '#two/three');
    expect(_lines(TagTree(store).tree()), ['#two 1', '  #two/three 1']);
  });

  test('suggestions are the tags beginning with what is typed, ignoring '
      'case, each before those under it', () {
    capture('#Garten #Garten/Beet #Gartenarbeit #life/health #gas');
    List<String> tags(String typed) => [
      for (final n in TagTree(store).suggestions(typed)) n.tag,
    ];
    expect(tags('#gar'), ['#Garten', '#Garten/Beet', '#Gartenarbeit']);
    expect(tags(' #GARTEN/ '), ['#Garten/Beet']);
    expect(tags('#'), [
      '#Garten',
      '#Garten/Beet',
      '#Gartenarbeit',
      '#gas',
      '#life',
      '#life/health',
    ]);
    expect(tags('garten'), isEmpty);
    expect(tags('#health'), isEmpty);
  });
}

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// The Notes that link to a Note (obf-1035), as `backlinks/tests.rs`
/// checks the Rust core.
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

  Note capture(
    String text, {
    String format = TextFormats.plain,
    String? place,
  }) {
    now = now.add(const Duration(minutes: 1));
    return notes.capture(text, format: format, place: place);
  }

  String link(String id) => AppLink(AppLinkKind.note, id).url;

  List<String> linking(String id, {Set<String> closed = const {}}) => [
    for (final note in Backlinks(store).to(id, closed: closed)) note.text,
  ];

  test('a Note lists the Notes whose text or Sections link to it, newest '
      'first', () {
    final ann = capture('Ann');
    final first = capture(
      'Call [Ann](${link(ann.id)})',
      format: TextFormats.markdown,
    );
    final other = capture('Groceries');
    now = now.add(const Duration(minutes: 1));
    notes.amend(
      other.id,
      '**[Ann](${link(ann.id)})**',
      format: TextFormats.markdown,
    );
    final table = capture(
      '| Who |\n| --- |\n| [Ann](${link(ann.id)}) |',
      format: TextFormats.markdown,
    );
    expect(linking(ann.id), [table.text, 'Groceries', first.text]);
    expect(linking(other.id), isEmpty);
  });

  test('only a formatted text links, and only a link to a Note counts', () {
    final ann = capture('Ann');
    final written = '[Ann](${link(ann.id)})';
    capture(written);
    capture(written, format: TextFormats.code);
    capture('`$written`', format: TextFormats.markdown);
    final place = AppLink(AppLinkKind.place, ann.id).url;
    capture('[Ann]($place)', format: TextFormats.markdown);
    expect(linking(ann.id), isEmpty);
  });

  test('the scheme in any case and an encoded id link too', () {
    final ann = capture('Ann');
    final upper = link(ann.id).replaceFirst('oldbrainfix://', 'OldBrainFix://');
    capture('[Ann]($upper)', format: TextFormats.markdown);
    final encoded = link(ann.id)
        .replaceAll(':', '%3A')
        .replaceFirst('%3A//', '://');
    capture('[Ann]($encoded)', format: TextFormats.markdown);
    expect(linking(ann.id), hasLength(2));
  });

  test('a Note is listed once and never under itself', () {
    final ann = capture('Ann');
    final twice = '[Ann](${link(ann.id)}) and [again](${link(ann.id)})';
    final note = capture(twice, format: TextFormats.markdown);
    notes.amend(
      note.id,
      '[Ann](${link(ann.id)})',
      format: TextFormats.markdown,
    );
    notes.edit(ann.id, '[me](${link(ann.id)})', format: TextFormats.markdown);
    expect(linking(ann.id), [twice]);
  });

  test('trashed Notes leave, and a merged one links from where it went', () {
    final ann = capture('Ann');
    final gone = capture(
      '[Ann](${link(ann.id)})',
      format: TextFormats.markdown,
    );
    notes.trash(gone.id);
    final host = capture('Household');
    final merged = capture(
      '[Ann](${link(ann.id)})',
      format: TextFormats.markdown,
    );
    notes.merge(merged.id, host.id);
    expect(linking(ann.id), ['Household']);
  });

  test('the Notes of a closed Area are left out', () {
    final places = Places(store);
    final locked = places.addArea('Locked');
    final topic = places.addTopic(locked.id, 'Inside');
    final ann = capture('Ann');
    final text = '[Ann](${link(ann.id)})';
    capture(text, format: TextFormats.markdown, place: topic.id);
    capture(text, format: TextFormats.markdown, place: locked.id);
    expect(linking(ann.id), hasLength(2));
    expect(linking(ann.id, closed: {locked.id}), isEmpty);
  });

  test('a link written later shows at once, and goes when it goes', () {
    final ann = capture('Ann');
    final note = capture('Call Ann');
    expect(linking(ann.id), isEmpty);
    notes.edit(
      note.id,
      'Call [Ann](${link(ann.id)})',
      format: TextFormats.markdown,
    );
    expect(linking(ann.id), hasLength(1));
    notes.edit(note.id, 'Call Ann');
    expect(linking(ann.id), isEmpty);
  });

  test('links are read from spans at any depth', () {
    const text =
        '- *see [a](oldbrainfix://note/a)*\n'
        '> ~~[b](oldbrainfix://note/b)~~ [c](oldbrainfix://area/c) '
        '[d](https://x.org)';
    expect(Backlinks.linksIn(TextFormats.markdown, text), {'a', 'b'});
    expect(Backlinks.linksIn(TextFormats.plain, text), isEmpty);
  });
}

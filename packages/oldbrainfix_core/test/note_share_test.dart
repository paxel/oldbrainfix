import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

final _at = DateTime.utc(2026, 10, 30, 9);
final _context = CaptureContext(captured: _at);

Note _note(
  String text, {
  String format = TextFormats.plain,
  List<String> photos = const [],
  List<String> recordings = const [],
  Map<String, String> transcripts = const {},
  Map<String, String> photoTexts = const {},
  List<String> removedRecordings = const [],
}) => Note(
  id: 'note:1',
  text: text,
  textFormat: format,
  context: _context,
  photos: photos,
  recordings: recordings,
  transcripts: transcripts,
  photoTexts: photoTexts,
  removedRecordings: removedRecordings,
);

Section _section(
  String text, {
  String format = TextFormats.plain,
  int day = 31,
  List<String> photos = const [],
  List<String> recordings = const [],
  Map<String, String> transcripts = const {},
}) => Section(
  id: 'section:$day',
  noteId: 'note:1',
  text: text,
  textFormat: format,
  context: CaptureContext(captured: DateTime.utc(2026, 10, day, 8)),
  photos: photos,
  recordings: recordings,
  transcripts: transcripts,
);

NoteShare _share(Note note, [List<Section> sections = const []]) =>
    NoteShare.of(note, sections, date: (d) => '${d.day}.${d.month}.');

void main() {
  test('the text and each Section under its date, plain without HTML', () {
    final share = _share(_note('Milk\n'), [
      _section('Eggs', day: 31),
      _section('', day: 1),
      _section('Bread', day: 2),
    ]);
    expect(share.text, 'Milk\n\n31.10.\nEggs\n\n2.10.\nBread');
    expect(share.html, isNull);
  });

  test('a formatted text goes as written, with HTML of the whole beside it', () {
    final share = _share(
      _note('**Milk** *now*', format: TextFormats.markdown),
      [_section('2*3*4 # x', day: 31)],
    );
    expect(share.text, '**Milk** *now*\n\n31.10.\n2*3*4 # x');
    expect(
      share.html,
      '<p><strong>Milk</strong> <em>now</em></p>\n<p>31.10.<br>2*3*4 # x</p>\n',
    );
  });

  test('Transcripts and Photo Texts go along only without words', () {
    final media = _note(
      '',
      photos: ['p1'],
      recordings: ['r1'],
      removedRecordings: ['r2'],
      photoTexts: {'p1': 'Sign: *open*'},
      transcripts: {'r1': 'buy milk', 'r2': 'and eggs '},
    );
    final share = _share(media, [
      _section('Seen', day: 31, recordings: ['r3'], transcripts: {'r3': 'x'}),
      _section('', day: 1, recordings: ['r4'], transcripts: {'r4': ''}),
    ]);
    expect(share.text, 'Sign: *open*\nbuy milk\nand eggs\n\n31.10.\nSeen');
    expect(share.html, isNull);
    expect(share.photos, ['p1']);
    expect(share.recordings, ['r1', 'r3', 'r4']);
    final worded = _share(
      _note('Shop', recordings: ['r1'], transcripts: {'r1': 'buy milk'}),
    );
    expect(worded.text, 'Shop');
    expect(worded.recordings, ['r1']);
  });

  test(
    'a Transcript beside a formatted Section reads as itself in the HTML',
    () {
      final share = _share(
        _note('', recordings: ['r1'], transcripts: {'r1': '*not bold*'}),
        [
          _section('- a', format: TextFormats.markdown, photos: ['p1', 'p1']),
        ],
      );
      expect(share.text, '*not bold*\n\n31.10.\n- a');
      expect(
        share.html,
        '<p>*not bold*</p>\n<p>31.10.</p>\n<ul><li>a</li></ul>\n',
      );
      expect(share.photos, ['p1']);
    },
  );
}

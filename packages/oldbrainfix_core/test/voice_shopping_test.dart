import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A recording; [n] tells them apart.
Uint8List _recording(int n) =>
    Uint8List.fromList([0, 0, 0, 0x18, 0x66, 0x74, 0x79, 0x70, n]);

DateTime _wall(DateTime utc) => utc;

/// A voice capture said into a Shopping List (#311).
void main() {
  group('The items said in a Transcript', () {
    test('split at commas, "und" and "and", in any case, whole words', () {
      expect(ShoppingLists.spokenItems('Milch, Eier und Brot.'), [
        'Milch',
        'Eier',
        'Brot',
      ]);
      expect(ShoppingLists.spokenItems('milk, eggs, and bread'), [
        'milk',
        'eggs',
        'bread',
      ]);
      expect(ShoppingLists.spokenItems('Butter UND Käse And tea'), [
        'Butter',
        'Käse',
        'tea',
      ]);
      // Part of a word is no "und".
      expect(ShoppingLists.spokenItems('Hundefutter und Brandsalbe'), [
        'Hundefutter',
        'Brandsalbe',
      ]);
      expect(ShoppingLists.spokenItems('sand and candy'), ['sand', 'candy']);
    });

    test('split at pauses, as Whisper marks them, and line breaks', () {
      expect(
        ShoppingLists.spokenItems('Milch. Eier? Brot! Käse; Wurst… Senf\nTee'),
        ['Milch', 'Eier', 'Brot', 'Käse', 'Wurst', 'Senf', 'Tee'],
      );
    });

    test('keep a number with a comma or point in it', () {
      expect(ShoppingLists.spokenItems('1,5 Liter Milch, 2.5 kg Mehl'), [
        '1,5 Liter Milch',
        '2.5 kg Mehl',
      ]);
      expect(ShoppingLists.spokenItems('3, 4'), ['3', '4']);
    });

    test('are trimmed, one space between words, none empty, in order', () {
      expect(ShoppingLists.spokenItems('  Oat\tmilk ,, and , und\n\n'), [
        'Oat milk',
      ]);
      expect(ShoppingLists.spokenItems(''), isEmpty);
      expect(ShoppingLists.spokenItems(' . , und '), isEmpty);
      // A repeat stays: adding it brings nothing twice.
      expect(ShoppingLists.spokenItems('Milch und Milch'), ['Milch', 'Milch']);
    });
  });

  group('A voice capture added to a Shopping List', () {
    late Store store;
    late Notes notes;
    late ShoppingLists lists;
    late String list;

    setUp(() {
      store = Store.inMemory();
      notes = Notes(store);
      lists = ShoppingLists(store);
      list = notes.capture('Groceries\nFlour').id;
      lists.make(list);
    });
    tearDown(() => store.close());

    List<String> shown() => [
      for (final line in lists.byNote(list, _wall)!.lines)
        '${line.struck ? '~' : ''}${line.text}',
    ];

    test('makes its typed lines Lines and stays on the list as a Section, '
        'owing the items said until they are transcribed', () {
      final voice = notes.capture('Sugar', recordings: [_recording(1)]);
      lists.addVoiceCapture(list, voice.id, _wall);

      expect(shown(), ['Flour', 'Sugar']);
      expect(notes.byId(voice.id)?.mergedInto, list);
      expect(notes.sections(list).single.recordings, voice.recordings);
      expect(lists.owedTo(voice.id), list);
      expect(notes.byId(voice.id)?.text, 'Sugar');
    });

    test('adds the items said once its Transcript comes, once', () {
      final voice = notes.capture('', recordings: [_recording(1)]);
      final hash = voice.recordings.single;
      lists.addVoiceCapture(list, voice.id, _wall);
      expect(lists.addOwedLines(voice.id, _wall), isEmpty);
      expect(shown(), ['Flour']);

      notes.setTranscript(voice.id, hash, 'Butter, flour und Eier');
      lists.addOwedLines(voice.id, _wall);
      expect(shown(), ['Flour', 'Butter', 'Eier']);
      expect(lists.owedTo(voice.id), isNull);

      // Struck, and transcribed again: it stays struck.
      final butter = lists
          .byNote(list, _wall)!
          .lines
          .firstWhere((l) => l.text == 'Butter');
      lists.check(butter.id, _wall);
      notes.setTranscript(voice.id, hash, 'Butter und Tee');
      expect(lists.addOwedLines(voice.id, _wall), isEmpty);
      expect(shown(), ['Flour', 'Eier', '~Butter']);
    });

    test('adds them at once when its recordings are transcribed already', () {
      final voice = notes.capture('Sugar', recordings: [_recording(1)]);
      notes.setTranscript(voice.id, voice.recordings.single, 'Salt and tea');
      lists.addVoiceCapture(list, voice.id, _wall);
      expect(shown(), ['Flour', 'Sugar', 'Salt', 'tea']);
      expect(lists.owedTo(voice.id), isNull);
    });

    test('makes the lines of a Transcript its text holds no Line, as the '
        'quick note puts it there, but adds the items said (obf-1082)', () {
      final voice = notes.capture(
        'Sugar\n\n Salt and tea ',
        recordings: [_recording(1)],
      );
      notes.addRecording(voice.id, _recording(2));
      final [first, second] = notes.byId(voice.id)!.recordings;
      notes.setTranscript(voice.id, first, 'Salt and tea');
      notes.setTranscript(voice.id, second, 'Milk und Honey');
      lists.addVoiceCapture(list, voice.id, _wall);
      expect(shown(), ['Flour', 'Sugar', 'Salt', 'tea', 'Milk', 'Honey']);
      expect(lists.owedTo(voice.id), isNull);
      expect(notes.byId(voice.id)?.text, 'Sugar\n\n Salt and tea ');
    });

    test('adds the items said of a Transcript its text holds once the '
        'other recording is transcribed', () {
      final voice = notes.capture('Salt and tea', recordings: [_recording(1)]);
      notes.addRecording(voice.id, _recording(2));
      final [first, second] = notes.byId(voice.id)!.recordings;
      notes.setTranscript(voice.id, first, 'Salt and tea');
      lists.addVoiceCapture(list, voice.id, _wall);
      expect(shown(), ['Flour']);
      notes.setTranscript(voice.id, second, 'Milk');
      lists.addOwedLines(voice.id, _wall);
      expect(shown(), ['Flour', 'Salt', 'tea', 'Milk']);
    });

    test('adds none of a Transcript its text held as a Line before it '
        'came', () {
      final voice = notes.capture('Tea and jam', recordings: [_recording(1)]);
      lists.addVoiceCapture(list, voice.id, _wall);
      notes.setTranscript(voice.id, voice.recordings.single, 'Tea and jam');
      lists.addOwedLines(voice.id, _wall);
      expect(shown(), ['Flour', 'Tea and jam']);
      expect(lists.owedTo(voice.id), isNull);
    });

    test('brings back the struck Lines its text and its Transcript both '
        'name', () {
      for (final text in ['Milk', 'Eggs']) {
        lists.check(lists.add(list, text, _wall), _wall);
      }
      expect(shown(), ['Flour', '~Milk', '~Eggs']);
      final voice = notes.capture('Milk\nEggs', recordings: [_recording(1)]);
      notes.setTranscript(voice.id, voice.recordings.single, 'Milk\nEggs');
      lists.addVoiceCapture(list, voice.id, _wall);
      expect(shown(), ['Flour', 'Milk', 'Eggs']);
    });

    test('adds a Transcript held only in part of a line of its text', () {
      final voice = notes.capture('Milchreis', recordings: [_recording(1)]);
      lists.addVoiceCapture(list, voice.id, _wall);
      notes.setTranscript(voice.id, voice.recordings.single, 'Milch');
      lists.addOwedLines(voice.id, _wall);
      expect(shown(), ['Flour', 'Milchreis', 'Milch']);
    });

    test('waits for every recording, then adds them in their order', () {
      final voice = notes.capture('', recordings: [_recording(1)]);
      notes.addRecording(voice.id, _recording(2));
      final [first, second] = notes.byId(voice.id)!.recordings;
      lists.addVoiceCapture(list, voice.id, _wall);
      notes.setTranscript(voice.id, second, 'Tea');
      lists.addOwedLines(voice.id, _wall);
      expect(shown(), ['Flour']);
      notes.setTranscript(voice.id, first, 'Milk');
      lists.addOwedLines(voice.id, _wall);
      expect(shown(), ['Flour', 'Milk', 'Tea']);
    });

    test('goes where the list lies', () {
      final home = Places(store).addArea('Home');
      notes.file(list, home.id);
      final voice = notes.capture('', recordings: [_recording(1)]);
      lists.addVoiceCapture(list, voice.id, _wall);
      expect(notes.byId(voice.id)?.place, home.id);
    });

    test('owes nothing once the Merge is undone or the list is a plain '
        'Note', () {
      final voice = notes.capture('', recordings: [_recording(1)]);
      lists.addVoiceCapture(list, voice.id, _wall);
      notes.undoMerge(voice.id);
      notes.setTranscript(voice.id, voice.recordings.single, 'Tea');
      expect(lists.addOwedLines(voice.id, _wall), isEmpty);
      expect(lists.owedTo(voice.id), isNull);
      expect(shown(), ['Flour']);

      final other = notes.capture('', recordings: [_recording(2)]);
      lists.addVoiceCapture(list, other.id, _wall);
      lists.toPlain(list, _wall);
      notes.setTranscript(other.id, other.recordings.single, 'Tea');
      expect(lists.addOwedLines(other.id, _wall), isEmpty);
      expect(lists.owedTo(other.id), isNull);
    });

    test('is refused for a Note that is no Shopping List, writing '
        'nothing', () {
      final plain = notes.capture('Plain').id;
      final voice = notes.capture('', recordings: [_recording(1)]);
      final before = store.currentSeq();
      expect(
        () => lists.addVoiceCapture(plain, voice.id, _wall),
        throwsArgumentError,
      );
      expect(store.currentSeq(), before);
    });
  });
}

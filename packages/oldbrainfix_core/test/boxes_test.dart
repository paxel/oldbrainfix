import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'shared_areas_test.dart' show ticking;

/// A box of the id [id] with the glyph of [state].
String box(String id, [BoxState state = BoxState.open]) =>
    '${state.glyph}${TextBoxes.encodeId(id)}';

/// Ids drawn from the counted entity ids `1`, `2`, …, as a fixture's.
String Function() counted() {
  var n = 0;
  return () => '${++n}';
}

void main() {
  group('finding boxes', () {
    test('a glyph with an id is a box, its selector belonging to it', () {
      final text = 'a${box('a7k2')}b ✅️${TextBoxes.encodeId('0000')} ☐';
      final found = TextBoxes.find(text);
      expect(found.map((b) => b.id), ['a7k2', '0000']);
      expect(found.map((b) => b.state), [BoxState.open, BoxState.done]);
      expect((found[0].start, found[0].idStart, found[0].end), (1, 2, 12));
      expect(found[1].idStart - found[1].start, 2);
      expect(text.substring(found[1].end), ' ☐');
    });

    test('a joiner after it makes another emoji, an id of another shape '
        'none, and an id the second time no box', () {
      final short = '☐${TextBoxes.encodeId('a7k2').substring(2)}';
      final text = '✅‍🔥 ✅️‍🔥 $short ${box('a7k2')} ${box('a7k2')}';
      expect(TextBoxes.find(text).map((b) => b.id), ['a7k2']);
      expect(TextBoxes.isId('a7k2'), isTrue);
      for (final id in ['A7K2', 'a7k', 'a7k22', 'a-k2', '']) {
        expect(TextBoxes.isId(id), isFalse, reason: id);
      }
    });
  });

  group('making boxes', () {
    test('every bare glyph becomes a box, mid-word too, ids drawn anew', () {
      final text = TextBoxes.convert('a☐b ✅ c ❌', newId: counted());
      final found = TextBoxes.find(text);
      expect(found.map((b) => b.state), [
        BoxState.open,
        BoxState.done,
        BoxState.failed,
      ]);
      expect(found.map((b) => b.id).toSet(), hasLength(3));
      expect(TextBoxes.strip(text), 'a☐b ✅ c ❌');
      // Saved again, nothing changes and no id is drawn.
      expect(
        TextBoxes.convert(text, newId: () => throw StateError('no id wanted')),
        text,
      );
    });

    test('an id the text holds already is drawn again', () {
      final ids = ['x', 'x', 'y'];
      var drawn = 0;
      final text = TextBoxes.convert('☐ ☐', newId: () => ids[drawn++]);
      expect(drawn, 3);
      expect(TextBoxes.find(text).map((b) => b.id).toSet(), hasLength(2));
    });

    test('a second box of an id gets a new one; joined emoji stay', () {
      final text = '${box('a7k2')} ${box('a7k2', BoxState.done)} ✅‍🔥';
      final saved = TextBoxes.convert(text, newId: counted());
      final found = TextBoxes.find(saved);
      expect(found.first.id, 'a7k2');
      expect(found.map((b) => b.id).toSet(), hasLength(2));
      expect(found.last.state, BoxState.done);
      expect(TextBoxes.strip(saved), '☐ ✅ ✅‍🔥');
    });

    test('formatted text: not in inline code nor a link\'s address, in '
        'its label and in table cells', () {
      const text =
          '- ☐ a `☐` [☐ b](https://x.org/☐)\n\n| ☐ c | d |\n| --- | --- |';
      final saved = TextBoxes.convert(
        text,
        format: TextFormats.markdown,
        newId: counted(),
      );
      expect(TextBoxes.find(saved), hasLength(3));
      expect(TextBoxes.strip(saved), text);
      expect(saved, contains('`☐`'));
      expect(saved, contains('https://x.org/☐)'));
      final plain = TextBoxes.convert(text, newId: counted());
      expect(TextBoxes.find(plain), hasLength(5));
    });
  });

  group('showing boxes', () {
    test('each box in the glyph of its latest tick, without its id', () {
      final text = 'Pack ${box('aaaa')} ${box('bbbb', BoxState.done)} x';
      final tick = BoxTick(
        state: BoxState.failed,
        at: DateTime.utc(2026, 3, 1),
        device: 'd',
        own: true,
      );
      final shown = TextBoxes.shown(text, {'aaaa': tick, 'gone': tick});
      expect(shown.text, 'Pack ❌ ✅ x');
      expect(shown.boxes.map((b) => b.state), [BoxState.failed, BoxState.done]);
      expect(shown.boxes.first.starting, BoxState.open);
      expect(shown.boxes.first.last, tick);
      expect(shown.boxes.last.last, isNull);
      for (final b in shown.boxes) {
        expect(shown.text.substring(b.start, b.end), b.state.glyph);
        expect(
          text.substring(b.idStart, b.sourceEnd),
          TextBoxes.encodeId(b.id),
        );
      }
      // A Sent Item keeps the ids, the glyphs as they show.
      final baked = TextBoxes.baked(text, {'aaaa': tick});
      expect(
        baked,
        'Pack ${box('aaaa', BoxState.failed)} ${box('bbbb', BoxState.done)} x',
      );
      expect(TextFormats.visible(TextFormats.plain, text), 'Pack ☐ ✅ x');
      expect(
        TextFormats.visible(TextFormats.markdown, '**${box('aaaa')}** x'),
        '☐ x',
      );
    });
  });

  group('in the store', () {
    late Store store;

    setUp(() {
      final clock = ticking();
      store = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    });

    tearDown(() {
      store.close();
    });

    test('capture, edit and Amend make boxes; a Transcript taken over '
        'too, and the text written comes back with its ids', () {
      final notes = Notes(store);
      final note = notes.capture('☐ milk');
      final first = note.boxes.single;
      expect(note.text, isNot('☐ milk'));
      expect(note.shownText, '☐ milk');
      notes.edit(note.id, '${note.text}\n✅ eggs');
      final edited = notes.byId(note.id)!;
      expect(edited.boxes.map((b) => b.id).first, first.id);
      expect(edited.boxes, hasLength(2));
      final section = notes.amend(note.id, '❌ bread');
      expect(section.boxes.single.state, BoxState.failed);
      final written = notes.takeOver(note.id, '☐ from a photo', append: true);
      expect(written, notes.byId(note.id)!.text);
      expect(TextBoxes.find(written), hasLength(3));
    });

    test('a tick shows, the later one wins, each stays in the history', () {
      final notes = Notes(store);
      final boxes = Boxes(store);
      final note = notes.capture('☐ call Ann');
      final id = note.boxes.single.id;
      expect(boxes.mayTick(note.id), isTrue);
      boxes.tick(note.id, id, BoxState.done);
      final before = store.allEntries().length;
      boxes.tick(note.id, id, BoxState.done);
      expect(store.allEntries().length, before);
      boxes.tick(note.id, id, BoxState.failed);
      // Dated back, it loses to the ticks after it.
      store.append(
        note.id,
        BoxKeys.tick(id),
        'open',
        date: DateTime.utc(2026, 1, 1),
      );
      final shown = notes.byId(note.id)!;
      expect(shown.shownText, '❌ call Ann');
      expect(shown.boxes.single.last?.own, isTrue);
      expect(shown.visibleText, '❌ call Ann');
      expect(boxes.history(note.id, id).map((t) => t.state), [
        BoxState.open,
        BoxState.done,
        BoxState.failed,
      ]);
      expect(
        () => boxes.tick(note.id, 'zzzz', BoxState.done),
        throwsArgumentError,
      );
      expect(Search(store).notes('Ann').single.id, note.id);
      final share = NoteShare.of(shown, const [], date: (d) => '$d');
      expect(share.text, '❌ call Ann');
    });

    test('a tick of a box no longer in the text counts for nothing, and '
        'again once it is back', () {
      final notes = Notes(store);
      final note = notes.capture('☐ a\n☐ b');
      final [a, b] = note.boxes;
      Boxes(store).tick(note.id, a.id, BoxState.done);
      notes.edit(note.id, note.text.substring(b.sourceStart));
      expect(notes.byId(note.id)!.boxes.single.id, b.id);
      expect(notes.byId(note.id)!.shownText, '☐ b');
      expect(Boxes(store).ticks(note.id), contains(a.id));
      notes.edit(note.id, note.text);
      expect(notes.byId(note.id)!.shownText, '✅ a\n☐ b');
    });

    test('a Section\'s boxes are ticked while its text stays', () {
      final notes = Notes(store);
      final note = notes.capture('Trip');
      final section = notes.amend(note.id, '☐ passport');
      final id = section.boxes.single.id;
      Boxes(store).tick(section.id, id, BoxState.done);
      expect(notes.sections(note.id).single.shownText, '✅ passport');
      expect(notes.sections(note.id).single.text, section.text);
    });

    test('ticks of another shape are skipped', () {
      final note = Notes(store).capture('☐ pills');
      final id = note.boxes.single.id;
      for (final (field, value) in [
        ('box@', 'done'),
        ('box@${id}x', 'done'),
        ('box@${id.toUpperCase()}', 'done'),
        ('box@@$id', 'done'),
        ('box@$id', 'Done'),
        ('box@$id', null),
        ('box@$id', ''),
      ]) {
        store.append(note.id, field, value);
      }
      final shown = Notes(store).byId(note.id)!;
      expect(shown.shownText, '☐ pills');
      expect(shown.boxes.single.last, isNull);
    });

    test('a Sent Item carries each box as it shows, its id kept', () {
      final notes = Notes(store);
      final note = notes.capture('☐ a ☐ b');
      final [a, b] = note.boxes;
      final section = notes.amend(note.id, '☐ c');
      Boxes(store).tick(note.id, b.id, BoxState.done);
      Boxes(store).tick(section.id, section.boxes.single.id, BoxState.failed);
      final item = sentItemOf(store, note.id);
      expect(TextBoxes.find(item.text).map((x) => (x.id, x.state)), [
        (a.id, BoxState.open),
        (b.id, BoxState.done),
      ]);
      expect(
        TextBoxes.find(item.sections.single.text).single.state,
        BoxState.failed,
      );
    });
  });
}

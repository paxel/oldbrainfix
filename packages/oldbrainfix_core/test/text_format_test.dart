import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'conflicts_test.dart' as conflicts;
import 'sent_items_test.dart' as sent;
import 'sync_test.dart' show ticking;

/// A text in a format this version does not know, as a later version
/// would store it.
const _future = '{"format":"x-future","text":"# Heading\\n**bold**"}';

/// A plain text in a later structure with a key this version does not
/// know.
const _later = '{"format":"plain","more":1,"text":"Later milk","v":2}';

void main() {
  group('the stored value', () {
    test('plain text is written with its format, keys sorted', () {
      expect(
        const StoredText.plain('Quote " and \\ and \n').encode(),
        '{"format":"plain","text":"Quote \\" and \\\\ and \\n"}',
      );
      expect(
        StoredText.read('{"format":"plain","text":"milk","v":1}'),
        const StoredText.plain('milk'),
      );
    });

    test('version 1 is the default and never written', () {
      expect(
        StoredText.read('{"format":"plain","text":"milk"}'),
        const StoredText.plain('milk'),
      );
      final future = StoredText.read(_future);
      expect(future.version, 1);
      expect(future.editable, isFalse);
      // Written with its default, it is read the same and written without.
      expect(
        StoredText.read('{"format":"plain","text":"milk","v":1}').encode(),
        '{"format":"plain","text":"milk"}',
      );
    });

    test('a value written before the format was recorded reads as plain', () {
      for (final value in [
        'milk',
        '',
        '{not json',
        '{"format":"plain"}',
        '{"format":3,"text":"x"}',
        '["format","text"]',
        ' {"format":"x-future","text":"x"}',
        '{"format":"plain","text":"x","v":"1"}',
        '{"format":"plain","text":"x","v":1.5}',
        '{"format":"plain","text":"x","v":0}',
        '{"format":"plain","text":"x","v":-1}',
        '{"format":"plain","text":"x","v":true}',
        '{"format":"plain","text":"x","v":null}',
        '{"format":"plain","v":1}',
        '{"text":"x","v":1}',
      ]) {
        final text = StoredText.read(value);
        expect(text.format, TextFormats.plain, reason: value);
        expect(text.text, value, reason: value);
        expect(text.editable, isTrue, reason: value);
      }
      expect(StoredText.read(null), const StoredText.plain(''));
    });

    test('an unknown format reads as its source text, not editable', () {
      final text = StoredText.read(_future);
      expect(text.format, 'x-future');
      expect(text.text, '# Heading\n**bold**');
      expect(text.editable, isFalse);
      // Anything beside format and text is not known here either.
      final more = StoredText.read('{"format":"plain","more":1,"text":"x"}');
      expect(more.format, TextFormats.plain);
      expect(more.text, 'x');
      expect(more.editable, isFalse);
      final versioned = StoredText.read(
        '{"format":"plain","more":1,"text":"x","v":1}',
      );
      expect(versioned.text, 'x');
      expect(versioned.editable, isFalse);
      final unknown = StoredText.read('{"format":"x-future","text":"x","v":1}');
      expect(unknown.format, 'x-future');
      expect(unknown.editable, isFalse);
    });

    test('a later structure shows its text, never editable', () {
      final later = StoredText.read(_later);
      expect(later.format, TextFormats.plain);
      expect(later.text, 'Later milk');
      expect(later.version, 2);
      expect(later.editable, isFalse);
      expect(later, isNot(const StoredText.plain('Later milk')));
      // Without text, or keys of another kind, it shows nothing.
      for (final value in [
        '{"format":"plain","v":2}',
        '{"format":"plain","text":["x"],"v":2}',
        '{"text":7,"v":3}',
      ]) {
        final text = StoredText.read(value);
        expect(text.text, '', reason: value);
        expect(text.editable, isFalse, reason: value);
      }
      expect(StoredText.read('{"text":7,"v":3}').format, '');
    });

    test('a text from elsewhere keeps its version', () {
      final later = StoredText.of(TextFormats.plain, 'x', 2);
      expect(later.editable, isFalse);
      expect(later.encode(), '{"format":"plain","text":"x","v":2}');
      expect(StoredText.read(later.encode()), later);
      expect(StoredText.of(TextFormats.plain, 'x').editable, isTrue);
      expect(StoredText.of('x-future', 'x', 1).editable, isFalse);
    });
  });

  group('Notes and Sections', () {
    late Store store;
    late Notes notes;

    setUp(() {
      store = Store.inMemory(clock: ticking());
      notes = Notes(store);
    });
    tearDown(() => store.close());

    test('capture, edit and Amend write plain text', () {
      final note = notes.capture('nees mor sauce');
      notes.edit(note.id, 'Needs more sauce');
      final section = notes.amend(note.id, 'Hot sauce');
      expect(
        [
          for (final id in [note.id, section.id])
            for (final e in store.history(id))
              if (e.field == NoteKeys.text) e.value,
        ],
        [
          '{"format":"plain","text":"nees mor sauce"}',
          '{"format":"plain","text":"Needs more sauce"}',
          '{"format":"plain","text":"Hot sauce"}',
        ],
      );
      final read = notes.byId(note.id)!;
      expect(read.text, 'Needs more sauce');
      expect(read.textFormat, TextFormats.plain);
      expect(read.textVersion, 1);
      expect(read.textEditable, isTrue);
      expect(notes.sections(note.id).single.textEditable, isTrue);
    });

    test('a text in a later structure is never overwritten here', () {
      final note = notes.capture('x');
      store.append(note.id, NoteKeys.text, _later);
      final read = notes.byId(note.id)!;
      expect(read.text, 'Later milk');
      expect(read.textVersion, 2);
      expect(read.textEditable, isFalse);
      expect(() => notes.edit(note.id, 'Plain now'), throwsStateError);
      final section = notes.amend(note.id, 'Amended');
      store.append(section.id, SectionKeys.text, _later);
      final shown = notes.sections(note.id).single;
      expect(shown.textVersion, 2);
      expect(shown.textEditable, isFalse);
      expect(store.current(note.id, NoteKeys.text), _later);
    });

    test('a text written before the format was recorded reads as plain', () {
      final note = notes.capture('x');
      store.append(note.id, NoteKeys.text, 'Old milk');
      expect(notes.byId(note.id)?.text, 'Old milk');
      expect(notes.byId(note.id)?.textEditable, isTrue);
      notes.edit(note.id, 'New milk');
      expect(notes.byId(note.id)?.text, 'New milk');
    });

    test('a text in an unknown format is never overwritten here', () {
      final note = notes.capture('x');
      store.append(note.id, NoteKeys.text, _future);
      final other = notes.capture('Other');
      final read = notes.byId(note.id)!;
      expect(read.text, '# Heading\n**bold**');
      expect(read.textFormat, 'x-future');
      expect(read.textEditable, isFalse);

      expect(() => notes.edit(note.id, 'Plain now'), throwsStateError);
      // Amend adds a Section; Merge points at the other Note.
      final section = notes.amend(note.id, 'Amended');
      expect(section.textEditable, isTrue);
      notes.merge(note.id, other.id);
      expect(store.current(note.id, NoteKeys.text), _future);
      final shown = notes.sections(other.id);
      expect(shown.first.text, '# Heading\n**bold**');
      expect(shown.first.textEditable, isFalse);
      // A Section in an unknown format shows as its source text.
      store.append(section.id, SectionKeys.text, _future);
      expect(notes.sections(other.id).last.textFormat, 'x-future');
    });

    test('Search finds the source text, never the format', () {
      final note = notes.capture('x');
      store.append(note.id, NoteKeys.text, _future);
      final amended = notes.capture('Dentist');
      final section = notes.amend(amended.id, 'Room 3');
      store.append(section.id, SectionKeys.text, _future);
      final search = Search(store);
      expect(search.notes('heading').map((n) => n.id), [amended.id, note.id]);
      expect(search.notes('format'), isEmpty);
      expect(search.notes('plain'), isEmpty);
    });
  });

  group('Conflicts', () {
    late Store phone;
    late Store desk;

    setUp(() {
      final clock = conflicts.ticking();
      phone = conflicts.device(clock);
      desk = conflicts.device(clock);
    });
    tearDown(() {
      phone.close();
      desk.close();
    });

    test('a text and its format conflict as one', () {
      final note = Notes(phone).capture('Milk');
      conflicts.exchange(phone, desk);
      Notes(phone).edit(note.id, 'Oat milk');
      desk.append(
        note.id,
        NoteKeys.text,
        '{"format":"x-future","text":"Oat milk"}',
      );
      conflicts.exchange(phone, desk);
      final conflict = phone.conflicts().single;
      expect(
        [for (final v in conflict.versions) v.text.format],
        ['x-future', TextFormats.plain],
      );
      expect(conflict.resolvable, isFalse);
      expect(
        () => phone.resolveConflict(conflict, conflict.versions.last),
        throwsStateError,
      );
      expect(
        () => phone.resolveConflict(conflict, conflict.versions.first),
        throwsStateError,
      );
      expect(phone.conflicts(), hasLength(1));
    });

    test('a text written before its format is the same as it marked plain', () {
      final note = Notes(phone).capture('Milk');
      conflicts.exchange(phone, desk);
      Notes(phone).edit(note.id, 'Oat milk');
      desk.append(note.id, NoteKeys.text, 'Oat milk');
      conflicts.exchange(phone, desk);
      expect(phone.conflicts(), isEmpty);
    });

    test('a text with version 1 written out is the same as without', () {
      final note = Notes(phone).capture('Milk');
      conflicts.exchange(phone, desk);
      Notes(phone).edit(note.id, 'Oat milk');
      desk.append(
        note.id,
        NoteKeys.text,
        '{"format":"plain","text":"Oat milk","v":1}',
      );
      conflicts.exchange(phone, desk);
      expect(phone.conflicts(), isEmpty);
    });

    test('a Conflict with a later structure is not resolved here', () {
      final note = Notes(phone).capture('Milk');
      conflicts.exchange(phone, desk);
      Notes(phone).edit(note.id, 'Oat milk');
      desk.append(note.id, NoteKeys.text, _later);
      conflicts.exchange(phone, desk);
      final conflict = phone.conflicts().single;
      expect(conflict.resolvable, isFalse);
      expect(
        () => phone.resolveConflict(conflict, conflict.versions.first),
        throwsStateError,
      );
    });
  });

  group('Sent Items', () {
    test('every text goes out with its format and arrives in it', () {
      final clock = ticking();
      final ann = sent.device(clock, 'Ann');
      final ben = sent.device(clock, 'Ben');
      final note = Notes(ann).capture('x');
      ann.append(note.id, NoteKeys.text, _future);
      final section = Notes(ann).amend(note.id, 'Room 3');
      final item = openSentItem(ben, sendNote(ann, note.id)).item;
      expect(item.text, '# Heading\n**bold**');
      expect(item.textFormat, 'x-future');
      expect(item.sections.single.format, TextFormats.plain);
      final payload = item.encode();
      expect(payload, contains('"note":{"format":"x-future","sections":['));
      expect(payload, contains(',"format":"plain","text":"Room 3"}'));
      // Version 1, the default, is not written.
      expect(payload, isNot(contains('"v":')));

      final got = acceptSentItem(ben, item)!;
      expect(ben.current(got, NoteKeys.text), _future);
      expect(
        ben.current(Notes(ben).sections(got).single.id, SectionKeys.text),
        ann.current(section.id, SectionKeys.text),
      );
    });

    test('a file without formats reads as plain', () {
      final clock = ticking();
      final ann = sent.device(clock, 'Ann');
      final item = SentItem.parse(
        SentItem(
          id: 'a',
          version: 1,
          from: ann.ownKeyRecord,
          profile: SignedProfile.fromJson(ann.ownProfileForOthers)!,
          text: 'Dentist',
          sections: [SentSection('Room 3', DateTime.utc(2026))],
        ).encode().replaceAll('"format":"plain",', ''),
      )!;
      expect(item.textFormat, TextFormats.plain);
      expect(item.textVersion, 1);
      expect(item.sections.single.format, TextFormats.plain);
      expect(item.sections.single.version, 1);
      expect(
        SentItem.parse(
          SentItem(
            id: 'a',
            version: 1,
            from: ann.ownKeyRecord,
            profile: SignedProfile.fromJson(ann.ownProfileForOthers)!,
            text: 'Dentist',
          ).encode().replaceAll('"format":"plain"', '"format":1'),
        ),
        isNull,
      );
      final file = SentItem(
        id: 'a',
        version: 1,
        from: ann.ownKeyRecord,
        profile: SignedProfile.fromJson(ann.ownProfileForOthers)!,
        text: 'Dentist',
        sections: [SentSection('Room 3', DateTime.utc(2026))],
      ).encode();
      String section(String v) =>
          file.replaceFirst('"text":"Room 3"}', '"text":"Room 3","v":$v}');
      String note(String v) =>
          file.replaceFirst('"text":"Dentist"}', '"text":"Dentist","v":$v}');
      // Version 1 said all the same reads as the default.
      expect(SentItem.parse(section('1'))?.sections.single.version, 1);
      expect(SentItem.parse(note('1'))?.textVersion, 1);
      expect(SentItem.parse(note('2'))?.textVersion, 2);
      // A version that is no whole number from 1 makes it unreadable.
      for (final v in ['"1"', '1.5', '0', 'null']) {
        expect(SentItem.parse(section(v)), isNull, reason: 'section $v');
        expect(SentItem.parse(note(v)), isNull, reason: 'note $v');
      }
    });

    test('a text in a later structure goes out and arrives in it', () {
      final clock = ticking();
      final ann = sent.device(clock, 'Ann');
      final ben = sent.device(clock, 'Ben');
      final note = Notes(ann).capture('x');
      ann.append(note.id, NoteKeys.text, _later);
      final section = Notes(ann).amend(note.id, 'Room 3');
      ann.append(section.id, SectionKeys.text, _later);
      final item = openSentItem(ben, sendNote(ann, note.id)).item;
      expect(item.textVersion, 2);
      expect(item.sections.single.version, 2);
      final got = acceptSentItem(ben, item)!;
      const arrived = '{"format":"plain","text":"Later milk","v":2}';
      expect(ben.current(got, NoteKeys.text), arrived);
      expect(Notes(ben).byId(got)?.textEditable, isFalse);
      final there = Notes(ben).sections(got).single;
      expect(ben.current(there.id, SectionKeys.text), arrived);
      expect(there.textEditable, isFalse);
    });

    test('an update never overwrites a text in an unknown format made '
        'here', () {
      final clock = ticking();
      final ann = sent.device(clock, 'Ann');
      final ben = sent.device(clock, 'Ben');
      final note = Notes(ann).capture('Dentist');
      final got = acceptSentItem(
        ben,
        openSentItem(ben, sendNote(ann, note.id)).item,
      )!;
      // A later version of Ben's formatted it.
      ben.append(got, NoteKeys.text, _future);
      Notes(ann).edit(note.id, 'Dentist moved');
      Notes(ann).amend(note.id, 'Room 3');
      final opened = openSentItem(ben, sendNote(ann, note.id));
      expect(opened.changes!.textBefore, isNull);
      expect(opened.changes!.sectionsAdded, hasLength(1));
      acceptSentItem(ben, opened.item);
      expect(ben.current(got, NoteKeys.text), _future);
      expect(Notes(ben).sections(got).single.text, 'Room 3');
    });

    test("the sender's own text in an unknown format follows their update", () {
      final clock = ticking();
      final ann = sent.device(clock, 'Ann');
      final ben = sent.device(clock, 'Ben');
      final note = Notes(ann).capture('x');
      ann.append(note.id, NoteKeys.text, _future);
      final got = acceptSentItem(
        ben,
        openSentItem(ben, sendNote(ann, note.id)).item,
      )!;
      ann.append(
        note.id,
        NoteKeys.text,
        '{"format":"x-future","text":"# Moved"}',
      );
      final opened = openSentItem(ben, sendNote(ann, note.id));
      expect(opened.changes!.textBefore, '# Heading\n**bold**');
      expect(opened.changes!.textEdited, isFalse);
      acceptSentItem(ben, opened.item);
      expect(Notes(ben).byId(got)?.text, '# Moved');
      expect(Notes(ben).byId(got)?.textFormat, 'x-future');
    });
  });

  test('sync carries a text in an unknown format as it is', () {
    final clock = conflicts.ticking();
    final phone = conflicts.device(clock);
    final desk = conflicts.device(clock);
    addTearDown(phone.close);
    addTearDown(desk.close);
    final note = Notes(phone).capture('x');
    phone.append(note.id, NoteKeys.text, _future);
    conflicts.exchange(phone, desk);
    expect(desk.current(note.id, NoteKeys.text), _future);
    expect(Notes(desk).byId(note.id)?.textEditable, isFalse);
  });
}

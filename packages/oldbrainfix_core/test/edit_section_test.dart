import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show syncAll;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
  });
  tearDown(() => ann.close());

  Section sectionOf(Store store, String id) => Notes(store).section(id)!;

  group('editing a Section', () {
    test('an Amend is fixed in place: created stays, modified and edited by '
        'show, the old text stays in the log', () {
      final note = notes.capture('Dentist on Tuesday');
      final amend = notes.amend(note.id, 'Moved to Wednesday');
      final later = notes.amend(note.id, 'Called, all set');
      expect(amend.modified, isNull);
      expect(amend.editedBy, isNull);
      expect(notes.mayEditSection(amend.id), isTrue);

      notes.editSection(amend.id, '  Moved to Thursday  ');
      final edited = sectionOf(ann, amend.id);
      expect(edited.text, 'Moved to Thursday');
      expect(edited.created, amend.created);
      expect(edited.modified, isNotNull);
      expect(edited.modified!.isAfter(edited.created), isTrue);
      expect(edited.editedBy?.name, 'Ann');
      expect(edited.editedBy?.id, ann.ownProfile!.id);
      // Its place among the Sections stays.
      expect(
        [for (final s in notes.sections(note.id)) s.id],
        [amend.id, later.id],
      );
      expect(sectionOf(ann, later.id).modified, isNull);
      expect(
        [
          for (final e in ann.history(amend.id))
            if (e.field == SectionKeys.text) StoredText.read(e.value).text,
        ],
        ['Moved to Wednesday', 'Moved to Thursday'],
      );
    });

    test('an unchanged text writes nothing; a blank one is refused', () {
      final note = notes.capture('Dentist');
      final amend = notes.amend(note.id, 'Bring ☐ the X-ray');
      final mark = ann.currentSeq();
      notes.editSection(amend.id, sectionOf(ann, amend.id).text);
      notes.editSection(amend.id, ' ${sectionOf(ann, amend.id).text}\n');
      expect(ann.entriesAfter(mark), isEmpty);
      expect(sectionOf(ann, amend.id).modified, isNull);
      expect(() => notes.editSection(amend.id, '  '), throwsArgumentError);
      expect(
        () => notes.editSection('section:none', 'Text'),
        throwsArgumentError,
      );
      expect(() => notes.editSection(note.id, 'Text'), throwsArgumentError);
      expect(
        () => notes.editSection(amend.id, 'Text', format: 'x-later'),
        throwsArgumentError,
      );
      expect(ann.entriesAfter(mark), isEmpty);
    });

    test('the format and the Boxes go as in an edit of the Note', () {
      final note = notes.capture('Trip');
      final amend = notes.amend(note.id, 'Tent');
      notes.editSection(amend.id, '**Tent** ☐', format: TextFormats.markdown);
      var section = sectionOf(ann, amend.id);
      expect(section.textFormat, TextFormats.markdown);
      expect(section.boxes, hasLength(1));
      expect(section.shownText, '**Tent** ☐');
      // Formatted for good, whatever the edit says; the Box keeps its id.
      final box = section.boxes.single.id;
      notes.editSection(
        amend.id,
        '${section.text} torch',
        format: TextFormats.plain,
      );
      section = sectionOf(ann, amend.id);
      expect(section.textFormat, TextFormats.markdown);
      expect(section.boxes.single.id, box);
      expect(section.shownText, '**Tent** ☐ torch');
    });

    test('a text in a format not known here is never overwritten', () {
      final note = notes.capture('Trip');
      final amend = notes.amend(note.id, 'Tent');
      ann.append(
        amend.id,
        SectionKeys.text,
        '{"format":"x-future","text":"# Tent"}',
      );
      expect(notes.mayEditSection(amend.id), isFalse);
      final mark = ann.currentSeq();
      expect(() => notes.editSection(amend.id, 'Tent'), throwsStateError);
      expect(ann.entriesAfter(mark), isEmpty);
    });

    test('a merged Note is edited as a Section of the Note it went into', () {
      final host = notes.capture('Dentist number');
      final merged = notes.capture('Ask about the crown');
      notes.merge(merged.id, host.id);
      expect(notes.mayEditSection(merged.id), isTrue);
      notes.editSection(merged.id, 'Ask about the bridge');
      final section = notes.sections(host.id).single;
      expect(section.id, merged.id);
      expect(section.text, 'Ask about the bridge');
      expect(section.modified, isNotNull);
      expect(section.editedBy?.name, 'Ann');
      expect(notes.byId(merged.id)!.text, 'Ask about the bridge');
    });

    test('a Section accepted from a Sent Item shows as never edited', () {
      final ben = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ben');
      addTearDown(ben.close);
      final note = notes.capture('Dentist');
      notes.amend(note.id, 'Moved to Wednesday');
      final file = encodeSentItem(sentItemOf(ann, note.id), ann.signingKey);
      final got = acceptSentItem(ben, openSentItem(ben, file).item)!;
      final section = Notes(ben).sections(got).single;
      expect(section.text, 'Moved to Wednesday');
      expect(section.modified, isNull);
      expect(section.editedBy, isNull);
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
      area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    test('a Member edits their own Amend, a Manager anyone\'s, and each '
        'side shows who did it', () {
      final note = notes.capture('Buy milk', place: area);
      syncAll(ben, ann);
      final own = Notes(ben).amend(note.id, 'Oat milk please');
      expect(Notes(ben).mayEditSection(own.id), isTrue);
      Notes(ben).editSection(own.id, 'Oat milk, two litres');
      syncAll(ben, ann);
      var onAnn = sectionOf(ann, own.id);
      expect(onAnn.text, 'Oat milk, two litres');
      expect(onAnn.editedBy?.name, 'Ben');
      expect(sectionOf(ben, own.id).editedBy?.name, 'Ben');

      expect(notes.mayEditSection(own.id), isTrue);
      notes.editSection(own.id, 'Oat milk, three litres');
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final section = sectionOf(store, own.id);
        expect(section.text, 'Oat milk, three litres');
        expect(section.editedBy?.name, 'Ann');
        expect(section.created, own.created);
      }
      onAnn = sectionOf(ann, own.id);
      expect(sectionOf(ben, own.id).modified, onAnn.modified);
    });

    test('a Member\'s edit of another\'s Amend is refused and, written all '
        'the same, ignored everywhere', () {
      final note = notes.capture('Buy milk', place: area);
      final theirs = notes.amend(note.id, 'Two litres');
      syncAll(ben, ann);
      final mark = ben.currentSeq();
      expect(Notes(ben).mayEditSection(theirs.id), isFalse);
      expect(
        () => Notes(ben).editSection(theirs.id, 'Ten litres'),
        throwsStateError,
      );
      expect(ben.entriesAfter(mark), isEmpty);
      // As a modified app would write it.
      ben.append(
        theirs.id,
        SectionKeys.text,
        StoredText.of(TextFormats.plain, 'Ten litres').encode(),
      );
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final section = sectionOf(store, theirs.id);
        expect(section.text, 'Two litres');
        expect(section.modified, isNull);
        expect(section.editedBy, isNull);
      }
    });
  });

  group('what carries it', () {
    late Directory dir;

    setUp(() => dir = Directory.systemTemp.createTempSync('section'));
    tearDown(() => dir.deleteSync(recursive: true));

    test('a Bundle carries the edit to a paired Device', () async {
      final phone = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(phone.close);
      final note = notes.capture('Dentist');
      final amend = notes.amend(note.id, 'Moved to Wednesday');
      exchange(phone, ann);
      notes.editSection(amend.id, 'Moved to Thursday');
      final path = '${dir.path}/b.$bundleExtension';
      expect(await writeBundle(ann, path), isNotNull);
      await importBundle(phone, path);
      final section = sectionOf(phone, amend.id);
      expect(section.text, 'Moved to Thursday');
      expect(section.modified, sectionOf(ann, amend.id).modified);
      expect(section.editedBy?.name, 'Ann');
    });
  });
}

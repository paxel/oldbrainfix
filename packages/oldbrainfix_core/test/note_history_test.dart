import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show syncAll;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;
  late NoteHistory history;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
    history = NoteHistory(ann);
  });
  tearDown(() => ann.close());

  List<(NoteChangeKind, String?, String?)> said(String noteId) => [
    for (final c in history.of(noteId)) (c.kind, c.before, c.after),
  ];

  test('every change of the text, newest first, with its text before and '
      'after, who and when', () {
    final note = notes.capture('Dentist on Tuesday');
    notes.edit(note.id, 'Dentist on Wednesday');
    notes.edit(note.id, 'Dentist on Thursday');
    expect(said(note.id), [
      (NoteChangeKind.text, 'Dentist on Wednesday', 'Dentist on Thursday'),
      (NoteChangeKind.text, 'Dentist on Tuesday', 'Dentist on Wednesday'),
      (NoteChangeKind.captured, null, 'Dentist on Tuesday'),
    ]);
    final changes = history.of(note.id);
    expect({for (final c in changes) c.by?.name}, {'Ann'});
    expect(changes.first.at.isAfter(changes.last.at), isTrue);
    expect(changes.last.at, note.captured);
    expect(history.of('note:none'), isEmpty);
  });

  test('an edit dated back goes where its date puts it, as latest wins '
      'reads it', () {
    final note = notes.capture('First');
    notes.edit(note.id, 'Second');
    ann.append(
      note.id,
      NoteKeys.text,
      StoredText.of(TextFormats.plain, 'Dated back').encode(),
      date: note.captured.add(const Duration(milliseconds: 1)),
    );
    expect(said(note.id), [
      (NoteChangeKind.text, 'Dated back', 'Second'),
      (NoteChangeKind.text, 'First', 'Dated back'),
      (NoteChangeKind.captured, null, 'First'),
    ]);
  });

  test('Amends added and edited, a Note merged in and its edits', () {
    final note = notes.capture('Dentist');
    final amend = notes.amend(note.id, 'Moved to Wednesday');
    notes.editSection(amend.id, 'Moved to Thursday');
    final other = notes.capture('Ask about the crown');
    notes.merge(other.id, note.id);
    notes.editSection(other.id, 'Ask about the bridge');
    expect(said(note.id), [
      (
        NoteChangeKind.amendEdited,
        'Ask about the crown',
        'Ask about the bridge',
      ),
      (NoteChangeKind.mergedIn, null, 'Ask about the bridge'),
      (NoteChangeKind.amendEdited, 'Moved to Wednesday', 'Moved to Thursday'),
      (NoteChangeKind.amended, null, 'Moved to Wednesday'),
      (NoteChangeKind.captured, null, 'Dentist'),
    ]);
    expect(history.of(note.id).first.entity, other.id);
    expect(history.of(note.id)[2].entity, amend.id);
  });

  test('settings: Importance, filing, the kind, the format, the trash, '
      'photos and recordings', () {
    final places = Places(ann);
    final home = places.addArea('Home');
    final note = notes.capture('Tent');
    notes.setImportance(note.id, Importance.important);
    notes.file(note.id, home.id);
    notes.setFormat(note.id, TextFormats.markdown);
    final photo = notes.addPhoto(note.id, Uint8List.fromList([1, 2, 3]));
    notes.setPhotoText(note.id, photo, 'Tent pegs');
    notes.removePhoto(note.id, photo);
    notes.addRecording(note.id, Uint8List.fromList([4, 5, 6]));
    notes.trash(note.id);
    notes.restore(note.id);
    expect(said(note.id), [
      (NoteChangeKind.restored, null, null),
      (NoteChangeKind.trashed, null, null),
      (NoteChangeKind.recordingAdded, null, null),
      (NoteChangeKind.photoRemoved, null, null),
      (NoteChangeKind.photoAdded, null, null),
      (NoteChangeKind.format, TextFormats.plain, TextFormats.markdown),
      (NoteChangeKind.filed, null, home.id),
      (NoteChangeKind.importance, null, 'important'),
      (NoteChangeKind.captured, null, 'Tent'),
    ]);
  });

  test('Lines added, renamed, checked, unchecked, deleted and restored, '
      'and the kind they came with', () {
    final note = notes.capture('Trip');
    final lists = Checklists(ann);
    lists.make(note.id);
    final tent = lists.add(note.id, 'Tent');
    lists.rename(tent, 'Big tent');
    lists.check(tent);
    lists.uncheck(tent);
    lists.delete(tent);
    lists.restore(tent);
    expect(said(note.id).take(7), [
      (NoteChangeKind.lineRestored, null, 'Big tent'),
      (NoteChangeKind.lineRemoved, null, 'Big tent'),
      (NoteChangeKind.lineUnchecked, null, 'Big tent'),
      (NoteChangeKind.lineChecked, null, 'Big tent'),
      (NoteChangeKind.lineRenamed, 'Tent', 'Big tent'),
      (NoteChangeKind.lineAdded, null, 'Tent'),
      (NoteChangeKind.kind, null, ChecklistKeys.checklist),
    ]);
  });

  test('"Use this version" writes the old text as a new version; nothing '
      'is deleted', () {
    final note = notes.capture('Dentist on Tuesday');
    notes.edit(note.id, 'Dentist on Wednesday');
    final amend = notes.amend(note.id, 'Bring the X-ray');
    notes.editSection(amend.id, 'Bring nothing');
    var changes = history.of(note.id);
    final captured = changes.last;
    final now = changes.firstWhere((c) => c.kind == NoteChangeKind.text);
    final amended = changes.firstWhere((c) => c.kind == NoteChangeKind.amended);
    // The text showing now is no version to go back to.
    expect(history.mayUse(note.id, now), isFalse);
    expect(history.mayUse(note.id, changes.first), isFalse);
    expect(history.mayUse(note.id, captured), isTrue);
    expect(history.mayUse(note.id, amended), isTrue);
    final before = ann.currentSeq();

    history.use(note.id, captured);
    history.use(note.id, amended);
    expect(notes.byId(note.id)!.text, 'Dentist on Tuesday');
    expect(notes.section(amend.id)!.text, 'Bring the X-ray');
    expect(ann.entriesAfter(before), hasLength(2));
    changes = history.of(note.id);
    expect(changes, hasLength(6));
    expect(
      (changes[0].kind, changes[0].after),
      (NoteChangeKind.amendEdited, 'Bring the X-ray'),
    );
    expect(
      (changes[1].kind, changes[1].before, changes[1].after),
      (NoteChangeKind.text, 'Dentist on Wednesday', 'Dentist on Tuesday'),
    );
    expect(history.mayUse(note.id, changes.last), isFalse);
    expect(() => history.use(note.id, changes.last), throwsStateError);
  });

  test('a text in a format not known here is never used again', () {
    final note = notes.capture('Plain');
    ann.append(
      note.id,
      NoteKeys.text,
      '{"format":"x-future","text":"# Later"}',
    );
    // As a version that knows it writes the next text.
    ann.append(
      note.id,
      NoteKeys.text,
      StoredText.of(TextFormats.plain, 'Plain again').encode(),
    );
    final changes = history.of(note.id);
    expect(changes[1].after, '# Later');
    expect(changes[1].format, isNull);
    expect(history.mayUse(note.id, changes[1]), isFalse);
    expect(history.mayUse(note.id, changes[2]), isTrue);
  });

  test('a box shows in the glyph it was typed with, without its id, and '
      'its text comes back with a box', () {
    final note = notes.capture('☐ Passport');
    final box = notes.byId(note.id)!.boxes.single.id;
    Boxes(ann).tick(note.id, box, BoxState.done);
    notes.edit(note.id, '${notes.byId(note.id)!.text} and visa');
    final changes = history.of(note.id);
    expect(said(note.id), [
      (NoteChangeKind.text, '☐ Passport', '☐ Passport and visa'),
      (NoteChangeKind.captured, null, '☐ Passport'),
    ]);
    expect(history.mayUse(note.id, changes.first), isFalse);
    history.use(note.id, changes.last);
    expect(notes.byId(note.id)!.boxes, hasLength(1));
    expect(notes.byId(note.id)!.visibleText, '☐ Passport');
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

    test('who is the Profile of each writer; what the roles refuse is not '
        'listed, and a Member uses no version of another\'s Note', () {
      final note = notes.capture('Buy milk', place: area);
      syncAll(ben, ann);
      final own = Notes(ben).amend(note.id, 'Oat milk');
      Notes(ben).editSection(own.id, 'Oat milk, two litres');
      // As a modified app would write it: a Member may not edit the Note.
      ben.append(
        note.id,
        NoteKeys.text,
        StoredText.of(TextFormats.plain, 'Buy beer').encode(),
      );
      syncAll(ben, ann);
      notes.edit(note.id, 'Buy milk and bread');
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        expect(
          [
            for (final c in NoteHistory(store).of(note.id))
              (c.kind, c.after, c.by?.name),
          ],
          [
            (NoteChangeKind.text, 'Buy milk and bread', 'Ann'),
            (NoteChangeKind.amendEdited, 'Oat milk, two litres', 'Ben'),
            (NoteChangeKind.amended, 'Oat milk', 'Ben'),
            (NoteChangeKind.filed, area, 'Ann'),
            (NoteChangeKind.captured, 'Buy milk', 'Ann'),
          ],
        );
      }
      final onBen = NoteHistory(ben).of(note.id);
      expect(NoteHistory(ben).mayUse(note.id, onBen.last), isFalse);
      expect(NoteHistory(ben).mayUse(note.id, onBen[2]), isTrue);
      expect(NoteHistory(ben).mayUse(note.id, onBen[1]), isFalse);
      expect(history.mayUse(note.id, history.of(note.id).last), isTrue);
    });
  });
}

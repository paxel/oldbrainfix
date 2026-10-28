/// Writes the fixture corpus the desktop core is tested against.
///
///     dart run tool/gen_fixtures.dart ../../desktop/fixtures
///
/// Every scenario fills an in-memory store with a fixed signing key, a
/// fixed clock and counted ids, and writes its log (`log.jsonl`, one
/// entry per line in log order) and what the core reads from it
/// (`expected.json`). The desktop core loads each log into its own store
/// and must read the same. The Routine scenario adds what the alarm plan,
/// the Missed Summary, the Nudge and the Agenda come to at a few held
/// moments (`alarms.json`), on a European wall clock. A scenario with
/// photos and voice recordings writes their bytes as the store keeps
/// them, under `photos/<hash>` and `audio/<hash>`. Next to them: the
/// store's schema, a signed entry with awkward text, the question-mark
/// scatter, and RRULEs with their Occurrences and the instants of
/// daylight-saving days (`schedules.json`).
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:image/image.dart' as img;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:sqlite3/sqlite3.dart';

/// The writer's fixed key seed: bytes 1 to 32.
final _seed = Uint8List.fromList(List<int>.generate(32, (i) => 1 + i));

/// A clock that starts at a fixed moment and moves one second per read,
/// unless it is held still for reading the results.
late DateTime _tick;
DateTime? _held;
DateTime _clock() => _held ?? (_tick = _tick.add(const Duration(seconds: 1)));

typedef Scenario = ({String name, String about, void Function(Store) build});

/// The wall clock of Central Europe, which Europe/Berlin keeps since
/// 1996: summer time (+2) from the last Sunday of March 01:00 UTC to the
/// last Sunday of October 01:00 UTC, winter time (+1) around it.
DateTime _europe(DateTime utc) {
  DateTime lastSunday(int year, int month) {
    final last = DateTime.utc(year, month + 1, 0);
    return DateTime.utc(year, month, last.day - last.weekday % 7, 1);
  }

  final u = utc.toUtc();
  final summer =
      !u.isBefore(lastSunday(u.year, 3)) && u.isBefore(lastSunday(u.year, 10));
  final wall = u.add(Duration(hours: summer ? 2 : 1));
  return DateTime.utc(
    wall.year,
    wall.month,
    wall.day,
    wall.hour,
    wall.minute,
    wall.second,
    wall.millisecond,
  );
}

ClockTime _at(int hour, [int minute = 0]) => ClockTime(hour, minute);

final _scenarios = <Scenario>[
  (
    name: 'notes',
    about:
        'Notes of every Importance; a text fixed in place; an edit dated '
        'back that loses to the text before it; two texts half a '
        'millisecond apart written in reverse order; text with quotes, '
        'backslashes, control characters, umlauts and an emoji.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Buy more sauce');
      notes.setImportance(a.id, Importance.important);
      final b = notes.capture('nees mor sauce');
      notes.edit(b.id, 'Needs more sauce');
      final c = notes.capture('Maybe paint the shed');
      notes.setImportance(c.id, Importance.unimportant);
      notes.setImportance(c.id, Importance.normal);
      notes.setImportance(c.id, Importance.unimportant);
      final d = notes.capture('First text');
      store.append(
        d.id,
        NoteKeys.text,
        'Dated back',
        date: DateTime.utc(2025, 12, 31),
      );
      final e = notes.capture('Sub-millisecond');
      store.append(
        e.id,
        NoteKeys.text,
        'Half a millisecond later',
        date: DateTime.utc(2026, 1, 2, 9, 0, 0, 0, 500),
      );
      store.append(
        e.id,
        NoteKeys.text,
        'Earlier, written later',
        date: DateTime.utc(2026, 1, 2, 9),
      );
      notes.capture(_awkward);
    },
  ),
  (
    name: 'sections',
    about:
        'Amends; a chain of Merges where the merged Note had an Amend of '
        'its own; two Notes merged into each other, kept under the lower '
        'id; a Merge into a Note this Device does not hold.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Dentist on Tuesday');
      notes.amend(a.id, 'Moved to Wednesday');
      final b = notes.capture('Ask about the crown');
      notes.amend(b.id, 'Bring the old X-ray');
      final c = notes.capture('Dentist number');
      notes.merge(b.id, a.id);
      notes.merge(a.id, c.id);
      notes.amend(c.id, 'Called, all set');
      final x = notes.capture('Loop one');
      final y = notes.capture('Loop two');
      store.append(y.id, NoteKeys.merged, x.id);
      store.append(x.id, NoteKeys.merged, y.id);
      final z = notes.capture('Merged into a stranger');
      store.append(z.id, NoteKeys.merged, 'note:elsewhere');
    },
  ),
  (
    name: 'formats',
    about:
        'Texts written with their format, plain; texts written before '
        'their format was recorded, one looking like a marked text; a Note '
        'and a Section in a format this version does not know, amended '
        'and merged into a plain Note; texts by the version of their '
        'structure, which only the last of these write out: 1 in plain, '
        '1 in a format not known, 2 with a key more, 2 without text, and '
        'one not a whole number.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Plain milk');
      notes.edit(a.id, 'Plain oat milk');
      final b = notes.capture('Before the format');
      store.append(b.id, NoteKeys.text, 'Written before the format');
      store.append(b.id, NoteKeys.text, '{"format":"plain"}');
      final f = notes.capture('Formatted later');
      store.append(f.id, NoteKeys.text, _future);
      final s = notes.amend(f.id, 'Amended here');
      store.append(s.id, SectionKeys.text, _future);
      notes.merge(f.id, notes.capture('Plain host').id);
      final g = notes.capture('Formatted, standing alone');
      store.append(g.id, NoteKeys.text, _future);
      for (final value in _versioned) {
        store.append(notes.capture('Versioned').id, NoteKeys.text, value);
      }
    },
  ),
  (
    name: 'places',
    about:
        'The whole Starter Set planted, one starter Area renamed; an Area '
        'of the user with a colour and an icon and a Topic; Notes filed '
        'into Areas and Topics; a Note whose Topic vanished falls back to '
        'Unsorted; a deleted Area takes its Topics with it.',
    build: (store) {
      final places = Places(store);
      final notes = Notes(store);
      places.plantStarterSet({
        for (final area in starterSet) ...{
          area.slug,
          for (final topic in area.topics) topic.slug,
        },
      });
      final car = places.areas().firstWhere((a) => a.starter == 'car');
      places.rename(car.id, 'Bike');
      final kids = places.addArea('Kids');
      places.setColour(kids.id, '6A1B9A');
      places.setIcon(kids.id, 'kids');
      final school = places.addTopic(kids.id, 'School');
      final household = places.areas().firstWhere(
        (a) => a.starter == 'household',
      );
      final kitchen = places
          .topics(household.id)
          .firstWhere((t) => t.starter == 'kitchen');
      notes.capture('Descale the kettle', place: kitchen.id);
      notes.capture('Sort the cellar', place: household.id);
      notes.capture('Parents evening', place: school.id);
      notes.capture('New tyres', place: car.id);
      final lost = notes.capture('Where did this go?', place: school.id);
      store.append(school.id, PlaceKeys.type, null);
      notes.capture('Still Unsorted');
      final gone = places.addArea('Gone');
      places.addTopic(gone.id, 'Gone too');
      places.deleteArea(gone.id, NotesGo.unsorted);
      notes.setImportance(lost.id, Importance.important);
    },
  ),
  (
    name: 'trash',
    about:
        'A Note in the trash; one trashed and restored; one trashed long '
        'enough ago to be past restoring; a merged Note whose target is '
        'in the trash.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Old shopping list');
      notes.trash(a.id);
      final b = notes.capture('Not so old after all');
      notes.trash(b.id);
      notes.restore(b.id);
      final c = notes.capture('Long gone');
      store.append(
        c.id,
        NoteKeys.trashed,
        DateTime.utc(2025, 11, 1).toIso8601String(),
      );
      final d = notes.capture('Into the bin with the other');
      final e = notes.capture('The other');
      notes.merge(d.id, e.id);
      notes.trash(e.id);
    },
  ),
  (
    name: 'edits',
    about:
        'What the desktop writes beyond the other scenarios: Areas and '
        'Topics made and renamed, an Area coloured and given an icon, '
        'another one with both cleared again; Notes captured with each '
        'Nudge time, some into a place; Notes filed and filed again; Topics '
        'deleted with their Notes sent back to Unsorted (one that spawned a '
        'Routine included), into the Area and into the trash, and an empty '
        'one; Areas deleted with their Topics, their Notes sent into the '
        'trash and back to Unsorted.',
    build: (store) {
      final notes = Notes(store);
      final places = Places(store);
      final home = places.addArea('Hom');
      places.rename(home.id, 'Home');
      places.setColour(home.id, '00838F');
      places.setIcon(home.id, 'home');
      final post = places.addTopic(home.id, 'Post');
      final bills = places.addTopic(home.id, 'Bils');
      places.rename(bills.id, 'Bills');
      final junk = places.addTopic(home.id, 'Junk');
      final empty = places.addTopic(home.id, 'Empty');
      final errands = places.addArea('Errands');
      places.setColour(errands.id, 'C62828');
      places.setIcon(errands.id, 'shopping');
      places.setColour(errands.id, null);
      places.setIcon(errands.id, null);
      final shop = places.addTopic(errands.id, 'Shop');
      final garage = places.addArea('Garage');
      final tools = places.addTopic(garage.id, 'Tools');
      final plumber = notes.capture(
        'Call the plumber',
        nudgeIn: const Duration(minutes: 10),
      );
      final plants = notes.capture(
        'Water the plants',
        nudgeIn: const Duration(minutes: 30),
      );
      final passport = notes.capture(
        'Renew the passport',
        nudgeIn: Notes.defaultNudge,
      );
      final hour = Notes.defaultNudge;
      notes.capture('Pay the rent', place: bills.id, nudgeIn: hour);
      notes.capture('Old flyer', place: junk.id, nudgeIn: hour);
      notes.capture('Buy stamps', place: shop.id, nudgeIn: hour);
      notes.capture('Return the drill', place: errands.id, nudgeIn: hour);
      notes.capture('Oil the saw', place: tools.id, nudgeIn: hour);
      notes.capture('Sweep the floor', place: garage.id, nudgeIn: hour);
      notes.file(plumber.id, post.id);
      notes.file(plumber.id, bills.id);
      notes.file(plants.id, post.id);
      notes.file(passport.id, post.id);
      // A Routine was spawned from it on another Device.
      store.append(
        passport.id,
        NoteKeys.spawned,
        DateTime.utc(2026, 1, 1, 12).toIso8601String(),
      );
      places.deleteTopic(post.id, NotesGo.unsorted);
      places.deleteTopic(bills.id, NotesGo.area);
      places.deleteTopic(junk.id, NotesGo.trash);
      places.deleteTopic(empty.id, NotesGo.unsorted);
      places.deleteArea(errands.id, NotesGo.trash);
      places.deleteArea(garage.id, NotesGo.unsorted);
    },
  ),
  (
    name: 'media',
    about:
        'Photos and voice recordings: Notes captured with photos, '
        'recordings or both, with and without text; photos and recordings '
        'added later; a photo whose Exif, ICC and comment are stripped; the '
        'same photo on two Notes kept once; Photo Texts and Transcripts, '
        'empty ones for nothing found; a Note with a Photo Text and one '
        'with an untranscribed recording merged into another; a trashed '
        'Note whose media waits for nothing; a photo from another Device '
        'whose bytes are not here; a photo that may be removed; a photo '
        'removed and its Photo Text kept, '
        'its bytes kept for the other Note showing it; a merged Note\'s '
        'photo and a recording removed, their bytes gone, their texts '
        'kept; Notes without words titled as a photo or as a voice '
        'recording, one with an empty Transcript.',
    build: (store) {
      final notes = Notes(store);
      final receipt = notes.capture('', photos: [_photo(1)]);
      notes.setPhotoText(
        receipt.id,
        receipt.photos.single,
        '  RECEIPT\nTotal 12,50  ',
      );
      final voice = notes.capture(
        'Groceries',
        recordings: [_recording(1)],
        nudgeIn: const Duration(minutes: 10),
      );
      notes.setTranscript(
        voice.id,
        voice.recordings.single,
        ' Milk, eggs and bread ',
      );
      notes.addPhoto(voice.id, _photo(1));
      final shed = notes.capture(
        'Shed',
        photos: [_photo(2), _photo(3)],
        recordings: [_recording(2)],
      );
      final key = notes.addPhoto(shed.id, _photo(4));
      notes.addRecording(shed.id, _recording(3));
      notes.setTranscript(shed.id, shed.recordings.single, '');
      notes.setPhotoText(shed.id, shed.photos.first, '');
      final dentist = notes.capture('Dentist');
      final card = notes.capture('', photos: [_photo(5)]);
      notes.setPhotoText(
        card.id,
        card.photos.single,
        'Dr. Müller, Tuesday 9:00',
      );
      notes.merge(card.id, dentist.id);
      final memo = notes.capture('', recordings: [_recording(4)]);
      notes.merge(memo.id, dentist.id);
      final binned = notes.capture(
        'Old flyer',
        photos: [_photo(6)],
        recordings: [_recording(5)],
      );
      notes.trash(binned.id);
      store.append(dentist.id, NoteKeys.photo('ab' * 32), NoteKeys.photoAdded);
      notes.setPhotoText(shed.id, key, 'Spare key under the pot');
      notes.removePhoto(receipt.id, receipt.photos.single);
      notes.removePhoto(card.id, card.photos.single);
      notes.removeRecording(voice.id, voice.recordings.single);
      notes.capture('', photos: [_photo(7)]);
      final hum = notes.capture('', recordings: [_recording(6)]);
      notes.setTranscript(hum.id, hum.recordings.single, '');
    },
  ),
  (
    name: 'routines',
    about:
        'Routines and Reminders spawned from Notes and made with their '
        'Note behind the scenes: several times a day with its own Re-nag '
        'and Escalation, all-day weekends, Reminders on the hour the clock '
        'skips and the one it shows twice, every other day, the last of '
        'the month rescheduled to two times, a Re-nag set and cleared, an '
        'unchanged reschedule that writes nothing; Occurrences Done, Done '
        'at a given time and unchecked; a Routine trashed, another trashed '
        'and restored; Notes trashed with their Routines and restored, '
        'trashed without them, and trashed with them for good; Unsorted '
        'Notes for the Nudge.',
    build: (store) {
      final notes = Notes(store);
      final routines = Routines(store);
      final pills = notes.capture('Take the pills');
      final medication = routines.spawn(
        pills.id,
        Schedule(
          start: Day(2026, 1, 1),
          times: [_at(20), _at(8), _at(14)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      routines.setRenag(medication.id, const Duration(minutes: 5));
      routines.setEscalation(medication.id, 3);
      routines.spawn(
        pills.id,
        Schedule(
          start: Day(2026, 3, 1),
          rule: Recurrence.parse('FREQ=MONTHLY;BYDAY=1SU'),
        ),
      );
      final plants = notes.capture(
        'Water the plants',
        nudgeIn: const Duration(minutes: 30),
      );
      final weekends = Schedule(
        start: Day(2026, 1, 3),
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=SA,SU'),
      );
      final weekend = routines.spawn(plants.id, weekends);
      routines.reschedule(weekend.id, weekends);
      final dentist = notes.capture('Dentist');
      routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 3, 29), times: [_at(2, 30)]),
      );
      routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 10, 25), times: [_at(2, 30)]),
      );
      final night = routines.create(
        '  Night meds  ',
        Schedule(
          start: Day(2026, 1, 1),
          times: [_at(23)],
          rule: Recurrence.parse('FREQ=DAILY;INTERVAL=2'),
        ),
      );
      routines.setRenag(night.id, const Duration(minutes: 15));
      routines.setRenag(night.id, null);
      final monthEnd = Recurrence.parse('FREQ=MONTHLY;BYMONTHDAY=-1');
      final rent = routines.create(
        'Pay the rent',
        Schedule(start: Day(2026, 1, 31), times: [_at(9)], rule: monthEnd),
      );
      routines.reschedule(
        rent.id,
        Schedule(
          start: Day(2026, 1, 31),
          times: [_at(9), _at(18)],
          rule: monthEnd,
        ),
      );
      routines.markDone(medication.id, Occurrence(Day(2026, 3, 28), _at(8)));
      routines.markDone(
        medication.id,
        Occurrence(Day(2026, 3, 28), _at(14)),
        at: DateTime.utc(2026, 3, 28, 15),
      );
      routines.markDone(medication.id, Occurrence(Day(2026, 3, 29), _at(8)));
      routines.unmarkDone(medication.id, Occurrence(Day(2026, 3, 29), _at(8)));
      routines.markDone(weekend.id, Occurrence(Day(2026, 10, 24)));
      final habit = routines.create(
        'Old habit',
        Schedule(
          start: Day(2026, 1, 1),
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      routines.trash(habit.id);
      final stretch = routines.create(
        'Stretch',
        Schedule(
          start: Day(2026, 1, 2),
          times: [_at(7, 30)],
          rule: Recurrence.parse('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH;WKST=SU'),
        ),
      );
      routines.trash(stretch.id);
      routines.restore(stretch.id);
      final walk = notes.capture('Walk the dog');
      routines.spawn(
        walk.id,
        Schedule(
          start: Day(2026, 1, 1),
          times: [_at(7)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      routines.trashNote(walk.id, withRoutines: true);
      routines.restoreNote(walk.id);
      final flyer = notes.capture('Flyer for the fair');
      routines.spawn(
        flyer.id,
        Schedule(start: Day(2026, 4, 1), times: [_at(10)]),
      );
      routines.trashNote(flyer.id, withRoutines: false);
      final car = notes.capture('Car inspection');
      routines.spawn(car.id, Schedule(start: Day(2026, 5, 1)));
      routines.trashNote(car.id, withRoutines: true);
      notes.capture('Unsorted thought', nudgeIn: const Duration(minutes: 10));
      notes.capture('Another thought');
    },
  ),
];

/// A small JPEG, different for every [n], with an APP0, an Exif APP1,
/// an ICC APP2 and a comment before its tables and scan: stripping keeps
/// APP0, the tables and the scan.
Uint8List _photo(int n) {
  List<int> segment(int marker, List<int> body) => [
    0xFF,
    marker,
    (body.length + 2) >> 8,
    (body.length + 2) & 0xFF,
    ...body,
  ];
  return Uint8List.fromList([
    0xFF, 0xD8, //
    ...segment(0xE0, utf8.encode('JFIF\u0000')),
    ...segment(0xE1, utf8.encode('Exif\u0000\u0000GPS 52.5N 13.4E')),
    ...segment(0xE2, utf8.encode('ICC_PROFILE')),
    ...segment(0xFE, utf8.encode('taken by the fixture')),
    ...segment(0xDB, [0, n, n, n]),
    0xFF, 0xDA, 0x00, 0x03, n, //
    1, 2, 3, n,
    0xFF, 0xD9,
  ]);
}

/// The bytes of a voice recording, different for every [n]; the store
/// keeps them as they are, whatever they hold.
Uint8List _recording(int n) =>
    Uint8List.fromList([...utf8.encode('RIFF voice $n WAVE'), 0, n, 255]);

/// The queries every scenario's search is checked with.
const _queries = [
  '',
  'sauce',
  'MILK',
  'receipt',
  'dentist',
  'x-ray',
  'müller',
  'called',
  'loop',
  ' e ',
  'heading',
  'format',
];

/// A text in a format this version does not know, as a later version
/// would store it.
const _future =
    '{"format":"x-future","text":"# Heading\\n**bold** \\u00fc","x":1}';

/// Texts by the version of their structure, `v`: 1 written out though it
/// is the default, 1 in a format not known, 2 in a known format with a
/// key more, 2 without text, and one that is not a whole number. Every
/// other text says none, version 1.
const _versioned = [
  '{"format":"plain","text":"Version one written out","v":1}',
  '{"format":"x-future","text":"# Version one","v":1}',
  '{"format":"plain","text":"Version two","v":2,"x":1}',
  '{"format":"plain","v":2}',
  '{"format":"plain","text":"Odd version","v":"1"}',
];

/// A value that tests how the signed bytes escape text.
const _awkward =
    'Quote " backslash \\ slash / newline \n tab \t bell \u0001 '
    'umlaut ü emoji 🧠';

Map<String, Object?> _note(Note note) => {
  'id': note.id,
  'text': note.text,
  'textFormat': note.textFormat,
  'textVersion': note.textVersion,
  'textEditable': note.textEditable,
  'captured': note.captured.toUtc().toIso8601String(),
  'place': note.place,
  'importance': note.importance.name,
  'trashed': note.trashed?.toUtc().toIso8601String(),
  'mergedInto': note.mergedInto,
  'spawned': note.spawned,
  'photos': note.photos,
  'recordings': note.recordings,
  'transcripts': note.transcripts,
  'photoTexts': note.photoTexts,
  'removedPhotos': note.removedPhotos,
  'removedRecordings': note.removedRecordings,
  'wordless': note.wordless.name,
};

Map<String, Object?> _section(Section section) => {
  'id': section.id,
  'noteId': section.noteId,
  'text': section.text,
  'textFormat': section.textFormat,
  'textVersion': section.textVersion,
  'textEditable': section.textEditable,
  'captured': section.context.captured.toUtc().toIso8601String(),
  'photos': section.photos,
  'recordings': section.recordings,
  'transcripts': section.transcripts,
  'photoTexts': section.photoTexts,
  'removedPhotos': section.removedPhotos,
  'removedRecordings': section.removedRecordings,
};

/// The photos and recordings of [note] this Device may remove and keep
/// the text of, as `photo/<hash>` and `audio/<hash>`.
List<String> _removable(Notes notes, Note note) => [
  for (final hash in note.photos)
    if (notes.mayRemovePhoto(note.id, hash)) 'photo/$hash',
  for (final hash in note.recordings)
    if (notes.mayRemoveRecording(note.id, hash)) 'audio/$hash',
];

List<Map<String, String>> _blobs(List<({String noteId, String hash})> list) => [
  for (final b in list) {'noteId': b.noteId, 'hash': b.hash},
];

/// What the core reads from [store]: the desktop core must read the same.
Map<String, Object?> _expected(Store store) {
  final notes = Notes(store);
  final places = Places(store);
  final noteIds = store.entitiesOfType(NoteKeys.typeNote)..sort();
  final all = notes.all();
  final trashed = notes.trashed();
  final placeIds = [
    for (final area in places.areas()) ...[
      area.id,
      for (final topic in places.topics(area.id)) topic.id,
    ],
  ];
  return {
    'now': store.clock().toIso8601String(),
    'device': store.deviceId,
    'areas': [
      for (final area in places.areas())
        {
          'id': area.id,
          'name': area.name,
          'starter': area.starter,
          'colour': area.colour,
          'icon': area.icon,
          'topics': [
            for (final topic in places.topics(area.id))
              {
                'id': topic.id,
                'areaId': topic.areaId,
                'name': topic.name,
                'starter': topic.starter,
              },
          ],
        },
    ],
    'notes': [
      for (final id in noteIds)
        if (notes.byId(id) case final note?) _note(note),
    ],
    'all': [for (final note in all) note.id],
    'unsorted': [for (final note in notes.unsorted()) note.id],
    'trashed': [for (final note in trashed) note.id],
    'inside': {
      for (final id in placeIds) id: [for (final n in notes.inside(id)) n.id],
    },
    'sections': {
      for (final note in [...all, ...trashed])
        note.id: [for (final s in notes.sections(note.id)) _section(s)],
    },
    'untranscribed': _blobs(notes.untranscribed()),
    'unreadPhotos': _blobs(notes.unreadPhotos()),
    'removable': {
      for (final id in noteIds)
        if (notes.byId(id) case final note?)
          if (_removable(notes, note) case final refs when refs.isNotEmpty)
            id: refs,
    },
    'search': {
      for (final query in _queries)
        query: [for (final n in Search(store).notes(query)) n.id],
    },
    'searchWithin': {
      for (final id in placeIds)
        id: [for (final n in Search(store).notes('e', within: id)) n.id],
    },
    'conflicts': _conflicts(store.conflicts()),
  };
}

/// [conflicts] as the fixtures hold them: the raw values of every
/// version, never formatted.
List<Map<String, Object?>> _conflicts(List<Conflict> conflicts) => [
  for (final c in conflicts)
    {
      'entity': c.entity,
      'unit': c.unit,
      'type': c.type,
      'resolvable': c.resolvable,
      'versions': [
        for (final v in c.versions)
          {
            'entry': v.entry.id,
            'device': v.entry.device,
            'recorded': v.entry.recorded.toUtc().toIso8601String(),
            'values': v.values,
          },
      ],
    },
];

/// Writes the photos and voice recordings [store] holds as it keeps
/// them, one file per blob named by its hash, under [out].
void _writeBlobs(Store store, Directory out) {
  for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
    final note = Notes(store).byId(id);
    if (note == null) continue;
    for (final (dir, hashes, bytes) in [
      ('photos', note.photos, store.photo),
      ('audio', note.recordings, store.audio),
    ]) {
      for (final hash in hashes) {
        final held = bytes(hash);
        if (held == null) continue;
        final file = File('${out.path}/$dir/$hash');
        file.parent.createSync(recursive: true);
        file.writeAsBytesSync(held);
      }
    }
  }
}

Map<String, Object?> _routine(Routine routine) => {
  'id': routine.id,
  'noteId': routine.noteId,
  'start': '${routine.schedule.start}',
  'times': [for (final t in routine.schedule.times) '$t'],
  'rule': routine.schedule.rule?.format(),
  'done': {
    for (final key in routine.done.keys.toList()..sort())
      key: routine.done[key]?.toUtc().toIso8601String(),
  },
  'renag': routine.renag?.inMinutes,
  'escalation': routine.escalation,
  'scheduledAt': routine.scheduledAt?.toUtc().toIso8601String(),
  'trashed': routine.trashed?.toUtc().toIso8601String(),
  'trashedWith': routine.trashedWith,
  'reminder': routine.isReminder,
};

/// How many alarms of each plan the fixture spells out, the soonest
/// first; the count covers them all.
const _alarmsShown = 40;

String _wall(WallMoment m) => '${m.day}T${m.time}';

/// What the Routines of [store] come to: the Routines themselves, then at
/// each held moment with this Device's settings as given, the Agenda, the
/// alarm plan, the Missed Summary and the Nudge, on [_europe].
Map<String, Object?> _alarms(Store store) {
  final routines = Routines(store);
  final ids = store.entitiesOfType(RoutineKeys.typeRoutine)..sort();
  String find(String text, {bool reminder = false}) => routines
      .all()
      .firstWhere(
        (r) => routines.note(r)?.text == text && r.isReminder == reminder,
      )
      .id;
  final medication = find('Take the pills');
  final weekend = find('Water the plants');
  final defaults = {
    'snoozes': '{}',
    'nudgeSnooze': '',
    'missedSeen': '',
    'renagMinutes': '',
    'quietHours': '',
    'quietHoursOff': '',
    'nudgeRepeatMinutes': '',
  };
  final checks = <(DateTime, Map<String, String>)>[
    // Saturday 22:30 in winter time: the Nudge waits for the morning
    // after the switch.
    (
      DateTime.utc(2026, 3, 28, 21, 30),
      {
        ...defaults,
        'missedSeen': DateTime.utc(2026, 3, 26).toIso8601String(),
        'snoozes': jsonEncode({
          '$medication@2026-03-28T20:00': DateTime.utc(
            2026,
            3,
            28,
            21,
            45,
          ).toIso8601String(),
        }),
      },
    ),
    // Sunday 09:00, just after the clock jumped; never seen, Re-nag and
    // repeat of this Device set.
    (
      DateTime.utc(2026, 3, 29, 7),
      {...defaults, 'renagMinutes': '20', 'nudgeRepeatMinutes': '30'},
    ),
    // In the hour the clock shows twice, the first time round; the Nudge
    // snoozed, Quiet Hours of its own.
    (
      DateTime.utc(2026, 10, 25, 0, 45),
      {
        ...defaults,
        'missedSeen': DateTime.utc(2026, 10, 23, 12).toIso8601String(),
        'quietHours': '23:30-06:15',
        'nudgeSnooze':
            '${DateTime.utc(2026, 10, 24, 20).toIso8601String()} '
            '${DateTime.utc(2026, 10, 25, 2).toIso8601String()}',
      },
    ),
    // Sunday noon after the switch, Quiet Hours off, a snooze past when
    // its Occurrence is Missed.
    (
      DateTime.utc(2026, 10, 25, 12),
      {
        ...defaults,
        'missedSeen': DateTime.utc(2026, 10, 24, 12).toIso8601String(),
        'quietHoursOff': 'true',
        'snoozes': jsonEncode({
          '$weekend@2026-10-25': DateTime.utc(
            2026,
            10,
            26,
            3,
          ).toIso8601String(),
          '$medication@2026-10-25T14:00': DateTime.utc(
            2026,
            10,
            25,
            13,
            20,
          ).toIso8601String(),
        }),
      },
    ),
    // Late on the last of the month, in winter time again.
    (
      DateTime.utc(2026, 10, 31, 22, 10),
      {...defaults, 'missedSeen': DateTime.utc(2026, 10, 30).toIso8601String()},
    ),
  ];
  List<Object?> rows(List<AgendaRow> rows) => [
    for (final row in rows)
      [
        row.routine.id,
        row.occurrence.key,
        row.note?.id,
        row.done?.toUtc().toIso8601String(),
      ],
  ];
  return {
    'routines': [
      for (final id in ids)
        if (routines.byId(id) case final routine?) _routine(routine),
    ],
    'all': [for (final r in routines.all()) r.id],
    'trashed': [for (final r in routines.trashed()) r.id],
    'ofNote': {
      for (final note in Notes(store).all())
        note.id: [for (final r in routines.ofNote(note.id)) r.id],
    },
    'checks': [
      for (final (now, local) in checks)
        () {
          _held = now;
          for (final MapEntry(:key, :value) in local.entries) {
            store.setLocalSetting(key, value);
          }
          final alarms = Alarms(store);
          final agenda = routines.agenda(_europe);
          final plan = alarms.plan(_europe);
          final missed = alarms.missed(_europe);
          final nudge = alarms.nudge(_europe);
          final later = now.add(const Duration(hours: 3));
          return {
            'now': now.toIso8601String(),
            'local': local,
            'today': '${agenda.today}',
            'agenda': {
              'today': rows(agenda.todayRows),
              'tomorrow': rows(agenda.tomorrowRows),
            },
            'alarmCount': plan.length,
            'alarms': [
              for (final a in plan.take(_alarmsShown))
                {
                  'id': a.id,
                  'kind': a.kind.name,
                  'routine': a.routineId,
                  'occurrence': a.occurrence.key,
                  'at': a.at.toUtc().toIso8601String(),
                  'wall': a.wall == null ? null : _wall(a.wall!),
                  'renag': a.renag.inMinutes,
                  'escalation': a.escalation,
                  'escalatesAt': a.escalatesAt?.toUtc().toIso8601String(),
                  'ends': _wall(a.ends),
                  'endsAt': a.endsAt.toUtc().toIso8601String(),
                  'text': a.text,
                  'wordless': a.wordless.name,
                  'firesAfterNow': a.firesAfter(now)?.toUtc().toIso8601String(),
                },
            ],
            'missed': {
              'count': missed.count,
              'nextChange': missed.nextChange?.toUtc().toIso8601String(),
              'routines': [
                for (final m in missed.routines)
                  {
                    'routine': m.routine.id,
                    'note': m.note?.id,
                    'missed': [
                      for (final o in m.missed)
                        [
                          o.occurrence.key,
                          o.missedAt.toUtc().toIso8601String(),
                        ],
                    ],
                  },
              ],
            },
            'nudge': nudge == null
                ? null
                : {
                    'notes': [for (final n in nudge.notes) n.id],
                    'at': nudge.at.toUtc().toIso8601String(),
                    'since': nudge.since?.toUtc().toIso8601String(),
                    'firesAfterLater': nudge
                        .firesAfter(later, _europe)
                        .toUtc()
                        .toIso8601String(),
                  },
            'recentlyDone': [
              for (final (id, occurrence) in alarms.recentlyDone(_europe))
                [id, occurrence.key],
            ],
          };
        }(),
    ],
  };
}

/// RRULEs as read and written back, refused with their reason, or as the
/// simple editor shows them; schedules with their Occurrences; days
/// normalised; and the instants a European clock shows wall-clock times
/// at around its daylight-saving switches.
Map<String, Object?> _schedules() {
  const rules = [
    'FREQ=DAILY',
    'rrule:freq=weekly;byday=mo,we,fr',
    'FREQ=WEEKLY;BYDAY=SA,MO',
    'FREQ=MONTHLY;BYDAY=1MO,-1FR',
    'FREQ=MONTHLY;BYDAY=+2TU',
    'FREQ=MONTHLY;INTERVAL=2;BYDAY=-1FR',
    'FREQ=MONTHLY;BYDAY=5MO',
    'FREQ=MONTHLY;BYMONTHDAY=15;COUNT=10',
    'FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261231',
    'FREQ=MONTHLY;BYMONTHDAY=13;BYDAY=FR',
    'FREQ=MONTHLY;BYMONTHDAY=-1',
    'FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=29',
    'FREQ=MONTHLY;BYMONTHDAY=31',
    'FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1,15,-1',
    'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SA;WKST=SU',
    'FREQ=DAILY;COUNT=5',
    'FREQ=DAILY;UNTIL=20260110T120000Z',
    'FREQ=DAILY;UNTIL=20260105',
    'FREQ=DAILY;BYHOUR=20,8;BYMINUTE=30,0',
    'FREQ=DAILY;BYMINUTE=15',
    'FREQ=YEARLY;BYDAY=1MO',
    'FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10,3',
    'FREQ=DAILY;BYDAY=SA,SU;BYMONTH=6,7,8',
    'FREQ=YEARLY',
    'FREQ=MONTHLY',
    'FREQ=WEEKLY;BYDAY=SU;BYMONTH=12',
    'FREQ=DAILY;INTERVAL=+3',
    'FREQ=DAILY;BYSECOND=0',
    'FREQ=DAILY;;INTERVAL=2;',
    '',
    'RRULE:',
    'FREQ',
    'FREQ=DAILY;FREQ=WEEKLY',
    'FREQ=DAILY;BYSETPOS=1',
    'INTERVAL=2',
    'FREQ=HOURLY',
    'FREQ=WEEKLY;BYDAY=1MO',
    'FREQ=MONTHLY;BYDAY=0MO',
    'FREQ=MONTHLY;BYDAY=54MO',
    'FREQ=MONTHLY;BYDAY=XX',
    'FREQ=MONTHLY;BYDAY=+MO',
    'FREQ=MONTHLY;BYDAY=',
    'FREQ=MONTHLY;BYMONTHDAY=0',
    'FREQ=MONTHLY;BYMONTHDAY=-0',
    'FREQ=MONTHLY;BYMONTHDAY=32',
    'FREQ=WEEKLY;BYMONTHDAY=3',
    'FREQ=DAILY;BYSECOND=5',
    'FREQ=DAILY;COUNT=2;UNTIL=20260101',
    'FREQ=DAILY;WKST=XY',
    'FREQ=DAILY;INTERVAL=0',
    'FREQ=DAILY;BYMONTH=13',
    'FREQ=DAILY;BYHOUR=24',
    'FREQ=DAILY;BYMINUTE=60',
    'FREQ=DAILY;COUNT=0',
    'FREQ=DAILY;UNTIL=20260230',
    'FREQ=DAILY;UNTIL=20260101T250000',
    'FREQ=DAILY;UNTIL=2026',
    'FREQ=DAILY;INTERVAL=two',
  ];
  Map<String, Object?>? simple(SimpleRepeat? simple) => simple == null
      ? null
      : {
          'kind': simple.kind.name,
          'every': simple.every,
          'unit': simple.unit.name,
          'weekdays': simple.weekdays.toList()..sort(),
          'monthDay': simple.monthDay,
          'nth': simple.nth?.toString(),
          'end': simple.end.name,
          'count': simple.count,
          'until': simple.until?.toString(),
          'rule': simple.toRule()?.format(),
        };
  final schedules =
      <(Day start, List<ClockTime> times, String? rule, Day from, Day to)>[
        (
          Day(2026, 1, 1),
          [_at(8), _at(20)],
          'FREQ=DAILY',
          Day(2026, 1, 1),
          Day(2026, 1, 4),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=DAILY;INTERVAL=3',
          Day(2026, 1, 5),
          Day(2026, 1, 20),
        ),
        (
          Day(2026, 1, 31),
          [_at(9)],
          'FREQ=MONTHLY',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 1, 31),
          [_at(9)],
          'FREQ=MONTHLY;BYMONTHDAY=-1',
          Day(2026, 1, 1),
          Day(2026, 6, 30),
        ),
        (
          Day(2024, 2, 29),
          [],
          'FREQ=YEARLY',
          Day(2024, 1, 1),
          Day(2033, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(7, 30)],
          'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SA;WKST=SU',
          Day(2026, 1, 1),
          Day(2026, 2, 28),
        ),
        (
          Day(2026, 1, 1),
          [_at(7, 30)],
          'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SA',
          Day(2026, 1, 1),
          Day(2026, 2, 28),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=MONTHLY;BYDAY=1MO,-1FR',
          Day(2026, 1, 1),
          Day(2026, 5, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(9)],
          'FREQ=MONTHLY;INTERVAL=2;UNTIL=20261231;BYDAY=-1FR',
          Day(2026, 1, 1),
          Day(2027, 3, 31),
        ),
        (
          Day(2026, 1, 20),
          [],
          'FREQ=MONTHLY;COUNT=4;BYMONTHDAY=15',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10,3',
          Day(2026, 1, 1),
          Day(2028, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=YEARLY;BYDAY=1MO',
          Day(2025, 1, 1),
          Day(2029, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(8), _at(12)],
          'FREQ=DAILY;COUNT=5',
          Day(2026, 1, 2),
          Day(2026, 1, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(8), _at(12)],
          'FREQ=DAILY;UNTIL=20260103T100000',
          Day(2026, 1, 1),
          Day(2026, 1, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(8)],
          'FREQ=DAILY;UNTIL=20260105',
          Day(2025, 12, 1),
          Day(2026, 1, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(6)],
          'FREQ=DAILY;BYHOUR=20,8;BYMINUTE=30,0',
          Day(2026, 1, 1),
          Day(2026, 1, 2),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=DAILY;BYDAY=SA,SU;BYMONTH=6,7',
          Day(2026, 1, 1),
          Day(2026, 7, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1,15,-1',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 3, 15),
          [],
          'FREQ=YEARLY;BYMONTHDAY=15',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=WEEKLY;BYDAY=SU;BYMONTH=12',
          Day(2026, 1, 1),
          Day(2027, 12, 31),
        ),
        (
          Day(2026, 10, 25),
          [_at(2, 30)],
          null,
          Day(2026, 10, 1),
          Day(2026, 10, 31),
        ),
        (Day(2026, 10, 25), [], null, Day(2026, 10, 26), Day(2026, 10, 31)),
        (
          Day(2026, 1, 1),
          [_at(8)],
          'FREQ=DAILY',
          Day(2026, 3, 10),
          Day(2026, 3, 12),
        ),
      ];
  final dstDays = [
    Day(2026, 3, 28),
    Day(2026, 3, 29),
    Day(2026, 3, 30),
    Day(2026, 10, 24),
    Day(2026, 10, 25),
    Day(2026, 10, 26),
    Day(2027, 3, 28),
    Day(2027, 10, 31),
  ];
  final dstTimes = [
    _at(0),
    _at(0, 30),
    _at(1, 59),
    _at(2),
    _at(2, 30),
    _at(2, 59),
    _at(3),
    _at(3, 30),
    _at(8),
    _at(23, 59),
  ];
  final quiet = [
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 3, 28, 21, 30)),
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 3, 28, 23, 30)),
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 3, 29, 6, 30)),
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 10, 25, 1, 30)),
    (QuietHours(_at(1), _at(3)), DateTime.utc(2026, 3, 29, 0, 30)),
    (QuietHours(_at(1), _at(3)), DateTime.utc(2026, 10, 25, 0, 30)),
    (QuietHours(_at(13), _at(14)), DateTime.utc(2026, 6, 1, 11, 30)),
    (QuietHours(_at(8), _at(8)), DateTime.utc(2026, 6, 1, 6)),
  ];
  return {
    'rules': [
      for (final text in rules)
        () {
          try {
            final rule = Recurrence.parse(text);
            return {
              'text': text,
              'format': rule.format(),
              'times': rule.times?.map((t) => '$t').toList(),
              'simple': simple(SimpleRepeat.of(rule, Day(2026, 1, 1))),
            };
          } on FormatException catch (e) {
            return {'text': text, 'error': e.message, 'source': '${e.source}'};
          }
        }(),
    ],
    'simpleOnce': simple(SimpleRepeat.of(null, Day(2026, 1, 1))),
    'schedules': [
      for (final (start, times, rule, from, to) in schedules)
        () {
          final schedule = Schedule(
            start: start,
            times: times,
            rule: rule == null ? null : Recurrence.parse(rule),
          );
          return {
            'start': '$start',
            'times': [for (final t in times) '$t'],
            'rule': rule,
            'from': '$from',
            'to': '$to',
            'dayTimes': [for (final t in schedule.dayTimes) '$t'],
            'between': [for (final o in schedule.between(from, to)) o.key],
            'next': [for (final o in schedule.next(from, 8)) o.key],
          };
        }(),
    ],
    'days': [
      for (final [y, m, d] in [
        [2026, 1, 32],
        [2026, 13, 1],
        [2026, 0, 0],
        [2024, 2, 30],
        [2026, -1, 15],
        [2025, 24, 31],
        [2026, 2, 29],
        [2000, 2, 29],
        [1900, 2, 29],
        [2026, 12, 31],
      ])
        {
          'in': [y, m, d],
          'day': '${Day(y, m, d)}',
          'weekday': Day(y, m, d).weekday,
        },
    ],
    'instants': [
      for (final day in dstDays)
        for (final time in dstTimes)
          [
            '$day',
            '$time',
            instantOf(day, time, _europe).toUtc().toIso8601String(),
          ],
    ],
    'quiet': [
      for (final (hours, instant) in quiet)
        [
          '$hours',
          instant.toIso8601String(),
          hours.defer(instant, _europe).toUtc().toIso8601String(),
        ],
    ],
  };
}

Store _store() {
  _tick = DateTime.utc(2026, 1, 1, 10);
  _held = null;
  var ids = 0;
  Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
  final store = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_seed),
  );
  store.setLocalSetting('author', 'Ada');
  return store;
}

/// The second Device's fixed key seed: bytes 33 to 64.
final _joinerSeed = Uint8List.fromList(List<int>.generate(32, (i) => 33 + i));

/// The settings of [store] that a sync reads: the keys it holds, its
/// Devices' and its Areas', and its own Profile (ADR 0017).
List<List<String>> _keySettings(Store store) => [
  for (final prefix in ['key:', 'areakey:', 'profile'])
    for (final (key, value) in store.localSettingsWithPrefix(prefix))
      [key, value],
];

/// Writes [store]'s log and its blobs under [dir]/[name].
void _writeDevice(Store store, Directory dir, String name) {
  final log = StringBuffer();
  for (final entry in store.allEntries()) {
    log.writeln(jsonEncode(entry.toJson()));
  }
  _write(File('${dir.path}/$name.jsonl'), log.toString());
  _writeBlobs(store, Directory('${dir.path}/$name'));
}

/// The joiner [a] and the host [b] sync, blobs included.
void _meet(Store a, Store b) {
  final done = answerSync(
    b,
    syncRequest(a, b.versionVector(), name: 'Phone'),
    name: 'Desk',
  )!;
  applyAnswer(a, done.answer);
  for (final ref in a.missingBlobs()) {
    final (kind, hash) = splitBlobRef(ref);
    if (b.blob(kind, hash) case final bytes?) a.putBlob(kind, hash, bytes);
  }
  for (final ref in (done.answer['wantBlobs'] as List).cast<String>()) {
    final (kind, hash) = splitBlobRef(ref);
    if (a.blob(kind, hash) case final bytes?) b.putBlob(kind, hash, bytes);
  }
}

/// Two Devices that met once and edited the same things offline since,
/// then a LAN session between them that finds the Conflicts, the host
/// picking a version of each, and a second session that carries the
/// picks over. Everything after the meeting is written at a held clock,
/// so the desktop core writes the same picks.
void _conflictsFixture(Directory root) {
  final out = Directory('${root.path}/conflicts');
  final host = _store();
  final joiner = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ada');
  try {
    final milk = Notes(host).capture('Buy milk');
    final mum = Notes(joiner).capture('Call mum');
    final house = Places(host).addArea('Household');
    final kitchen = Places(host).addTopic(house.id, 'Kitchen');
    final pills = Routines(host).create(
      'Pills',
      Schedule(
        start: Day(2026, 1, 5),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    _meet(joiner, host);
    // Offline on both: the same text, the same Area name and the same
    // schedule changed apart; the same value written on both; a Topic
    // renamed on one side only.
    Notes(joiner).edit(milk.id, 'Buy oat milk');
    Notes(host).edit(milk.id, 'Buy milk and eggs');
    Places(host).rename(house.id, 'House');
    Places(joiner).rename(house.id, 'Home');
    Routines(host).reschedule(
      pills.id,
      Schedule(
        start: Day(2026, 1, 5),
        times: [ClockTime(9, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    Routines(joiner).reschedule(
      pills.id,
      Schedule(
        start: Day(2026, 1, 5),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO,TH'),
      ),
    );
    Notes(host).edit(mum.id, 'Call mum tonight');
    Notes(joiner).edit(mum.id, 'Call mum tonight');
    Places(joiner).rename(kitchen.id, 'Cooking');

    _writeDevice(host, out, 'host');
    _writeDevice(joiner, out, 'joiner');
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert({'host': _keySettings(host), 'joiner': _keySettings(joiner)})}\n',
    );
    _held = _tick.add(const Duration(days: 1));
    final hostVector = host.versionVector();
    final request = syncRequest(joiner, hostVector, name: 'Phone');
    final done = answerSync(host, request, name: 'Desk')!;
    _write(File('${out.path}/vector.json'), '${_json.convert(hostVector)}\n');
    _write(File('${out.path}/request.json'), '${_json.convert(request)}\n');
    _write(File('${out.path}/answer.json'), '${_json.convert(done.answer)}\n');
    final back = applyAnswer(joiner, done.answer);
    final found = _conflicts(host.conflicts());
    if (jsonEncode(found) != jsonEncode(_conflicts(joiner.conflicts())) ||
        jsonEncode(found) != jsonEncode(_conflicts(done.session.conflicts)) ||
        jsonEncode(found) != jsonEncode(_conflicts(back.conflicts))) {
      throw StateError('the two Devices of the conflicts fixture disagree');
    }
    // The host keeps the older version of each.
    final mark = host.currentSeq();
    final picks = [
      for (final c in host.conflicts())
        [c.entity, c.unit, c.versions.length - 1],
    ];
    for (final [entity, unit, index] in picks) {
      final c = host.conflictOn(entity as String, unit as String)!;
      host.resolveConflict(c, c.versions[index as int]);
    }
    final picked = StringBuffer();
    for (final e in host.entriesAfter(mark)) {
      picked.writeln(jsonEncode(e.toJson()));
    }
    _write(File('${out.path}/picks.jsonl'), picked.toString());
    _meet(joiner, host);
    final expected = _expected(host)..remove('device');
    final joined = _expected(joiner)..remove('device');
    if (jsonEncode(expected) != jsonEncode(joined) ||
        host.conflicts().isNotEmpty) {
      throw StateError('the picks of the conflicts fixture did not land');
    }
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({'now': _held?.toIso8601String(), 'conflicts': found, 'picks': picks, 'after': expected})}\n',
    );
  } finally {
    host.close();
    joiner.close();
  }
}

/// The keys [store] holds, the own Profile's backup key among them (ADR
/// 0016).
List<List<String>> _keysWithBackup(Store store) => [
  ..._keySettings(store),
  ..._backupSettings(store),
];

/// The backup key [store] holds, with its sealed copy and version.
List<List<String>> _backupSettings(Store store) => [
  for (final key in ['backup:key', 'backup:pass', 'backup:version'])
    if (store.localSetting(key) case final value?) [key, value],
];

/// Gives [store] the backup key of bytes counted from [from], with a
/// sealed copy that is only bytes as well: what travels, not what opens.
void _plantBackupKey(Store store, int from) => store.keepBackupKeys(
  Uint8List.fromList(List<int>.generate(32, (i) => from + i)),
  BackupPass(
    const BackupKdf(memory: 64, iterations: 1),
    Uint8List.fromList(List<int>.generate(16, (i) => from + 100 + i)),
    Uint8List.fromList(List<int>.generate(72, (i) => from + 50 + i)),
  ),
  version: 1,
);

/// Two Devices that met once and wrote more since, then one more LAN
/// session between them: the logs and keys before it, the joiner's
/// request, the host's answer, and what both read after it.
void _sync(Directory root) {
  final out = Directory('${root.path}/sync');
  final host = _store();
  final joiner = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ada');
  try {
    // Met once: both planted some of the Starter Set, the joiner a Note
    // with a voice recording, the host one with a photo.
    Places(host).plantStarterSet({'work', 'meetings'});
    Places(joiner).plantStarterSet({'work', 'health', 'doctor'});
    final milk = Notes(joiner).capture('Buy milk', recordings: [_recording(2)]);
    final dentist = Notes(host).capture(
      'Call the dentist',
      photos: [_photo(5)],
      place: Places.starterId(PlaceKeys.typeTopic, 'meetings'),
    );
    _meet(joiner, host);
    // Since then: edits on both sides, one of them on the same field.
    Notes(joiner).edit(milk.id, 'Buy oat milk');
    Notes(host).edit(milk.id, 'Buy milk and eggs');
    Places(joiner)
        .rename(Places.starterId(PlaceKeys.typeArea, 'work'), 'Office');
    Notes(joiner).capture('Pills at eight', photos: [_photo(6)]);
    Notes(host).amend(dentist.id, 'Tuesday works');
    // Each made a Profile meanwhile, the host's named by the user, the
    // joiner's still the defaults: the joiner takes the host's over, and
    // with it the host's backup key in place of its own.
    host
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ada');
    joiner.applyProfileDefaults(name: 'Pixel 7');
    _plantBackupKey(host, 41);
    _plantBackupKey(joiner, 73);
    // The joiner hosts too: it names its certificate, which the host
    // keeps once the joiner's proof holds.
    joiner.hostIdentity(random: _counted(6000));

    _writeDevice(host, out, 'host');
    _writeDevice(joiner, out, 'joiner');
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert({
        'host': _keysWithBackup(host),
        'joiner': [
          ..._keysWithBackup(joiner),
          for (final key in [HostIdentities.certKey, HostIdentities.keyKey])
            if (joiner.localSetting(key) case final value?) [key, value],
        ],
      })}\n',
    );
    final hostVector = host.versionVector();
    final request = syncRequest(
      joiner,
      hostVector,
      name: 'Phone',
      hostDevice: host.deviceId,
      pin: '246810',
    );
    final done = answerSync(host, request, name: 'Desk', pin: '246810')!;
    if (done.session.fingerprint != hex(joiner.hostIdentity().fingerprint)) {
      throw StateError('the host of the sync fixture lost the certificate');
    }
    _write(File('${out.path}/vector.json'), '${_json.convert(hostVector)}\n');
    _write(File('${out.path}/request.json'), '${_json.convert(request)}\n');
    _write(File('${out.path}/answer.json'), '${_json.convert(done.answer)}\n');
    applyAnswer(joiner, done.answer);
    _write(
      File('${out.path}/backup_keys.json'),
      '${_json.convert({'host': _backupSettings(host), 'joiner': _backupSettings(joiner)})}\n',
    );
    if (host.ownProfileCode != joiner.ownProfileCode ||
        joiner.recoveryCode != host.recoveryCode) {
      throw StateError('the joiner of the sync fixture kept its Profile');
    }
    _write(
      File('${out.path}/profile.json'),
      '${_json.convert({'host': _profileSettings(host), 'joiner': _profileSettings(joiner)})}\n',
    );
    _held = _tick.add(const Duration(days: 1));
    final expected = _expected(host)..remove('device');
    final joined = _expected(joiner)..remove('device');
    if (jsonEncode(expected) != jsonEncode(joined)) {
      throw StateError('the two Devices of the sync fixture disagree');
    }
    _write(File('${out.path}/expected.json'), '${_json.convert(expected)}\n');
  } finally {
    host.close();
    joiner.close();
  }
}

/// The own Profile's settings of [store], as a map.
Map<String, String> _profileSettings(Store store) => {
  for (final (key, value) in store.localSettingsWithPrefix('profile'))
    key: value,
};

/// Counted bytes for keys and nonces, so the folder comes out the same
/// on every run: the [n]th call gives `n, n+1, …` from a start of its
/// own.
Uint8List Function(int) _counted(int start) {
  var n = start;
  return (length) {
    n++;
    return Uint8List.fromList([
      for (var i = 0; i < length; i++) (n * 37 + i) & 0xff,
    ]);
  };
}

/// Writes [folder] as it is under [out], one file per file.
void _writeFolder(MemorySyncFolder folder, Directory out) {
  for (final MapEntry(key: dir, value: files) in folder.dirs.entries) {
    for (final MapEntry(key: name, value: bytes) in files.entries) {
      final file = File('${out.path}/${dir.isEmpty ? '' : '$dir/'}$name');
      file.parent.createSync(recursive: true);
      file.writeAsBytesSync(bytes);
    }
  }
}

/// Two Devices that met once over LAN, so the joiner holds the host's
/// keys; the host writes more, a new Area among it, and syncs through
/// the shared folder: the logs and keys before, the folder the host
/// wrote with counted nonces, and what the joiner reads from it.
Future<void> _folder(Directory root) async {
  final out = Directory('${root.path}/folder');
  final keep = secretBytes;
  secretBytes = _counted(100);
  final host = _store();
  final joiner = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ada');
  try {
    final house = Places(host).addArea('Household');
    final kitchen = Places(host).addTopic(house.id, 'Kitchen');
    Notes(host).capture('Buy milk', place: kitchen.id);
    Notes(joiner).capture('Call mum');
    // One Profile since they met, renamed on the host since.
    host
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ada');
    joiner.applyProfileDefaults(name: 'Pixel 7');
    _meet(joiner, host);
    host.renameOwnProfile('Ada Lovelace');
    // Since: a new Area whose key goes only through the folder, Notes
    // in and out of Areas, a photo and a recording, a Note moved, an
    // Amend, a Routine and a Named Place.
    final doctor = Places(host).addArea('Doctor');
    final blood = Notes(host)
        .capture('Blood test on Friday', place: doctor.id, photos: [_photo(7)]);
    Notes(host).amend(blood.id, 'Fasting from midnight');
    final loose = Notes(host)
        .capture('nees mor sauce', recordings: [_recording(3)]);
    Notes(host).file(loose.id, kitchen.id);
    Notes(host).edit(loose.id, 'Needs more sauce');
    Notes(host).capture('Unsorted thought');
    Routines(host).create(
      'Pills',
      Schedule(start: Day(2026, 1, 5), times: [ClockTime(8, 0)]),
    );
    NamedPlaces(host).add('Home', Coordinates(52.52, 13.405), radius: 150);
    host.ensureAreaKeys();
    // Backups set up on the host: its keys file carries the backup key.
    _plantBackupKey(host, 97);

    _writeDevice(host, out, 'host');
    _writeDevice(joiner, out, 'joiner');
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert({'host': _keysWithBackup(host), 'joiner': _keysWithBackup(joiner)})}\n',
    );
    final folder = MemorySyncFolder();
    final nonces = _counted(1000);
    final wrote = await folderSync(
      host,
      folder,
      nonce: () => nonces(sealNonceBytes),
    );
    _writeFolder(folder, Directory('${out.path}/$syncFolderRoot'));
    _held = _tick.add(const Duration(days: 1));
    final read = await folderSync(joiner, folder);
    final expected = _expected(joiner)..remove('device');
    final written = _expected(host)..remove('device');
    if (jsonEncode(expected) != jsonEncode(written) ||
        read.unreadable != 0 ||
        joiner.ownProfile?.name != 'Ada Lovelace' ||
        joiner.recoveryCode != host.recoveryCode) {
      throw StateError('the joiner of the folder fixture did not read it all');
    }
    _write(
      File('${out.path}/profile.json'),
      '${_json.convert({'joiner': _profileSettings(joiner)})}\n',
    );
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({'entriesOut': wrote.entriesOut, 'blobsOut': wrote.blobsOut, 'applied': read.applied.length, 'blobsIn': read.blobsIn, 'backupKey': _backupSettings(joiner), 'read': expected})}\n',
    );
  } finally {
    secretBytes = keep;
    host.close();
    joiner.close();
  }
}

/// Two Devices that met once over LAN; the host writes more, a new Area
/// among it, and writes a bundle for the joiner with counted nonces: the
/// logs, keys and what the host was told the joiner holds before, the
/// bundle, and what the joiner reads from it.
Future<void> _bundle(Directory root) async {
  final out = Directory('${root.path}/bundle')..createSync(recursive: true);
  final keep = secretBytes;
  secretBytes = _counted(200);
  final host = _store();
  final joiner = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ada');
  try {
    final house = Places(host).addArea('Household');
    Notes(host).capture('Buy milk', place: house.id);
    Notes(joiner).capture('Call mum');
    _meet(joiner, host);
    // Since: what the bundle carries, and nothing from before.
    final doctor = Places(host).addArea('Doctor');
    Notes(
      host,
    ).capture('Blood test on Friday', place: doctor.id, photos: [_photo(11)]);
    Notes(host)
        .capture('Hum this', place: house.id, recordings: [_recording(5)]);
    Notes(host).capture('Unsorted thought');
    host.ensureAreaKeys();
    // Each made a Profile since and named it: "Which one is you?" waits
    // on the joiner.
    host
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ada');
    joiner
      ..applyProfileDefaults(name: 'Pixel 7')
      ..renameOwnProfile('Ada L.');

    _writeDevice(host, out, 'host');
    _writeDevice(joiner, out, 'joiner');
    List<List<String>> settings(Store store) => [
      ..._keySettings(store),
      for (final (key, value) in store.localSettingsWithPrefix('held:'))
        [key, value],
    ];
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert({'host': settings(host), 'joiner': settings(joiner)})}\n',
    );
    final nonces = _counted(2000);
    final file = '${out.path}/host.$bundleExtension';
    final wrote = await writeBundle(
      host,
      file,
      forDevice: joiner.deviceId,
      nonce: () => nonces(sealNonceBytes),
    );
    _held = _tick.add(const Duration(days: 1));
    final read = await importBundle(joiner, file);
    final expected = _expected(joiner)..remove('device');
    final written = _expected(host)..remove('device');
    if (wrote == null ||
        jsonEncode(expected) != jsonEncode(written) ||
        read.unreadable != 0 ||
        joiner.profileQuestion?.profile.name != 'Ada') {
      throw StateError('the joiner of the bundle fixture did not read it all');
    }
    _write(
      File('${out.path}/profile.json'),
      '${_json.convert({'joiner': _profileSettings(joiner)})}\n',
    );
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({'entries': wrote.entries, 'blobs': wrote.blobs, 'applied': read.applied.length, 'blobsIn': read.blobsIn, 'held': joiner.localSetting('held:${host.deviceId}'), 'read': expected})}\n',
    );
  } finally {
    secretBytes = keep;
    host.close();
    joiner.close();
  }
}

/// The new Device's fixed key seed in the backup fixture: bytes 65 to 96.
final _restoredSeed = Uint8List.fromList(List<int>.generate(32, (i) => 65 + i));

/// Ada's Device with Areas, a Shared Area, Notes with a photo and a
/// recording longer than a chunk, a Routine checked off, a Moment,
/// another Device's key and her Profile sets Backups up with the
/// standard cost and writes a Backup with counted random bytes; a new
/// Device restores it with the passphrase at a held clock, her Profile
/// with it.
Future<void> _backupFixture(Directory root) async {
  final out = Directory('${root.path}/backup')..createSync(recursive: true);
  final keep = secretBytes;
  secretBytes = _counted(600);
  final ada = _store();
  final restored = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_restoredSeed),
  );
  try {
    final house = Places(ada).addArea('Household');
    final mark = ada.currentSeq();
    final milk = Notes(ada).capture(
      'Buy milk',
      place: house.id,
      photos: [_photo(13)],
      recordings: [
        Uint8List.fromList(List.generate(140000, (i) => (i * 7 + 3) & 0xff)),
      ],
    );
    ada.addMoment(seq: mark, cause: MomentCause.sync, label: 'Desk');
    Notes(ada).amend(milk.id, 'Oat milk, "the green one"');
    final pills = Routines(ada).create(
      'Pills',
      Schedule(start: Day(2026, 1, 5), times: [ClockTime(8, 0)]),
    );
    Routines(ada)
        .markDone(pills.id, Occurrence(Day(2026, 1, 5), ClockTime(8, 0)));
    final family = Places(ada).addArea('Family');
    ada.ensureAreaKeys();
    ada.share(family.id);
    final joiner = Store.inMemory(
      newKey: () => SigningKey.fromSeed(_joinerSeed),
    );
    ada.learnKeys([joiner.ownKeyRecord], verifiedDevice: joiner.deviceId);
    joiner.close();
    // Her Profile, named by her, goes into the Backup and comes back.
    ada
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ada');
    // The one Device setting a Backup carries.
    ada.playbackSpeed = 1.5;
    const passphrase = 'Grüße an die Katze';
    final code = await setUpBackups(ada, passphrase);
    final moment = ada.momentRows().single;

    _writeDevice(ada, out, 'device');
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert({
        'device': [
          ..._keySettings(ada),
          for (final (key, value) in ada.localSettingsWithPrefix('backup:')) [key, value],
          ['playbackSpeed', '${ada.playbackSpeed}'],
        ],
      })}\n',
    );
    _write(
      File('${out.path}/moments.json'),
      '${_json.convert([
        {'seq': moment.seq, 'cause': moment.cause, 'label': moment.label, 'at': moment.at.toIso8601String()},
      ])}\n',
    );
    _held = _tick.add(const Duration(days: 1));
    secretBytes = _counted(5000);
    final file = '${out.path}/backup.$backupExtension';
    final counts = await writeBackup(ada, file);

    final header = readBackupHeader(file);
    if (header == null) throw StateError('the backup fixture has no header');
    final byPass = await unlockBackup(header, passphrase: passphrase);
    final byCode = await unlockBackup(header, recoveryCode: code);
    if (byPass == null || byCode == null || hex(byPass) != hex(byCode)) {
      throw StateError('the backup fixture does not open');
    }
    _held = _held!.add(const Duration(days: 1));
    final result = await restoreBackup(restored, file, byPass);
    final log = StringBuffer();
    for (final entry in restored.allEntries()) {
      log.writeln(jsonEncode(entry.toJson()));
    }
    _write(File('${out.path}/restored.jsonl'), log.toString());
    final expected = _expected(restored)..remove('device');
    final before = _expected(ada)..remove('device');
    before['now'] = expected['now'];
    if (result.roles != 1 ||
        jsonEncode(expected['notes']) != jsonEncode(before['notes']) ||
        restored.ownProfileCode != ada.ownProfileCode) {
      throw StateError('the backup fixture did not restore it all');
    }
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({
        'passphrase': passphrase,
        'recoveryCode': code,
        'backupKey': hex(byPass),
        'created': header.created.toIso8601String(),
        'counts': {'entries': counts.entries, 'moments': counts.moments, 'keys': counts.keys, 'pinned': counts.pinned, 'blobs': counts.blobs},
        'restoredAt': restored.clock().toIso8601String(),
        'roles': result.roles,
        'moments': [
          for (final m in restored.momentRows()) {'seq': m.seq, 'cause': m.cause, 'label': m.label, 'at': m.at.toIso8601String()},
        ],
        'settings': [
          for (final (key, value) in restored.localSettingsWithPrefix(''))
            if (key != 'device' && key != 'signseed') [key, value],
        ],
        'read': expected,
      })}\n',
    );
  } finally {
    secretBytes = keep;
    ada.close();
    restored.close();
  }
}

/// Ada sends a Note with a Section, a Routine and a Reminder to Ben,
/// once signed only and once encrypted to his X25519 key, with counted
/// random bytes; Ben opens both and accepts. The send and the accept
/// are at a held clock with counted ids, so the desktop core writes the
/// same entries and files.
void _sentFixture(Directory root) {
  final out = Directory('${root.path}/sent')..createSync(recursive: true);
  final ada = _store();
  final ben = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ben');
  try {
    final note = Notes(ada).capture('Dentist on Friday, "Dr. Müller"');
    Notes(ada).amend(note.id, 'Bring the card\nand the form');
    Routines(ada).spawn(
      note.id,
      Schedule(start: Day(2026, 1, 9), times: [ClockTime(9, 30)]),
    );
    Routines(ada).spawn(
      note.id,
      Schedule(
        start: Day(2026, 1, 2),
        times: [ClockTime(8, 0), ClockTime(20, 0)],
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO,TH'),
      ),
    );
    _write(File('${out.path}/sender.jsonl'), _log(ada));
    // Her Profile, as the app makes it at its first start, travels in
    // every file she sends (#51).
    ada.applyProfileDefaults(name: 'Ada');
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert([
        for (final (key, value) in ada.localSettingsWithPrefix('profile')) [key, value],
      ])}\n',
    );
    final adaId = ada.ownProfile!.id;

    var ids = 0x300;
    Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
    _held = DateTime.utc(2026, 1, 3, 12);
    var mark = ada.currentSeq();
    final item = sentItemOf(ada, note.id);
    _writeAfter(ada, mark, File('${out.path}/sent.jsonl'));
    final signed = encodeSentItem(item, ada.signingKey);
    final box = ben.ownKeyRecord.boxKey!;
    final sealed = encodeSentItem(
      item,
      ada.signingKey,
      to: [box],
      random: _counted(4000),
    );
    File('${out.path}/signed.$sentItemExtension').writeAsBytesSync(signed);
    File('${out.path}/sealed.$sentItemExtension').writeAsBytesSync(sealed);

    final fromSigned = openSentItem(ben, signed);
    final fromSealed = openSentItem(ben, sealed);
    if (!fromSealed.encrypted ||
        fromSealed.item.encode() != fromSigned.item.encode()) {
      throw StateError('Ben did not read the same Sent Item twice');
    }
    // Ben does not know Ada yet: her item is held unseen until "Show"
    // (#91), once for both files, and accepted from there.
    ids = 0x3f0;
    _held = DateTime.utc(2026, 1, 4, 17);
    final held = receiveSentItem(ben, sealed);
    if (held is! HeldSentItem) throw StateError('Ben did not hold it');
    final again = receiveSentItem(ben, signed);
    if (again is! HeldSentItem || again.id != held.id) {
      throw StateError('Ben did not hold it once');
    }
    _write(File('${out.path}/held.json'), ben.pendingSet(held.id)!.encode());
    if (showHeldSentItem(ben, held.id)?.item.encode() !=
        fromSigned.item.encode()) {
      throw StateError('Ben does not see what is held');
    }
    ids = 0x400;
    _held = DateTime.utc(2026, 1, 4, 18);
    mark = ben.currentSeq();
    final accepted =
        acceptHeldSentItem(ben, held.id) ??
        (throw StateError('Ben did not accept it'));
    if (ben.pendingSets.isNotEmpty) {
      throw StateError('the held Sent Item outlived its Accept');
    }
    _writeAfter(ben, mark, File('${out.path}/accepted.jsonl'));
    final opened = fromSigned.item;
    final first = {
      'note': _note(Notes(ben).byId(accepted)!),
      'routines': [
        for (final r in Routines(
          ben,
        ).ofNote(accepted)..sort((a, b) => a.id.compareTo(b.id)))
          _routine(r),
      ],
    };
    final acceptedAgain = openSentItem(ben, signed);
    final keptFirst = _personSettings(ben);
    final authorFirst = ben.authorOf(accepted)?.id;

    // Ben makes it his own: his text, an Amend, an Area, an Importance,
    // a Done mark and a Reminder of his.
    ids = 0x500;
    _held = DateTime.utc(2026, 1, 5, 9);
    mark = ben.currentSeq();
    Notes(ben).edit(accepted, 'Dentist on Friday (Dr. Müller)');
    Notes(ben).amend(accepted, 'Ask about the bill');
    final health = Places(ben).addArea('Health');
    Notes(ben).file(accepted, health.id);
    Notes(ben).setImportance(accepted, Importance.important);
    final weekly = Routines(ben)
        .ofNote(accepted)
        .firstWhere((r) => !r.isReminder);
    Routines(ben)
        .markDone(weekly.id, Occurrence(Day(2026, 1, 5), ClockTime(8, 0)));
    Routines(ben).spawn(
      accepted,
      Schedule(start: Day(2026, 1, 7), times: [ClockTime(18, 0)]),
    );
    _writeAfter(ben, mark, File('${out.path}/receiver.jsonl'));

    // Ada changes the text, amends, moves the Reminder from Friday 09:30
    // to Thursday 14:00, takes the weekly Routine away and adds a daily
    // one, then sends the update, signed only.
    ids = 0x600;
    _held = DateTime.utc(2026, 1, 6, 10);
    mark = ada.currentSeq();
    Notes(ada).edit(note.id, 'Dentist on Thursday, "Dr. Müller"');
    Notes(ada).amend(note.id, 'Room 3 now');
    final adaRoutines = Routines(ada).ofNote(note.id);
    Routines(ada).reschedule(
      adaRoutines.firstWhere((r) => r.isReminder).id,
      Schedule(start: Day(2026, 1, 8), times: [ClockTime(14, 0)]),
    );
    Routines(ada).trash(adaRoutines.firstWhere((r) => !r.isReminder).id);
    Routines(ada).spawn(
      note.id,
      Schedule(
        start: Day(2026, 1, 6),
        times: [ClockTime(7, 30)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    _writeAfter(ada, mark, File('${out.path}/changed.jsonl'));
    // She renames herself: the update brings her newer Profile.
    ada.renameOwnProfile('Ada Lovelace');
    _held = DateTime.utc(2026, 1, 6, 11);
    mark = ada.currentSeq();
    final update = encodeSentItem(sentItemOf(ada, note.id), ada.signingKey);
    _writeAfter(ada, mark, File('${out.path}/update.jsonl'));
    File('${out.path}/update.$sentItemExtension').writeAsBytesSync(update);

    // Ben applies it.
    ids = 0x700;
    _held = DateTime.utc(2026, 1, 7, 8);
    final toUpdate = openSentItem(ben, update);
    final changes =
        toUpdate.changes ?? (throw StateError('Ben sees no update'));
    mark = ben.currentSeq();
    // He compared her short code in person before applying it.
    acceptSentItem(ben, toUpdate.item, verified: true);
    final keptUpdated = _personSettings(ben);
    _writeAfter(ben, mark, File('${out.path}/updated.jsonl'));
    final updated = _received(ben, accepted);
    final stale = openSentItem(ben, signed).state.name;

    // Ada cancels it; Ben accepts the cancellation.
    _held = DateTime.utc(2026, 1, 8, 7);
    mark = ada.currentSeq();
    final cancel = encodeSentItem(
      sentItemOf(ada, note.id, cancel: true),
      ada.signingKey,
    );
    _writeAfter(ada, mark, File('${out.path}/cancel.jsonl'));
    File('${out.path}/cancel.$sentItemExtension').writeAsBytesSync(cancel);
    ids = 0x800;
    _held = DateTime.utc(2026, 1, 8, 12);
    final toCancel = openSentItem(ben, cancel);
    if (ben.authorOf(accepted)?.name != 'Ada Lovelace') {
      throw StateError('Ben does not see her newer Profile');
    }
    mark = ben.currentSeq();
    acceptSentItem(ben, toCancel.item);
    _writeAfter(ben, mark, File('${out.path}/cancelled.jsonl'));

    // Eve signs a cancellation of Ada's id with her own key.
    final eve = Store.inMemory(
      clock: _clock,
      newKey: () => SigningKey.fromSeed(Uint8List(32)..fillRange(0, 32, 9)),
    )..applyProfileDefaults(name: 'Eve');
    final forged = encodeSentItem(
      SentItem(
        id: opened.id,
        version: 9,
        cancel: true,
        from: eve.ownKeyRecord,
        profile: SignedProfile.fromJson(eve.ownProfileForOthers)!,
        text: 'Cancelled',
      ),
      eve.signingKey,
    );
    File('${out.path}/forged.$sentItemExtension').writeAsBytesSync(forged);
    var refused = false;
    try {
      openSentItem(ben, forged);
    } on SentItemNotFromSender {
      refused = true;
    }
    // Eve signs a Note of her own with Ada's Profile in it.
    final impostor = encodeSentItem(
      SentItem(
        id: 'f' * 32,
        version: 1,
        from: eve.ownKeyRecord,
        profile: SignedProfile.fromJson(ada.ownProfileForOthers)!,
        text: 'Hello, it is Ada',
      ),
      eve.signingKey,
    );
    File('${out.path}/impostor.$sentItemExtension').writeAsBytesSync(impostor);
    try {
      openSentItem(ben, impostor);
      refused = false;
    } on NotASentItem {
      // As it must be.
    }
    final elsewhere = openSentItem(eve, cancel).state.name;
    eve.close();
    if (!refused) throw StateError('Ben took a forged file');

    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({
        'box': base64.encode(box),
        'item': {
          'id': opened.id,
          'version': opened.version,
          'device': opened.from.device,
          'text': opened.text,
          'sections': [
            for (final s in opened.sections) {'text': s.text, 'captured': s.captured.toIso8601String()},
          ],
          'routines': [
            for (final r in opened.routines) {
                'id': r.id,
                'start': '${r.schedule.start}',
                'times': [for (final t in r.schedule.times) '$t'],
                'rule': r.schedule.rule?.format(),
              },
          ],
        },
        'sender': {'id': adaId, 'name': fromSigned.sender.name, 'code': fromSigned.code},
        'trust': fromSigned.trust.name,
        'people': keptFirst,
        'author': authorFirst,
        'state': fromSigned.state.name,
        'accepted': accepted,
        ...first,
        'acceptedAgain': acceptedAgain.accepted,
        'stateAgain': acceptedAgain.state.name,
        'held': {'id': held.id, 'sender': held.sender.name, 'at': held.held.toIso8601String()},
        'update': {
          'version': toUpdate.item.version,
          'state': toUpdate.state.name,
          'changes': {
            'textBefore': changes.textBefore,
            'textEdited': changes.textEdited,
            'sectionsAdded': [for (final s in changes.sectionsAdded) s.text],
            'rescheduled': [
              for (final r in changes.rescheduled) {'routine': r.routine, 'before': _schedule(r.before), 'after': _schedule(r.after)},
            ],
            'routinesAdded': [for (final r in changes.routinesAdded) r.id],
            'routinesRemoved': [for (final r in changes.routinesRemoved) r.id],
            'restores': changes.restores,
          },
          ...updated,
          'sender': toUpdate.sender.name,
          'trust': toUpdate.trust.name,
          'people': keptUpdated,
          'stale': stale,
        },
        'cancel': {'version': toCancel.item.version, 'state': toCancel.state.name, 'trust': toCancel.trust.name, ..._received(ben, accepted), 'elsewhere': elsewhere, 'forged': 'notFromSender', 'impostor': 'notASentItem'},
      })}\n',
    );
  } finally {
    ada.close();
    ben.close();
  }
}

/// The Profiles of other people [store] keeps, as its `person:`
/// settings.
List<List<String>> _personSettings(Store store) => [
  for (final (key, value) in store.localSettingsWithPrefix('person:'))
    [key, value],
];

/// What [store] holds of the accepted Note [id]: the Note, its
/// Sections and every Routine and Reminder of it, those in the trash too.
Map<String, Object?> _received(Store store, String id) => {
  'note': _note(Notes(store).byId(id)!),
  'sections': [for (final s in Notes(store).sections(id)) _section(s)],
  'routines': [
    for (final r in store.entitiesOfType(RoutineKeys.typeRoutine)..sort())
      if (Routines(store).byId(r) case final routine? when routine.noteId == id)
        _routine(routine),
  ],
};

Map<String, Object?> _schedule(Schedule s) => {
  'start': '${s.start}',
  'times': [for (final t in s.times) '$t'],
  'rule': s.rule?.format(),
};

/// What the roles of [area] come to on [store]: this Device's access,
/// the people, what it may change and every entry no role allows.
Map<String, Object?> _roles(Store store, String area, List<String> ids) => {
  'access': store.accessIn(area).name,
  'shared': store.isShared(area),
  'people': [
    for (final p in store.people(area))
      {
        'id': p.id,
        'name': store.personShown(p.id).name,
        'role': p.role.name,
        'devices': p.devices,
      },
  ],
  'authors': {for (final id in ids) id: store.authorOf(id)?.id},
  'mayChange': {for (final id in ids) id: store.mayChange(id)},
  'mayManage': store.mayManage(area),
  'ignored': store.judged.ignored.toList()..sort(),
  'routines': [
    for (final id in store.entitiesOfType(RoutineKeys.typeRoutine)..sort())
      if (Routines(store).byId(id) case final r?) _routine(r),
  ],
};

/// Writes the entries [store] holds after the mark [seq] to [file].
void _writeAfter(Store store, int seq, File file) {
  final out = StringBuffer();
  for (final e in store.entriesAfter(seq)) {
    out.writeln(jsonEncode(e.toJson()));
  }
  _write(file, out.toString());
}

/// Ann shares her Household, a Starter Set Area, with Ben as a Member and
/// keeps her Health and Unsorted Notes private; Ben writes what a Member
/// may and, as a modified app would, what not; Ann files a Note into the
/// Area, they sync, she makes Ben a Manager and then removes him. Every
/// write of the sharing itself, and every session, is at a held clock so
/// the desktop core writes the same.
Future<void> _sharedFixture(Directory root) async {
  final out = Directory('${root.path}/shared');
  final keep = secretBytes;
  secretBytes = _counted(300);
  final ann = _store();
  final ben = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ada');
  try {
    Places(ann).plantStarterSet({'household', 'kitchen', 'health'});
    final kitchen = Places.starterId(PlaceKeys.typeTopic, 'kitchen');
    final health = Places.starterId(PlaceKeys.typeArea, 'health');
    final milk = Notes(ann).capture('Buy milk', place: kitchen);
    final bins = Routines(ann).spawn(
      milk.id,
      Schedule(
        start: Day(2026, 1, 5),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    final blood = Notes(ann).capture('Blood results', place: health);
    final loose = Notes(ann).capture('Fix the tap');
    Notes(ann).amend(loose.id, 'The hot one');
    Notes(ben).capture('Ben private');
    // Their Profiles: Ann named hers, Ben's has the defaults.
    ann
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    ben.applyProfileDefaults(name: 'Ben');
    ann.ensureAreaKeys();
    ben.ensureAreaKeys();
    _write(File('${out.path}/before.jsonl'), _log(ann));
    _write(File('${out.path}/joiner.jsonl'), _log(ben));
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert({'host': _keySettings(ann), 'joiner': _keySettings(ben)})}\n',
    );

    // Ann shares the Household: fresh ids, she its first Manager.
    var ids = 0x100;
    Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
    final shareAt = DateTime.utc(2026, 2, 1, 9);
    _held = shareAt;
    var mark = ann.currentSeq();
    final area = ann.share(Places.starterId(PlaceKeys.typeArea, 'household'));
    _writeAfter(ann, mark, File('${out.path}/share.jsonl'));
    // The moved Area's key, made here so the desktop core holds the same.
    ann.ensureAreaKeys();
    final keysAfterShare = _keySettings(ann);

    // Her desktop invites Ben's phone: he sends nothing of his own but
    // his Profile, which her desktop shows before it answers.
    final sessionAt = DateTime.utc(2026, 2, 1, 10);
    _held = sessionAt;
    final invite = Invite(area, role: AreaRole.member);
    final request = syncRequest(
      ben,
      ann.versionVector(),
      name: 'Phone',
      hostDevice: ann.deviceId,
      pin: '246810',
      invited: true,
    );
    final done = answerSync(ann, request, name: 'Desk', inviting: invite)!;
    _write(File('${out.path}/request.json'), '${_json.convert(request)}\n');
    _write(File('${out.path}/answer.json'), '${_json.convert(done.answer)}\n');
    applyAnswer(ben, done.answer, shared: true);
    for (final (store, peer, name) in [
      (ann, ben.deviceId, 'Phone'),
      (ben, ann.deviceId, 'Desk'),
    ]) {
      store.rememberDevice(
        PairedDevice(
          id: peer,
          name: name,
          fingerprint: '',
          host: null,
          port: null,
          lastSync: sessionAt,
          shared: true,
        ),
      );
    }
    final afterInvite = {
      'host': _expected(ann)..remove('device'),
      'joiner': _expected(ben)..remove('device'),
      'hostPeople': _people(ann),
      'joinerPeople': _people(ben),
    };

    // Ann gives Ben a name and picture for herself alone (#52): kept in
    // her log under the personal key, never sent to Ben.
    final overrideAt = DateTime.utc(2026, 2, 1, 10, 15);
    _held = overrideAt;
    final benId = ben.ownProfile!.id;
    final overridePicture = squareProfilePicture(_gradient(40, 30))!;
    _writeBytes(File('${out.path}/override.jpg'), overridePicture);
    mark = ann.currentSeq();
    ann
      ..setOverrideName(benId, 'Benny')
      ..setOverridePicture(benId, overridePicture);
    _writeAfter(ann, mark, File('${out.path}/override.jsonl'));
    final overridden = {
      'person': benId,
      'shown': ann.personShown(benId).name,
      'theirs': ann.personTheirs(benId).name,
    };

    // Ann renames herself and syncs through a shared folder: her Profile
    // and Ben's go sealed under the Household's key, and Ben reads hers.
    final folderAt = DateTime.utc(2026, 2, 1, 10, 30);
    _held = folderAt;
    ann.renameOwnProfile('Ann Lovelace');
    final folder = MemorySyncFolder();
    final nonces = _counted(2000);
    await folderSync(ann, folder, nonce: () => nonces(sealNonceBytes));
    final profilesFile = profilesFileName(
      ann.deviceId,
      ann.areaKeys().firstWhere((k) => k.scope == area).id,
    );
    _writeBytes(
      File('${out.path}/$profilesFile'),
      (await folder.read('keys', profilesFile))!,
    );
    await folderSync(ben, folder);
    if (ben.personShown(ann.ownProfile!.id).name != 'Ann Lovelace') {
      throw StateError('Ben did not read Ann\'s Profile from the folder');
    }
    final afterFolder = {
      'profilesFile': profilesFile,
      'joinerPeople': _people(ben),
    };

    // Ben, a Member: his own Note, an Amend and a Done count; editing
    // and trashing Ann's Note, renaming the Area, rescheduling and a
    // Topic and a Routine of his own are ignored everywhere.
    _held = DateTime.utc(2026, 2, 1, 11);
    mark = ben.currentSeq();
    final topic = Places(ben).topics(area).single.id;
    final own = Notes(ben).capture('Ben buys bread', place: topic);
    Notes(ben).edit(own.id, 'Ben buys rye bread');
    Notes(ben).amend(milk.id, 'Oat milk please');
    Routines(ben).markDone(bins.id, Occurrence(Day(2026, 2, 1), _at(8)));
    Notes(ben).edit(milk.id, 'Buy beer');
    Notes(ben).trash(milk.id);
    Places(ben).rename(area, "Ben's");
    Routines(ben)
        .reschedule(bins.id, Schedule(start: Day(2026, 1, 5), times: [_at(6)]));
    ben.batch(() {
      ben.append('topic:garage', PlaceKeys.type, PlaceKeys.typeTopic);
      ben.append('topic:garage', PlaceKeys.area, area);
      ben.append('topic:garage', PlaceKeys.name, 'Garage');
      ben.append('routine:ben', RoutineKeys.type, RoutineKeys.typeRoutine);
      ben.append('routine:ben', RoutineKeys.note, own.id);
      ben.append('routine:ben', RoutineKeys.start, '2026-02-03');
      // And a grant of his own making.
      ben.append(
        area,
        ShareKeys.role(ben.deviceId),
        const Grant(AreaRole.manager, 'nobody').encode(),
      );
    });
    _writeAfter(ben, mark, File('${out.path}/member.jsonl'));

    // Ann files the Note she captured Unsorted into the Area, and writes
    // one more private one.
    _held = DateTime.utc(2026, 2, 1, 11, 30);
    mark = ann.currentSeq();
    Notes(ann).file(loose.id, area);
    Notes(ann).capture('Still private');
    _writeAfter(ann, mark, File('${out.path}/owner.jsonl'));

    // They sync as paired Devices: only the Area, the filed Note whole.
    final syncAt = DateTime.utc(2026, 2, 1, 12);
    _held = syncAt;
    final request2 = syncRequest(
      ben,
      ann.versionVector(),
      name: 'Phone',
      hostDevice: ann.deviceId,
    );
    final done2 = answerSync(ann, request2, name: 'Desk')!;
    _write(File('${out.path}/request2.json'), '${_json.convert(request2)}\n');
    _write(
      File('${out.path}/answer2.json'),
      '${_json.convert(done2.answer)}\n',
    );
    applyAnswer(ben, done2.answer, shared: true);
    final checked = [milk.id, own.id, loose.id, blood.id];
    final after = {
      'host': _expected(ann)..remove('device'),
      'joiner': _expected(ben)..remove('device'),
      'hostRoles': _roles(ann, area, checked),
      'joinerRoles': _roles(ben, area, checked),
    };
    // What no role allows of the Topic and the Routine Ben made up lies
    // nowhere for him, so it never even went to Ann.
    if (ben.allEntries().any((e) => e.entity.startsWith(OverrideKeys.prefix))) {
      throw StateError('Ann\'s Override reached Ben');
    }
    if (Notes(ben).byId(blood.id) != null ||
        Notes(ben).byId(loose.id)?.text != 'Fix the tap' ||
        Notes(ann).byId(milk.id)?.text != 'Buy milk') {
      throw StateError('the two Devices of the shared fixture disagree');
    }

    // Ann makes Ben a Manager, then removes him.
    final person = ann
        .people(area)
        .firstWhere((p) => ann.personTheirs(p.id).name == 'Ben')
        .id;
    final roleAt = DateTime.utc(2026, 2, 1, 13);
    _held = roleAt;
    mark = ann.currentSeq();
    ann.setRole(area, person, AreaRole.manager);
    _writeAfter(ann, mark, File('${out.path}/role.jsonl'));
    // Ben learns of it, then reschedules the Routine and renames the
    // Topic as a Manager: both count now, his edit as a Member still not.
    void session(DateTime at) {
      _held = at;
      final asked = syncRequest(
        ben,
        ann.versionVector(),
        name: 'Phone',
        hostDevice: ann.deviceId,
      );
      applyAnswer(
        ben,
        answerSync(ann, asked, name: 'Desk')!.answer,
        shared: true,
      );
    }

    final promoteSyncAt = DateTime.utc(2026, 2, 1, 13, 15);
    session(promoteSyncAt);
    _held = DateTime.utc(2026, 2, 1, 13, 30);
    mark = ben.currentSeq();
    Routines(ben).reschedule(
      bins.id,
      Schedule(
        start: Day(2026, 1, 5),
        times: [_at(9)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    Places(ben).rename(topic, 'Fridge');
    _writeAfter(ben, mark, File('${out.path}/manager.jsonl'));
    final managerSyncAt = DateTime.utc(2026, 2, 1, 13, 45);
    session(managerSyncAt);
    final promoted = {
      'host': _roles(ann, area, checked),
      'joiner': _roles(ben, area, checked),
    };
    if (Routines(ann).byId(bins.id)?.schedule.times.single != _at(9) ||
        Places(ann).topicById(topic)?.name != 'Fridge' ||
        Notes(ann).byId(milk.id)?.text != 'Buy milk') {
      throw StateError('the Manager of the shared fixture did not count');
    }
    final removeAt = DateTime.utc(2026, 2, 1, 14);
    _held = removeAt;
    mark = ann.currentSeq();
    ann.removePerson(area, person);
    _writeAfter(ann, mark, File('${out.path}/remove.jsonl'));
    final lastAt = DateTime.utc(2026, 2, 1, 15);
    _held = lastAt;
    final request3 = syncRequest(
      ben,
      ann.versionVector(),
      name: 'Phone',
      hostDevice: ann.deviceId,
    );
    final done3 = answerSync(ann, request3, name: 'Desk')!;
    _write(
      File('${out.path}/answer3.json'),
      '${_json.convert(done3.answer)}\n',
    );
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({'area': area, 'person': person, 'ids': 0x100, 'keysAfterShare': keysAfterShare, 'shareAt': shareAt.toIso8601String(), 'sessionAt': sessionAt.toIso8601String(), 'overrideAt': overrideAt.toIso8601String(), 'override': overridden, 'folderAt': folderAt.toIso8601String(), 'afterFolder': afterFolder, 'syncAt': syncAt.toIso8601String(), 'roleAt': roleAt.toIso8601String(), 'promoteSyncAt': promoteSyncAt.toIso8601String(), 'managerSyncAt': managerSyncAt.toIso8601String(), 'removeAt': removeAt.toIso8601String(), 'lastAt': lastAt.toIso8601String(), 'afterInvite': afterInvite, 'after': after, 'promoted': promoted, 'removed': _roles(ann, area, checked)})}\n',
    );
  } finally {
    secretBytes = keep;
    ann.close();
    ben.close();
  }
}

/// Every local setting of [store] but its own key, by key.
List<List<String>> _settings(Store store) => [
  for (final (key, value) in store.localSettingsWithPrefix(''))
    if (key != 'device' && key != 'signseed') [key, value],
];

/// What blocking left on [store]: the entries it holds, how far it tells
/// others it holds each Device, its refusal marks, its Profiles of other
/// people as kept, the people blocked and the photos it holds.
Map<String, Object?> _blockState(Store store, List<String> photos) => {
  'entries': [for (final e in store.allEntries()) e.id],
  'vector': Map.fromEntries(
    store.versionVector().entries.toList()
      ..sort((a, b) => a.key.compareTo(b.key)),
  ),
  'refused': [
    for (final (key, value) in store.localSettingsWithPrefix('refused:'))
      [key, value],
  ],
  'people': [
    for (final (key, value) in store.localSettingsWithPrefix('person:'))
      [key, value],
  ],
  'blocked': [
    for (final p in store.blockedPeople)
      {'id': p.id, 'name': p.name, 'picture': p.picture != null},
  ],
  'shown': [for (final p in store.peopleShown) p.id],
  'photos': [
    for (final hash in photos)
      if (store.photo(hash) != null) hash,
  ],
  'read': _expected(store)..remove('device'),
};

/// Ann shares her Household with Carol as a Member and joins Carol's
/// School as one; Carol writes a Note with a photo and its Photo Text,
/// Amends Ann's Note and checks off Ann's Routine, and Ann makes a
/// Reminder of Carol's Note. Ann blocks Carol, which leaves the School
/// Carol manages; Carol writes more, which Ann takes in; Ann unblocks
/// her and takes in Carol's whole log again. The block and the unblock
/// are written at a held clock.
void _blockFixture(Directory root) {
  final out = Directory('${root.path}/block');
  final keep = secretBytes;
  secretBytes = _counted(600);
  final ann = _store();
  final carol = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Ada');
  try {
    ann
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    carol
      ..applyProfileDefaults(name: 'Carol')
      ..setOwnProfilePicture(squareProfilePicture(_gradient(40, 30)));
    ann.ensureAreaKeys();
    carol.ensureAreaKeys();

    void remember(Store store, Store peer, String name) => store.rememberDevice(
      PairedDevice(
        id: peer.deviceId,
        name: name,
        fingerprint: '',
        host: null,
        port: null,
        lastSync: DateTime.utc(2026, 1, 1),
        shared: true,
      ),
    );
    void invite(Store host, Store joiner, String area) {
      final done = answerSync(
        host,
        syncRequest(
          joiner,
          host.versionVector(),
          name: 'Phone',
          hostDevice: host.deviceId,
          invited: true,
        ),
        name: 'Desk',
        inviting: Invite(area, role: AreaRole.member),
      )!;
      applyAnswer(joiner, done.answer, shared: true);
      remember(host, joiner, 'Phone');
      remember(joiner, host, 'Desk');
    }

    void session() {
      final done = answerSync(
        ann,
        syncRequest(
          carol,
          ann.versionVector(),
          name: 'Phone',
          hostDevice: ann.deviceId,
        ),
        name: 'Desk',
      )!;
      applyAnswer(carol, done.answer, shared: true);
      for (final ref in ann.missingBlobs()) {
        final (kind, hash) = splitBlobRef(ref);
        if (carol.blob(kind, hash) case final bytes?) {
          ann.putBlob(kind, hash, bytes);
        }
      }
    }

    final household = ann.share(Places(ann).addArea('Household').id);
    invite(ann, carol, household);
    final school = carol.share(Places(carol).addArea('School').id);
    invite(carol, ann, school);
    final milk = Notes(ann).capture('Buy milk', place: household);
    final bins = Routines(ann).spawn(
      milk.id,
      Schedule(
        start: Day(2026, 1, 1),
        times: [_at(8)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    session();
    final rude = Notes(carol)
        .capture('Rude words', place: household, photos: [_gradient(16, 12)]);
    final hash = rude.photos.single;
    // A Note of her own, which never leaves her Device.
    Notes(carol).capture('Carol alone');
    Notes(carol).setPhotoText(rude.id, hash, 'Rude picture');
    Notes(carol).amend(milk.id, 'Rude too');
    Routines(carol).markDone(bins.id, Occurrence(Day(2026, 1, 1), _at(8)));
    session();
    final cake = Routines(ann)
        .spawn(rude.id, Schedule(start: Day(2026, 1, 2), times: [_at(9)]));
    final carolId = carol.ownProfile!.id;
    _write(File('${out.path}/before.jsonl'), _log(ann));
    _writeBlobs(ann, Directory('${out.path}/before'));
    _write(
      File('${out.path}/settings.json'),
      '${_json.convert(_settings(ann))}\n',
    );
    final leave = ann.areasToLeaveForBlocking(carolId);

    // Ann blocks Carol, leaving the School she manages.
    final blockAt = DateTime.utc(2026, 1, 3, 9);
    _held = blockAt;
    var mark = ann.currentSeq();
    ann.blockPerson(carolId, leaveAreas: true);
    _writeAfter(ann, mark, File('${out.path}/block.jsonl'));
    final blocked = _blockState(ann, [hash]);
    // Carol learns that Ann left the School, and of nothing else.
    carol.applyEntries([
      for (final e in ann.entriesAfter(mark))
        if (e.entity == school) e,
    ]);

    // Carol writes more: a Note and a check-off; Ann takes them in.
    _held = null;
    mark = carol.currentSeq();
    Notes(carol).capture('Ruder words', place: household);
    Routines(carol).markDone(bins.id, Occurrence(Day(2026, 1, 2), _at(8)));
    _writeAfter(carol, mark, File('${out.path}/incoming.jsonl'));
    final incomingAt = DateTime.utc(2026, 1, 3, 12);
    _held = incomingAt;
    ann.applyEntries(carol.entriesAfter(mark));
    final incoming = _blockState(ann, [hash]);

    // Ann unblocks her, and Carol's Device syncs with hers: what Ann
    // still shares with Carol comes back, the School and Carol's own Note
    // do not, and nothing of Carol's is asked for again.
    final unblockAt = DateTime.utc(2026, 1, 4, 9);
    _held = unblockAt;
    mark = ann.currentSeq();
    ann.unblockPerson(carolId);
    _writeAfter(ann, mark, File('${out.path}/unblock.jsonl'));
    final unblocked = _blockState(ann, [hash]);
    // Without the Profiles, whose pictures each core makes again its
    // own way.
    final refetch = syncRequest(
      carol,
      ann.versionVector(),
      name: 'Phone',
      hostDevice: ann.deviceId,
    )..remove('profiles');
    _write(File('${out.path}/refetch.json'), '${_json.convert(refetch)}\n');
    answerSync(ann, refetch, name: 'Desk');
    final refetched = _blockState(ann, [hash]);
    final again = syncRequest(
      carol,
      ann.versionVector(),
      name: 'Phone',
      hostDevice: ann.deviceId,
    );
    if (Notes(ann).byId(rude.id)?.text != 'Rude words' ||
        !ann.missingBlobs().contains('photo/$hash') ||
        Places(ann).areaById(school)?.name != null ||
        ann.refusalMarks().isNotEmpty ||
        (again['entries']! as List).any(
          (e) => (e as Map)['device'] == carol.deviceId,
        )) {
      throw StateError('the block fixture did not fetch Carol\'s Note again');
    }
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({'carol': carolId, 'household': household, 'school': school, 'note': rude.id, 'cake': cake.id, 'photo': hash, 'leave': leave, 'blockAt': blockAt.toIso8601String(), 'incomingAt': incomingAt.toIso8601String(), 'unblockAt': unblockAt.toIso8601String(), 'blocked': blocked, 'incoming': incoming, 'unblocked': unblocked, 'refetched': refetched})}\n',
    );
  } finally {
    secretBytes = keep;
    _held = null;
    ann.close();
    carol.close();
  }
}

/// Pending Changes (ADR 0022, #111): Ann's AI Task drafts changes and a
/// bundle brings Bob's entries; Ann edits some of the same fields
/// herself meanwhile, declines a third set and accepts the two. Every
/// step at a held clock, so the desktop core can take the same steps.
void _pendingFixture(Directory root) {
  final out = Directory('${root.path}/pending');
  final ann = _store();
  final bob = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  )..setLocalSetting('author', 'Bob');
  try {
    final milk = Notes(ann).capture('Milk');
    final bread = Notes(ann).capture('Bread');
    final household = Places(ann).addArea('Household').id;
    final routine = Routines(ann).spawn(
      milk.id,
      Schedule(
        start: Day(2026, 1, 2),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    _write(File('${out.path}/before.jsonl'), _log(ann));
    bob.applyEntries(ann.entriesSince({}), keys: ann.keyRecords());
    var mark = bob.currentSeq();
    Notes(bob).edit(bread.id, 'Bread from Bob');
    Notes(bob).capture('Call Bob back');
    _writeAfter(bob, mark, File('${out.path}/received.jsonl'));
    _write(
      File('${out.path}/key.json'),
      '${_json.convert(bob.ownKeyRecord.toJson())}\n',
    );

    var ids = 0x800;
    Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
    final times = [
      for (var minute = 0; minute < 14; minute++)
        DateTime.utc(2026, 1, 2, 9, minute),
    ];
    var step = 0;
    void tick() => _held = times[step++];
    final before = ann.currentSeq();

    // The AI Task's changes, and the user's own edits among them.
    tick();
    final ai = ann.openPending('ai-task', shown: true);
    tick();
    ann.draftPending(
      ai,
      milk.id,
      NoteKeys.text,
      StoredText.plain('Oat milk').encode(),
    );
    tick();
    ann.draftPending(ai, milk.id, NoteKeys.place, household);
    tick();
    final eggs = 'note:${Store.newId()}';
    ann.draftPending(ai, eggs, NoteKeys.type, NoteKeys.typeNote);
    ann.draftPending(ai, eggs, NoteKeys.captured, _held!.toIso8601String());
    ann.draftPending(
      ai,
      eggs,
      NoteKeys.text,
      StoredText.plain('Eggs').encode(),
    );
    tick();
    ann.draftPending(ai, routine.id, RoutineKeys.rule, 'FREQ=WEEKLY');
    tick();
    Notes(ann).edit(milk.id, 'Milk, two litres');
    tick();
    ann.append(routine.id, RoutineKeys.start, '2026-01-03');

    // A bundle with Bob's entries, and an edit of the Note he edited.
    tick();
    final bundle = ann.openPending('bundle', shown: true);
    ann.receivePending(
      bundle,
      bob.entriesAfter(mark),
      keys: [bob.ownKeyRecord],
    );
    tick();
    Notes(ann).edit(bread.id, 'Brown bread');
    _writeAfter(ann, before, File('${out.path}/edits.jsonl'));
    _write(File('${out.path}/ai.json'), ann.pendingSet(ai)!.encode());
    _write(File('${out.path}/bundle.json'), ann.pendingSet(bundle)!.encode());
    tick();
    final shown = _expected(ann);
    final routineShown = ann.currentFields(routine.id);

    // A third set, declined: the log stays as it was.
    tick();
    final other = ann.openPending('ai-task', shown: true);
    ann.draftPending(
      other,
      bread.id,
      NoteKeys.text,
      StoredText.plain('Never').encode(),
    );
    ann.declinePending(other);
    final declined = _expected(ann);
    final routineDeclined = ann.currentFields(routine.id);

    List<List<String?>> kept(AcceptedChanges accepted) => [
      for (final k in accepted.kept) [k.entity, k.field, k.pending, k.kept],
    ];
    tick();
    mark = ann.currentSeq();
    final aiAccepted = ann.acceptPending(ai);
    _writeAfter(ann, mark, File('${out.path}/accept-ai.jsonl'));
    tick();
    mark = ann.currentSeq();
    final bundleAccepted = ann.acceptPending(bundle);
    _writeAfter(ann, mark, File('${out.path}/accept-bundle.jsonl'));
    tick();
    final accepted = _expected(ann);
    final routineAccepted = ann.currentFields(routine.id);
    if (jsonEncode(routineShown) != jsonEncode(routineAccepted) ||
        aiAccepted.kept.length != 2 ||
        bundleAccepted.kept.length != 1 ||
        ann.conflicts().isNotEmpty ||
        Notes(ann).byId(bread.id)?.text != 'Brown bread') {
      throw StateError('the pending fixture did not keep the own edits');
    }
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({
        'times': [for (final t in times) t.toIso8601String()],
        'ai': ai,
        'bundle': bundle,
        'other': other,
        'eggs': eggs,
        'milk': milk.id,
        'bread': bread.id,
        'routine': routine.id,
        'household': household,
        'shown': shown,
        'routineShown': routineShown,
        'declined': declined,
        'routineDeclined': routineDeclined,
        'aiKept': kept(aiAccepted),
        'bundleKept': kept(bundleAccepted),
        'taken': [for (final e in bundleAccepted.taken) e.id],
        'accepted': accepted,
        'routineAccepted': routineAccepted,
      })}\n',
    );
  } finally {
    _held = null;
    ann.close();
    bob.close();
  }
}

/// The Profiles of other people [store] keeps, as both cores keep them:
/// signed as received, by id.
Map<String, Object?> _people(Store store) => {
  for (final (key, value) in store.localSettingsWithPrefix('person:'))
    key: (jsonDecode(value) as Map)['profile'],
};

/// [store]'s log, one entry per line.
String _log(Store store) {
  final log = StringBuffer();
  for (final entry in store.allEntries()) {
    log.writeln(jsonEncode(entry.toJson()));
  }
  return log.toString();
}

/// Sealed messages, key ids and blob names the desktop core must make
/// the same.
Map<String, Object?> _crypto() {
  final bytes = _counted(500);
  final sealed = [
    for (final (aad, plain) in [
      (SealedAs.entry, 'Pills at eight'),
      (SealedAs.keys('device'), ''),
      (SealedAs.blob('photo/ab'), 'Grüße, "quotes" \\ and 🧠'),
    ])
      () {
        final key = bytes(32);
        final nonce = bytes(24);
        return {
          'key': hex(key),
          'nonce': hex(nonce),
          'aad': aad,
          'plain': plain,
          'sealed': hex(seal(key, utf8.encode(plain), aad: aad, nonce: nonce)),
        };
      }(),
  ];
  final key = bytes(32);
  final ref = 'photo/${'0f' * 32}';
  return {
    'sealed': sealed,
    'keyId': {'key': hex(key), 'id': areaKeyId(key)},
    'blobName': {'key': hex(key), 'ref': ref, 'name': blobName(key, ref)},
  };
}

/// Pair codes the desktop must write and read the same.
Map<String, Object?> _pairCodes() {
  final fp = [for (var i = 0; i < 32; i++) (i * 7 + 3) & 0xff];
  final codes = [
    for (final (host, port, pin) in [
      ('192.168.1.23', 45231, '042817'),
      ('10.0.0.5', 8080, '999999'),
      ('172.16.4.200', 1, '000000'),
    ])
      {
        'host': host,
        'port': port,
        'pin': pin,
        'fingerprint': hex(fp),
        'full': encodePairCode(host, port, pin, fp),
        'typed': encodePairCode(host, port, pin, fp, typed: true),
      },
  ];
  return {
    'codes': codes,
    'typedAs': {
      'R2M02 5XGNW 0AEG8 3188H G7S65 MTO': 'r2m02_5xgnw_0aeg8_3188h_g7s65_mt0',
    },
    'invalid': ['nope', '', 'r2m02_5xgnw_0aeg8'],
  };
}

/// The writer's key record, as it travels.
Map<String, Object?> _keys() {
  final key = SigningKey.fromSeed(_seed);
  final record = KeyRecord.make(key);
  final other = KeyRecord.make(SigningKey.fromSeed(_joinerSeed));
  return {
    'record': record.toJson(),
    // Another key claiming the writer's device: not valid.
    'claimed': KeyRecord(
      record.device,
      other.publicKey,
      other.signature,
    ).toJson(),
  };
}

/// Profiles (ADR 0017): names as typed and as kept, initials, colours,
/// short codes, records and their bytes, the own Profile as this Device
/// keeps it, and a picture made here for the desktop to take in.
void _profilesFixture(Directory root) {
  final dir = Directory('${root.path}/profiles');
  final writer = SigningKey.fromSeed(_seed).publicKey;
  final joiner = SigningKey.fromSeed(_joinerSeed).publicKey;
  const england =
      '\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}';
  final names = [
    '  Ada  ',
    'Jürgen Müller',
    '李小龍',
    'مريم',
    'Mum 👩\u200D👧 ❤\uFE0F',
    'A\u0000d\u0007a\n\tB\u007F\u0085',
    '\u202EAda\u202C \u2066Bo\u2069\u200E\u200F\u061C',
    '\u200B Ada\u2060\uFEFF\u00AD ',
    'A\u200Bd\u3164a',
    '\u200DAda\u200D',
    'A\u200D d',
    'A\u200D\u200Dd',
    'می\u200Cخواهم',
    '\uFE0FAda',
    '❤\uFE0F\uFE0F',
    'Ada $england',
    'Ada\u{E0041}\u{E0042}',
    '\u{E0001}Ada',
    'x' * 50,
    '😀' * 45,
    '${'a' * 39} bc',
    '${'a' * 39}👩\u200D👧',
    '${'a' * 36}$england',
    '',
    ' \u200B\u202E\n ',
  ];
  final picture = makeProfilePicture(
    _gradient(320, 240),
    left: 40,
    top: 0,
    size: 240,
  )!;
  _writeBytes(File('${dir.path}/picture.jpg'), picture);
  final records = [
    const Profile(id: 'p1', name: 'Ada', version: 1),
    Profile(
      id: '00000000000000000000000000000abc',
      name: 'Jürgen "Jo" Müller 👩\u200D👧',
      picture: Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xD9]),
      version: 7,
    ),
    const Profile(id: 'p2', name: '', version: 2),
  ];
  var ids = 0;
  Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
  final store = Store.inMemory(newKey: () => SigningKey.fromSeed(_seed));
  final own = <Map<String, Object?>>[];
  void step(String what) => own.add({
    'step': what,
    'settings': {
      for (final (key, value) in store.localSettingsWithPrefix('profile'))
        key: value,
    },
    'code': store.ownProfileCode,
  });
  try {
    store.applyProfileDefaults(name: ' Ada’s desk ');
    step('defaults');
    store.applyProfileDefaults(name: 'desk', picture: picture);
    step('new defaults');
    store.renameOwnProfile('\u202EAda\u200B');
    step('renamed');
    store.applyProfileDefaults(name: 'other');
    step('defaults after the rename');
    store.setOwnProfilePicture(null);
    step('placeholder chosen');
  } finally {
    store.close();
  }
  final takes = _profileTakes();
  final fixture = {
    'names': [
      for (final raw in names) {'raw': raw, 'kept': sanitizeProfileName(raw)},
    ],
    'initials': [
      for (final name in [
        'ada lovelace',
        'Ada King Lovelace',
        'Ada',
        '',
        'Élodie',
        '李小龍',
      ])
        {'name': name, 'initials': profileInitials(name)},
    ],
    'colours': [
      for (final id in ['p1', 'p2', '00000000000000000000000000000abc', 'é'])
        {'id': id, 'colour': profileColor(id)},
    ],
    'codes': [
      for (final (id, key) in [
        ('p1', writer),
        ('p1', joiner),
        ('00000000000000000000000000000001', writer),
      ])
        {
          'id': id,
          'key': base64.encode(key),
          'code': profileShortCode(id, key),
        },
    ],
    'records': [
      for (final r in records) {'payload': r.encode()},
    ],
    'refused': [
      'nope',
      '{"id":"","name":"A","version":1}',
      '{"id":"p","name":"A","version":0}',
      '{"id":"p","name":"A","picture":"%%","version":1}',
      '{"id":"p","name":"A","picture":"AQ-_","version":1}',
      '{"id":"p","name":"A","picture":"AQJ=","version":1}',
      '{"id":"p","name":"\\ud800A","version":1}',
    ],
    'own': own,
    'takes': takes,
    'people': _personTakes(),
  };
  _write(File('${dir.path}/profiles.json'), '${_json.convert(fixture)}\n');
}

/// Profiles another of the writer's own Devices (the joiner) sends
/// (#48), each taken by the writer's store as it was before: a Profile
/// named Ada with the joiner's key bound. What became of each, and the
/// own Profile's settings after.
Map<String, Object?> _profileTakes() {
  final writer = SigningKey.fromSeed(_seed);
  final joiner = SigningKey.fromSeed(_joinerSeed);
  final stranger = SigningKey.fromSeed(
    Uint8List.fromList(List<int>.filled(32, 9)),
  );
  var ids = 100;
  Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
  Store open() => Store.inMemory(newKey: () => SigningKey.fromSeed(_seed));
  final base = open();
  final Map<String, String> before;
  final Profile mine;
  final SignedProfile sent;
  try {
    base
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ada')
      ..takeOwnProfile(null, from: joiner.publicKey);
    before = _profileSettings(base);
    mine = base.ownProfile!;
    sent = base.ownSignedProfile!;
  } finally {
    base.close();
  }
  String record(String id, String name, int version) =>
      Profile(id: id, name: name, version: version).encode();
  Map<String, Object?> signed(
    SigningKey by,
    String record, {
    String? first,
    List<ProfileKey>? keys,
    List<ProfileAlias> aliases = const [],
    bool fresh = false,
  }) => SignedProfile(
    record: record,
    signer: by.publicBase64,
    sig: by.sign(profileRecordBytes(record)),
    first: first ?? by.publicBase64,
    keys: keys ?? const [],
    aliases: aliases,
    fresh: fresh,
  ).toJson();
  const other = '000000000000000000000000000000f1';
  final cases = <String, Object?>{
    'newer, from a bound key': signed(
      joiner,
      record(mine.id, 'Ada Lovelace', mine.version + 1),
      first: sent.first,
      keys: sent.keys,
    ),
    'older': signed(
      joiner,
      record(mine.id, 'Ada at first', 1),
      first: sent.first,
      keys: sent.keys,
    ),
    'signed by a key not bound': signed(
      stranger,
      record(mine.id, 'Mallory', mine.version + 5),
      first: sent.first,
      keys: sent.keys,
    ),
    'a key bound by itself': signed(
      stranger,
      record(mine.id, 'Mallory', mine.version + 5),
      first: sent.first,
      keys: [
        ...sent.keys,
        ProfileKey(
          key: stranger.publicBase64,
          by: stranger.publicBase64,
          sig: stranger.sign(profileKeyBytes(mine.id, stranger.publicBase64)),
        ),
      ],
    ),
    'a copied id': signed(stranger, record(mine.id, 'Mallory', 9)),
    'the record changed after signing': {
      ...signed(
        joiner,
        record(mine.id, 'Ada', mine.version + 1),
        first: sent.first,
        keys: sent.keys,
      ),
      'record': record(mine.id, 'Mallory', mine.version + 1),
    },
    'another, named by the user': signed(joiner, record(other, 'Ada L.', 3)),
    'another, with the defaults': signed(
      joiner,
      record(other, 'Pixel 7', 1),
      fresh: true,
    ),
    'another, naming this one its alias': signed(
      joiner,
      record(other, 'Ada L.', 3),
      aliases: [
        ProfileAlias(
          alias: mine.id,
          to: other,
          by: joiner.publicBase64,
          sig: joiner.sign(profileAliasBytes(mine.id, other)),
        ),
      ],
    ),
  };
  final out = <Map<String, Object?>>[];
  for (final MapEntry(key: what, value: profile) in cases.entries) {
    final store = open();
    try {
      before.forEach(store.setLocalSetting);
      final taken = store.takeOwnProfile(profile, from: joiner.publicKey);
      out.add({
        'case': what,
        'profile': profile,
        'taken': taken.name,
        'settings': _profileSettings(store),
      });
    } finally {
      store.close();
    }
  }
  if (out.map((c) => c['taken']).toSet().length != ProfileTaken.values.length) {
    throw StateError('the profile takes do not cover every outcome');
  }
  return {
    'writer': writer.publicBase64,
    'from': joiner.publicBase64,
    'before': before,
    'cases': out,
  };
}

/// Profiles of other people (#50), each taken by a store that keeps
/// Ben's Profile (first key the joiner's) and Cat's (first key the
/// second key) as they were before: what became of each, the signed
/// Profiles kept after, and the ids Ben's and Cat's stand for then.
Map<String, Object?> _personTakes() {
  final writer = SigningKey.fromSeed(_seed);
  final joiner = SigningKey.fromSeed(_joinerSeed);
  final second = SigningKey.fromSeed(
    Uint8List.fromList(List<int>.filled(32, 7)),
  );
  final stranger = SigningKey.fromSeed(
    Uint8List.fromList(List<int>.filled(32, 9)),
  );
  const ben = '000000000000000000000000000000b1';
  const cat = '000000000000000000000000000000c1';
  const other = '000000000000000000000000000000d1';
  const mallory = '000000000000000000000000000000e1';
  String record(String id, String name, int version) =>
      Profile(id: id, name: name, version: version).encode();
  ProfileKey bind(SigningKey by, SigningKey key, String id) => ProfileKey(
    key: key.publicBase64,
    by: by.publicBase64,
    sig: by.sign(profileKeyBytes(id, key.publicBase64)),
  );
  Map<String, Object?> signed(
    SigningKey by,
    String record, {
    SigningKey? first,
    List<ProfileKey> keys = const [],
    List<ProfileAlias> aliases = const [],
  }) => SignedProfile(
    record: record,
    signer: by.publicBase64,
    sig: by.sign(profileRecordBytes(record)),
    first: (first ?? by).publicBase64,
    keys: keys,
    aliases: aliases,
  ).toJson();
  final held = signed(joiner, record(ben, 'Ben', 2));
  final heldCat = signed(second, record(cat, 'Cat', 1));
  final cases = <String, Object?>{
    'first sight of another': signed(stranger, record(other, 'Dan', 1)),
    'newer, from the first key': signed(joiner, record(ben, 'Benjamin', 3)),
    'older': signed(joiner, record(ben, 'Benny', 1)),
    'newer, from a key a bound key joined': signed(
      second,
      record(ben, 'Ben B.', 3),
      first: joiner,
      keys: [bind(joiner, second, ben)],
    ),
    'signed by a key not bound': signed(
      stranger,
      record(ben, 'Mallory', 9),
      first: joiner,
    ),
    'a key bound by itself': signed(
      stranger,
      record(ben, 'Mallory', 9),
      first: joiner,
      keys: [bind(stranger, stranger, ben)],
    ),
    'a copied id': signed(stranger, record(ben, 'Mallory', 9)),
    'the record changed after signing': {
      ...signed(joiner, record(ben, 'Ben', 3)),
      'record': record(ben, 'Mallory', 3),
    },
    'the own Profile': signed(writer, record('own', 'Ada', 5)),
    // Cat's Device took Ben's Profile over: her key joined it, and she
    // gave her own id up with it.
    'an alias made by a key of the Profile given up': signed(
      joiner,
      record(ben, 'Ben', 2),
      keys: [bind(joiner, second, ben)],
      aliases: [
        ProfileAlias(
          alias: cat,
          to: ben,
          by: second.publicBase64,
          sig: second.sign(profileAliasBytes(cat, ben)),
        ),
      ],
    ),
    'an alias made by another key': signed(
      stranger,
      record(mallory, 'Mallory', 1),
      aliases: [
        ProfileAlias(
          alias: ben,
          to: mallory,
          by: stranger.publicBase64,
          sig: stranger.sign(profileAliasBytes(ben, mallory)),
        ),
      ],
    ),
  };
  final out = <Map<String, Object?>>[];
  for (final MapEntry(key: what, value: profile) in cases.entries) {
    final store = Store.inMemory(
      clock: () => DateTime.utc(2026, 3, 1),
      newKey: () => SigningKey.fromSeed(_seed),
    );
    try {
      store
        ..setLocalSetting('profile', record('own', 'Ada', 1))
        ..setLocalSetting('profile:first', writer.publicBase64)
        ..takePerson(held)
        ..takePerson(heldCat);
      final taken = store.takePerson(profile);
      out.add({
        'case': what,
        'profile': profile,
        'taken': taken.name,
        'people': _people(store),
        'resolved': {
          for (final id in [ben, cat]) id: store.resolvePerson(id),
        },
      });
    } finally {
      store.close();
    }
  }
  if (out.map((c) => c['taken']).toSet().length != PersonTaken.values.length) {
    throw StateError('the person takes do not cover every outcome');
  }
  return {
    'held': [held, heldCat],
    'cases': out,
  };
}

/// A [width] × [height] PNG shading from red to blue, made the same way
/// every time.
Uint8List _gradient(int width, int height) {
  final image = img.Image(width: width, height: height);
  for (final p in image) {
    p
      ..r = 255 * (width - p.x) ~/ width
      ..g = 255 * p.y ~/ height
      ..b = 255 * p.x ~/ width;
  }
  return img.encodePng(image);
}

void _writeBytes(File file, List<int> bytes) {
  file.parent.createSync(recursive: true);
  file.writeAsBytesSync(bytes);
}

/// The ARB key of each Starter Set place's translation in the app.
const _starterKeys = {
  'household': 'starterHousehold',
  'kitchen': 'starterKitchen',
  'garden': 'starterGarden',
  'cleaning': 'starterCleaning',
  'work': 'starterWork',
  'meetings': 'starterMeetings',
  'to-sort-out': 'starterToSortOut',
  'car': 'starterCar',
  'service': 'starterService',
  'repairs': 'starterRepairs',
  'health': 'starterHealth',
  'doctor': 'starterDoctor',
  'medication': 'starterMedication',
};

/// The app's words Whisper is told and the Starter Set's names, as the
/// app's [locale] shows them, read from its ARB file.
({List<String> terms, Map<String, String> starters}) _uiTexts(String locale) {
  final arb = jsonDecode(
    File.fromUri(Platform.script.resolve('../../../lib/l10n/app_$locale.arb'))
        .readAsStringSync(),
  ) as Map<String, Object?>;
  String text(String key) => arb[key]! as String;
  return (
    terms: [text('unsorted'), text('routine'), text('agenda')],
    starters: {
      for (final MapEntry(:key, :value) in _starterKeys.entries)
        key: text(value),
    },
  );
}

/// The name [slug] and [name] show under: a Starter Set place's
/// translation in [starters] while it keeps its canonical name, else the
/// name as stored.
String _shownName(Map<String, String> starters, String? slug, String name) {
  if (slug == null) return name;
  final canonical = {
    for (final area in starterSet) ...{
      area.slug: area.name,
      for (final topic in area.topics) topic.slug: topic.name,
    },
  };
  if (canonical[slug] != name) return name;
  return starters[slug] ?? name;
}

/// The lead-ins Whisper gets: name lists as given, and three stores,
/// each seen in English and German: one with the Starter Set, one with
/// names too long to all fit, one with more names than are listed. Then
/// Transcripts guarded against the prompts Whisper got.
Map<String, Object?> _transcriptPrompts() {
  final lists = [
    (names: ['Work', 'Garten', 'Fahrradladen'], terms: ['Unsortiert']),
    (names: <String>[], terms: <String>[]),
    (names: <String>[], terms: [' Unsorted ', '', 'Agenda']),
    (
      names: [' Work ', '', 'work', 'WORK', '  ', '﻿Home ', 'home'],
      terms: <String>[],
    ),
    (
      names: [
        'ΟΔΟΣ',
        'οδοσ',
        'Straße',
        'STRASSE',
        'İstanbul',
        'istanbul',
        '𐐀',
        '𐐨',
      ],
      terms: <String>[],
    ),
    (names: ['x' * 500, 'Work', '🚲 Fahrrad', 'Café'], terms: ['Unsortiert']),
    (names: [for (var i = 0; i < 100; i++) 'N$i'], terms: <String>[]),
    (
      names: [
        for (var i = 0; i < 200; i++) 'Grundstücksverwaltungsangelegenheit $i',
      ],
      terms: ['Unsortiert', 'Routine', 'Agenda'],
    ),
    (names: ['Work'], terms: ['y' * 500]),
  ];
  final stores = <String, void Function(Places)>{
    'starters': (places) {
      places.plantStarterSet({
        for (final area in starterSet) ...{
          area.slug,
          for (final topic in area.topics) topic.slug,
        },
      });
      final car = places.areas().firstWhere((a) => a.starter == 'car');
      places.rename(car.id, 'Bike');
      final kids = places.addArea('Kids');
      places.addTopic(kids.id, 'School');
    },
    'many': (places) {
      places.plantStarterSet({'household', 'kitchen', 'garden', 'work'});
      places.addArea('garten');
      places.addArea('🚲 Fahrrad');
      places.addArea(
        'Grundstücksverwaltungsangelegenheiten der Familie '
        'Müller-Lüdenscheidt',
      );
      final short = places.addArea('Kurz');
      for (var i = 1; i <= 45; i++) {
        places.addTopic(short.id, 'K$i');
      }
      places.addTopic(short.id, 'k1');
    },
    'numbers': (places) {
      final numbers = places.addArea('Nummern');
      for (var i = 1; i <= 50; i++) {
        places.addTopic(numbers.id, '$i');
      }
    },
  };
  final texts = {
    for (final locale in ['en', 'de']) locale: _uiTexts(locale),
  };
  final storeJson = <String, Object?>{};
  final views = <Map<String, Object?>>[];
  for (final MapEntry(key: name, value: build) in stores.entries) {
    final store = _store();
    try {
      final places = Places(store);
      build(places);
      storeJson[name] = [for (final e in store.allEntries()) e.toJson()];
      for (final MapEntry(key: locale, value: ui) in texts.entries) {
        final prompts = TranscriptPrompts.forPlaces(
          places,
          terms: ui.terms,
          areaName: (area) => _shownName(ui.starters, area.starter, area.name),
          topicName: (topic) =>
              _shownName(ui.starters, topic.starter, topic.name),
        );
        views.add({
          'store': name,
          'locale': locale,
          'english': prompts.english,
          'german': prompts.german,
        });
      }
    } finally {
      store.close();
    }
  }
  const words = ['Unsortiert', 'Routine', 'Agenda'];
  const places = ['Arzt', 'Work', 'Meetings'];
  final german = TranscriptPrompts.of(names: places, terms: words).german;
  const leadIn = TranscriptPrompts.germanLeadIn;
  final guards = [
    (names: places, terms: words, transcript: leadIn),
    (names: places, terms: words, transcript: ' $leadIn'),
    (
      names: places,
      terms: words,
      transcript: '$leadIn Morgen um zehn den Zahnarzt anrufen.',
    ),
    (names: places, terms: words, transcript: 'Arzt, Work, Meetings.'),
    (
      names: places,
      terms: words,
      transcript: 'Morgen um zehn zum Arzt, Rezept nicht vergessen.',
    ),
    (names: places, terms: words, transcript: german),
    (names: places, terms: words, transcript: 'Unsortiert, Routine, Agenda.'),
    (
      names: places,
      terms: words,
      transcript:
          'notiz auf deutsch mit englischen wörtern. nach dem meeting das '
          'update ans team. feedback vor der deadline!',
    ),
    (names: places, terms: words, transcript: '$leadIn $leadIn'),
    (
      names: places,
      terms: words,
      transcript: 'Notiz auf Deutsch mit englischen Wörtern:',
    ),
    (
      names: places,
      terms: words,
      transcript: 'Notiz auf Deutsch mit englischen Sätzen.',
    ),
    (
      names: places,
      terms: words,
      transcript: 'Nach dem Meeting das Update ans Team.',
    ),
    (names: places, terms: words, transcript: 'Brot holen. $leadIn'),
    (
      names: places,
      terms: words,
      transcript: 'Erst Arzt, Work, Meetings – dann Pause.',
    ),
    (names: places, terms: words, transcript: 'Arzt – Work – Meetings'),
    (names: places, terms: words, transcript: 'Arzt und Work.'),
    (names: places, terms: words, transcript: 'Work, Arzt.'),
    (names: ['Garten'], terms: words, transcript: 'Garten.'),
    (names: ['Arzt', 'Work'], terms: words, transcript: 'Arzt, Work.'),
    (names: ['Arzt', 'Work'], terms: words, transcript: 'Work, Arzt.'),
    (
      names: ['Haushalt', 'Küche'],
      terms: words,
      transcript: 'Haushalt, Küche, Milch kaufen.',
    ),
    (
      names: ['Dr. Müller', 'Müller-Lüdenscheidt', 'Garten'],
      terms: <String>[],
      transcript: '„Dr. Müller, Müller-Lüdenscheidt, Garten“',
    ),
    (
      names: ['İstanbul', 'Straße', 'Café', '🚲 Fahrrad'],
      terms: <String>[],
      transcript: 'İSTANBUL, Straße, CAFÉ, 🚲 Fahrrad. Bis bald',
    ),
    (names: places, terms: words, transcript: ''),
    (names: places, terms: words, transcript: ' ... '),
    (names: <String>[], terms: <String>[], transcript: '$leadIn Hallo.'),
  ];
  return {
    'germanLeadIn': TranscriptPrompts.germanLeadIn,
    'maxLength': TranscriptPrompts.maxLength,
    'maxNames': TranscriptPrompts.maxNames,
    'lists': [
      for (final (:names, :terms) in lists)
        if (TranscriptPrompts.of(names: names, terms: terms) case final p)
          {
            'names': names,
            'terms': terms,
            'english': p.english,
            'german': p.german,
          },
    ],
    'ui': {
      for (final MapEntry(key: locale, value: ui) in texts.entries)
        locale: {'terms': ui.terms, 'starters': ui.starters},
    },
    'stores': storeJson,
    'views': views,
    'guards': [
      for (final (:names, :terms, :transcript) in guards)
        {
          'names': names,
          'terms': terms,
          'transcript': transcript,
          'guarded': TranscriptPrompts.of(
            names: names,
            terms: terms,
          ).guard(transcript),
        },
    ],
  };
}

const _json = JsonEncoder.withIndent('  ');

void _write(File file, String text) {
  file.parent.createSync(recursive: true);
  file.writeAsStringSync(text);
}

/// The store's schema, as SQLite keeps it, in a stable order.
String _schema(Directory work) {
  final path = '${work.path}/schema.db';
  Store.open(path, newKey: () => SigningKey.fromSeed(_seed)).close();
  final db = sqlite3.open(path);
  try {
    final rows = db.select(
      "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name",
    );
    return '${[for (final row in rows) '${row['sql']};'].join('\n')}\n';
  } finally {
    db.close();
  }
}

/// Entries signed by the writer's key, with the exact bytes the
/// signature covers: awkward text, a null value, and edits with an empty
/// and a two-Device causal context.
Map<String, Object?> _signing() {
  final key = SigningKey.fromSeed(_seed);
  const date = '2026-01-01T10:00:00.000Z';
  const recorded = '2026-01-01T10:00:00.123456Z';
  final device = deviceIdFromKey(key.publicKey);
  final bytes = entryBytes(
    device: device,
    dseq: 7,
    entity: 'note:0001',
    field: 'text',
    value: _awkward,
    dateIso: date,
    author: 'Ada "the writer"',
    recordedIso: recorded,
  );
  final nullBytes = entryBytes(
    device: device,
    dseq: 8,
    entity: 'note:0001',
    field: 'trashed',
    value: null,
    dateIso: date,
    author: '',
    recordedIso: recorded,
  );
  final ctxBytes = [
    for (final ctx in ['', 'a1b2:3 c4d5:12'])
      (
        ctx,
        entryBytes(
          device: device,
          dseq: 9,
          entity: 'note:0001',
          field: 'text',
          value: 'Edited',
          dateIso: date,
          author: '',
          recordedIso: recorded,
          ctx: ctx,
        ),
      ),
  ];
  return {
    'seed': key.seedHex,
    'publicKey': key.publicBase64,
    'device': device,
    'code': key.code,
    'entries': [
      {
        'device': device,
        'dseq': 7,
        'entity': 'note:0001',
        'field': 'text',
        'value': _awkward,
        'date': date,
        'author': 'Ada "the writer"',
        'recorded': recorded,
        'bytes': base64.encode(bytes),
        'sig': key.sign(bytes),
      },
      {
        'device': device,
        'dseq': 8,
        'entity': 'note:0001',
        'field': 'trashed',
        'value': null,
        'date': date,
        'author': '',
        'recorded': recorded,
        'bytes': base64.encode(nullBytes),
        'sig': key.sign(nullBytes),
      },
      for (final (ctx, bytes) in ctxBytes)
        {
          'device': device,
          'dseq': 9,
          'entity': 'note:0001',
          'field': 'text',
          'value': 'Edited',
          'date': date,
          'author': '',
          'recorded': recorded,
          'ctx': ctx,
          'bytes': base64.encode(bytes),
          'sig': key.sign(bytes),
        },
    ],
  };
}

/// A later version's entries (ADR 0019): one with a key this version
/// does not know, which applies, and one of structure version 2 with a
/// key more, which is kept and passed on but not applied; signed with
/// those keys. A Device of this version takes them in and passes them on
/// byte for byte; the same lines with a key changed or added on the way
/// are refused.
void _versionsFixture(Directory root) {
  final out = Directory('${root.path}/versions');
  final writer = _store();
  final reader = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_joinerSeed),
  );
  try {
    final note = Notes(writer).capture('Kept as it came');
    final key = writer.signingKey;
    final device = writer.deviceId;
    var dseq = writer.versionVector()[device] ?? 0;
    // Lines as a later version writes them: the keys of version 1 in
    // their order, then its own, sorted.
    Map<String, Object?> later(
      String field,
      String value,
      Map<String, Object?> extra,
    ) {
      dseq++;
      final date = _clock().toUtc().toIso8601String();
      final sig = key.sign(
        entryBytes(
          device: device,
          dseq: dseq,
          entity: note.id,
          field: field,
          value: value,
          dateIso: date,
          author: 'Ada',
          recordedIso: date,
          extra: extra,
        ),
      );
      return {
        'device': device,
        'dseq': dseq,
        'entity': note.id,
        'field': field,
        'value': value,
        'date': date,
        'author': 'Ada',
        'recorded': date,
        'sig': sig,
        ...extra,
      };
    }

    final unknown = later(
      'text',
      const StoredText.plain('Edited by a later version').encode(),
      {
        'later': {'b': 2, 'a': 'x'},
      },
    );
    final v2 = later(
      'text',
      const StoredText.plain('Not applied here').encode(),
      {'more': 'kept', 'v': 2},
    );
    final lines = [
      for (final e in writer.allEntries()) e.wire,
      canonicalEntry(unknown),
      canonicalEntry(v2),
    ];
    _write(File('${out.path}/incoming.jsonl'), '${lines.join('\n')}\n');
    final record = writer.ownKeyRecord;
    _write(File('${out.path}/key.json'), '${_json.convert(record.toJson())}\n');
    final tampered = [
      canonicalEntry({...unknown, 'later': 'changed'}),
      canonicalEntry({...v2, 'v': 3}),
      canonicalEntry({...v2, 'added': true}),
    ];
    _write(File('${out.path}/tampered.jsonl'), '${tampered.join('\n')}\n');

    final report = ImportReport();
    final refused = reader.applyEntries(
      [
        for (final line in tampered)
          Entry.fromJson(jsonDecode(line) as Map<String, dynamic>, text: line),
      ],
      keys: [record],
      report: report,
    );
    final applied = reader.applyEntries(
      [
        for (final line in lines)
          Entry.fromJson(jsonDecode(line) as Map<String, dynamic>, text: line),
      ],
      keys: [record],
    );
    final forwarded = [for (final e in reader.entriesSince(const {})) e.wire];
    _write(File('${out.path}/forwarded.jsonl'), '${forwarded.join('\n')}\n');
    final text = Notes(reader).byId(note.id)?.text;
    _write(
      File('${out.path}/expected.json'),
      '${_json.convert({
        'refused': report.refusedCount,
        'refusedApplied': refused.length,
        'applied': [for (final e in applied) e.id],
        'vector': reader.versionVector(),
        'text': text,
      })}\n',
    );
  } finally {
    writer.close();
    reader.close();
  }
}

/// The marks over a region around the origin, negative cells included.
Map<String, Object?> _questionMarks() => {
  'shapes': [
    for (final shape in questionMarkShapes)
      {'strokes': shape.strokes, 'dot': shape.dot, 'weight': shape.weight},
  ],
  'seed': questionMarkSeed,
  'region': [-400, -300, 700, 500],
  'marks': [
    for (final m in questionMarksIn(-400, -300, 700, 500))
      {
        'shape': m.shape,
        'x': m.x,
        'y': m.y,
        'size': m.size,
        'rotation': m.rotation,
      },
  ],
  'otherSeed': [
    for (final m in questionMarksIn(0, 0, 300, 300, seed: 1))
      {
        'shape': m.shape,
        'x': m.x,
        'y': m.y,
        'size': m.size,
        'rotation': m.rotation,
      },
  ],
};

Future<void> main(List<String> args) async {
  if (args.length != 1) {
    stderr.writeln('usage: dart run tool/gen_fixtures.dart <fixtures dir>');
    exit(2);
  }
  final root = Directory(args[0])..createSync(recursive: true);
  // The Area keys every LAN session hands over, counted too.
  secretBytes = _counted(0);
  // A scenario dropped from the list leaves no folder behind.
  for (final old in root.listSync().whereType<Directory>()) {
    old.deleteSync(recursive: true);
  }
  final index = StringBuffer(
    '# Fixture corpus\n\n'
    'Generated by `packages/oldbrainfix_core/tool/gen_fixtures.dart` from '
    'the Dart core; the desktop core loads every `log.jsonl` and must read '
    'the same `expected.json`. Do not edit by hand.\n\n'
    '- `schema.sql`: the tables and indexes of the store.\n'
    '- `signing.json`: entries signed by the writer key, with the bytes '
    'the signatures cover.\n'
    '- `question_marks.json`: the background\'s shapes and their scatter '
    'over a region.\n'
    '- `schedules.json`: RRULEs read, written back or refused, schedules '
    'with their Occurrences, days normalised, and the instants of '
    'wall-clock times around daylight-saving switches in Central Europe.\n'
    '- `routines/alarms.json`: the Agenda, alarm plan, Missed Summary and '
    'Nudge of the `routines` log at held moments with this Device\'s '
    'settings, on the same clock.\n'
    '- `pair_codes.json`: pair codes for a QR and for typing, codes as '
    'typed, and what is no code.\n'
    '- `keys.json`: the writer\'s key record, and another key claiming its '
    'device.\n'
    '- `sync/`: two Devices that met once and wrote more since: their logs '
    '(`host.jsonl`, `joiner.jsonl`) with their blobs and the keys they '
    'hold (`settings.json`), the joiner\'s host certificate among them '
    '(`tls:cert`, `tls:key`, random bytes counted from 6001 on), the '
    'host\'s vector, the joiner\'s `POST /sync` request (with its proof '
    'for the PIN 246810 and its certificate\'s fingerprint, which the host '
    'keeps) and the host\'s answer, '
    'both with the Area keys, the own Profile and its backup key, the '
    'backup key each then holds, the host\'s on both '
    '(`backup_keys.json`), the own Profiles both hold after it, the '
    'host\'s named by the user and taken over by the joiner, whose own '
    'still had the defaults (`profile.json`), and what both read after '
    'the joiner took it in (`expected.json`).\n'
    '- `conflicts/`: two Devices that met once, then edited the same Note '
    'text, Area name and Routine schedule apart, wrote the same value on '
    'both sides and renamed a Topic on one side only: their logs, the '
    'session between them, the Conflicts both find (`expected.json`), the '
    'host keeping the older version of each (`picks.jsonl`, at the '
    'held clock), and what both read after a second session.\n'
    '- `crypto.json`: messages sealed with XChaCha20-Poly1305 under given '
    'keys, nonces and what they are bound to; a key\'s id; a blob\'s name '
    'in the shared folder.\n'
    '- `transcript_prompts.json`: the lead-ins Whisper gets: name lists '
    'with the app\'s words as given, and three stores, as their entries, '
    'seen with the English and the German app\'s words and Starter Set '
    'names (`ui`, from the app\'s ARB files): the whole Starter Set with '
    'one Area renamed, one with names too long to all fit, repeats in '
    'another case among them, and one with more names than are listed; '
    'Transcripts guarded against the prompts Whisper got for names and '
    'words as given (`guards`): the lead-in alone, at the start, twice, in '
    'another case and punctuation, cut short and after speech; the name '
    'list, the app\'s words, the whole German prompt; one, two and three '
    'names alone and amid speech, names of several words, and real '
    'speech naming a place, kept as it is.\n'
    '- `folder/`: two Devices that met once, so the joiner holds the '
    'host\'s keys; the host wrote more since, a new Area among it, and '
    'synced through the shared folder: both logs with their blobs and the '
    'keys each holds (`settings.json`), the folder the host wrote '
    '(`$syncFolderRoot/`, nonces counted from 1001 on; its keys file '
    'carries the own Profile, renamed since they met, and its backup key), '
    'what the joiner reads from it and the backup key it holds then '
    '(`expected.json`) and its own Profile after (`profile.json`).\n'
    '- `bundle/`: two Devices that met once; the host wrote more since, a '
    'new Area among it, and a bundle for the joiner: both logs with their '
    'blobs, the keys each holds and what each was told the other holds '
    '(`settings.json`), the bundle (`host.$bundleExtension`, nonces counted '
    'from 2001 on), what the joiner reads from it (`expected.json`) and its '
    'own Profile after, both Profiles named by the user since, so "Which '
    'one is you?" waits (`profile.json`). '
    'The desktop core compares the files inside the zip byte for byte, not '
    'the zip itself, whose headers its zip library writes its own way.\n'
    '- `shared/`: Ann shares a Starter Set Area with Ben as a Member: her '
    'log before (`before.jsonl`), what sharing wrote (`share.jsonl`, ids '
    'counted from 0x101, at a held clock), Ben\'s log (`joiner.jsonl`), the '
    'invite session (`request.json`, `answer.json`) with the Profiles '
    'each then keeps, Ann giving Ben a name and picture for herself alone '
    '(`override.jsonl` with `override.jpg`, an Override that never reaches '
    'him), Ann renamed and publishing the Area\'s Profiles '
    'into a shared folder (`<device>.<key id>.profiles`, nonces counted '
    'from 2000) and what Ben keeps after reading it, what Ben wrote, '
    'allowed and not (`member.jsonl`), Ann filing a Note in '
    '(`owner.jsonl`), the paired session after (`request2.json`, '
    '`answer2.json`), Ben made Manager (`role.jsonl`) and writing as one '
    '(`manager.jsonl`), then removed '
    '(`remove.jsonl`) with the answer he gets then (`answer3.json`), and '
    'what both read, which entries the roles leave out and whose Profile '
    'marks each Note (`expected.json`).\n'
    '- `versions/`: Ada\'s Device of a later version (ADR 0019): its log, '
    'then an edit with a key this version does not know and one of '
    'structure version 2 with a key more, both signed with those keys '
    '(`incoming.jsonl`), her key record (`key.json`), the same two with a '
    'key changed, the version raised and a key added on the way '
    '(`tampered.jsonl`, all refused), what a Device of this version passes '
    'on after taking them in, byte for byte (`forwarded.jsonl`), and what '
    'it applied and reads (`expected.json`).\n'
    '- `sent/`: Ada sends a Note with a Section, a Routine and a Reminder '
    'to Ben: her log (`sender.jsonl`), her Profile\'s settings, which every '
    'file she sends carries (`settings.json`), what sending wrote (`sent.jsonl`, '
    'ids counted from 0x301, at a held clock), the file signed only '
    '(`signed.$sentItemExtension`) and encrypted to Ben\'s X25519 key '
    '(`sealed.$sentItemExtension`, random bytes counted from 4001 on), '
    'Ben, who does not know Ada, holding it unseen until "Show" as a '
    'pending set (`held.json`, once for both files, its id counted from '
    '0x3f1), what accepting it wrote (`accepted.jsonl`, ids counted from 0x401), '
    'keeping her Profile; then Ben makes it his own (`receiver.jsonl`, from '
    '0x501), Ada changes it (`changed.jsonl`, from 0x601), renames herself '
    'and sends an update (`update.jsonl`, `update.$sentItemExtension`), '
    'which Ben applies after comparing her short code (`updated.jsonl`, '
    'from 0x701); Ada cancels it (`cancel.jsonl`, '
    '`cancel.$sentItemExtension`), Ben accepts that (`cancelled.jsonl`), '
    'and refuses a cancellation another key signed '
    '(`forged.$sentItemExtension`) and a file whose Profile is not bound to '
    'the key that signed it (`impostor.$sentItemExtension`); what he reads '
    'and the Profiles he keeps at each step '
    '(`expected.json`).\n'
    '- `backup/`: Ada\'s Device with Areas, a Shared Area, a photo, a '
    'recording longer than a chunk, a Routine checked off, a Moment and '
    'another Device\'s key: her log with its blobs (`device.jsonl`), the '
    'keys, her Profile, the backup key she holds and the playback speed '
    'she chose (`settings.json`), '
    'her Moments '
    '(`moments.json`), the Backup she wrote with the standard Argon2id '
    'cost (`backup.$backupExtension`, random bytes counted from 5001 on, '
    'at a held clock), and a new Device restoring it with the passphrase '
    'a day later: its log (`restored.jsonl`), the settings it keeps, her '
    'Profile with the new Device\'s key bound to it by the old one\'s '
    'and her playback speed among them, and what it reads '
    '(`expected.json`).\n'
    '- `profiles/`: Profile names as typed and as kept, initials, '
    'placeholder colours, short codes from an id and a first key, records '
    'as they are written and records refused, the own Profile\'s local '
    'settings after each step (ids counted from 1), signed Profiles another '
    'of the own Devices sends with what became of each and the settings '
    'after (`takes`, ids counted from 0x65), Profiles of another person '
    'taken by a Device that keeps one with what became of each and what '
    'it keeps after (`people`), and a Profile picture made here '
    '(`picture.jpg`).\n'
    '- `block/`: Ann shares her Household with Carol as a Member and '
    'joins the School Carol manages; Carol writes a Note with a photo and '
    'its Photo Text, Amends Ann\'s Note and checks off Ann\'s Routine, and '
    'Ann makes a Reminder of Carol\'s Note: Ann\'s log with its photo '
    '(`before.jsonl`) and every setting of hers but her key '
    '(`settings.json`); Ann blocking Carol, which leaves the School '
    '(`block.jsonl`, at a held clock); Carol writing more '
    '(`incoming.jsonl`), which Ann takes in; Ann unblocking her '
    '(`unblock.jsonl`, at a held clock), and Carol\'s Device syncing with '
    'hers (`refetch.json`, the request without its Profiles), which brings '
    'back what they still share and neither the School Ann left nor a Note '
    'Carol kept to herself; after each step the entries Ann holds, the '
    'vector she tells, her refusal marks, the Profiles she keeps, the '
    'people blocked, the photo and what she reads (`expected.json`).\n'
    '- `pending/`: Pending Changes (ADR 0022): Ann\'s log '
    '(`before.jsonl`); Bob, who holds it, edits her Note and writes one of '
    'his own (`received.jsonl`, with his key record in `key.json`). At '
    'held clocks (`times`), ids counted from 0x801, Ann opens an AI Task\'s '
    'set and drafts a text, a filing, a new Note and a Routine\'s rule; '
    'she edits that text and the Routine\'s start herself; she opens a '
    'set for a bundle and takes Bob\'s entries into it, then edits the '
    'Note he edited (her edits: `edits.jsonl`). Both sets\' files '
    '(`ai.json`, `bundle.json`), what she reads with them on top of her '
    'log (`shown`), the Routine\'s fields, which keep the schedule she '
    'edited herself (`routineShown`), a third set declined, which leaves '
    'the log as it was, '
    'Accept of each (`accept-ai.jsonl`, `accept-bundle.jsonl`) with the '
    'fields her own edits kept, and what she reads after '
    '(`expected.json`).\n',
  );
  final work = Directory.systemTemp.createTempSync('oldbrainfix_fixture_');
  try {
    _write(File('${root.path}/schema.sql'), _schema(work));
  } finally {
    work.deleteSync(recursive: true);
  }
  _write(File('${root.path}/signing.json'), '${_json.convert(_signing())}\n');
  _write(
    File('${root.path}/question_marks.json'),
    '${_json.convert(_questionMarks())}\n',
  );
  _write(
    File('${root.path}/schedules.json'),
    '${_json.convert(_schedules())}\n',
  );
  _write(
    File('${root.path}/pair_codes.json'),
    '${_json.convert(_pairCodes())}\n',
  );
  _write(File('${root.path}/keys.json'), '${_json.convert(_keys())}\n');
  _write(File('${root.path}/crypto.json'), '${_json.convert(_crypto())}\n');
  _write(
    File('${root.path}/transcript_prompts.json'),
    '${_json.convert(_transcriptPrompts())}\n',
  );
  _sync(root);
  _conflictsFixture(root);
  await _folder(root);
  await _bundle(root);
  await _sharedFixture(root);
  _sentFixture(root);
  await _backupFixture(root);
  _profilesFixture(root);
  _versionsFixture(root);
  _blockFixture(root);
  _pendingFixture(root);
  for (final scenario in _scenarios) {
    final store = _store();
    try {
      scenario.build(store);
      final out = Directory('${root.path}/${scenario.name}');
      final log = StringBuffer();
      for (final entry in store.allEntries()) {
        log.writeln(jsonEncode(entry.toJson()));
      }
      _write(File('${out.path}/log.jsonl'), log.toString());
      _writeBlobs(store, out);
      _held = _tick.add(const Duration(days: 1));
      _write(
        File('${out.path}/expected.json'),
        '${_json.convert(_expected(store))}\n',
      );
      if (store.entitiesOfType(RoutineKeys.typeRoutine).isNotEmpty) {
        _write(
          File('${out.path}/alarms.json'),
          '${_json.convert(_alarms(store))}\n',
        );
      }
      index.writeln('- `${scenario.name}`: ${scenario.about}');
    } finally {
      store.close();
    }
  }
  _write(File('${root.path}/README.md'), index.toString());
  stdout.writeln('Wrote ${_scenarios.length} scenarios to ${root.path}');
}

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// What every comparison searches for: the fixture's queries, queries
/// of one, two and three characters, marks of the index's own syntax,
/// letters lowered on their own, astral characters, a box's glyph and
/// queries that find nothing.
const _queries = [
  'sauce',
  'MILK',
  'receipt',
  'x-ray',
  'müller',
  ' e ',
  'ce',
  'sau',
  '**',
  'milk #code',
  '4711',
  '"',
  'say "hi"',
  '*',
  'a OR b',
  'NEAR(x y)',
  'buy  milk',
  'İzmir',
  '#izmir',
  'οδοσ',
  '𝔸x',
  '𝔸',
  '✅ call',
  '☐',
  'tent',
  'oat',
  'after work',
  'parcel',
  'lease',
  'rent due',
  'plan.dwg',
  'opening',
  'bags',
  'nothing at all',
  'buy sauce',
  'sauce after',
  'odd place',
  'nul\u0000',
];

DateTime _wall(DateTime utc) => utc;

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

  Note capture(String text, {String? place, String? format}) {
    now = now.add(const Duration(minutes: 1));
    return notes.capture(
      text,
      place: place,
      format: format ?? TextFormats.plain,
    );
  }

  /// Every query found with the index as without it, everywhere, and
  /// among the trash.
  void same([String? reason]) {
    final places = [
      null,
      for (final area in Places(store).areas()) ...[
        area.id,
        for (final topic in Places(store).topics(area.id)) topic.id,
      ],
    ];
    final trashed = notes.trashed();
    for (final query in _queries) {
      for (final place in places) {
        expect(
          [for (final n in Search(store).notes(query, within: place)) n.id],
          [
            for (final n in Search.scanning(store).notes(query, within: place))
              n.id,
          ],
          reason: '$reason: "$query" in $place',
        );
      }
      expect(
        [for (final n in Search(store).among(query, trashed)) n.id],
        [for (final n in Search.scanning(store).among(query, trashed)) n.id],
        reason: '$reason: "$query" in the trash',
      );
    }
  }

  /// Notes of every kind search reads.
  Map<String, String> fill() {
    final places = Places(store);
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    final work = places.addArea('Work');
    final sauce = capture('Buy SAUCE and milk', place: kitchen.id);
    capture('**bold** sauce `code`', format: TextFormats.markdown);
    capture('milk #code tea', format: TextFormats.code);
    capture('#İzmir trip and ΟΔΟΣ 𝔸x, say "hi"', place: home.id);
    final boxes = capture('☐ call the x-ray clinic\n☐ Müller');
    final listed = capture(
      '- ☐ buy **sauce** pan',
      format: TextFormats.markdown,
    );
    capture('nul\u0000 then sauce after it');
    // Values no Note holds, or holds unreadable: candidates read away.
    store.append('section:odd', NoteKeys.text, 'sauce in an odd place');
    store.append('place:odd', 'file:abc', 'odd place sauce');
    store.append(sauce.id, 'file:def', '{not json sauce');
    final voice = notes.capture(
      '',
      recordings: [Uint8List.fromList(utf8.encode('voice'))],
    );
    notes.setTranscript(voice.id, voice.recordings.single, 'Pick up parcel');
    final photo = notes.capture(
      '',
      photos: [Uint8List.fromList(utf8.encode('photo'))],
    );
    notes.setPhotoText(photo.id, photo.photos.single, 'RECEIPT 4711');
    notes.removePhotoText(photo.id, photo.photos.single);
    notes.setPhotoText(photo.id, photo.photos.single, 'receipt 12,50');
    final letter = notes.capture(
      'Papers',
      files: [
        NewFile(name: 'Lease.txt', bytes: Uint8List.fromList([1, 2])),
        NewFile(name: 'plan.dwg', bytes: Uint8List.fromList([3])),
      ],
      place: work.id,
    );
    notes.setAttachmentText(
      letter.id,
      letter.attachments.first.hash,
      'Rent due on the first',
    );
    final host = capture('Errands', place: home.id);
    final merged = capture('Opening hours');
    now = now.add(const Duration(minutes: 1));
    notes.amend(merged.id, 'and the bags');
    notes.merge(merged.id, host.id);
    final tally = capture('Push-ups');
    final tallies = Tallies(store);
    final reps = tallies.addField(tally.id, name: 'times', unit: 'times');
    tallies.add(tally.id, values: {reps: 20}, line: 'after work');
    final gone = tallies.add(tally.id, values: {reps: 5}, line: 'sauce run');
    tallies.trashEntry(gone.id);
    final trip = capture('Trip\nTent\nSleeping bag');
    Checklists(store).make(trip.id);
    final shop = capture('Shop\nOat milk\nSauce');
    ShoppingLists(store).make(shop.id);
    final old = capture('Old sauce recipe');
    notes.trash(old.id);
    return {
      'sauce': sauce.id,
      'boxes': boxes.id,
      'listed': listed.id,
      'host': host.id,
      'merged': merged.id,
      'tally': tally.id,
      'trip': trip.id,
      'shop': shop.id,
      'old': old.id,
      'letter': letter.id,
    };
  }

  test('the index finds what reading every Note finds', () {
    fill();
    same('built');
  });

  test('every change is taken in before the next search', () {
    final ids = fill();
    same('built');
    notes.edit(ids['sauce']!, 'Buy tea');
    same('edited');
    now = now.add(const Duration(minutes: 1));
    notes.amend(ids['sauce']!, 'Update: more MILK and sauce');
    same('amended');
    final [box, _] = Notes(store).byId(ids['boxes']!)!.boxes;
    Boxes(store).tick(ids['boxes']!, box.id, BoxState.done);
    same('ticked');
    final [listedBox] = Notes(store).byId(ids['listed']!)!.boxes;
    Boxes(store).tick(ids['listed']!, listedBox.id, BoxState.done);
    same('ticked in a formatted text');
    notes.undoMerge(ids['merged']!);
    same('merge undone');
    notes.merge(ids['host']!, ids['sauce']!);
    same('merged in a chain');
    final tallies = Tallies(store);
    final reps = tallies.byNote(ids['tally']!)!.fields.single.id;
    tallies.add(ids['tally']!, values: {reps: 3}, line: 'sauce at night');
    same('entry added');
    final lines = Checklists(store);
    final tent = lines.byNote(ids['trip']!)!.lines[0].id;
    lines.delete(tent);
    lines.add(ids['trip']!, 'Sauce pan');
    same('lines changed');
    lines.toPlain(ids['trip']!);
    same('back to a plain Note');
    lines.make(ids['trip']!);
    same('made again');
    final shopping = ShoppingLists(store);
    final oat = shopping.add(ids['shop']!, 'Milk powder', _wall);
    shopping.setReappear(oat, Reappear.once);
    shopping.check(oat, _wall);
    same('a Line gone');
    notes.restore(ids['old']!);
    same('restored');
    notes.trash(ids['letter']!);
    same('trashed');
  });

  test('what another Device wrote is found once it is in the log', () {
    fill();
    same('built');
    final other = Store.inMemory(clock: () => now);
    addTearDown(other.close);
    final theirs = Notes(other).capture('Sauce from the other Device');
    Notes(other).amend(theirs.id, 'with milk');
    for (final e in other.allEntries()) {
      store.insert(e);
    }
    same('received');
    expect(Search(store).notes('other device').map((n) => n.id), [theirs.id]);
    store.deleteEntries(other.deviceId, [
      for (final e in other.allEntries())
        if (e.field == 'text' && e.entity.startsWith('section:')) e.dseq,
    ]);
    same('deleted');
    expect(Search(store).notes('with milk'), isEmpty);
  });

  test('another store on the same file finds what this one wrote', () {
    final dir = Directory.systemTemp.createTempSync('obf-search-');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    final one = Store.open(path, clock: () => now);
    final two = Store.open(path, clock: () => now);
    addTearDown(one.close);
    addTearDown(two.close);
    final first = Notes(one).capture('first sauce');
    expect(Search(two).notes('sauce').map((n) => n.id), [first.id]);
    final second = Notes(one).capture('second sauce');
    two.changedElsewhere();
    expect(Search(two).notes('SAUCE').map((n) => n.id).toSet(), {
      first.id,
      second.id,
    });
    expect(Search(one).notes('second').map((n) => n.id), [second.id]);
  });

  test('a missing or older index is built anew', () {
    final ids = fill();
    same('built');
    store.database.execute('DROP TABLE search_texts');
    expect(
      Search(store).notes('sauce').map((n) => n.id),
      contains(ids['sauce']),
    );
    same('table dropped');
    store.database.execute(
      "UPDATE search_state SET value = 0 WHERE key = 'version'",
    );
    store.database.execute('DELETE FROM search_texts');
    same('another version');
    store.database.execute("DELETE FROM search_state WHERE key = 'seq'");
    store.database.execute('DELETE FROM search_texts');
    same('no mark');
  });

  test('a shown Pending Change is found as the reads show it', () {
    final ids = fill();
    same('built');
    final id = store.openPending('ai-task', shown: true);
    store.draftPending(
      id,
      ids['sauce']!,
      NoteKeys.text,
      const StoredText.plain('a drafted gooseberry').encode(),
    );
    expect(Search(store).notes('gooseberry').map((n) => n.id), [ids['sauce']]);
    same('drafted');
  });
}

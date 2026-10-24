import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  test('a captured Note carries its text and capture time', () {
    final at = DateTime.utc(2026, 10, 24, 19, 40);
    final store = Store.inMemory(clock: () => at);
    addTearDown(store.close);
    final note = Notes(store).capture('nees mor sauce');
    expect(note.text, 'nees mor sauce');
    expect(note.captured, at);
    expect(note.id, startsWith('note:'));
  });

  test('Notes are listed newest capture first', () {
    var now = DateTime.utc(2026, 10, 24, 8);
    final store = Store.inMemory(clock: () => now);
    addTearDown(store.close);
    final notes = Notes(store);
    notes.capture('first');
    now = now.add(const Duration(hours: 1));
    notes.capture('second');
    expect(notes.all().map((n) => n.text), ['second', 'first']);
  });

  test('Notes survive a reopen', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    final first = Store.open(path);
    Notes(first).capture('milk');
    first.close();
    final second = Store.open(path);
    addTearDown(second.close);
    expect(Notes(second).all().single.text, 'milk');
  });

  test('other entities are not Notes', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('area:x', r'$type', 'area');
    expect(Notes(store).all(), isEmpty);
    expect(Notes(store).byId('area:x'), isNull);
  });

  test('a Note captured into a place is filed there at once', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    final places = Places(store);
    final kitchen = places.addTopic(places.addArea('Household').id, 'Kitchen');
    final note = Notes(store).capture('nees mor sauce', place: kitchen.id);
    expect(note.place, kitchen.id);
    expect(Notes(store).byId(note.id)?.isUnsorted, isFalse);
  });

  test('capturing into something that is no place captures nothing', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(
      () => Notes(store).capture('x', place: 'area:gone'),
      throwsArgumentError,
    );
    expect(Notes(store).all(), isEmpty);
  });

  test('an Area holds its own Notes and its Topics\' ones, newest first', () {
    var now = DateTime.utc(2026, 10, 24, 8);
    final store = Store.inMemory(clock: () => now);
    addTearDown(store.close);
    final places = Places(store);
    final notes = Notes(store);
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    final work = places.addArea('Work');
    Note capture(String text, String? place) {
      now = now.add(const Duration(minutes: 1));
      return notes.capture(text, place: place);
    }

    capture('in the kitchen', kitchen.id);
    capture('at home', home.id);
    capture('at work', work.id);
    capture('unsorted', null);
    expect(notes.inside(home.id).map((n) => n.text), [
      'at home',
      'in the kitchen',
    ]);
    expect(notes.inside(kitchen.id).map((n) => n.text), ['in the kitchen']);
    expect(places.notesInside(home.id), 2);
  });
}

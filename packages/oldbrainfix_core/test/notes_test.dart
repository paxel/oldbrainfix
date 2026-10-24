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
}

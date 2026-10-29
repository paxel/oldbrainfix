import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  test('own entries are signed and verify under the Device key', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    final e = store.append('note:a', 'text', 'hello');
    final bytes = entryBytes(
      device: e.device,
      dseq: e.dseq,
      entity: e.entity,
      field: e.field,
      value: e.value,
      dateIso: e.date.toIso8601String(),
      author: e.author,
      recordedIso: e.recorded.toIso8601String(),
    );
    expect(verifySignature(store.signingKey.publicKey, bytes, e.sig), isTrue);
    expect(store.deviceId, deviceIdFromKey(store.signingKey.publicKey));
  });

  test('the latest date wins, not the latest append', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('note:a', 'text', 'new', date: DateTime.utc(2026, 5, 2));
    store.append('note:a', 'text', 'old', date: DateTime.utc(2026, 5, 1));
    expect(store.current('note:a', 'text'), 'new');
  });

  test('dseq counts up per Device and shows in the version vector', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('note:a', 'text', '1');
    final e = store.append('note:a', 'text', '2');
    expect(e.dseq, 2);
    expect(store.versionVector(), {store.deviceId: 2});
  });

  test('a failing batch writes nothing', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(
      () => store.batch(() {
        store.append('note:a', 'text', 'x');
        throw StateError('boom');
      }),
      throwsStateError,
    );
    expect(store.allEntries(), isEmpty);
  });

  test('a batch inside a batch joins it: committed and heard once, or '
      'rolled back as a whole', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    var heard = 0;
    store.addListener(() => heard++);
    store.batch(() {
      store.append('note:a', 'text', 'one');
      store.batch(() => store.append('note:b', 'text', 'two'));
      expect(heard, 0, reason: 'nothing is heard before the commit');
      expect(store.current('note:b', 'text'), 'two');
      store.append('note:c', 'text', 'three');
    });
    expect(heard, 1);
    expect(store.allEntries(), hasLength(3));
    expect(
      () => store.batch(() {
        store.append('note:d', 'text', 'four');
        store.batch(() {
          store.append('note:e', 'text', 'five');
          throw StateError('boom');
        });
      }),
      throwsStateError,
    );
    expect(heard, 1, reason: 'a failing batch changed nothing');
    expect(store.allEntries(), hasLength(3));
    expect(store.current('note:d', 'text'), isNull);
    store.append('note:f', 'text', 'six');
    expect(heard, 2, reason: 'out of the batch, appends are heard again');
  });

  test('the log and the Device identity survive a reopen', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    final first = Store.open(path);
    first.append('note:a', 'text', 'kept');
    final device = first.deviceId;
    first.close();
    final second = Store.open(path);
    addTearDown(second.close);
    expect(second.deviceId, device);
    expect(second.current('note:a', 'text'), 'kept');
  });

  test('listeners hear each append, a batch once, and settings never', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    var heard = 0;
    void listener() => heard++;
    store.addListener(listener);
    store.append('note:a', 'text', 'one');
    expect(heard, 1);
    store.batch(() {
      store.append('note:a', 'text', 'two');
      store.append('note:a', 'text', 'three');
    });
    expect(heard, 2);
    store.setLocalSetting('theme', 'dark');
    expect(heard, 2);
    expect(
      () => store.batch(() {
        store.append('note:a', 'text', 'four');
        throw StateError('no');
      }),
      throwsStateError,
    );
    expect(heard, 2, reason: 'a failing batch changed nothing');
    store.removeListener(listener);
    store.append('note:a', 'text', 'five');
    expect(heard, 2);
  });

  test('new ids come from the id maker, random by default', () {
    final ids = {Store.newId(), Store.newId()};
    expect(ids, hasLength(2));
    expect(ids.first, matches(RegExp(r'^[0-9a-f]{32}$')));
    final random = Store.idMaker;
    addTearDown(() => Store.idMaker = random);
    var n = 0;
    Store.idMaker = () => 'id${++n}';
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(Notes(store).capture('x').id, 'note:id1');
  });
}

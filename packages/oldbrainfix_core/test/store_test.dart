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
}

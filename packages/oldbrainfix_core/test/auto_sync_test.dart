import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'test_host.dart';

DateTime Function() ticking() {
  var now = DateTime.utc(2026, 3, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

Matcher failsWith(SyncFailure failure) =>
    throwsA(isA<SyncException>().having((e) => e.failure, 'failure', failure));

void main() {
  group('a paired Device syncing on its own', () {
    late Store desk;
    late Store phone;
    late TestHost host;

    /// Pairs the phone with the desk once by code, and the desk
    /// remembers the phone as the desktop does after a session.
    Future<PairedDevice> pair() async {
      await lanSync(phone, decodePairCode(host.code())!, name: 'Phone');
      desk.rememberDevice(
        PairedDevice(
          id: phone.deviceId,
          name: 'Phone',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: desk.clock(),
        ),
      );
      return phone.pairedDevice(desk.deviceId)!;
    }

    setUp(() async {
      final clock = ticking();
      desk = Store.inMemory(clock: clock);
      phone = Store.inMemory(clock: clock);
      host = TestHost(desk);
      await host.start();
    });

    tearDown(() => host.stop());

    test('the proof covers the host and the nonce, as the desktop reads '
        'it', () {
      expect(
        String.fromCharCodes(pairedProofBytes('desk', 'n0nce')),
        '["auto","desk","n0nce"]',
      );
    });

    test('syncs without the PIN once paired, a fresh nonce each '
        'time', () async {
      final device = await pair();
      Notes(desk).capture('Done on the desk');
      final result = await pairedSync(
        phone,
        device,
        host: '127.0.0.1',
        port: host.port,
        name: 'Phone',
      );
      expect(result.session.applied, isNotEmpty);
      expect(
        [for (final n in Notes(phone).all()) n.text],
        ['Done on the desk'],
      );
      Notes(phone).capture('From the phone');
      await pairedSync(
        phone,
        device,
        host: '127.0.0.1',
        port: host.port,
        name: 'Phone',
      );
      expect(Notes(desk).all().map((n) => n.text), contains('From the phone'));
      expect(host.nonces, hasLength(2));
      final remembered = phone.pairedDevice(desk.deviceId)!;
      expect(remembered.port, host.port);
      expect(remembered.fingerprint, device.fingerprint);
    });

    test('a host that does not know this Device, or switched it off, '
        'says no', () async {
      final device = await pair();
      desk.setAutoSync(phone.deviceId, false);
      Notes(phone).capture('Not for the desk');
      await expectLater(
        pairedSync(
          phone,
          device,
          host: '127.0.0.1',
          port: host.port,
          name: 'Phone',
        ),
        failsWith(SyncFailure.declined),
      );
      desk.setAutoSync(phone.deviceId, true);
      desk.forgetDevice(phone.deviceId);
      await expectLater(
        pairedSync(
          phone,
          device,
          host: '127.0.0.1',
          port: host.port,
          name: 'Phone',
        ),
        failsWith(SyncFailure.declined),
      );
      expect(Notes(desk).all(), isEmpty);
    });

    test('another certificate than the pinned one gets nothing, not even '
        'a request', () async {
      final device = await pair();
      final before = host.requests;
      final other = PairedDevice(
        id: device.id,
        name: device.name,
        fingerprint: '00' * 32,
        host: device.host,
        port: device.port,
        lastSync: device.lastSync,
      );
      await expectLater(
        pairedSync(
          phone,
          other,
          host: '127.0.0.1',
          port: host.port,
          name: 'Phone',
        ),
        failsWith(SyncFailure.wrongHost),
      );
      expect(host.requests, before);
    });

    test('a Device that never hosted, or a public address, is not '
        'tried', () async {
      final device = await pair();
      final joinedOnly = PairedDevice(
        id: device.id,
        name: device.name,
        fingerprint: '',
        host: null,
        port: null,
        lastSync: device.lastSync,
      );
      final before = host.requests;
      await expectLater(
        pairedSync(
          phone,
          joinedOnly,
          host: '127.0.0.1',
          port: host.port,
          name: 'Phone',
        ),
        failsWith(SyncFailure.wrongHost),
      );
      await expectLater(
        pairedSync(phone, device, host: '8.8.8.8', port: 443, name: 'Phone'),
        failsWith(SyncFailure.notPrivate),
      );
      expect(host.requests, before);
    });

    test('only a verified key of a paired Device that syncs on its own is '
        'trusted', () async {
      await pair();
      final proof = phone.signingKey.sign(
        pairedProofBytes(desk.deviceId, 'n1'),
      );
      bool trusted(String nonce, String? proof) => trustsPairedJoiner(
        desk,
        device: phone.deviceId,
        nonce: nonce,
        proof: proof,
      );
      expect(trusted('n1', proof), isTrue);
      expect(trusted('n2', proof), isFalse, reason: 'another nonce');
      expect(trusted('n1', null), isFalse);
      final impostor = Store.inMemory(clock: ticking());
      expect(
        trusted(
          'n1',
          impostor.signingKey.sign(pairedProofBytes(desk.deviceId, 'n1')),
        ),
        isFalse,
        reason: 'a claimed id',
      );
    });
  });

  group('the paired Devices', () {
    test('sync on their own unless switched off, and forgetting one '
        'clears its switch', () {
      final store = Store.inMemory(clock: ticking());
      PairedDevice device(String id, String fingerprint) => PairedDevice(
        id: id,
        name: id,
        fingerprint: fingerprint,
        host: '192.168.1.5',
        port: 4000,
        lastSync: DateTime.utc(2026),
      );
      store.rememberDevice(device('desk', 'ab' * 32));
      store.rememberDevice(device('phone', ''));
      expect(store.autoSyncOn('desk'), isTrue);
      expect([for (final d in store.autoSyncDevices()) d.id], ['desk']);
      store.setAutoSync('desk', false);
      expect(store.autoSyncOn('desk'), isFalse);
      expect(store.autoSyncDevices(), isEmpty);
      store.forgetDevice('desk');
      expect(store.autoSyncOn('desk'), isTrue);
      expect(store.allEntries(), isEmpty, reason: 'never synced');
    });

    test('remember Conflicts a background sync brought until they are '
        'announced', () {
      final store = Store.inMemory(clock: ticking());
      expect(store.conflictsArrived, isFalse);
      store.conflictsArrived = true;
      expect(store.conflictsArrived, isTrue);
      store.conflictsArrived = false;
      expect(store.conflictsArrived, isFalse);
    });
  });

  group('the schedule', () {
    final t0 = DateTime.utc(2026, 3, 1, 9);
    DateTime at(int seconds) => t0.add(Duration(seconds: seconds));

    test('syncs every Device at once on start, then every minute', () {
      final s = AutoSyncSchedule()..wantAll(['a', 'b'], t0);
      expect(s.take(['a', 'b'], t0), ['a', 'b']);
      expect(s.take(['a', 'b'], at(1)), isEmpty);
      expect(s.next(['a', 'b']), at(60));
      expect(s.take(['a', 'b'], at(60)), ['a', 'b']);
    });

    test('waits 5 s after the last change before sending it', () {
      final s = AutoSyncSchedule()
        ..changed(t0)
        ..changed(at(3));
      expect(s.take(['a'], at(5)), isEmpty);
      expect(s.next(['a']), at(8));
      expect(s.take(['a'], at(8)), ['a']);
      expect(s.take(['a'], at(9)), isEmpty, reason: 'sent');
    });

    test('a change during a sync goes out after it', () {
      final s = AutoSyncSchedule()..wantAll(['a'], t0);
      expect(s.take(['a'], t0), ['a']);
      s.changed(at(1));
      s.succeeded('a', at(2));
      expect(s.take(['a'], at(6)), ['a']);
    });

    test('leaving sends what changed at once', () {
      final s = AutoSyncSchedule()..changed(t0);
      s.flush(at(1));
      expect(s.take(['a'], at(1)), ['a']);
    });

    test('a Device showing up syncs, unless it just did', () {
      final s = AutoSyncSchedule()..found('a', t0);
      expect(s.take(['a', 'b'], t0), ['a']);
      s.succeeded('a', at(1));
      s.found('a', at(30));
      expect(s.take(['a'], at(30)), isEmpty, reason: 'discovery repeats');
      s.found('a', at(61));
      expect(s.take(['a'], at(61)), ['a']);
    });

    test('a Device showing up after a failed try syncs at once', () {
      final s = AutoSyncSchedule()..wantAll(['a'], t0);
      expect(s.take(['a'], t0), ['a']);
      s.found('a', at(10));
      expect(s.take(['a'], at(10)), ['a']);
    });

    test('nothing is due without a reason', () {
      final s = AutoSyncSchedule();
      expect(s.next(['a']), isNull);
      expect(s.take(['a'], t0), isEmpty);
    });
  });
}

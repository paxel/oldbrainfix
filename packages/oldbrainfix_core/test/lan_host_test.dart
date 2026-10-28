import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A clock that moves a millisecond at every look, so a session stays
/// well within a nonce's two minutes; [at] moves it on by hand.
class TestClock {
  var now = DateTime.utc(2026, 3, 1, 9);

  DateTime call() => now = now.add(const Duration(milliseconds: 1));

  void pass(Duration d) => now = now.add(d);
}

/// A raw request to the host on [port], as a joiner of another build or
/// an intruder sends it: the status and the text that came back.
Future<(int, String, HttpHeaders)> call(
  int port,
  String method,
  String path, {
  Map<String, String> headers = const {},
  List<int>? body,
}) async {
  final client = HttpClient()..badCertificateCallback = (_, _, _) => true;
  try {
    final req = await client.openUrl(
      method,
      Uri(scheme: 'https', host: '127.0.0.1', port: port, path: path),
    );
    headers.forEach(req.headers.set);
    if (body != null) req.add(body);
    final res = await req.close();
    final text = await utf8.decodeStream(res);
    return (res.statusCode, text, res.headers);
  } finally {
    client.close(force: true);
  }
}

Matcher fails(SyncFailure failure) =>
    throwsA(isA<SyncException>().having((e) => e.failure, 'failure', failure));

List<String> texts(Store store) =>
    [for (final n in Notes(store).all()) n.text]..sort();

void main() {
  group('the host identity', () {
    test('is made once and kept, its fingerprint over the DER', () {
      final store = Store.inMemory(clock: TestClock().call);
      addTearDown(store.close);
      expect(store.heldHostIdentity, isNull);
      final made = store.hostIdentity();
      final again = store.hostIdentity();
      expect(again.certDer, made.certDer);
      expect(again.keyDer, made.keyDer);
      expect(made.fingerprint, sha256.convert(made.certDer).bytes);
      expect(
        store.localSetting(HostIdentities.certKey),
        base64.encode(made.certDer),
      );
    });

    test('the same bytes and moment make the same certificate', () {
      Uint8List counted(int n) =>
          Uint8List.fromList([for (var i = 0; i < n; i++) i + 7]);
      final at = DateTime.utc(2026, 10, 27, 12);
      final one = makeHostIdentity(now: at, random: counted);
      final two = makeHostIdentity(now: at, random: counted);
      expect(one.certDer, two.certDer);
      expect(one.keyDer, two.keyDer);
      final other = makeHostIdentity(now: at);
      expect(other.certDer, isNot(one.certDer));
    });
  });

  group('a LAN address', () {
    InternetAddress ip(String s) => InternetAddress(s);

    test('is Wi-Fi or Ethernet before a mobile network', () {
      expect(
        pickLanAddress([
          ('lo', ip('127.0.0.1')),
          ('rmnet_data0', ip('10.12.0.7')),
          ('wlan0', ip('192.168.1.23')),
        ])?.address,
        '192.168.1.23',
      );
      expect(
        pickLanAddress([
          ('rmnet0', ip('10.12.0.7')),
          ('lo', ip('127.0.0.1')),
          ('wwan0', ip('100.64.0.2')),
        ]),
        isNull,
      );
      expect(
        pickLanAddress([('foo0', ip('172.16.0.4'))])?.address,
        '172.16.0.4',
      );
    });
  });

  group('a phone hosts', () {
    late TestClock clock;
    late Store a;
    late Store b;
    final asked = <JoinAsk>[];
    var trust = true;

    setUp(() {
      clock = TestClock();
      a = Store.inMemory(clock: clock.call);
      b = Store.inMemory(clock: clock.call);
      asked.clear();
      trust = true;
      addTearDown(a.close);
      addTearDown(b.close);
    });

    Future<PairingHost> pairing(Store store, {Invite? inviting}) async {
      final host = await PairingHost.start(
        store,
        name: 'Phone A',
        bind: InternetAddress.loopbackIPv4,
        inviting: inviting,
        ask: (ask, _) async {
          asked.add(ask);
          return trust;
        },
      );
      addTearDown(host.stop);
      return host;
    }

    Future<SyncResult> join(PairingHost host, Store joiner) => lanSync(
      joiner,
      decodePairCode(host.pairCode(typed: true))!,
      name: 'Phone B',
    );

    test('Pairing: entries both ways, the user asked, each remembering '
        "the other's certificate", () async {
      Notes(a).capture('From A');
      Notes(b).capture('From B');
      final sessions = <SyncSession>[];
      final host = await PairingHost.start(
        a,
        name: 'Phone A',
        bind: InternetAddress.loopbackIPv4,
        ask: (ask, profile) async {
          asked.add(ask);
          expect(profile, isNull);
          return true;
        },
        onSession: sessions.add,
      );
      addTearDown(host.stop);
      final result = await join(host, b);
      expect(result.session.name, 'Phone A');
      expect([for (final x in asked) x.device], [b.deviceId]);
      expect(sessions.single.peer, b.deviceId);
      for (final store in [a, b]) {
        expect(texts(store), ['From A', 'From B']);
      }
      final onA = a.pairedDevice(b.deviceId)!;
      expect(onA.name, 'Phone B');
      expect(onA.fingerprint, hex(b.hostIdentity().fingerprint));
      final onB = b.pairedDevice(a.deviceId)!;
      expect(onB.fingerprint, hex(a.hostIdentity().fingerprint));
      // Both sync on their own now, each with the other.
      expect([for (final d in a.autoSyncDevices()) d.id], [b.deviceId]);
      expect([for (final d in b.autoSyncDevices()) d.id], [a.deviceId]);

      // Paired before, it is not asked about again.
      await host.stop();
      final next = await pairing(a);
      await join(next, b);
      expect(asked, hasLength(1));
    });

    test('the user saying no sends nothing back and takes nothing', () async {
      Notes(a).capture('From A');
      Notes(b).capture('From B');
      trust = false;
      final host = await pairing(a);
      await expectLater(join(host, b), fails(SyncFailure.declined));
      expect(texts(a), ['From A']);
      expect(texts(b), ['From B']);
      expect(a.pairedDevices(), isEmpty);
    });

    test('after Pairing either one joins the other on its own', () async {
      final host = await pairing(a);
      await join(host, b);
      await host.stop();

      Notes(a).capture('A later');
      final onA = await PairedHost.start(
        a,
        name: 'Phone A',
        bind: InternetAddress.loopbackIPv4,
      );
      addTearDown(onA.stop);
      await pairedSync(
        b,
        b.pairedDevice(a.deviceId)!,
        host: '127.0.0.1',
        port: onA.port,
        name: 'Phone B',
      );
      expect(texts(b), ['A later']);
      await onA.stop();

      // The other way round: the Device that joined hosts, and the one
      // that showed the code joins it by the certificate it was told.
      Notes(b).capture('B later');
      final onB = await PairedHost.start(
        b,
        name: 'Phone B',
        bind: InternetAddress.loopbackIPv4,
      );
      addTearDown(onB.stop);
      final result = await pairedSync(
        a,
        a.pairedDevice(b.deviceId)!,
        host: '127.0.0.1',
        port: onB.port,
        name: 'Phone A',
      );
      expect(result.session.peer, b.deviceId);
      expect(texts(a), ['A later', 'B later']);
      expect(a.pairedDevice(b.deviceId)?.port, onB.port);
      // The paired host keeps its port from run to run.
      expect(b.localSetting(LanServing.pairedPortKey), '${onB.port}');
    });

    test('the paired host answers paired Devices only', () async {
      final stranger = Store.inMemory(clock: clock.call);
      addTearDown(stranger.close);
      final host = await pairing(a);
      await join(host, b);
      await host.stop();
      final onA = await PairedHost.start(
        a,
        name: 'Phone A',
        bind: InternetAddress.loopbackIPv4,
      );
      addTearDown(onA.stop);
      final posing = PairedDevice(
        id: a.deviceId,
        name: 'A',
        fingerprint: hex(a.hostIdentity().fingerprint),
        host: '127.0.0.1',
        port: onA.port,
        lastSync: clock.now,
      );
      await expectLater(
        pairedSync(
          stranger,
          posing,
          host: '127.0.0.1',
          port: onA.port,
          name: 'X',
        ),
        fails(SyncFailure.declined),
      );
      // A PIN gets nobody in.
      final (status, _, _) = await call(
        onA.port,
        'GET',
        '/vector',
        headers: {pinHeader: ''},
      );
      expect(status, 403);
      // A paired Device syncs in its own name only.
      final (_, text, _) = await call(onA.port, 'GET', '/challenge');
      final nonce = (jsonDecode(text) as Map)['nonce'] as String;
      final (posed, said, _) = await call(
        onA.port,
        'POST',
        '/sync',
        headers: {
          formatHeader: encodeFormats(ownFormats),
          deviceHeader: b.deviceId,
          nonceHeader: nonce,
          proofHeader: b.signingKey.sign(pairedProofBytes(a.deviceId, nonce)),
        },
        body: utf8.encode(
          jsonEncode(
            syncRequest(stranger, const {}, name: 'X', hostDevice: a.deviceId),
          ),
        ),
      );
      expect((posed, said), (403, 'declined'));
      expect(a.pairedDevice(stranger.deviceId), isNull);
      // Switched off, the paired Device is declined too.
      a.setAutoSync(b.deviceId, false);
      await expectLater(
        pairedSync(
          b,
          b.pairedDevice(a.deviceId)!,
          host: '127.0.0.1',
          port: onA.port,
          name: 'Phone B',
        ),
        fails(SyncFailure.declined),
      );
    });

    test('a nonce holds two minutes', () async {
      final host = await pairing(a);
      await join(host, b);
      await host.stop();
      final onA = await PairedHost.start(
        a,
        name: 'Phone A',
        bind: InternetAddress.loopbackIPv4,
      );
      addTearDown(onA.stop);
      Future<int> vector(String nonce) async {
        final (status, _, _) = await call(
          onA.port,
          'GET',
          '/vector',
          headers: {
            formatHeader: encodeFormats(ownFormats),
            deviceHeader: b.deviceId,
            nonceHeader: nonce,
            proofHeader: b.signingKey.sign(pairedProofBytes(a.deviceId, nonce)),
          },
        );
        return status;
      }

      final (_, text, _) = await call(onA.port, 'GET', '/challenge');
      final nonce = (jsonDecode(text) as Map)['nonce'] as String;
      expect(nonce, hasLength(64));
      expect(await vector(nonce), 200);
      expect(await vector('made-up'), 403);
      clock.pass(challengeTtl);
      expect(await vector(nonce), 403);
    });

    test('wrong PINs lock the address out, the right one included', () async {
      final host = await pairing(a);
      final port = host.host.port;
      // Asking for a nonce is no PIN try, and is not answered here.
      for (var i = 0; i < pinFailuresPerAddress + 1; i++) {
        final (status, _, _) = await call(port, 'GET', '/challenge');
        expect(status, 404);
      }
      for (var i = 0; i < pinFailuresPerAddress; i++) {
        final (status, text, _) = await call(
          port,
          'GET',
          '/vector',
          headers: {pinHeader: '000000'},
        );
        expect((status, text), (403, ''));
      }
      final (status, text, _) = await call(
        port,
        'GET',
        '/vector',
        headers: {pinHeader: host.host.pin},
      );
      expect((status, text), (403, 'locked'));
      await expectLater(join(host, b), fails(SyncFailure.locked));
      expect(host.lockedOut, isFalse, reason: 'one address only');
    });

    test('a joiner of a format this build does not speak is told so, and '
        'kept to be announced once', () async {
      final host = await pairing(a);
      final (status, text, headers) = await call(
        host.host.port,
        'GET',
        '/vector',
        headers: {
          pinHeader: host.host.pin,
          formatHeader: '1',
          deviceHeader: b.deviceId,
        },
      );
      expect(status, 403);
      expect(jsonDecode(text), {
        'refusal': 'format',
        'formats': encodeFormats(ownFormats),
      });
      expect(headers.value(formatHeader), encodeFormats(ownFormats));
      expect(a.takeUpdateNotices(), [(b.deviceId, UpdateNeeded.there)]);
    });

    test('only a blob the log mentions is taken', () {
      final (refused, _) = a.serve(
        HostRequest(
          method: 'POST',
          path: '/blob/photo/${'ab' * 32}',
          body: Uint8List.fromList([9]),
        ),
        name: 'Phone A',
      );
      expect(refused.status, 404);
      expect(a.blob('photo', 'ab' * 32), isNull);
    });

    test('a phone invites another person to a Shared Area', () async {
      a.applyProfileDefaults(name: 'Ann');
      b.applyProfileDefaults(name: 'Ben');
      final house = Places(a).addArea('Household');
      Notes(a).capture('Private');
      final area = a.share(house.id);
      final profiles = <ProfileChoice?>[];
      final host = await PairingHost.start(
        a,
        name: 'Phone A',
        bind: InternetAddress.loopbackIPv4,
        inviting: Invite(area, role: AreaRole.member),
        ask: (ask, profile) async {
          profiles.add(profile);
          return true;
        },
      );
      addTearDown(host.stop);
      final result = await join(host, b);
      expect(result.session.sharedArea, area);
      expect(profiles.single?.profile.id, b.ownProfile?.id);
      expect([for (final x in Places(b).areas()) x.id], contains(area));
      expect(texts(b), isEmpty, reason: 'nothing outside the Area');
      expect(b.pairedDevice(a.deviceId)?.shared, isTrue);
      final onA = a.pairedDevice(b.deviceId)!;
      expect(onA.shared, isTrue);
      expect(onA.fingerprint, hex(b.hostIdentity().fingerprint));
    });
  });

  group('the certificate a joiner names', () {
    late Store a;
    late Store b;
    late Store c;

    setUp(() {
      final clock = TestClock();
      a = Store.inMemory(clock: clock.call);
      b = Store.inMemory(clock: clock.call);
      c = Store.inMemory(clock: clock.call);
      for (final s in [a, b, c]) {
        addTearDown(s.close);
      }
    });

    Map<String, Object?> request(Store joiner, String pin) => syncRequest(
      joiner,
      a.versionVector(),
      name: 'Phone',
      hostDevice: a.deviceId,
      pin: pin,
    );

    test('is kept only with its proof for this host and PIN', () {
      b.hostIdentity();
      final fingerprint = hex(b.hostIdentity().fingerprint);
      expect(request(b, '246810')['fingerprint'], fingerprint);
      final done = answerSync(
        a,
        request(b, '246810'),
        name: 'A',
        pin: '246810',
      )!;
      expect(done.session.fingerprint, fingerprint);
      // A proof for another PIN proves nothing here.
      final other = answerSync(
        a,
        request(b, '000000'),
        name: 'A',
        pin: '246810',
      )!;
      expect(other.session.fingerprint, isNull);
    });

    test('is not kept for another Device named in the body', () {
      // A met C before, so C's key is pinned here.
      answerSync(a, request(c, '246810'), name: 'A', pin: '246810');
      b.hostIdentity();
      final posing = {...request(b, '246810'), 'deviceId': c.deviceId};
      final done = answerSync(a, posing, name: 'A', pin: '246810')!;
      expect(done.session.peer, c.deviceId);
      expect(done.session.fingerprint, isNull);
    });

    test('a joiner remembered keeps the address it hosted at', () {
      final fingerprint = 'ab' * 32;
      a.rememberDevice(
        PairedDevice(
          id: b.deviceId,
          name: 'B',
          fingerprint: fingerprint,
          host: '192.168.1.7',
          port: 4000,
          lastSync: DateTime.utc(2026),
        ),
      );
      SyncSession session(String fp) => SyncSession(
        peer: b.deviceId,
        name: 'B',
        applied: const [],
        moment: null,
        report: ImportReport(),
        fingerprint: fp,
      );
      a.rememberJoiner(session(fingerprint));
      expect(a.pairedDevice(b.deviceId)?.host, '192.168.1.7');
      expect(a.pairedDevice(b.deviceId)?.port, 4000);
      a.rememberJoiner(session('cd' * 32));
      expect(a.pairedDevice(b.deviceId)?.host, isNull);
      expect(a.pairedDevice(b.deviceId)?.fingerprint, 'cd' * 32);
    });
  });
}

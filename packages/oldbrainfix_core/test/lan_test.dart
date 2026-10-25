import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'test_host.dart';

DateTime Function() ticking() {
  var now = DateTime.utc(2026, 3, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

void main() {
  group('pair codes', () {
    // The same values as Catlog's, whose phones and desk share them.
    final fp = [for (var i = 0; i < 32; i++) (i * 7 + 3) & 0xff];

    test('carry host, port, PIN and the fingerprint', () {
      final full = encodePairCode('192.168.1.23', 45231, '042817', fp);
      expect(
        full,
        'r2m02_5xgnw_0aeg8_3188h_g7s65_mt3pg_j9a1b_nwsbc_edx83_24fjt_et9ax_'
        'jq70c_fkpnv_g',
      );
      final typed = encodePairCode(
        '192.168.1.23',
        45231,
        '042817',
        fp,
        typed: true,
      );
      expect(typed, 'r2m02_5xgnw_0aeg8_3188h_g7s65_mt0');
      final info = decodePairCode(full)!;
      expect(info.host, '192.168.1.23');
      expect(info.port, 45231);
      expect(info.pin, '042817');
      expect(info.fingerprint, fp);
      expect(pairCodeLines(typed), ['r2m02_5xgnw_0aeg8', '3188h_g7s65_mt0']);
    });

    test('forgive how they are typed, and refuse what is no code', () {
      final info = decodePairCode('R2M02 5XGNW 0AEG8 3188H G7S65 MTO')!;
      expect(info.fingerprint, fp.take(8));
      expect(info.pin, '042817');
      expect(decodePairCode('nope'), isNull);
      expect(decodePairCode(''), isNull);
      expect(decodePairCode('r2m02_5xgnw_0aeg8'), isNull, reason: 'no print');
    });

    test('only private addresses are partners', () {
      for (final host in [
        '10.0.0.5',
        '172.16.0.1',
        '172.31.255.1',
        '192.168.1.23',
        '169.254.3.4',
        '127.0.0.1',
      ]) {
        expect(isPrivateHost(host), isTrue, reason: host);
      }
      for (final host in ['8.8.8.8', '172.32.0.1', '192.169.0.1', '::1', 'x']) {
        expect(isPrivateHost(host), isFalse, reason: host);
      }
    });

    test('a fingerprint matches in full or by its first bytes', () {
      expect(fingerprintMatches(fp, fp), isTrue);
      expect(fingerprintMatches(fp, fp.take(8).toList()), isTrue);
      expect(fingerprintMatches(fp, fp.take(4).toList()), isFalse);
      expect(fingerprintMatches(fp, [...fp.take(7), 0]), isFalse);
    });
  });

  group('joining a host over TLS', () {
    late Store desk;
    late Store phone;
    late TestHost host;

    setUp(() async {
      final clock = ticking();
      desk = Store.inMemory(clock: clock);
      phone = Store.inMemory(clock: clock);
      host = TestHost(desk);
      await host.start();
    });

    tearDown(() => host.stop());

    test('syncs entries and blobs both ways and pairs the host', () async {
      Notes(desk).capture(
        'From the desk',
        photos: [
          Uint8List.fromList([1]),
        ],
      );
      Notes(phone).capture(
        'From the phone',
        photos: [
          Uint8List.fromList([2]),
        ],
      );
      final result = await lanSync(
        phone,
        decodePairCode(host.code())!,
        name: 'Phone',
      );
      expect(result.blobsReceived, 1);
      expect(result.blobsSent, 1);
      expect(result.session.name, 'Desk');
      expect(result.session.moment, isNotNull);
      for (final store in [phone, desk]) {
        expect([for (final n in Notes(store).all()) n.text]..sort(), [
          'From the desk',
          'From the phone',
        ]);
        expect(store.missingBlobs(), isEmpty);
      }
      final paired = phone.pairedDevice(desk.deviceId)!;
      expect(paired.name, 'Desk');
      expect(paired.fingerprint, hex(certificateFingerprint(testCertDer)));
      expect(paired.port, host.port);
      expect(phone.pinnedKey(desk.deviceId)?.trust, KeyTrust.verified);
      expect(desk.pinnedKey(phone.deviceId)?.trust, KeyTrust.verified);
      // Each now knows the other holds all it holds, for bundles.
      expect(phone.heldBy(desk.deviceId), phone.versionVector());
      expect(desk.heldBy(phone.deviceId), desk.versionVector());
      expect(host.sessions.single.name, 'Phone');
      // The joiner proves it holds its key, for this host and PIN only.
      final proof = host.bodies.single['proof'] as String?;
      bool proofFor(String pin) => verifySignature(
        phone.signingKey.publicKey,
        pairingProofBytes(desk.deviceId, pin),
        proof,
      );
      expect(proofFor(host.pin), isTrue);
      expect(proofFor('000000'), isFalse);

      final again = await lanSync(
        phone,
        decodePairCode(host.code(typed: false))!,
        name: 'Phone',
      );
      expect(again.session.applied, isEmpty);
      expect(again.entriesSent, 0);
    });

    test('a wrong PIN is refused', () async {
      final info = decodePairCode(host.code())!;
      final wrong = PairInfo(info.host, info.port, '000000', info.fingerprint);
      await expectLater(
        lanSync(phone, wrong, name: 'Phone'),
        throwsA(
          isA<SyncException>().having(
            (e) => e.failure,
            'failure',
            SyncFailure.wrongPin,
          ),
        ),
      );
      expect(desk.allEntries(), isEmpty);
    });

    test('another certificate than the code names is refused', () async {
      final info = decodePairCode(host.code())!;
      final other = PairInfo(info.host, info.port, info.pin, List.filled(8, 0));
      await expectLater(
        lanSync(phone, other, name: 'Phone'),
        throwsA(
          isA<SyncException>().having(
            (e) => e.failure,
            'failure',
            SyncFailure.wrongHost,
          ),
        ),
      );
      expect(phone.pairedDevices(), isEmpty);
    });

    test('a public address is refused before anything is sent', () async {
      final info = PairInfo('8.8.8.8', 443, '123456', List.filled(8, 1));
      await expectLater(
        lanSync(phone, info, name: 'Phone'),
        throwsA(
          isA<SyncException>().having(
            (e) => e.failure,
            'failure',
            SyncFailure.notPrivate,
          ),
        ),
      );
    });
  });
}

import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'sync_test.dart' show device, ticking;

Uint8List _hex(String s) => fromHex(s.replaceAll(RegExp(r'\s'), ''));

/// [other] is another person's Device, paired by invite, its key met in
/// person.
void _pairByInvite(Store store, Store other, {String name = 'Ben'}) {
  store.rememberDevice(
    PairedDevice(
      id: other.deviceId,
      name: name,
      fingerprint: '',
      host: null,
      port: null,
      lastSync: DateTime.utc(2026),
      shared: true,
    ),
  );
  store.learnKeys([other.ownKeyRecord], verifiedDevice: other.deviceId);
}

void main() {
  group('keys to encrypt to', () {
    test('HKDF-SHA256 gives RFC 5869 test case 1', () {
      final okm = hkdfSha256(
        _hex('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b'),
        salt: _hex('000102030405060708090a0b0c'),
        info: _hex('f0f1f2f3f4f5f6f7f8f9'),
        length: 42,
      );
      expect(
        hex(okm),
        '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf'
        '34007208d5b887185865',
      );
    });

    test('X25519 gives the RFC 7748 shared secret', () {
      final alice = _hex(
        '77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a',
      );
      final bob = _hex(
        '5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb',
      );
      expect(
        hex(boxPublicOf(alice)),
        '8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a',
      );
      expect(
        hex(x25519(alice, boxPublicOf(bob))!),
        '4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742',
      );
      expect(x25519(alice, Uint8List(32)), isNull, reason: 'low order');
    });

    test('a key record carries its X25519 key, signed', () {
      final key = SigningKey.generate();
      final record = KeyRecord.make(key);
      expect(record.boxKey, boxPublicOf(boxSecretOf(key.seed)));
      final back = KeyRecord.fromJson(jsonDecode(jsonEncode(record.toJson())));
      expect(back?.boxKey, record.boxKey);
      expect(record.toJson().keys, ['box', 'boxSig', 'device', 'key', 'sig']);
      final swapped = KeyRecord(
        record.device,
        record.publicKey,
        record.signature,
        box: boxPublicOf(boxSecretOf(SigningKey.generate().seed)),
        boxSignature: record.boxSignature,
      );
      expect(swapped.valid, isTrue);
      expect(swapped.boxKey, isNull, reason: 'not signed by the key');
      final bare = KeyRecord(record.device, record.publicKey, record.signature);
      expect(bare.boxKey, isNull);
      expect(bare.toJson().keys, ['device', 'key', 'sig']);
    });

    test('a key held without its X25519 key takes it, trust kept', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final full = ben.ownKeyRecord;
      final bare = KeyRecord(full.device, full.publicKey, full.signature);
      ann.learnKeys([bare], verifiedDevice: ben.deviceId);
      expect(ann.pinnedKey(ben.deviceId)?.record.boxKey, isNull);
      ann.learnKeys([full]);
      expect(ann.pinnedKey(ben.deviceId)?.record.boxKey, full.boxKey);
      expect(ann.pinnedKey(ben.deviceId)?.trust, KeyTrust.verified);
    });
  });

  group('sending', () {
    test('a Note goes out signed, under the same id every time', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final note = Notes(ann).capture('Dentist on Friday');
      Notes(ann).amend(note.id, 'Bring the card');
      final routines = Routines(ann);
      routines.spawn(
        note.id,
        Schedule(start: Day(2026, 3, 6), times: [ClockTime(9, 30)]),
      );
      routines.spawn(
        note.id,
        Schedule(
          start: Day(2026, 3, 2),
          times: [ClockTime(8, 0), ClockTime(20, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );

      final first = sendNote(ann, note.id);
      final text = utf8.decode(first);
      expect(text, contains('Dentist on Friday'), reason: 'signed only');
      final ben = device(clock, 'Ben');
      final opened = openSentItem(ben, first);
      expect(opened.encrypted, isFalse);
      expect(opened.trust, SenderTrust.unknown);
      expect(opened.accepted, isNull);
      final item = opened.item;
      expect(item.version, 1);
      expect(item.name, 'Ann');
      expect(item.from.device, ann.deviceId);
      expect(item.text, 'Dentist on Friday');
      expect([for (final s in item.sections) s.text], ['Bring the card']);
      expect(item.routines, hasLength(2));
      expect(
        {for (final r in item.routines) r.schedule.times.join(',')},
        {'09:30', '08:00,20:00'},
      );

      final again = openSentItem(ben, sendNote(ann, note.id)).item;
      expect(again.id, item.id);
      expect(again.version, 2);
      final other = Notes(ann).capture('Other');
      expect(
        openSentItem(ben, sendNote(ann, other.id)).item.id,
        isNot(item.id),
      );
    });

    test('the Capture Context stays at home', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final note = Notes(ann).capture(
        'Pick up',
        sourceApp: const SourceApp('com.example.chat', label: 'Chat'),
      );
      Notes(ann).locate(note.id, Coordinates(52.52, 13.405));
      Notes(ann).attachContext(note.id, calendarEvents: ['Board meeting']);
      final text = utf8.decode(sendNote(ann, note.id));
      for (final secret in ['com.example', 'Chat', '52.52', 'Board']) {
        expect(text, isNot(contains(secret)));
      }
    });

    test('a known person gets it encrypted, nobody else can read it', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final annDesk = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final eve = device(clock, 'Eve');
      _pairByInvite(ann, ben);
      // Her own Device is no recipient.
      ann.rememberDevice(
        PairedDevice(
          id: annDesk.deviceId,
          name: 'Desk',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: DateTime.utc(2026),
        ),
      );
      ann.learnKeys([annDesk.ownKeyRecord]);
      final recipients = sentItemRecipients(ann);
      expect([for (final r in recipients) r.name], ['Ben']);
      expect(recipients.single.keys.single, ben.ownKeyRecord.boxKey);

      final note = Notes(ann).capture('Secret plan');
      final file = sendNote(ann, note.id, to: recipients.single);
      expect(utf8.decode(file), isNot(contains('Secret')));
      expect(utf8.decode(file), isNot(contains('Ann')));
      expect(looksLikeSentItem(file), isTrue);

      ben.learnKeys([ann.ownKeyRecord], verifiedDevice: ann.deviceId);
      final opened = openSentItem(ben, file);
      expect(opened.encrypted, isTrue);
      expect(opened.trust, SenderTrust.verified);
      expect(opened.item.text, 'Secret plan');
      expect(() => openSentItem(eve, file), throwsA(isA<SentItemNotForUs>()));
    });

    test('a key that is not known is not offered', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final record = ben.ownKeyRecord;
      _pairByInvite(ann, ben);
      // As a build before X25519 keys pinned it.
      ann.forgetDevice(ben.deviceId);
      ann.setLocalSetting(
        'key:${ben.deviceId}',
        jsonEncode(
          PinnedKey(
            KeyRecord(record.device, record.publicKey, record.signature),
            KeyTrust.tofu,
          ).toJson(),
        ),
      );
      _pairByInvite(ann, device(clock, 'Cid'), name: 'Cid');
      expect([for (final r in sentItemRecipients(ann)) r.name], ['Cid']);
    });

    test('what is changed, forged or no Sent Item is refused', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final note = Notes(ann).capture('Buy milk');
      final text = utf8.decode(sendNote(ann, note.id));
      final changed = utf8.encode(text.replaceFirst('milk', 'beer'));
      expect(() => openSentItem(ben, changed), throwsA(isA<NotASentItem>()));
      for (final junk in [
        utf8.encode('not json'),
        utf8.encode('{"format":1}'),
        utf8.encode('[]'),
        [0x50, 0x4b, 3, 4],
        Uint8List(maxSentItemBytes + 1),
      ]) {
        expect(() => openSentItem(ben, junk), throwsA(isA<NotASentItem>()));
      }
      // Signed by another key than the one it names.
      final mallory = device(clock, 'Mallory');
      final forged = encodeSentItem(
        SentItem(
          id: 'x',
          version: 1,
          from: ann.ownKeyRecord,
          name: 'Ann',
          text: 'Cancelled',
        ),
        mallory.signingKey,
      );
      expect(() => openSentItem(ben, forged), throwsA(isA<NotASentItem>()));
      expect(looksLikeSentItem([0x50, 0x4b, 3, 4]), isFalse);
      expect(looksLikeSentItem(utf8.encode(' \n{')), isTrue);
    });
  });

  group('receiving', () {
    test('accepting lands it in Unsorted with its alarms on, once', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final note = Notes(ann).capture('Dentist');
      Notes(ann).amend(note.id, 'Room 3');
      final sent = Routines(ann).spawn(
        note.id,
        Schedule(start: Day(2026, 3, 6), times: [ClockTime(9, 30)]),
      );
      final file = sendNote(ann, note.id);

      final before = ben.allEntries().length;
      final opened = openSentItem(ben, file);
      expect(ben.allEntries(), hasLength(before), reason: 'preview only');

      final id = acceptSentItem(ben, opened.item);
      final got = Notes(ben).byId(id)!;
      expect(got.text, 'Dentist');
      expect(got.isUnsorted, isTrue);
      expect(Notes(ben).unsorted().map((n) => n.id), [id]);
      expect([for (final s in Notes(ben).sections(id)) s.text], ['Room 3']);
      final routine = Routines(ben).ofNote(id).single;
      expect(routine.schedule.start, Day(2026, 3, 6));
      expect(routine.schedule.times, [ClockTime(9, 30)]);
      expect(ben.current(routine.id, SentKeys.received), sent.id);
      final link = jsonDecode(ben.current(id, SentKeys.received)!);
      expect(link, {
        'from': base64.encode(ann.signingKey.publicKey),
        'id': opened.item.id,
        'name': 'Ann',
        'version': 1,
      });
      expect(Routines(ben).all(), hasLength(1), reason: 'alarms come from it');

      final count = ben.allEntries().length;
      final reopened = openSentItem(ben, sendNote(ann, note.id));
      expect(reopened.accepted, id);
      expect(acceptSentItem(ben, reopened.item), id);
      expect(ben.allEntries(), hasLength(count));
    });

    test('the same id from another key is another Sent Item', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final eve = device(clock, 'Eve');
      final ben = device(clock, 'Ben');
      final item = openSentItem(
        ben,
        sendNote(ann, Notes(ann).capture('Real').id),
      ).item;
      final first = acceptSentItem(ben, item);
      final copy = SentItem(
        id: item.id,
        version: 2,
        from: eve.ownKeyRecord,
        name: 'Ann',
        text: 'Fake',
      );
      final opened = openSentItem(ben, encodeSentItem(copy, eve.signingKey));
      expect(opened.accepted, isNull);
      expect(acceptSentItem(ben, opened.item), isNot(first));
    });
  });
}

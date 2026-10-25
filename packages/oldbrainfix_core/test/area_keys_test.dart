import 'dart:convert';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart' show SecretKeyData;
import 'package:cryptography/dart.dart' show DartXchacha20;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'sync_test.dart' show device, exchange, ticking;

Uint8List _hex(String s) => fromHex(s.replaceAll(' ', ''));

Uint8List _bytes(int length, int from) =>
    Uint8List.fromList(List.generate(length, (i) => (from + i) & 0xff));

void main() {
  group('sealing', () {
    test('XChaCha20-Poly1305 gives the draft standard\'s test vector', () {
      // draft-irtf-cfrg-xchacha-03, A.3.1.
      const aead = DartXchacha20.poly1305Aead();
      final box = aead.encryptSync(
        utf8.encode(
          "Ladies and Gentlemen of the class of '99: If I could offer you "
          'only one tip for the future, sunscreen would be it.',
        ),
        secretKey: SecretKeyData(_bytes(32, 0x80)),
        nonce: _bytes(24, 0x40),
        aad: _hex('50515253c0c1c2c3c4c5c6c7'),
      );
      expect(
        hex(box.cipherText),
        'bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb'
        '731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452'
        '2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9'
        '21f9664c97637da9768812f615c68b13b52e',
      );
      expect(hex(box.mac.bytes), 'c0875924c1c7987947deafd8780acf49');
    });

    test('a sealed message opens only with its key and what it is for', () {
      final key = _bytes(32, 1);
      final nonce = _bytes(24, 100);
      final sealed = seal(
        key,
        utf8.encode('Pills at eight'),
        aad: SealedAs.entry,
        nonce: nonce,
      );
      expect(sealed.length, 24 + 14 + 16);
      expect(sealed.sublist(0, 24), nonce);
      expect(
        utf8.decode(unseal(key, sealed, aad: SealedAs.entry)!),
        'Pills at eight',
      );
      expect(unseal(_bytes(32, 2), sealed, aad: SealedAs.entry), isNull);
      expect(unseal(key, sealed, aad: SealedAs.blob('photo/x')), isNull);
      final changed = Uint8List.fromList(sealed)..[30] ^= 1;
      expect(unseal(key, changed, aad: SealedAs.entry), isNull);
      expect(unseal(key, sealed.sublist(0, 20), aad: SealedAs.entry), isNull);
      // Fresh nonces: the same text never seals the same twice.
      final again = seal(key, utf8.encode('Pills at eight'), aad: 'x');
      expect(again, isNot(seal(key, utf8.encode('Pills at eight'), aad: 'x')));
    });

    test('a blob\'s folder name hides its hash and depends on the key', () {
      final ref = 'photo/${'ab' * 32}';
      final name = blobName(_bytes(32, 1), ref);
      expect(name, hasLength(64));
      expect(name, isNot(contains('abab')));
      expect(blobName(_bytes(32, 1), ref), name);
      expect(blobName(_bytes(32, 2), ref), isNot(name));
    });
  });

  group('Area keys', () {
    test('every Area and the personal scope get a key of their own', () {
      final store = device(ticking(), 'Ada');
      final house = Places(store).addArea('Household');
      final work = Places(store).addArea('Work');
      store.ensureAreaKeys();
      final scopes = {for (final k in store.areaKeys()) k.scope};
      expect(scopes, {personalScope, house.id, work.id});
      final keys = store.areaKeys();
      expect({for (final k in keys) hex(k.key)}, hasLength(3));
      for (final k in keys) {
        expect(k.id, hasLength(16));
        expect(k.id, areaKeyId(k.key));
        expect(k.id, isNot(contains('area')));
      }
      // Made once: nothing changes the second time.
      store.ensureAreaKeys();
      expect(store.areaKeys(), hasLength(3));
    });

    test('an entity is sealed with the key of the Area it is in now', () {
      final store = device(ticking(), 'Ada');
      final places = Places(store);
      final house = places.addArea('Household');
      final kitchen = places.addTopic(house.id, 'Kitchen');
      final work = places.addArea('Work');
      final notes = Notes(store);
      final loose = notes.capture('Unsorted thought');
      final pan = notes.capture('New pan', place: kitchen.id);
      final memo = notes.capture('Memo', place: work.id);
      final section = notes.amend(pan.id, 'Cast iron');
      final pills = Routines(store).create(
        'Pills',
        Schedule(start: Day(2026, 3, 1), times: [ClockTime(8, 0)]),
      );
      NamedPlaces(store).add('Home', Coordinates(52.5, 13.4), radius: 100);
      store.ensureAreaKeys();
      expect(store.scopeOf(house.id), house.id);
      expect(store.scopeOf(kitchen.id), house.id);
      expect(store.scopeOf(pan.id), house.id);
      expect(store.scopeOf(section.id), house.id);
      expect(store.scopeOf(memo.id), work.id);
      expect(store.scopeOf(loose.id), personalScope);
      expect(store.scopeOf(pills.id), personalScope);
      expect(store.scopeOf('namedplace:x'), personalScope);
      expect(store.sealingKey(pan.id)?.scope, house.id);
      // Moved: what it writes from now on goes under the new key.
      notes.file(pan.id, work.id);
      expect(store.sealingKey(pan.id)?.scope, work.id);
      expect(store.sealingKey(section.id)?.scope, work.id);
      // An Area without a key here falls back to the personal one.
      store.append('note:stray', NoteKeys.place, 'area:unknown');
      expect(store.sealingKey('note:stray')?.scope, personalScope);
    });

    test('of two keys for a scope, every writer picks the lower id', () {
      final store = device(ticking(), 'Ada');
      final a = AreaKey(personalScope, _bytes(32, 1));
      final b = AreaKey(personalScope, _bytes(32, 2));
      expect(store.learnAreaKeys([a, b, a]), 2);
      final low = a.id.compareTo(b.id) < 0 ? a : b;
      expect(store.writingKey(personalScope)?.id, low.id);
      expect(store.areaKeys(), hasLength(2), reason: 'none is ever dropped');
    });

    test('what is no key is not taken', () {
      expect(
        AreaKey.fromJson({
          'scope': 'topic:x',
          'key': base64.encode(_bytes(32, 0)),
        }),
        isNull,
      );
      expect(
        AreaKey.fromJson({
          'scope': personalScope,
          'key': base64.encode(_bytes(16, 0)),
        }),
        isNull,
      );
      expect(AreaKey.fromJson({'scope': personalScope, 'key': '!!'}), isNull);
      expect(AreaKey.fromJson('x'), isNull);
      final key = AreaKey('area:a', _bytes(32, 3));
      expect(AreaKey.fromJson(key.toJson())?.id, key.id);
    });

    test('a LAN session hands every key over both ways', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final house = Places(desk).addArea('Household');
      final kids = Places(phone).addArea('Kids');
      // Both made their keys before they met.
      desk.ensureAreaKeys();
      exchange(phone, desk);
      final ids = {for (final k in desk.areaKeys()) k.id};
      expect(ids, hasLength(4), reason: 'two personal keys and two Areas');
      expect({for (final k in phone.areaKeys()) k.id}, ids);
      expect(desk.writingKey(house.id)?.id, phone.writingKey(house.id)?.id);
      expect(desk.writingKey(kids.id)?.id, phone.writingKey(kids.id)?.id);
      expect(
        desk.writingKey(personalScope)?.id,
        phone.writingKey(personalScope)?.id,
        reason: 'both now write with the same personal key',
      );
    });

    test('a Device that has no keys yet takes the ones handed over', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      Places(phone).addArea('Kids');
      exchange(phone, desk);
      expect(desk.areaKeys(), hasLength(2));
      expect(
        {for (final k in desk.areaKeys()) k.id},
        {for (final k in phone.areaKeys()) k.id},
      );
    });
  });
}

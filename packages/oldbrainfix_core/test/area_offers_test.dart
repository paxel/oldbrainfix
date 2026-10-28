import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

Uint8List _photo(int n) => Uint8List.fromList([
  0xff,
  0xd8,
  for (var i = 0; i < 64; i++) (i * n) & 0xff,
  0xff,
  0xd9,
]);

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;
  late Directory dir;
  var n = 0;
  String path() => '${dir.path}/o${n++}.$bundleExtension';

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    ben = device(clock, 'Ben')..renameOwnProfile('Ben');
    dir = Directory.systemTemp.createTempSync('area_offers_test_');
    // Each sent the other their Profile card through a messenger.
    ben.openProfileCard(ann.profileCard()!);
    ann.openProfileCard(ben.profileCard(back: true)!);
  });

  tearDown(() => dir.deleteSync(recursive: true));

  String idOf(Store store) => store.ownProfile!.id;

  /// Ann's Household with a Note and its photo, a Routine and a private
  /// Note beside it, shared with Ben as [role]; changes travel as
  /// [transport]. Returns the Area and the offer bundle.
  Future<({String area, String note, String file})> offered({
    AreaRole role = AreaRole.member,
    Transport transport = Transport.bundle,
  }) async {
    final house = Places(ann).addArea('Household');
    final milk = Notes(ann)
        .capture('Buy milk', place: house.id, photos: [_photo(3)]);
    Routines(ann).spawn(
      milk.id,
      Schedule(start: Day(2026, 3, 2), times: [ClockTime(8, 0)]),
    );
    Notes(ann).capture('Private');
    final area = shareAreaWith(
      ann,
      house.id,
      idOf(ben),
      role: role,
      transport: transport,
    );
    final file = path();
    await writeAreaOffer(ann, file, area, idOf(ben));
    return (area: area, note: milk.id, file: file);
  }

  List<String> texts(Store store) =>
      Notes(store).all().map((n) => n.text).toList()..sort();

  group('sharing from afar', () {
    test('the offer is held until accepted, and nothing of the Area is '
        'kept before', () async {
      final o = await offered();
      final before = ben.allEntries().length;
      final got = await importBundle(ben, o.file);
      final held = got.offer!;
      expect(got.applied, isEmpty);
      expect(held.sender.name, 'Ann');
      expect(held.areaName, 'Household');
      expect(held.role, AreaRole.member);
      expect(held.trust, SenderTrust.known);
      expect(held.transport, Transport.bundle);
      expect(ben.allEntries(), hasLength(before));
      expect(Notes(ben).byId(o.note), isNull);
      expect(ben.writingKey(o.area), isNull);
      expect(heldAreaOffers(ben).map((h) => h.id), [held.id]);
      // The same offer again is the one held.
      expect((await importBundle(ben, o.file)).offer?.id, held.id);

      expect(acceptAreaOffer(ben, held.id), o.area);
      expect(heldAreaOffers(ben), isEmpty);
      expect(texts(ben), ['Buy milk']);
      final hash = Notes(ben).byId(o.note)!.photos.single;
      expect(ben.photo(hash), ann.photo(hash));
      expect(Routines(ben).ofNote(o.note), hasLength(1));
      expect(ben.accessIn(o.area), AreaAccess.member);
      expect(ben.writingKey(o.area)?.id, ann.writingKey(o.area)?.id);
      expect(ben.transportWith(idOf(ann)), Transport.bundle);
      expect(ben.moments().first.cause, MomentCause.bundle);
    });

    test('changes travel by bundles both ways, and the roles hold', () async {
      final o = await offered();
      final held = (await importBundle(ben, o.file)).offer!;
      acceptAreaOffer(ben, held.id);

      Notes(ben).capture('From Ben', place: o.area);
      // A Member may not change Ann's Note: her Devices ignore it.
      ben.append(o.note, NoteKeys.text, 'Changed by Ben');
      final back = path();
      expect(await writeBundle(ben, back, forPerson: idOf(ann)), isNotNull);
      final read = await importBundle(ann, back);
      expect(read.unreadable, 0);
      expect(texts(ann), ['Buy milk', 'From Ben', 'Private']);

      Notes(ann).capture('From Ann', place: o.area);
      final again = path();
      final wrote = await writeBundle(ann, again, forPerson: idOf(ben));
      expect(wrote, isNotNull);
      await importBundle(ben, again);
      expect(texts(ben), ['Buy milk', 'From Ann', 'From Ben']);
    });

    test('through a shared folder changes travel on their own, from every '
        'Device of the person', () async {
      final o = await offered(transport: Transport.folder);
      final held = (await importBundle(ben, o.file)).offer!;
      expect(held.transport, Transport.folder);
      acceptAreaOffer(ben, held.id);
      expect(ben.transportWith(idOf(ann)), Transport.folder);
      // Ben's computer, paired as his own, takes his role over.
      final benDesk = device(clock, 'Ben');
      exchange(benDesk, ben);
      exchange(benDesk, ben);
      expect(benDesk.accessIn(o.area), AreaAccess.member);

      final folder = MemorySyncFolder();
      Notes(benDesk).capture('From the desk', place: o.area);
      await folderSync(ann, folder);
      await folderSync(benDesk, folder);
      await folderSync(ben, folder);
      Notes(ann).capture('From Ann', place: o.area);
      await folderSync(ann, folder);
      await folderSync(ben, folder);
      // Ann met Ben's computer nowhere: its key came with the Area.
      expect(texts(ann), ['Buy milk', 'From Ann', 'From the desk', 'Private']);
      expect(texts(ben), ['Buy milk', 'From Ann', 'From the desk']);
    });

    test(
      'a bundle for a Device met from afar holds only what it shares',
      () async {
        final o = await offered();
        acceptAreaOffer(ben, (await importBundle(ben, o.file)).offer!.id);
        Notes(ann).capture('Still private');
        final file = path();
        final wrote = await writeBundle(ann, file, forDevice: ben.deviceId);
        expect(wrote, isNotNull);
        final read = await importBundle(ben, file);
        // Nothing in it is sealed with a key Ben lacks.
        expect(read.unreadable, 0);
        expect(texts(ben), ['Buy milk']);
      },
    );
  });

  group('declining and refusing', () {
    test('declining keeps nothing: no entry, key, Profile or photo', () async {
      final o = await offered();
      final settings = ben.localSettingsWithPrefix('');
      final entries = ben.allEntries().length;
      final held = (await importBundle(ben, o.file)).offer!;
      final hash = Notes(ann).byId(o.note)!.photos.single;
      expect(ben.photo(hash), isNotNull);

      expect(declineAreaOffer(ben, held.id), isTrue);
      expect(heldAreaOffers(ben), isEmpty);
      expect(ben.allEntries(), hasLength(entries));
      expect(ben.localSettingsWithPrefix(''), settings);
      expect(ben.photo(hash), isNull);
      expect(ben.pendingSets, isEmpty);
    });

    test('decline and block blocks the sharer, keeps their name and '
        'nothing of the Area', () async {
      final o = await offered();
      final held = (await importBundle(ben, o.file)).offer!;
      final hash = Notes(ann).byId(o.note)!.photos.single;
      expect(blockAreaOfferSender(ben, held.id), isTrue);
      expect(ben.isBlocked(idOf(ann)), isTrue);
      expect(ben.blockedPeople.map((p) => p.name), ['Ann']);
      expect(ben.pendingSets, isEmpty);
      expect(ben.photo(hash), isNull);
      expect(Notes(ben).byId(o.note), isNull);
    });

    test('an offer of a blocked person is refused before its preview, and '
        'one held goes when they are blocked', () async {
      final o = await offered();
      final held = (await importBundle(ben, o.file)).offer!;
      ben.blockPerson(idOf(ann));
      expect(heldAreaOffer(ben, held.id), isNull);
      expect(ben.pendingSets, isEmpty);
      await expectLater(
        importBundle(ben, o.file),
        throwsA(isA<AreaOfferFromBlocked>()),
      );
      expect(ben.pendingSets, isEmpty);
    });

    test('an offer of an Area held here as its own is refused', () async {
      final o = await offered();
      ben.append(o.area, PlaceKeys.type, PlaceKeys.typeArea);
      await expectLater(
        importBundle(ben, o.file),
        throwsA(isA<AreaOfferRefused>()),
      );
      expect(ben.pendingSets, isEmpty);
    });

    test('an offer sealed for somebody else is not for us', () async {
      final o = await offered();
      final cat = device(clock, 'Cat');
      await expectLater(
        importBundle(cat, o.file),
        throwsA(isA<AreaOfferNotForUs>()),
      );
      expect(cat.pendingSets, isEmpty);
    });

    test('an offer that grants this Device nothing is not for us', () async {
      final o = await offered();
      final cat = device(clock, 'Cat');
      cat.openProfileCard(ann.profileCard()!);
      ann.openProfileCard(cat.profileCard(back: true)!);
      // Sealed for Cat, but the grants name only Ben.
      final folder = MemorySyncFolder();
      await writeSealed(
        ann,
        folder,
        ann.areaHistory(o.area),
        blobs: const {},
        nonce: () => secretBytes(sealNonceBytes),
      );
      final bytes = encodeAreaOffer(
        areaOfferFor(ann, o.area, idOf(cat)),
        ann.signingKey,
        to: [ann.pinnedKey(cat.deviceId)!.record.boxKey!],
      );
      await expectLater(
        receiveAreaOffer(cat, folder, bytes),
        throwsA(isA<AreaOfferNotForUs>()),
      );
      expect(cat.pendingSets, isEmpty);
    });

    test('the preview names the Area as its Managers do', () async {
      final o = await offered();
      acceptAreaOffer(ben, (await importBundle(ben, o.file)).offer!.id);
      // Ben, a Member, renames the Area: relayed, but it counts nowhere.
      ben.append(o.area, PlaceKeys.name, 'Ben\'s');
      final back = path();
      await writeBundle(ben, back, forPerson: idOf(ann));
      await importBundle(ann, back);
      expect(Places(ann).areaById(o.area)!.name, 'Household');
      final cat = device(clock, 'Cat');
      cat.openProfileCard(ann.profileCard()!);
      ann.openProfileCard(cat.profileCard(back: true)!);
      shareAreaWith(
        ann,
        o.area,
        idOf(cat),
        role: AreaRole.member,
        transport: Transport.bundle,
      );
      final file = path();
      await writeAreaOffer(ann, file, o.area, idOf(cat));
      expect((await importBundle(cat, file)).offer!.areaName, 'Household');
    });

    test('an offer only a Manager signed counts', () async {
      final o = await offered();
      final cat = device(clock, 'Cat');
      cat.openProfileCard(ann.profileCard()!);
      ann.openProfileCard(cat.profileCard(back: true)!);
      shareAreaWith(
        ann,
        o.area,
        idOf(cat),
        role: AreaRole.member,
        transport: Transport.bundle,
      );
      final file = path();
      await writeAreaOffer(ann, file, o.area, idOf(ben));
      acceptAreaOffer(ben, (await importBundle(ben, file)).offer!.id);
      // Ben, a Member, passes the Area on to Cat in his own name: her
      // grant is real, but he may not offer.
      final folder = MemorySyncFolder();
      await writeSealed(
        ben,
        folder,
        ben.areaHistory(o.area),
        blobs: const {},
        nonce: () => secretBytes(sealNonceBytes),
      );
      final bytes = encodeAreaOffer(
        areaOfferFor(ben, o.area, idOf(cat)),
        ben.signingKey,
        to: [cat.ownKeyRecord.boxKey!],
      );
      await expectLater(
        receiveAreaOffer(cat, folder, bytes),
        throwsA(isA<AreaOfferNotForUs>()),
      );
      expect(cat.pendingSets, isEmpty);
    });

    test(
      'an offer signed by a key its Profile does not bind is none',
      () async {
        final o = await offered();
        final eve = SigningKey.generate();
        final offer = areaOfferFor(ann, o.area, idOf(ben));
        final forged = AreaOffer(
          area: offer.area,
          areaKeys: offer.areaKeys,
          from: KeyRecord.make(eve),
          keys: offer.keys,
          profile: offer.profile,
          transport: offer.transport,
        );
        final bytes = encodeAreaOffer(
          forged,
          eve,
          to: [ann.pinnedKey(ben.deviceId)!.record.boxKey!],
        );
        await expectLater(
          receiveAreaOffer(ben, MemorySyncFolder(), bytes),
          throwsA(isA<NotAnAreaOffer>()),
        );
      },
    );

    test('an offer never moves the user\'s own Notes into the Area', () async {
      final mine = Notes(ben).capture('Ben private');
      final house = Places(ann).addArea('Household');
      Notes(ann).capture('Buy milk', place: house.id);
      // Ann's modified app files Ben's Note under her Area.
      ann.append(mine.id, NoteKeys.place, house.id);
      ann.append(mine.id, NoteKeys.text, 'Taken');
      shareAreaWith(
        ann,
        house.id,
        idOf(ben),
        role: AreaRole.manager,
        transport: Transport.bundle,
      );
      final file = path();
      await writeAreaOffer(ann, file, house.id, idOf(ben));
      acceptAreaOffer(ben, (await importBundle(ben, file)).offer!.id);
      expect(ben.scopeOf(mine.id), personalScope);
      expect(Notes(ben).byId(mine.id)!.text, 'Ben private');
    });

    test('an offer carries keys of its Area only', () {
      final house = Places(ann).addArea('Household');
      ann.ensureAreaKeys();
      final offer = areaOfferFor(ann, house.id, idOf(ben));
      expect(offer.areaKeys.map((k) => k.scope), [house.id]);
      final personal = ann.writingKey(personalScope)!;
      final widened = AreaOffer(
        area: offer.area,
        areaKeys: [...offer.areaKeys, personal],
        from: offer.from,
        keys: offer.keys,
        profile: offer.profile,
        transport: offer.transport,
      );
      expect(AreaOffer.parse(widened.encode()), isNull);
      expect(AreaOffer.parse(offer.encode())?.encode(), offer.encode());
    });

    test('sharing needs a key of the person to seal for', () {
      final house = Places(ann).addArea('Household');
      final stranger = device(clock, 'Dan');
      ann.takePerson(stranger.ownProfileForOthers);
      expect(
        () => shareAreaWith(
          ann,
          house.id,
          idOf(stranger),
          role: AreaRole.member,
          transport: Transport.bundle,
        ),
        throwsStateError,
      );
      expect(ann.isShared(house.id), isFalse);
    });
  });
}

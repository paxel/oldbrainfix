import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'profiles_test.dart' show halves;
import 'shared_areas_test.dart' show exchange, ticking;

/// The name the tests give Ben for Ann alone: whatever carries it is
/// easy to find.
const _marker = 'Mum-Marker';

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store annDesk;
  late Store ben;
  late Uint8List picture;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    annDesk = device(clock, 'Ann at the desk');
    ben = device(clock, 'Ben');
    picture = squareProfilePicture(halves(64, 48))!;
  });

  String idOf(Store store) => store.ownProfile!.id;

  /// Ann's household shared with Ben as a Member, Ann's desk one of her
  /// own Devices.
  String household() {
    exchange(annDesk, ann);
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(annDesk, ann);
    return area;
  }

  /// Every entry of an Override [store] holds.
  List<Entry> overrides(Store store) => [
    for (final e in store.allEntries())
      if (e.entity.startsWith(OverrideKeys.prefix)) e,
  ];

  group('showing', () {
    test('every view shows the Override; the Profile view their own', () {
      final area = household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, _marker);
      ann.setOverridePicture(benId, picture);
      expect(ann.personShown(benId).name, _marker);
      expect(ann.personShown(benId).picture, picture);
      expect(ann.personTheirs(benId).name, 'Ben');
      expect(ann.personTheirs(benId).picture, isNull);
      expect(ann.personOverride(benId)?.name, _marker);
      // Member lists, the People list and other people's Notes.
      expect(ann.people(area).map((p) => ann.personShown(p.id).name), [
        'Ann',
        _marker,
      ]);
      expect(ann.peopleShown.map((p) => p.name), [_marker]);
      final note = Notes(ben).capture('Milk', place: area);
      exchange(ben, ann);
      expect(ann.authorOf(note.id)?.name, _marker);
      expect(ann.authorOf(note.id)?.picture, picture);
      // Only the name: their own picture stays.
      ann.setOverridePicture(benId, null);
      expect(ann.personShown(benId).picture, isNull);
      expect(ann.personShown(benId).name, _marker);
      // Taken away, their own name shows again.
      ann.setOverrideName(benId, '  ');
      expect(ann.personOverride(benId), isNull);
      expect(ann.personShown(benId).name, 'Ben');
    });

    test('a name is sanitised like a Profile name', () {
      household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, ' \u202eM\u200bum\u0007 ');
      expect(ann.personShown(benId).name, 'Mum');
      ann.setOverrideName(benId, 'x' * 60);
      expect(ann.personShown(benId).name, 'x' * profileNameMax);
    });

    test('writing the same again writes nothing', () {
      household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, 'Mum');
      ann.setOverridePicture(benId, picture);
      final count = ann.allEntries().length;
      ann.setOverrideName(benId, 'Mum ');
      ann.setOverridePicture(benId, picture);
      expect(ann.allEntries(), hasLength(count));
    });

    test('the own Profile takes none', () {
      ann.setOverrideName(idOf(ann), 'Me');
      expect(overrides(ann), isEmpty);
      expect(ann.personShown(idOf(ann)).name, 'Ann');
    });

    test('a picture that is no JPEG, or too large, shows none', () {
      household();
      final benId = idOf(ben);
      final entity = '${OverrideKeys.prefix}$benId';
      ann.append(entity, OverrideKeys.picture, base64.encode(halves(8, 8)));
      expect(ann.personShown(benId).picture, isNull);
      final large = Uint8List(profilePictureMaxBytes + 1)
        ..setAll(0, const [0xFF, 0xD8, 0xFF]);
      ann.append(entity, OverrideKeys.picture, base64.encode(large));
      expect(ann.personShown(benId).picture, isNull);
      ann.append(entity, OverrideKeys.picture, 'not base64!');
      expect(ann.personShown(benId).picture, isNull);
    });

    test('a Sent Item shows the Override only for the Profile kept', () {
      final benId = idOf(ben);
      final file = sendNote(ben, Notes(ben).capture('Dentist').id);
      // Not kept yet: their own name, whatever the log says.
      ann.setOverrideName(benId, _marker);
      expect(openSentItem(ann, file).sender.name, 'Ben');
      acceptSentItem(ann, openSentItem(ann, file).item);
      expect(openSentItem(ann, file).sender.name, _marker);
    });

    test('an Override another person\'s Device wrote counts for nothing', () {
      household();
      final benId = idOf(ben);
      // As a modified app would: Ben names himself in Ann's log.
      final forged = ben.append(
        '${OverrideKeys.prefix}$benId',
        OverrideKeys.name,
        'Mallory',
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(ann.personShown(benId).name, 'Ben');
      expect(ann.personOverride(benId), isNull);
    });

    test('an Override of a later structure shows and is changed only '
        'there (ADR 0034)', () {
      household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, _marker);
      // As Ann's desk of a later version writes it.
      final later = annDesk.append(
        '${OverrideKeys.prefix}$benId',
        OverrideKeys.version,
        '2',
      );
      ann.applyEntries([later], verifiedDevice: annDesk.deviceId);
      expect(ann.overrideIsNewer(benId), isTrue);
      expect(ann.personShown(benId).name, _marker);
      final before = overrides(ann).length;
      expect(() => ann.setOverrideName(benId, 'Ben'), throwsStateError);
      // The same name again, as a page left unchanged keeps it, is fine.
      ann.setOverrideName(benId, _marker);
      expect(() => ann.setOverridePicture(benId, picture), throwsStateError);
      expect(overrides(ann), hasLength(before));
      expect(ann.overrideIsNewer(idOf(ann)), isFalse);
      for (final (value, newer) in [('1', false), (null, false), ('x', true)]) {
        ann.append('${OverrideKeys.prefix}$benId', OverrideKeys.version, value);
        expect(ann.overrideIsNewer(benId), newer, reason: '$value');
      }
    });

    test('a version another person\'s Device wrote locks nothing', () {
      household();
      final benId = idOf(ben);
      // As a modified app would: Ben locks the name Ann gives him.
      final forged = ben.append(
        '${OverrideKeys.prefix}$benId',
        OverrideKeys.version,
        '2',
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(ann.overrideIsNewer(benId), isFalse);
      ann.setOverrideName(benId, _marker);
      expect(ann.personShown(benId).name, _marker);
    });
  });

  group("one's own Devices", () {
    test('a LAN session carries it', () {
      household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, _marker);
      ann.setOverridePicture(benId, picture);
      var told = 0;
      annDesk.addProfileListener(() => told++);
      exchange(annDesk, ann);
      expect(annDesk.personShown(benId).name, _marker);
      expect(annDesk.personShown(benId).picture, picture);
      // Every view showing a person hears of it.
      expect(told, greaterThan(0));
      // Changed on the desk, it comes back.
      annDesk.setOverrideName(benId, 'Mum');
      exchange(annDesk, ann);
      expect(ann.personShown(benId).name, 'Mum');
    });

    test('the shared folder carries it', () async {
      household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, _marker);
      final folder = MemorySyncFolder();
      await folderSync(ann, folder);
      await folderSync(annDesk, folder);
      expect(annDesk.personShown(benId).name, _marker);
    });

    test('a bundle carries it', () async {
      household();
      final benId = idOf(ben);
      ann.setOverrideName(benId, _marker);
      final dir = Directory.systemTemp.createTempSync('overrides');
      addTearDown(() => dir.deleteSync(recursive: true));
      final path = '${dir.path}/ann.$bundleExtension';
      await writeBundle(ann, path, forDevice: annDesk.deviceId);
      await importBundle(annDesk, path);
      expect(annDesk.personShown(benId).name, _marker);
    });
  });

  group('never to anyone else', () {
    test('a LAN session with another person\'s Device carries none', () {
      household();
      ann.setOverrideName(idOf(ben), _marker);
      ann.setOverridePicture(idOf(ben), picture);
      Notes(ann).capture('Written since');
      // Both ways: Ben joins Ann, and Ann joins Ben.
      final joined = exchange(ben, ann);
      final hosted = exchange(ann, ben);
      for (final wire in [
        joined.request,
        joined.answer,
        hosted.request,
        hosted.answer,
      ]) {
        expect(jsonEncode(wire), isNot(contains(_marker)));
      }
      expect(overrides(ben), isEmpty);
      expect(ben.personShown(idOf(ben)).name, 'Ben');
    });

    test('the shared folder holds none another person can open', () async {
      final area = household();
      ann.setOverrideName(idOf(ben), _marker);
      Notes(ann).capture('Milk', place: area);
      final folder = MemorySyncFolder();
      await folderSync(ann, folder);
      await folderSync(ben, folder);
      expect(overrides(ben), isEmpty);
      // Every line of every file, opened with every key Ben holds.
      final keys = ben.areaKeys();
      final annPersonal = {
        for (final k in ann.areaKeys())
          if (k.scope == personalScope) k.id,
      };
      expect(keys.map((k) => k.id).toSet().intersection(annPersonal), isEmpty);
      var opened = 0;
      for (final files in folder.dirs.values) {
        for (final bytes in files.values) {
          for (final line in latin1.decode(bytes).split('\n')) {
            final Uint8List sealed;
            try {
              sealed = base64.decode(line.trim());
            } on FormatException {
              continue;
            }
            for (final key in keys) {
              final plain = unseal(key.key, sealed, aad: SealedAs.entry);
              if (plain == null) continue;
              opened++;
              expect(utf8.decode(plain), isNot(contains(_marker)));
            }
          }
        }
      }
      expect(opened, greaterThan(0));
    });

    test('a bundle for another person carries none', () async {
      final area = household();
      ann.setOverrideName(idOf(ben), _marker);
      Notes(ann).capture('Milk', place: area);
      final dir = Directory.systemTemp.createTempSync('overrides');
      addTearDown(() => dir.deleteSync(recursive: true));
      final path = '${dir.path}/ann.$bundleExtension';
      final wrote = await writeBundle(ann, path, forDevice: ben.deviceId);
      expect(wrote, isNotNull);
      await importBundle(ben, path);
      expect(Notes(ben).all().map((n) => n.text), contains('Milk'));
      expect(overrides(ben), isEmpty);
    });

    test('a Sent Item carries none', () {
      household();
      ann.setOverrideName(idOf(ben), _marker);
      final note = Notes(ann).capture('Dentist');
      final file = sendNote(ann, note.id);
      expect(latin1.decode(file), isNot(contains(_marker)));
      final cid = device(clock, 'Cid');
      acceptSentItem(cid, openSentItem(cid, file).item);
      expect(overrides(cid), isEmpty);
    });
  });
}

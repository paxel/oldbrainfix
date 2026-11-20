import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show ticking;

/// The texts as an English Device writes them.
final english = HelpTexts(
  areaName: 'Old Brain Fix',
  whatsNewTitle: (v) => "What's new in $v",
);

/// The same as a German one writes them.
final german = HelpTexts(
  areaName: 'Old Brain Fix',
  whatsNewTitle: (v) => 'Neu in $v',
);

/// Out of order, as a map may hold them.
const bundle = {
  '0.5.0': {'en': 'Added\n• More.', 'de': 'Neu\n• Mehr.'},
  '0.4.0': {'en': 'Added\n• Help Notes.', 'de': 'Neu\n• Hilfe-Notizen.'},
  '0.4.1': {'en': 'Fixed\n• A thing.'},
};

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  List<String> make(
    Store store, {
    String version = '0.4.0',
    bool fresh = true,
    String language = 'en',
  }) => HelpNotes(store).makeDue(
    version: version,
    fresh: fresh,
    language: language,
    texts: language == 'de' ? german : english,
    whatsNew: bundle,
  );

  String person(Store store) => store.ownProfile!.id;

  /// A Device of Ann's that holds what [from] holds, her Profile included.
  Store ownDevice(Store from) {
    final desk = Store.inMemory(clock: clock);
    addTearDown(desk.close);
    takeFrom(desk, from);
    desk.takeOwnProfile(from.ownProfileJson, from: from.signingKey.publicKey);
    return desk;
  }

  group('a fresh install', () {
    test('makes nothing and records the version', () {
      final written = ann.allEntries().length;
      expect(make(ann), isEmpty);
      expect(ann.allEntries().length, written);
      expect(ann.localSetting(HelpNotes.versionSetting), '0.4.0');
      // No Area without a Note to go into it.
      expect(Places(ann).areaById(HelpNotes.areaId(person(ann))), isNull);
    });

    test('the same start again writes nothing', () {
      make(ann);
      final written = ann.allEntries().length;
      expect(make(ann), isEmpty);
      expect(ann.allEntries().length, written);
    });

    test('nothing is made before the own Profile exists', () {
      final bare = Store.inMemory(clock: clock);
      addTearDown(bare.close);
      expect(make(bare, fresh: false), isEmpty);
      expect(bare.allEntries(), isEmpty);
      expect(bare.localSetting(HelpNotes.versionSetting), isNull);
    });
  });

  group('Quick start and What the icons mean', () {
    // Spotlights explain the screens instead (obf-1043, ADR 0055).
    test('are made by no start: neither a fresh one, an update nor one '
        'that recorded no version', () {
      final p = person(ann);
      final made = [...make(ann), ...make(ann, version: '0.4.1', fresh: false)];
      ann.removeLocalSetting(HelpNotes.versionSetting);
      made.addAll(make(ann, version: '0.5.0', fresh: false));
      expect(made, [
        HelpNotes.whatsNewId(p, '0.4.1'),
        HelpNotes.whatsNewId(p, '0.4.0'),
        HelpNotes.whatsNewId(p, '0.5.0'),
      ]);
      for (final id in [HelpNotes.quickStartId(p), HelpNotes.iconsId(p)]) {
        expect(ann.loggedHistory(id), isEmpty, reason: id);
      }
      expect(Checklists(ann).agendaRows(), isEmpty);
    });

    test('made before, stay as they are', () {
      final p = person(ann);
      // As 0.6.1 made them, in short.
      for (final id in [HelpNotes.quickStartId(p), HelpNotes.iconsId(p)]) {
        ann.append(id, NoteKeys.type, NoteKeys.typeNote);
        ann.append(id, NoteKeys.place, HelpNotes.areaId(p));
      }
      final before = [
        for (final id in [HelpNotes.quickStartId(p), HelpNotes.iconsId(p)])
          ann.loggedHistory(id).length,
      ];
      expect(make(ann, version: '0.4.0', fresh: false), [
        HelpNotes.whatsNewId(p, '0.4.0'),
      ]);
      expect([
        for (final id in [HelpNotes.quickStartId(p), HelpNotes.iconsId(p)])
          ann.loggedHistory(id).length,
      ], before);
    });
  });

  group('an update', () {
    test('from a Device that recorded nothing makes every What\'s new after '
        '0.3.1, oldest first, into their own Area', () {
      final p = person(ann);
      final made = make(ann, version: '0.4.1', fresh: false);
      expect(made, [
        HelpNotes.whatsNewId(p, '0.4.0'),
        HelpNotes.whatsNewId(p, '0.4.1'),
      ]);
      final note = Notes(ann).byId(HelpNotes.whatsNewId(p, '0.4.0'))!;
      expect(note.text, "What's new in 0.4.0\n\nAdded\n• Help Notes.");
      expect(note.textFormat, TextFormats.plain);
      final at = [for (final id in made) Notes(ann).byId(id)!.captured];
      expect(at, [...at]..sort());
      final area = Places(ann).areaById(HelpNotes.areaId(p))!;
      expect(area.name, 'Old Brain Fix');
      expect(area.colour, HelpNotes.areaColour);
      expect(area.icon, 'brain');
      expect(areaIcons, contains(area.icon));
      final notes = Notes(ann).inside(area.id);
      expect(notes, hasLength(2));
      for (final note in notes) {
        expect(note.nudge, isNull);
        expect(note.place, area.id);
      }
      expect(Notes(ann).unsorted(), isEmpty);
    });

    test('makes the What\'s new of the versions passed since the last one '
        'only', () {
      final p = person(ann);
      make(ann);
      expect(make(ann, version: '0.5.0', fresh: false), [
        HelpNotes.whatsNewId(p, '0.4.1'),
        HelpNotes.whatsNewId(p, '0.5.0'),
      ]);
      expect(ann.localSetting(HelpNotes.versionSetting), '0.5.0');
    });

    test('a version without Play notes makes no What\'s new', () {
      make(ann);
      expect(make(ann, version: '0.4.2', fresh: false), [
        HelpNotes.whatsNewId(person(ann), '0.4.1'),
      ]);
    });

    test('a What\'s new without the language is made in English', () {
      make(ann, language: 'de');
      final p = person(ann);
      make(ann, version: '0.4.1', fresh: false, language: 'de');
      expect(
        Notes(ann).byId(HelpNotes.whatsNewId(p, '0.4.1'))!.text,
        'Neu in 0.4.1\n\nFixed\n• A thing.',
      );
      make(ann, version: '0.5.0', fresh: false, language: 'de');
      expect(
        Notes(ann).byId(HelpNotes.whatsNewId(p, '0.5.0'))!.text,
        'Neu in 0.5.0\n\nNeu\n• Mehr.',
      );
    });
  });

  group('once per person', () {
    test('a second Device of the same person makes nothing, in no other '
        'language', () {
      make(ann, version: '0.4.1', fresh: false);
      final desk = ownDevice(ann);
      final written = desk.allEntries().length;
      expect(make(desk, version: '0.4.1', language: 'de'), isEmpty);
      expect(desk.allEntries().length, written);
      desk.removeLocalSetting(HelpNotes.versionSetting);
      expect(make(desk, version: '0.4.1', fresh: false), isEmpty);
      expect(desk.allEntries().length, written);
    });

    test('a deleted help Note never returns', () {
      final p = person(ann);
      make(ann, version: '0.4.1', fresh: false);
      Notes(ann).trash(HelpNotes.whatsNewId(p, '0.4.0'));
      // Purged, as 30 days in the trash do.
      ann.append(HelpNotes.whatsNewId(p, '0.4.1'), NoteKeys.type, null);
      final desk = ownDevice(ann);
      expect(make(desk, version: '0.5.0', fresh: false), [
        HelpNotes.whatsNewId(p, '0.5.0'),
      ]);
      ann.removeLocalSetting(HelpNotes.versionSetting);
      expect(make(ann, version: '0.4.1', fresh: false), isEmpty);
    });

    test('two Devices that make one apart hold one after a sync', () {
      final desk = ownDevice(ann);
      make(ann, version: '0.4.0', fresh: false);
      make(desk, version: '0.4.0', fresh: false, language: 'de');
      takeFrom(ann, desk);
      takeFrom(desk, ann);
      final p = person(ann);
      for (final store in [ann, desk]) {
        expect(Places(store).areas(), hasLength(1));
        final inside = Notes(store).inside(HelpNotes.areaId(p));
        // The later Device's words win: one Note in one language, no
        // copy.
        expect(
          [for (final n in inside) n.text],
          [
            anyOf(
              "What's new in 0.4.0\n\nAdded\n• Help Notes.",
              'Neu in 0.4.0\n\nNeu\n• Hilfe-Notizen.',
            ),
          ],
        );
      }
    });

    test('a Note another person wrote under the id counts for nothing', () {
      final ben = device(clock, 'Ben');
      final p = person(ann);
      final id = HelpNotes.whatsNewId(p, '0.4.0');
      // Ben writes under Ann's id; his Device is no Device of hers.
      ben.append(id, NoteKeys.type, NoteKeys.typeNote);
      ann.applyEntries(ben.entriesSince(const {}), keys: [ben.ownKeyRecord]);
      expect(make(ann, fresh: false), contains(id));
    });
  });

  group('the Area', () {
    test('a renamed one stays as it is', () {
      make(ann, fresh: false);
      final p = person(ann);
      Places(ann).rename(HelpNotes.areaId(p), 'Help');
      make(ann, version: '0.4.1', fresh: false);
      expect(Places(ann).areaById(HelpNotes.areaId(p))!.name, 'Help');
      expect(
        Notes(ann).byId(HelpNotes.whatsNewId(p, '0.4.1'))!.place,
        HelpNotes.areaId(p),
      );
    });

    test('a deleted one is made again with its look for a new What\'s new, '
        'so it is not Unsorted', () {
      make(ann, fresh: false);
      final p = person(ann);
      Places(ann).setColour(HelpNotes.areaId(p), 'FF0000');
      Places(ann).deleteArea(HelpNotes.areaId(p), NotesGo.trash);
      expect(make(ann, version: '0.4.1', fresh: false), [
        HelpNotes.whatsNewId(p, '0.4.1'),
      ]);
      final area = Places(ann).areaById(HelpNotes.areaId(p))!;
      expect(area.colour, 'FF0000');
      expect(Notes(ann).unsorted(), isEmpty);
    });

    test('is made only when a Note goes into it', () {
      make(ann, fresh: false);
      final p = person(ann);
      Places(ann).deleteArea(HelpNotes.areaId(p), NotesGo.trash);
      expect(make(ann, version: '0.4.2', fresh: false), [
        HelpNotes.whatsNewId(p, '0.4.1'),
      ]);
      Places(ann).deleteArea(HelpNotes.areaId(p), NotesGo.trash);
      ann.setLocalSetting(HelpNotes.versionSetting, '0.4.1');
      expect(make(ann, version: '0.4.2', fresh: false), isEmpty);
      expect(Places(ann).areaById(HelpNotes.areaId(p)), isNull);
    });
  });

  test('versions compare by their numbers', () {
    expect(compareVersions('0.4.0', '0.3.1'), 1);
    expect(compareVersions('0.10.0', '0.9.9'), 1);
    expect(compareVersions('0.4.0', '0.4.0'), 0);
    expect(compareVersions('0.4', '0.4.0'), 0);
    expect(compareVersions('1.0.0', '1.0.1'), -1);
  });
}

/// [into] takes in everything [from] holds, as a sync brings it.
void takeFrom(Store into, Store from) =>
    into.applyEntries(from.entriesSince(const {}), keys: [from.ownKeyRecord]);

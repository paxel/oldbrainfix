import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show ticking;

/// The texts as an English Device writes them.
final english = HelpTexts(
  areaName: 'Old Brain Fix',
  quickStartTitle: 'Quick start',
  quickStartLines: const ['Capture a thought with +', 'File it into an Area'],
  iconsText: 'What the icons mean\n\nKind: what the Note is',
  iconPictures: [
    Uint8List.fromList([0x89, 0x50, 0x4E, 0x47, 1]),
  ],
  whatsNewTitle: (v) => "What's new in $v",
);

/// The same as a German one writes them.
final german = HelpTexts(
  areaName: 'Old Brain Fix',
  quickStartTitle: 'Schnellstart',
  quickStartLines: const [
    'Einen Gedanken mit + erfassen',
    'Ihn in einen Bereich einsortieren',
  ],
  iconsText: 'Was die Symbole bedeuten',
  iconPictures: [
    Uint8List.fromList([0x89, 0x50, 0x4E, 0x47, 2]),
  ],
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
    HelpTexts? texts,
  }) => HelpNotes(store).makeDue(
    version: version,
    fresh: fresh,
    language: language,
    texts: texts ?? (language == 'de' ? german : english),
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
    test('files Quick start and What the icons mean into their own Area, '
        'and no What\'s new', () {
      final made = make(ann);
      final p = person(ann);
      expect(made, [HelpNotes.quickStartId(p), HelpNotes.iconsId(p)]);
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
      final list = Checklists(ann).byNote(HelpNotes.quickStartId(p))!;
      expect(Notes(ann).byId(HelpNotes.quickStartId(p))!.text, 'Quick start');
      expect([for (final l in list.lines) l.text], english.quickStartLines);
      expect(
        [for (final l in list.lines) l.id],
        [HelpNotes.quickStartLineId(p, 0), HelpNotes.quickStartLineId(p, 1)],
      );
      final icons = Notes(ann).byId(HelpNotes.iconsId(p))!;
      expect(icons.text, english.iconsText);
      expect(icons.photos, hasLength(1));
      expect(ann.photo(icons.photos.single), english.iconPictures.single);
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
      expect(make(bare), isEmpty);
      expect(bare.allEntries(), isEmpty);
      expect(bare.localSetting(HelpNotes.versionSetting), isNull);
    });
  });

  group('an update', () {
    test('from a Device that recorded nothing makes every What\'s new after '
        '0.3.1, oldest first, and the help Notes once', () {
      final p = person(ann);
      final made = make(ann, version: '0.4.1', fresh: false);
      expect(made, [
        HelpNotes.quickStartId(p),
        HelpNotes.iconsId(p),
        HelpNotes.whatsNewId(p, '0.4.0'),
        HelpNotes.whatsNewId(p, '0.4.1'),
      ]);
      expect(
        Notes(ann).byId(HelpNotes.whatsNewId(p, '0.4.0'))!.text,
        "What's new in 0.4.0\n\nAdded\n• Help Notes.",
      );
      final at = [for (final id in made) Notes(ann).byId(id)!.captured];
      expect(at, [...at]..sort());
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
      expect(make(desk, version: '0.4.1', fresh: false), isEmpty);
    });

    test('a deleted help Note never returns', () {
      final p = person(ann);
      make(ann, version: '0.4.1', fresh: false);
      final notes = Notes(ann);
      notes.trash(HelpNotes.quickStartId(p));
      notes.trash(HelpNotes.whatsNewId(p, '0.4.0'));
      // Purged, as 30 days in the trash do.
      ann.append(HelpNotes.iconsId(p), NoteKeys.type, null);
      final desk = ownDevice(ann);
      expect(make(desk, version: '0.5.0', fresh: false), [
        HelpNotes.whatsNewId(p, '0.5.0'),
      ]);
      ann.removeLocalSetting(HelpNotes.versionSetting);
      expect(make(ann, version: '0.4.1', fresh: false), isEmpty);
    });

    test('two Devices that make them apart hold one of each after a sync', () {
      final desk = ownDevice(ann);
      make(ann, version: '0.4.0', fresh: false);
      make(desk, version: '0.4.0', fresh: false, language: 'de');
      takeFrom(ann, desk);
      takeFrom(desk, ann);
      final p = person(ann);
      for (final store in [ann, desk]) {
        expect(Places(store).areas(), hasLength(1));
        final inside = Notes(store).inside(HelpNotes.areaId(p));
        expect(inside, hasLength(3));
        final list = Checklists(store).byNote(HelpNotes.quickStartId(p))!;
        // The later Device's words win on every field, the Lines as
        // many in each language: one list in one language, no copy.
        final words = [for (final l in list.lines) l.text];
        final title = Notes(store).byId(HelpNotes.quickStartId(p))!.text;
        expect(
          [title, ...words],
          anyOf(
            equals([english.quickStartTitle, ...english.quickStartLines]),
            equals([german.quickStartTitle, ...german.quickStartLines]),
          ),
        );
      }
    });

    test('a Note another person wrote under the id counts for nothing', () {
      final ben = device(clock, 'Ben');
      final p = person(ann);
      // Ben writes under Ann's id; his Device is no Device of hers.
      ben.append(HelpNotes.quickStartId(p), NoteKeys.type, NoteKeys.typeNote);
      ann.applyEntries(ben.entriesSince(const {}), keys: [ben.ownKeyRecord]);
      expect(make(ann), contains(HelpNotes.quickStartId(p)));
    });
  });

  group('the Area', () {
    test('a renamed one stays as it is', () {
      make(ann);
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
      make(ann);
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
      make(ann);
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

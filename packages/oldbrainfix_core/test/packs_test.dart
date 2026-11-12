import 'dart:convert';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show ticking;

DateTime _wall(DateTime utc) => utc;

Uint8List _bytes(int seed, [int length = 300]) =>
    Uint8List.fromList(List<int>.generate(length, (i) => (seed + i) % 251));

/// The payload of the Pack file [bytes], as a text.
String _payloadOf(Uint8List bytes) =>
    (jsonDecode(utf8.decode(bytes)) as Map<String, Object?>)['pack']! as String;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    ben = device(clock, 'Ben')..renameOwnProfile('Ben');
  });

  /// Ann's Health: a Note of the Area itself, and in its Topic Running a
  /// Tally, a Session picking it and a Tally of her Garden, a Checklist
  /// and a Note with a photo, a recording and their texts.
  ({
    String area,
    String running,
    String plain,
    String tally,
    String session,
    String garden,
    String checklist,
    String media,
  })
  health() {
    final places = Places(ann);
    final notes = Notes(ann);
    final health = places.addArea('Health');
    final running = places.addTopic(health.id, 'Running');
    final gardenArea = places.addArea('Garden');
    final plain = notes.capture('Drink water', place: health.id);
    final tally = notes.capture('Laps', place: running.id);
    Tallies(ann).addField(tally.id, name: 'Laps', unit: '', target: 10);
    final garden = notes.capture('Digging', place: gardenArea.id);
    Tallies(ann).addField(garden.id, name: 'Minutes', unit: 'min');
    final session = notes.capture('Training', place: running.id);
    Sessions(ann).make(
      session.id,
      pool: [tally.id, garden.id],
      count: 1,
      rule: SessionRule.rolling,
    );
    final checklist = notes.capture('Bag\nShoes\nBottle', place: running.id);
    Checklists(ann).make(checklist.id);
    final media = notes.capture(
      'Route',
      place: running.id,
      photos: [_bytes(1)],
      recordings: [_bytes(2, 600)],
    );
    final m = notes.byId(media.id)!;
    notes.setPhotoText(media.id, m.photos.single, 'Map text');
    notes.setTranscript(media.id, m.recordings.single, 'Go left');
    return (
      area: health.id,
      running: running.id,
      plain: plain.id,
      tally: tally.id,
      session: session.id,
      garden: garden.id,
      checklist: checklist.id,
      media: media.id,
    );
  }

  /// The Note of [pack] whose text starts with [text].
  int indexOf(Pack pack, String text) =>
      pack.notes.indexWhere((n) => n.text.startsWith(text));

  group('the tree', () {
    test('from an Area: its own Notes, then each Topic with its Notes; '
        'the Tally of another Area under the Session that picks it', () {
      final h = health();
      final pack = ann.packOf(h.area);
      final tree = pack.tree();
      expect(tree.map((i) => i.kind), [PackItemKind.area]);
      final area = tree.single;
      expect(pack.areas.first.name, 'Health');
      expect(area.children.map((i) => i.kind), [
        PackItemKind.note,
        PackItemKind.topic,
      ]);
      expect(pack.notes[area.children.first.note!].text, 'Drink water');
      final topic = area.children.last;
      final texts = [for (final i in topic.children) pack.notes[i.note!].text];
      expect(texts, containsAll(['Laps', 'Training', 'Route']));
      expect(texts, isNot(contains('Digging')));
      final session = topic.children.firstWhere(
        (i) => pack.notes[i.note!].text == 'Training',
      );
      expect(
        [for (final c in session.children) pack.notes[c.note!].text],
        ['Digging'],
      );
      final garden = pack.notes[indexOf(pack, 'Digging')];
      expect(garden.outside, isTrue);
      expect(pack.areas[garden.area!].name, 'Garden');
    });

    test('from a Topic only its Notes; from a Note that Note and the '
        'Tallies its Session picks from', () {
      final h = health();
      final topic = ann.packOf(h.running);
      expect(topic.notes.map((n) => n.text), isNot(contains('Drink water')));
      final note = ann.packOf(h.session);
      expect(note.notes.map((n) => n.text), ['Training', 'Laps', 'Digging']);
      expect(note.notes.skip(1).every((n) => n.outside), isTrue);
      final tree = note.tree();
      expect(tree.single.kind, PackItemKind.area);
      final session = tree.single.children.single.children.single;
      expect(session.children.map((c) => note.notes[c.note!].text), [
        'Laps',
        'Digging',
      ]);
    });

    test('a Tally of a closed Area never goes along with a Session', () {
      final h = health();
      final garden = Places(ann).areas().firstWhere((a) => a.name == 'Garden');
      final pack = ann.packOf(h.session, closed: {garden.id});
      expect(pack.notes.map((n) => n.text), ['Training', 'Laps']);
      expect(pack.notes.first.session!.pool, [1]);
      expect(pack.areas.map((a) => a.name), ['Health']);
    });

    test('of everything (#214): every open Area by name with its Topics, '
        'then the Notes without one; nothing of a closed Area', () {
      final h = health();
      final loose = Notes(ann).capture('Loose thought');
      // Filed by another Device into a place deleted here meanwhile.
      final stray = Notes(ann).capture('Stray');
      ann.append(stray.id, NoteKeys.place, 'gone');
      final trashed = Notes(ann).capture('Gone', place: h.area);
      Notes(ann).trash(trashed.id);
      final pack = ann.packOfAll();
      expect(pack.areas.map((a) => a.name), ['Garden', 'Health']);
      final tree = pack.tree();
      expect(tree.map((i) => i.kind), [
        PackItemKind.area,
        PackItemKind.area,
        PackItemKind.note,
        PackItemKind.note,
      ]);
      expect(
        [for (final i in tree.skip(2)) pack.notes[i.note!].text],
        ['Stray', 'Loose thought'],
      );
      expect(pack.notes.map((n) => n.text), isNot(contains('Gone')));
      expect(pack.notes.where((n) => n.outside), isEmpty);
      expect(pack.notes.first.text, 'Digging');
      expect(pack.notes.length, 8);
      expect(loose.id, isNotEmpty);

      final garden = Places(ann).areas().firstWhere((a) => a.name == 'Garden');
      final closed = ann.packOfAll(closed: {garden.id});
      expect(closed.areas.map((a) => a.name), ['Health']);
      expect(closed.notes.map((n) => n.text), isNot(contains('Digging')));
      final session = closed.notes[indexOf(closed, 'Training')];
      expect(session.session!.pool, [indexOf(closed, 'Laps')]);
    });

    test("of everything in the person's own order of the Areas (#280), "
        'those missing from it by the name the app shows', () {
      final h = health();
      ann.moveArea(h.area, 0);
      final places = Places(ann);
      Notes(ann).capture('Boxes', place: places.addArea('attic').id);
      Notes(ann).capture('Tyres', place: places.addArea('Bike').id);
      expect(ann.packOfAll().areas.map((a) => a.name), [
        'Health',
        'Garden',
        'attic',
        'Bike',
      ]);
      final shown = ann.packOfAll(
        shown: (a) => a.name == 'attic' ? 'Loft' : a.name,
      );
      expect(shown.areas.map((a) => a.name), [
        'Health',
        'Garden',
        'Bike',
        'attic',
      ]);
    });

    test('everything is ticked but Reminders and files; the size follows '
        'the ticks', () {
      final h = health();
      Routines(ann).spawn(
        h.plain,
        Schedule(
          start: Day(2026, 3, 2),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      Routines(ann).spawn(
        h.plain,
        Schedule(start: Day(2026, 3, 9), times: [ClockTime(9, 0)]),
      );
      final pack = ann.packOf(h.area);
      final ticks = pack.exportTicks();
      final p = indexOf(pack, 'Drink water');
      final m = indexOf(pack, 'Route');
      final routines = pack.notes[p].routines;
      final daily = routines.indexWhere((r) => !r.once);
      final once = routines.indexWhere((r) => r.once);
      expect(ticks, contains(PackItemKeys.routine(p, daily)));
      expect(ticks, isNot(contains(PackItemKeys.routine(p, once))));
      expect(ticks, isNot(contains(PackItemKeys.photo(m, 0))));
      expect(ticks, isNot(contains(PackItemKeys.recording(m, 0))));
      expect(ticks, contains(PackItemKeys.photoText(m, 0)));
      expect(ticks, contains(PackItemKeys.transcript(m, 0)));
      final before = pack.size(ticks);
      final withPhoto = pack.size({...ticks, PackItemKeys.photo(m, 0)});
      expect(withPhoto - before, 400);
      expect(pack.size(pack.allKeys()), greaterThan(withPhoto));
      expect(pack.size({}), 0);
      // A file of a Note not ticked counts for nothing.
      final without = pack.tick(ticks, PackItemKeys.note(m), false);
      expect(
        pack.size({...without, PackItemKeys.photo(m, 0)}),
        pack.size(without),
      );
    });
  });

  group('what a Note brings', () {
    test('never a Tally entry, a tick, a check-off, a Capture Context or '
        'an eaten Frog', () {
      final h = health();
      Tallies(ann).add(
        h.tally,
        values: {Tallies(ann).byNote(h.tally)!.fields.single.id: 7777},
        line: 'secret entry line',
      );
      final lines = Checklists(ann).byNote(h.checklist)!.lines;
      Checklists(ann).check(lines.first.id);
      final routine = Routines(ann).spawn(
        h.plain,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      Routines(ann)
          .markDone(routine.id, Occurrence(Day(2026, 3, 1), ClockTime(8, 0)));
      Notes(ann).locate(h.plain, Coordinates(52.52, 13.405));
      final app = Notes(ann).capture(
        'Shared in',
        place: h.area,
        sourceApp: const SourceApp('com.example.secretapp', label: 'Secret'),
      );
      final eaten = Notes(ann).capture('Eaten frog', place: h.area);
      final frog = Frogs(ann).spawn(eaten.id);
      Frogs(ann).markDone(frog.id);
      final open = Notes(ann).capture('Open frog', place: h.area);
      Frogs(ann).spawn(open.id);
      final pack = ann.packOf(h.area);
      final payload = _payloadOf(ann.writePack(pack, pack.allKeys()));
      for (final leak in [
        '7777',
        'secret entry line',
        'secretapp',
        'Secret',
        '52.52',
        'done@',
        '"checked"',
        '2026-03-01T09',
      ]) {
        expect(payload, isNot(contains(leak)), reason: leak);
      }
      expect(payload, contains('"text":"Shoes"'));
      expect(payload, contains(app.text));
      final picked = jsonDecode(payload) as Map<String, Object?>;
      final frogs = [
        for (final n in picked['notes']! as List<Object?>)
          if (n case {'frog': true, 'text': final t}) t,
      ];
      expect(frogs, ['Open frog']);
    });

    test('a photo and a recording only when ticked; their texts go '
        'without them', () {
      final h = health();
      final pack = ann.packOf(h.media);
      final ticks = pack.exportTicks();
      final opened = ben.openPack(ann.writePack(pack, ticks));
      final note = opened.pack.notes.single;
      expect(note.photos.single.hasFile, isFalse);
      expect(note.photos.single.text, 'Map text');
      expect(note.recordings.single.hasFile, isFalse);
      expect(note.recordings.single.text, 'Go left');
      ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      final added = Notes(ben).all().singleWhere((n) => n.text == 'Route');
      expect(added.photos, isEmpty);
      expect(added.removedPhotos, hasLength(1));
      expect(added.photoTexts.values, ['Map text']);
      expect(added.transcripts.values, ['Go left']);

      final whole = ben.openPack(
        ann.writePack(
          pack,
          {...ticks, PackItemKeys.photo(0, 0), PackItemKeys.recording(0, 0)}
            ..remove(PackItemKeys.transcript(0, 0)),
        ),
      );
      final kept = whole.pack.notes.single;
      expect(kept.photos.single.bytes, isNotNull);
      expect(kept.recordings.single.text, isNull);
      ben.addPack(
        whole,
        whole.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      final second = Notes(ben)
          .all()
          .firstWhere((n) => n.text == 'Route' && n.photos.isNotEmpty);
      expect(ben.photo(second.photos.single), isNotNull);
      expect(ben.audio(second.recordings.single), _bytes(2, 600));
      expect(second.transcripts, isEmpty);
    });

    test('a Checklist arrives unticked, a Tally with its fields and no '
        'entries, a Session with its pool', () {
      final h = health();
      Checklists(ann)
          .check(Checklists(ann).byNote(h.checklist)!.lines.first.id);
      Tallies(ann).add(
        h.tally,
        values: {Tallies(ann).byNote(h.tally)!.fields.single.id: 3},
      );
      final pack = ann.packOf(h.running);
      final opened = ben.openPack(ann.writePack(pack, pack.allKeys()));
      ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      final notes = Notes(ben).all();
      String id(String text) => notes.firstWhere((n) => n.text == text).id;
      final list = Checklists(ben).byNote(id('Bag'))!;
      expect(list.lines.map((l) => l.text), ['Shoes', 'Bottle']);
      expect(list.lines.any((l) => l.checked), isFalse);
      final tally = Tallies(ben).byNote(id('Laps'))!;
      expect(tally.fields.single.name, 'Laps');
      expect(tally.fields.single.target, 10);
      expect(Tallies(ben).entries(id('Laps')), isEmpty);
      final setup = Sessions(ben).setupOf(id('Training'))!;
      expect(setup.pool, [id('Laps'), id('Digging')]);
      expect(setup.rule, SessionRule.rolling);
    });
  });

  group('the Session rule', () {
    test('ticking a Session ticks its pool; unticking the last of its '
        'Tallies unticks it, and it never goes with an empty pool', () {
      final h = health();
      final pack = ann.packOf(h.session);
      final s = PackItemKeys.note(0);
      final laps = PackItemKeys.note(1);
      final digging = PackItemKeys.note(2);
      var ticks = pack.tick({}, s, true);
      expect(ticks, containsAll([s, laps, digging]));
      ticks = pack.tick(ticks, laps, false);
      expect(ticks, contains(s));
      expect(pack.picked(ticks).notes.first.session!.pool, [1]);
      ticks = pack.tick(ticks, digging, false);
      expect(ticks, isNot(contains(s)));
      // A Session ticked without any Tally of its pool still stays out.
      final forced = {s};
      expect(pack.picked(forced).notes, isEmpty);
    });

    test('the same holds in the tree of a Pack opened', () {
      final h = health();
      final pack = ann.packOf(h.session);
      final opened = ben.openPack(ann.writePack(pack, pack.allKeys()));
      var ticks = opened.pack.allKeys();
      ticks = opened.pack.tick(ticks, PackItemKeys.note(1), false);
      ticks = opened.pack.tick(ticks, PackItemKeys.note(2), false);
      expect(ticks, isNot(contains(PackItemKeys.note(0))));
      final added = ben.addPack(
        opened,
        {...ticks, PackItemKeys.note(0)},
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      expect(added.notes, 0);
      expect(Notes(ben).all(), isEmpty);
    });
  });

  group('adding', () {
    test('lands in new Areas and Topics with a suffix where a name is '
        'taken; a Tally from outside in its own new Area', () {
      final h = health();
      Places(ben).addArea('Health');
      Places(ben).addArea('Health (Pack)');
      final pack = ann.packOf(h.running);
      final opened = ben.openPack(ann.writePack(pack, pack.allKeys()));
      final added = ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      final names = [
        for (final id in added.areas) Places(ben).areaById(id)!.name,
      ];
      expect(names, ['Health (Pack 2)', 'Garden']);
      final topics = Places(ben).topics(added.areas.first);
      expect(topics.map((t) => t.name), ['Running']);
      final laps = Notes(ben).all().firstWhere((n) => n.text == 'Laps');
      expect(laps.place, topics.single.id);
      final digging = Notes(ben).all().firstWhere((n) => n.text == 'Digging');
      expect(digging.place, added.areas.last);
      expect(Notes(ben).unsorted(), isEmpty);
      // The same Pack again adds the copies again.
      final again = ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      expect(Places(ben).areaById(again.areas.first)!.name, 'Health (Pack 3)');
      expect(Places(ben).areaById(again.areas.last)!.name, 'Garden (Pack)');
    });

    test('Routines start on the day of Add; a Reminder keeps its day, one '
        'past is left out', () {
      final plain = Notes(ann).capture('Pills');
      Routines(ann).spawn(
        plain.id,
        Schedule(
          start: Day(2025, 1, 6),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY;COUNT=3'),
        ),
      );
      Routines(ann).spawn(
        plain.id,
        Schedule(start: Day(2026, 3, 9), times: [ClockTime(9, 0)]),
      );
      Routines(ann).spawn(
        plain.id,
        Schedule(start: Day(2026, 3, 1), times: [ClockTime(8, 0)]),
      );
      final pack = ann.packOf(plain.id);
      final opened = ben.openPack(ann.writePack(pack, pack.allKeys()));
      final added = ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      expect(Places(ben).areaById(added.areas.single)!.name, 'From a Pack');
      final note = Notes(ben).all().single;
      final routines = Routines(ben).ofNote(note.id);
      expect(routines, hasLength(2));
      final daily = routines.firstWhere((r) => !r.schedule.once).schedule;
      expect(daily.start, Day(2026, 3, 1));
      expect(daily.rule!.format(), 'FREQ=DAILY;COUNT=3');
      expect(daily.times, [ClockTime(8, 0)]);
      final once = routines.firstWhere((r) => r.schedule.once).schedule;
      expect(once.start, Day(2026, 3, 9));
    });

    test('with "Claim as mine" unticked each Note has a "From" badge the '
        'receiver can remove; ticked it has none', () {
      final h = health();
      final pack = ann.packOf(h.plain);
      final bytes = ann.writePack(pack, pack.allKeys(), addProfile: true);
      final opened = ben.openPack(bytes);
      expect(opened.from!.name, 'Ann');
      expect(opened.from!.id, ann.ownProfile!.id);
      ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: false,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      final note = Notes(ben).all().single;
      expect(ben.packFrom(note.id), (id: ann.ownProfile!.id, name: 'Ann'));
      ben.removePackFrom(note.id);
      expect(ben.packFrom(note.id), isNull);
      // Undone (#268): the badge comes back as it was, in one entry.
      final before = ben.allEntries().length;
      ben.restorePackFrom(note.id);
      expect(ben.packFrom(note.id), (id: ann.ownProfile!.id, name: 'Ann'));
      expect(ben.allEntries().length, before + 1);
      ben.restorePackFrom(note.id);
      expect(ben.allEntries().length, before + 1);
      ben.removePackFrom(note.id);
      ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      expect(Notes(ben).all().map((n) => ben.packFrom(n.id)), [null, null]);
    });

    test("what another person's Device wrote counts for nothing (Q554 (a)): "
        'it neither sets, changes nor hides the badge, and it can still be '
        'removed', () {
      final h = health();
      final pack = ann.packOf(h.plain);
      final opened = ben.openPack(
        ann.writePack(pack, pack.allKeys(), addProfile: true),
      );
      ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: false,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      final badged = Notes(ben).all().single.id;
      final own = Notes(ben).capture('Milk').id;
      final badge = (id: ann.ownProfile!.id, name: 'Ann');
      // As a modified app in a Shared Area would: Cy's Device writes the
      // field of Ben's Notes.
      final cy = device(clock, 'Cy');
      String from(String name) => jsonEncode({'id': 'cy', 'name': name});
      ben
        ..insert(cy.append(badged, PackKeys.from, from('Cy')))
        ..insert(cy.append(own, PackKeys.from, from('Cy')));
      expect(ben.packFrom(badged), badge);
      expect(ben.packFrom(own), isNull);
      ben.insert(cy.append(badged, PackKeys.from, null));
      expect(ben.packFrom(badged), badge);
      ben.removePackFrom(badged);
      expect(ben.packFrom(badged), isNull);
      ben.removePackFrom(own);
      expect(ben.current(own, PackKeys.from), from('Cy'));
      // Undone, it is Ann's again, never Cy's; a Note that never had
      // one gets none.
      ben.restorePackFrom(badged);
      expect(ben.packFrom(badged), badge);
      final before = ben.allEntries().length;
      ben.restorePackFrom(own);
      expect(ben.packFrom(own), isNull);
      expect(ben.allEntries().length, before);
    });

    test('a help Note arrives under a new id', () {
      final note = Notes(ann).capture('Quick start');
      final pack = ann.packOf(note.id);
      final opened = ben.openPack(ann.writePack(pack, pack.allKeys()));
      ben.addPack(
        opened,
        opened.pack.allKeys(),
        claim: true,
        wallClock: _wall,
        suffix: 'Pack',
        noArea: 'From a Pack',
      );
      expect(Notes(ben).all().single.id, isNot(note.id));
    });
  });

  group('opening', () {
    test('"Add my Profile" signs the Pack; a changed one is refused', () {
      final h = health();
      final pack = ann.packOf(h.plain);
      final signed = ann.writePack(pack, pack.allKeys(), addProfile: true);
      final json = jsonDecode(utf8.decode(signed)) as Map<String, Object?>;
      expect(json.keys, ['pack', 'sig', 'type']);
      expect(ben.openPack(signed).from!.name, 'Ann');
      final changed = utf8.encode(
        utf8.decode(signed).replaceFirst('Drink water', 'Drink wine'),
      );
      expect(() => ben.openPack(changed), throwsA(isA<NotAPack>()));
      final unsigned = jsonEncode({...json}..remove('sig'));
      expect(
        () => ben.openPack(utf8.encode(unsigned)),
        throwsA(isA<NotAPack>()),
      );
      final plain = ann.writePack(pack, pack.allKeys());
      expect(jsonDecode(utf8.decode(plain)), isNot(contains('sig')));
      expect(ben.openPack(plain).from, isNull);
    });

    test('a blocked sender\'s Pack is refused; a later format needs an '
        'update; other files are no Pack', () {
      final h = health();
      final pack = ann.packOf(h.plain);
      final signed = ann.writePack(pack, pack.allKeys(), addProfile: true);
      ben.openProfileCard(ann.profileCard()!);
      ben.blockPerson(ann.ownProfile!.id);
      expect(() => ben.openPack(signed), throwsA(isA<PackFromBlocked>()));
      final json = jsonDecode(utf8.decode(signed)) as Map<String, Object?>;
      for (final later in [2, '1', 1.5, null]) {
        expect(
          () =>
              ben.openPack(utf8.encode(jsonEncode({...json, 'format': later}))),
          throwsA(isA<PackNeedsUpdate>()),
          reason: '$later',
        );
      }
      expect(looksLikePack(signed), isTrue);
      expect(looksLikePack(ann.profileCardFile()!), isFalse);
      expect(looksLikeProfileCard(signed), isFalse);
      expect(() => ben.openPack(utf8.encode('{}')), throwsA(isA<NotAPack>()));
    });

    test('a Pack is written and opened off the Store\'s isolate', () async {
      final h = health();
      final pack = ann.packOf(h.plain);
      final ticks = pack.allKeys();
      final job = ann.packWrite(pack, ticks, addProfile: true);
      final bytes = await Isolate.run(job.encode);
      expect(bytes, ann.writePack(pack, ticks, addProfile: true));
      final checked = await _checkAside(bytes);
      expect(checked.card?.id, ann.ownProfile!.id);
      ben.refuseBlockedSender(checked);
      final read = await _readAside(checked);
      final opened = ben.openedPack(checked, read);
      expect(opened.from!.name, 'Ann');
      expect(
        read.notes.map((n) => n.text),
        ben.openPack(bytes).pack.notes.map((n) => n.text),
      );
      ben.blockPerson(ann.ownProfile!.id);
      expect(
        () => ben.refuseBlockedSender(checked),
        throwsA(isA<PackFromBlocked>()),
      );
      expect(() => _checkAside(utf8.encode('{}')), throwsA(isA<NotAPack>()));
    });

    test('the card and the signature are checked before the Areas and '
        'Notes are read (ADR 0043)', () {
      final card = ann.profileCardText()!;
      final type = (jsonDecode(
        utf8.decode(ann.writePack(ann.packOf(health().plain), {})),
      ) as Map<String, Object?>)['type'];
      // A payload with a Note that is none: it does not parse.
      final payload = jsonEncode({
        'areas': <Object?>[],
        'notes': [
          {'bad': true},
        ],
        'profile': card,
      });
      List<int> file(String sig) =>
          utf8.encode(jsonEncode({'pack': payload, 'sig': sig, 'type': type}));
      final signed = file(ann.signingKey.sign(packBytes(payload)));
      final badSig = file(ann.signingKey.sign(packBytes('$payload ')));
      expect(() => ben.openPack(signed), throwsA(isA<NotAPack>()));
      expect(() => ben.openPack(badSig), throwsA(isA<NotAPack>()));
      ben.openProfileCard(card);
      ben.blockPerson(ann.ownProfile!.id);
      // Only the order tells the two apart: the sender is refused before
      // the Notes fail.
      expect(() => ben.openPack(signed), throwsA(isA<PackFromBlocked>()));
      expect(() => ben.openPack(badSig), throwsA(isA<NotAPack>()));
    });
  });
}

// Top level, so the closures carry no Store to the other isolate.
Future<CheckedPack> _checkAside(List<int> bytes) =>
    Isolate.run(() => checkPack(bytes));

Future<Pack> _readAside(CheckedPack checked) =>
    Isolate.run(() => readPack(checked));

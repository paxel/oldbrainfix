import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;

/// The wall clock of these tests: UTC.
DateTime utc(DateTime instant) => instant;

void main() {
  // 09:00 on Monday, 2 March 2026; Quiet Hours 22:00 to 08:00, a repeat
  // every hour: the defaults.
  var now = DateTime.utc(2026, 3, 2, 9);
  late Store ann;
  late Store ben;
  late Store cat;
  late Directory dir;
  var n = 0;
  String path() => '${dir.path}/c${n++}.$bundleExtension';

  setUp(() {
    now = DateTime.utc(2026, 3, 2, 9);
    ann = device(() => now, 'Ann')..renameOwnProfile('Ann');
    ben = device(() => now, 'Ben')..renameOwnProfile('Ben');
    cat = device(() => now, 'Cat')..renameOwnProfile('Cat');
    dir = Directory.systemTemp.createTempSync('changes_nudges_test_');
    for (final other in [ben, cat]) {
      other.openProfileCard(ann.profileCard()!);
      ann.openProfileCard(other.profileCard(back: true)!);
    }
  });

  tearDown(() => dir.deleteSync(recursive: true));

  String idOf(Store store) => store.ownProfile!.id;

  DateTime at(int hour, [int minute = 0, int day = 2]) =>
      DateTime.utc(2026, 3, day, hour, minute);

  /// Ann shares a new Area [name] with [to], changes travelling as
  /// [transport]; [to] accepts the offer. Returns the Area.
  Future<String> shared(
    Store to, {
    String name = 'Household',
    Transport transport = Transport.bundle,
  }) async {
    final area = shareAreaWith(
      ann,
      Places(ann).addArea(name).id,
      idOf(to),
      role: AreaRole.member,
      transport: transport,
    );
    final file = path();
    await writeAreaOffer(ann, file, area, idOf(to));
    acceptAreaOffer(to, (await importBundle(to, file)).offer!.id);
    return area;
  }

  /// Ann sends [to] a bundle of what is new for them, as the share sheet
  /// does once the user picked an app.
  Future<void> send(Store to) async {
    final seq = ann.currentSeq();
    expect(await writeBundle(ann, path(), forPerson: idOf(to)), isNotNull);
    ann.noteChangesSent(idOf(to), seq);
  }

  /// When the Changes Nudge fires after now, [count] times.
  List<DateTime> firings(int count) {
    final nudge = Alarms(ann).changesNudge(utc);
    if (nudge == null) return const [];
    var after = now;
    return [
      for (var i = 0; i < count; i++) after = nudge.firesAfter(after, utc),
    ];
  }

  test('sharing and taking an offer leave nothing waiting', () async {
    await shared(ben);
    expect(ann.waitingForChanges(), isEmpty);
    expect(ben.waitingForChanges(), isEmpty);
    expect(Alarms(ann).changesNudge(utc), isNull);
  });

  test('the Changes Nudge comes 30 minutes after the first unsent change, '
      'again every hour, and waits out the Quiet Hours', () async {
    final area = await shared(ben);
    Notes(ann).capture('Buy milk', place: area);
    now = at(9, 10);
    Notes(ann).capture('Buy bread', place: area);
    expect(ann.waitingForChanges(), {idOf(ben): at(9)});

    var nudge = Alarms(ann).changesNudge(utc)!;
    expect(nudge.people, [idOf(ben)]);
    expect(nudge.since, isNull);
    expect(nudge.at, at(9, 30));
    expect(firings(3), [at(9, 30), at(10, 30), at(11, 30)]);

    now = at(9, 30);
    nudge = Alarms(ann).changesNudge(utc)!;
    expect(nudge.since, at(9, 30));
    expect(nudge.at, at(10, 30));

    // Ignored all day: 22:30 waits for 08:00, and the hours count on.
    now = at(21);
    expect(firings(3), [at(21, 30), at(8, 0, 3), at(9, 0, 3)]);
    now = at(23);
    expect(Alarms(ann).changesNudge(utc)!.at, at(8, 0, 3));
  });

  test('sending the bundle ends it; it keeps everything until the person '
      'sends back', () async {
    final area = await shared(ben);
    Notes(ann).capture('Buy milk', place: area);
    now = at(10);
    await send(ben);
    expect(ann.waitingForChanges(), isEmpty);
    expect(Alarms(ann).changesNudge(utc), isNull);
    // The bundle may never arrive: the next one carries it again.
    expect(ann.entriesForPerson(idOf(ben)), isNotEmpty);

    now = at(10, 5);
    Notes(ann).capture('Buy bread', place: area);
    expect(Alarms(ann).changesNudge(utc)!.at, at(10, 35));
  });

  test('a share sheet left without an app leaves it waiting', () async {
    final area = await shared(ben);
    Notes(ann).capture('Buy milk', place: area);
    expect(await writeBundle(ann, path(), forPerson: idOf(ben)), isNotNull);
    expect(ann.waitingForChanges(), {idOf(ben): at(9)});
  });

  test('one Changes Nudge covers everyone waiting; sending to one leaves '
      'the other', () async {
    final house = await shared(ben);
    final garden = await shared(cat, name: 'Garden');
    Notes(ann).capture('Water the roses', place: garden);
    expect(Alarms(ann).changesNudge(utc)!.at, at(9, 30));

    // Ben's first change came after it fired: it comes again at his own
    // time, sooner than the repeat, for both.
    now = at(9, 40);
    Notes(ann).capture('Buy milk', place: house);
    final nudge = Alarms(ann).changesNudge(utc)!;
    expect(nudge.people, [idOf(cat), idOf(ben)]);
    expect(nudge.since, at(9, 30));
    expect(nudge.at, at(10, 10));
    expect(firings(2), [at(10, 10), at(11, 10)]);

    await send(cat);
    expect(Alarms(ann).changesNudge(utc)!.people, [idOf(ben)]);
    expect(Alarms(ann).changesNudge(utc)!.at, at(10, 10));
  });

  test('a change the person sent back never waits for them', () async {
    final area = await shared(ben);
    Notes(ben).capture('From Ben', place: area);
    final back = path();
    await writeBundle(ben, back, forPerson: idOf(ann));
    await importBundle(ann, back);
    expect(ann.waitingForChanges(), isEmpty);
  });

  test('changes by a shared folder, or for a blocked person, wait for no '
      'bundle', () async {
    final house = await shared(ben, transport: Transport.folder);
    Notes(ann).capture('Buy milk', place: house);
    expect(ann.waitingForChanges(), isEmpty);

    final garden = await shared(cat, name: 'Garden');
    Notes(ann).capture('Water the roses', place: garden);
    expect(ann.waitingForChanges().keys, [idOf(cat)]);
    ann.blockPerson(idOf(cat));
    expect(ann.waitingForChanges(), isEmpty);
    expect(Alarms(ann).changesNudge(utc), isNull);
  });

  test('another Device\'s change counts from when it came here, never from '
      'its own clock', () async {
    final area = await shared(ben);
    final file = path();
    shareAreaWith(
      ann,
      area,
      idOf(cat),
      role: AreaRole.member,
      transport: Transport.bundle,
    );
    await writeAreaOffer(ann, file, area, idOf(cat));
    acceptAreaOffer(cat, (await importBundle(cat, file)).offer!.id);

    // Ben's clock runs a year ahead; his change reaches Ann at 11:00.
    now = DateTime.utc(2027, 3, 2, 9);
    Notes(ben).capture('From Ben', place: area);
    final back = path();
    await writeBundle(ben, back, forPerson: idOf(ann));
    now = at(11);
    await importBundle(ann, back);
    // Ben lacks Cat's grant since 09:00; Cat lacks Ben's Note since
    // 11:00, not since a year ahead.
    expect(ann.waitingForChanges(), {idOf(ben): at(9), idOf(cat): at(11)});
  });

  test('taking an offer counts what came with it as sent to the sharer, '
      'other people\'s changes too', () async {
    final area = await shared(cat);
    Notes(cat).capture('From Cat', place: area);
    final back = path();
    await writeBundle(cat, back, forPerson: idOf(ann));
    await importBundle(ann, back);

    // Ann offers the Area, Cat's Note in it, to Ben.
    shareAreaWith(
      ann,
      area,
      idOf(ben),
      role: AreaRole.member,
      transport: Transport.bundle,
    );
    final file = path();
    await writeAreaOffer(ann, file, area, idOf(ben));
    acceptAreaOffer(ben, (await importBundle(ben, file)).offer!.id);
    expect(Notes(ben).all().map((n) => n.text), contains('From Cat'));
    expect(ben.waitingForChanges(), isEmpty);
  });

  test('the sent marks are kept by Area, each only growing', () {
    ann.noteAreaSent(idOf(ben), 'area:b', 7);
    ann.noteAreaSent(idOf(ben), 'area:a', 3);
    ann.noteAreaSent(idOf(ben), 'area:b', 5);
    expect(
      ann.localSetting('changesSent:${idOf(ben)}'),
      '{"area:a":3,"area:b":7}',
    );
    expect(ann.changesSentTo(idOf(ben)), {'area:a': 3, 'area:b': 7});
  });

  test('the plan sorts the people by their first change, then by id', () {
    final plan = changesNudgePlan(
      waiting: {'b': at(9), 'a': at(9), 'c': at(8)},
      now: at(7),
      wallClock: utc,
      repeat: const Duration(hours: 1),
    )!;
    expect(plan.people, ['c', 'a', 'b']);
    expect(plan.at, at(8, 30));
    expect(
      changesNudgePlan(
        waiting: const {},
        now: at(7),
        wallClock: utc,
        repeat: const Duration(hours: 1),
      ),
      isNull,
    );
  });
}

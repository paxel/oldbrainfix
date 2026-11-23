import 'dart:io';
import 'dart:math';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:test/test.dart';

import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  test('own entries are signed and verify under the Device key', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    final e = store.append('note:a', 'text', 'hello');
    final bytes = entryBytes(
      device: e.device,
      dseq: e.dseq,
      entity: e.entity,
      field: e.field,
      value: e.value,
      dateIso: e.date.toIso8601String(),
      author: e.author,
      recordedIso: e.recorded.toIso8601String(),
    );
    expect(verifySignature(store.signingKey.publicKey, bytes, e.sig), isTrue);
    expect(store.deviceId, deviceIdFromKey(store.signingKey.publicKey));
  });

  test('reads the fields of many entities in one query as one by one '
      '(#204)', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('note:a', 'text', 'new', date: DateTime.utc(2026, 5, 2));
    store.append('note:a', 'text', 'old', date: DateTime.utc(2026, 5, 1));
    store.append('note:a', 'place', 'home');
    store.append('note:b', 'text', 'b');
    store.append('note:b', 'text', null);
    final ids = [for (var i = 0; i < 1200; i++) 'note:x$i'];
    for (final id in ids) {
      store.append(id, 'text', id);
    }
    // Shown pending changes count as they do one by one.
    final set = store.openPending('ai-task', shown: true);
    store.draftPending(set, 'note:a', 'place', 'work');
    store.draftPending(set, 'note:c', 'text', 'drafted');
    final all = ['note:a', 'note:b', 'note:c', 'note:none', ...ids];
    final before = store.historyReads;
    final read = store.currentFieldsOf(all);
    expect(store.historyReads - before, 1);
    expect(read.keys, isNot(contains('note:none')));
    expect(read, {
      for (final id in all)
        if (store.currentFields(id) case final fields when fields.isNotEmpty)
          id: fields,
    });
    expect(read['note:a'], {'text': 'new', 'place': 'work'});
    expect(read['note:c'], {'text': 'drafted'});
    expect(store.currentFieldsOf(const []), isEmpty);
  });

  test('reads the fields of every entity of a type in one query as one '
      'by one (#216)', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    for (var i = 0; i < 30; i++) {
      store.append('thing:$i', r'$type', 'thing');
      store.append('thing:$i', 'text', 't$i');
    }
    store.append('thing:0', r'$type', 'other');
    store.append('other:a', r'$type', 'other');
    // Shown pending changes count as they do one by one.
    final set = store.openPending('ai-task', shown: true);
    store.draftPending(set, 'thing:new', r'$type', 'thing');
    store.draftPending(set, 'thing:1', 'text', 'drafted');
    final before = store.historyReads;
    final read = store.currentFieldsOfType('thing');
    expect(store.historyReads - before, 1);
    expect(read, {
      for (final id in store.entitiesOfType('thing'))
        id: store.currentFields(id),
    });
    expect(read.keys, isNot(contains('thing:0')));
    expect(read['thing:1'], {r'$type': 'thing', 'text': 'drafted'});
    expect(read.keys, contains('thing:new'));
  });

  test('lists the entities of a type in one read as one by one '
      '(obf-1069)', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    // More than one query names at once.
    for (var i = 0; i < 600; i++) {
      store.append('thing:$i', r'$type', 'thing');
      store.append('thing:$i', 'text', 't$i');
    }
    store.append('thing:0', r'$type', 'other');
    store.append('thing:1', r'$type', null);
    // Dated before it was a thing: it stays one.
    store.append('thing:2', r'$type', 'other', date: DateTime.utc(2000));
    store.append('other:a', r'$type', 'other');
    // Shown pending changes count as they do one by one.
    final set = store.openPending('ai-task', shown: true);
    store.draftPending(set, 'thing:new', r'$type', 'thing');
    store.draftPending(set, 'thing:3', r'$type', 'other');
    store.draftPending(set, 'other:a', r'$type', 'thing');
    final before = store.historyReads;
    final listed = store.entitiesOfType('thing');
    expect(store.historyReads - before, 1);
    expect(listed, [
      for (final id in store.everOfType('thing'))
        if (store.current(id, r'$type') == 'thing') id,
    ]);
    expect(listed, containsAll(['thing:2', 'thing:new', 'other:a']));
    expect(listed, isNot(contains('thing:0')));
    expect(listed, isNot(contains('thing:1')));
    expect(listed, isNot(contains('thing:3')));
    expect(store.entitiesOfType('none'), isEmpty);
  });

  test('lists the entities of a type without a \$type the roles ignore '
      '(obf-1069)', () {
    final clock = ticking();
    final ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    addTearDown(ann.close);
    final ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
    addTearDown(ben.close);
    final house = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(house, role: AreaRole.member));
    final milk = Notes(ann).capture('Buy milk', place: house).id;
    exchange(ben, ann);
    // Not Ben's to do, as a modified app would write it all the same.
    ben.append(milk, NoteKeys.type, null);
    exchange(ben, ann);
    expect(ann.judged.ignored, isNotEmpty);
    final listed = ann.entitiesOfType(NoteKeys.typeNote);
    expect(listed, contains(milk));
    expect(listed, [
      for (final id in ann.everOfType(NoteKeys.typeNote))
        if (ann.current(id, NoteKeys.type) == NoteKeys.typeNote) id,
    ]);
  });

  test('reads a stored time as DateTime.parse does (obf-1069)', () {
    void same(String text) {
      final DateTime expected;
      try {
        expected = DateTime.parse(text);
      } on FormatException {
        expect(() => parseStoredDate(text), throwsFormatException);
        return;
      }
      final read = parseStoredDate(text);
      expect(read, expected, reason: text);
      expect(read.isUtc, expected.isUtc, reason: text);
    }

    final random = Random(1069);
    for (var i = 0; i < 2000; i++) {
      final at = DateTime.fromMicrosecondsSinceEpoch(
        random.nextInt(1 << 32) * 1000000 + random.nextInt(1000000),
        isUtc: true,
      );
      same(at.toIso8601String());
      same(DateTime.utc(at.year, at.month, at.day, at.hour).toIso8601String());
    }
    for (final text in [
      '2021-02-30T00:00:00.000Z',
      '2026-13-01T24:60:61.000Z',
      '2026-11-22T10:00:00Z',
      '2026-11-22T10:00:00.000+01:00',
      '2026-11-22T10:00:00.000',
      '2026-11-22 10:00:00.000Z',
      '+012026-11-22T10:00:00.000Z',
      '2026-1x-22T10:00:00.000Z',
      '2026-11-22T10:00:00.00xZ',
    ]) {
      same(text);
    }
  });

  test('a read sees what another connection wrote after it (obf-1069)', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    final first = Store.open(path);
    addTearDown(first.close);
    final second = Store.open(path);
    addTearDown(second.close);
    expect(first.history('note:a'), isEmpty);
    expect(first.localSetting('seen'), isNull);
    final seq = first.currentSeq();
    second.append('note:a', 'text', 'from the other one');
    second.setLocalSetting('seen', 'yes');
    expect(first.history('note:a').single.value, 'from the other one');
    expect(first.localSetting('seen'), 'yes');
    expect(first.currentSeq(), seq + 1);
  });

  test('the latest date wins, not the latest append', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('note:a', 'text', 'new', date: DateTime.utc(2026, 5, 2));
    store.append('note:a', 'text', 'old', date: DateTime.utc(2026, 5, 1));
    expect(store.current('note:a', 'text'), 'new');
  });

  test('dseq counts up per Device and shows in the version vector', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.append('note:a', 'text', '1');
    final e = store.append('note:a', 'text', '2');
    expect(e.dseq, 2);
    expect(store.versionVector(), {store.deviceId: 2});
  });

  test('a failing batch writes nothing', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(
      () => store.batch(() {
        store.append('note:a', 'text', 'x');
        throw StateError('boom');
      }),
      throwsStateError,
    );
    expect(store.allEntries(), isEmpty);
  });

  test('a batch inside a batch joins it: committed and heard once, or '
      'rolled back as a whole', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    var heard = 0;
    store.addListener(() => heard++);
    store.batch(() {
      store.append('note:a', 'text', 'one');
      store.batch(() => store.append('note:b', 'text', 'two'));
      expect(heard, 0, reason: 'nothing is heard before the commit');
      expect(store.current('note:b', 'text'), 'two');
      store.append('note:c', 'text', 'three');
    });
    expect(heard, 1);
    expect(store.allEntries(), hasLength(3));
    expect(
      () => store.batch(() {
        store.append('note:d', 'text', 'four');
        store.batch(() {
          store.append('note:e', 'text', 'five');
          throw StateError('boom');
        });
      }),
      throwsStateError,
    );
    expect(heard, 1, reason: 'a failing batch changed nothing');
    expect(store.allEntries(), hasLength(3));
    expect(store.current('note:d', 'text'), isNull);
    store.append('note:f', 'text', 'six');
    expect(heard, 2, reason: 'out of the batch, appends are heard again');
  });

  test('the log and the Device identity survive a reopen', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/log.db';
    final first = Store.open(path);
    first.append('note:a', 'text', 'kept');
    final device = first.deviceId;
    first.close();
    final second = Store.open(path);
    addTearDown(second.close);
    expect(second.deviceId, device);
    expect(second.current('note:a', 'text'), 'kept');
  });

  test('a sync through another connection to the same file shows once '
      'that one says so (#225)', () {
    final dir = Directory.systemTemp.createTempSync('obf');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/oldbrainfix.db';
    final clock = ticking();
    final ui = Store.open(path, clock: clock)
      ..applyProfileDefaults(name: 'Ann');
    addTearDown(ui.close);
    var told = 0;
    var profile = 0;
    ui.addListener(() => told++);
    ui.addProfileListener(() => profile++);
    // The sync's own connection, as its isolate opens it.
    final sync = Store.open(path, clock: clock);
    addTearDown(sync.close);
    expect(sync.deviceId, ui.deviceId);
    final ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
    addTearDown(ben.close);
    final house = ui.share(Places(ui).addArea('Household').id);
    told = 0;
    expect(ui.keptPeople, isEmpty);
    exchange(ben, sync, inviting: Invite(house, role: AreaRole.member));
    Notes(ben).capture('From Ben', place: house);
    exchange(ben, sync);
    expect(told, 0, reason: 'the other connection told only its own');
    expect(ui.keptPeople, isEmpty, reason: 'what was read stays until told');
    ui.changedElsewhere(profile: true);
    expect(told, 1);
    expect(profile, 1);
    expect(ui.keptPeople.map((p) => p.name), ['Ben']);
    expect([for (final n in Notes(ui).all()) n.text], contains('From Ben'));
    ui.changedElsewhere();
    expect(told, 2);
    expect(profile, 1, reason: 'only when the Profiles changed');
  });

  group('the database version', () {
    late Directory dir;
    late String path;

    setUp(() {
      dir = Directory.systemTemp.createTempSync('obf');
      path = '${dir.path}/oldbrainfix.db';
      final store = Store.open(path);
      store.append('note:a', 'text', 'kept');
      store.close();
    });
    tearDown(() => dir.deleteSync(recursive: true));

    int version() {
      final db = sqlite3.open(path);
      try {
        return db.select('PRAGMA user_version').first.columnAt(0) as int;
      } finally {
        db.close();
      }
    }

    void setVersion(int version) {
      final db = sqlite3.open(path);
      db.execute('PRAGMA user_version = $version');
      db.close();
    }

    /// Every file beside the database, with its bytes.
    Map<String, List<int>> files() => {
      for (final file in dir.listSync(recursive: true).whereType<File>())
        file.path: file.readAsBytesSync(),
    };

    test('is recorded', () {
      expect(version(), storeVersion);
      expect(storeVersion, 1);
    });

    test('a database from before it was recorded keeps its log and gets '
        'it', () {
      setVersion(0);
      final store = Store.open(path);
      addTearDown(store.close);
      expect(store.current('note:a', 'text'), 'kept');
      expect(version(), 1);
    });

    test('a database a newer version saved is not opened, and nothing in '
        'it or beside it changes', () {
      // The directories beside it are made again only by an open that
      // goes ahead.
      Directory('${dir.path}/photos').deleteSync(recursive: true);
      Directory('${dir.path}/audio').deleteSync(recursive: true);
      for (final newer in [2, 1000, -1]) {
        setVersion(newer);
        final before = files();
        expect(
          () => Store.open(path),
          throwsA(
            isA<StoreNeedsUpdate>().having((e) => e.version, 'version', newer),
          ),
        );
        expect(files(), before);
        expect(Directory('${dir.path}/photos').existsSync(), isFalse);
        expect(version(), newer);
      }
      setVersion(1);
      final store = Store.open(path);
      addTearDown(store.close);
      expect(store.current('note:a', 'text'), 'kept');
    });
  });

  test('listeners hear each append, a batch once, and settings never', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    var heard = 0;
    void listener() => heard++;
    store.addListener(listener);
    store.append('note:a', 'text', 'one');
    expect(heard, 1);
    store.batch(() {
      store.append('note:a', 'text', 'two');
      store.append('note:a', 'text', 'three');
    });
    expect(heard, 2);
    store.setLocalSetting('theme', 'dark');
    expect(heard, 2);
    expect(
      () => store.batch(() {
        store.append('note:a', 'text', 'four');
        throw StateError('no');
      }),
      throwsStateError,
    );
    expect(heard, 2, reason: 'a failing batch changed nothing');
    store.removeListener(listener);
    store.append('note:a', 'text', 'five');
    expect(heard, 2);
  });

  test('a setting a later version wrote is kept as it is, and noted '
      '(ADR 0034)', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    const later = '{"generation":3,"v":2}';
    const listed = '[{"v":1},{"id":"a","v":"2"}]';
    store
      ..setLocalSetting('folderRead:x', later)
      ..setLocalSetting('changes:x', listed);
    expect(store.takeUpdateNotices(), isEmpty);
    store
      ..setLocalSetting('folderRead:x', '{"generation":1,"lines":{}}')
      ..removeLocalSetting('changes:x')
      ..forgetFolderReads();
    expect(store.localSetting('folderRead:x'), later);
    expect(store.localSetting('changes:x'), listed);
    expect(store.takeUpdateNotices(), [
      (UpdateNotices.settingsSource, UpdateNeeded.here),
    ]);
    // Version 1, said or not, and what is no object of a later one are
    // this version's.
    for (final mine in [
      '{"v":1}',
      '[{"v":1},2]',
      '"v"',
      '2',
      'not json',
      '{"a":{"v":2}}',
    ]) {
      store
        ..setLocalSetting('k', mine)
        ..setLocalSetting('k', 'next');
      expect(store.localSetting('k'), 'next', reason: mine);
      store.removeLocalSetting('k');
    }
    // The own Profile's record has a rule of its own.
    store
      ..setLocalSetting('profile', '{"id":"p","v":2,"version":1}')
      ..setLocalSetting('profile', '{"id":"p","version":2}');
    expect(store.localSetting('profile'), '{"id":"p","version":2}');
  });

  test('new ids come from the id maker, random by default', () {
    final ids = {Store.newId(), Store.newId()};
    expect(ids, hasLength(2));
    expect(ids.first, matches(RegExp(r'^[0-9a-f]{32}$')));
    final random = Store.idMaker;
    addTearDown(() => Store.idMaker = random);
    var n = 0;
    Store.idMaker = () => 'id${++n}';
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(Notes(store).capture('x').id, 'note:id1');
  });
}

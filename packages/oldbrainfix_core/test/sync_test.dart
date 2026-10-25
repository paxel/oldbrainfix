import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A clock shared by every store of a test, one second per read, so
/// Devices write in a known order.
DateTime Function() ticking() {
  var now = DateTime.utc(2026, 3, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

Store device(DateTime Function() clock, String author) =>
    Store.inMemory(clock: clock)..setLocalSetting('author', author);

/// One full exchange as the LAN session runs it, without the network:
/// the joiner sends what the host lacks, the host answers with what the
/// joiner lacks; the blobs follow both ways.
void exchange(Store joiner, Store host) {
  final request = syncRequest(joiner, host.versionVector(), name: 'Phone');
  final done = answerSync(
    host,
    jsonDecode(jsonEncode(request)) as Map<String, Object?>,
    name: 'Desk',
  )!;
  applyAnswer(
    joiner,
    jsonDecode(jsonEncode(done.answer)) as Map<String, Object?>,
  );
  for (final ref in joiner.missingBlobs()) {
    final (kind, hash) = splitBlobRef(ref);
    final bytes = host.blob(kind, hash);
    if (bytes != null) joiner.putBlob(kind, hash, bytes);
  }
  for (final ref in done.answer['wantBlobs'] as List) {
    final (kind, hash) = splitBlobRef(ref as String);
    final bytes = joiner.blob(kind, hash);
    if (bytes != null && host.knowsBlob(ref)) host.putBlob(kind, hash, bytes);
  }
}

List<String> texts(Store store) =>
    [for (final n in Notes(store).all()) n.text]..sort();

/// The [n]th edit of [noteId]'s text by [writer] as a later version
/// writes it (ADR 0019): with the keys of [extra], signed with them, in
/// canonical form; signed without them when [signExtra] is false.
String laterLine(
  Store writer,
  String noteId,
  int n,
  String text,
  Map<String, Object?> extra, {
  bool signExtra = true,
}) {
  final dseq = (writer.versionVector()[writer.deviceId] ?? 0) + n;
  final date = writer.clock().toUtc().toIso8601String();
  final value = StoredText.plain(text).encode();
  final json = <String, Object?>{
    'device': writer.deviceId,
    'dseq': dseq,
    'entity': noteId,
    'field': 'text',
    'value': value,
    'date': date,
    'author': 'Ada',
    'recorded': date,
    'sig': writer.signingKey.sign(
      entryBytes(
        device: writer.deviceId,
        dseq: dseq,
        entity: noteId,
        field: 'text',
        value: value,
        dateIso: date,
        author: 'Ada',
        recordedIso: date,
        extra: signExtra ? extra : const {},
      ),
    ),
    ...extra,
  };
  return canonicalEntry(json);
}

Entry entryOf(String line) =>
    Entry.fromJson(jsonDecode(line) as Map<String, dynamic>, text: line);

void main() {
  group('entries of a later version (ADR 0019)', () {
    late Store writer;
    late Store middle;
    late String noteId;

    setUp(() {
      final clock = ticking();
      writer = device(clock, 'Ada');
      middle = device(clock, 'Ada');
      noteId = Notes(writer).capture('First').id;
      middle.applyEntries(
        writer.entriesSince(const {}),
        keys: [writer.ownKeyRecord],
      );
    });

    test('are kept, applied up to the known version, and passed on byte '
        'for byte, signatures holding', () {
      final unknown = laterLine(writer, noteId, 1, 'Edited later', {
        'later': {'b': 2, 'a': 'x'},
      });
      final v2 = laterLine(writer, noteId, 2, 'Version two', {
        'more': 'kept',
        'v': 2,
      });
      final applied = middle.applyEntries([entryOf(unknown), entryOf(v2)]);
      // Only the entry of the known version counts; the other is held.
      expect([for (final e in applied) e.raw], [unknown]);
      expect(Notes(middle).byId(noteId)?.text, 'Edited later');
      expect(middle.holds(writer.deviceId, entryOf(v2).dseq), isTrue);
      final forwarded = [for (final e in middle.entriesSince(const {})) e.wire];
      expect(forwarded.sublist(forwarded.length - 2), [unknown, v2]);
      // Over the LAN to a Device of the later version's kind, which checks
      // every signature again.
      final third = device(ticking(), 'Ben');
      exchange(third, middle);
      final passed = [for (final e in third.entriesSince(const {})) e.wire];
      expect(passed, containsAllInOrder([unknown, v2]));
      expect(third.versionVector(), middle.versionVector());
      expect(Notes(third).byId(noteId)?.text, 'Edited later');
      // Its own entries and those of version 1 keep nothing extra.
      expect([for (final e in third.allEntries()) e.raw].nonNulls, [
        unknown,
        v2,
      ]);
    });

    test('with a key changed, added or unsigned on the way are '
        'refused', () {
      final line = laterLine(writer, noteId, 1, 'Edited later', {
        'later': 'x',
        'v': 2,
      });
      final json = jsonDecode(line) as Map<String, dynamic>;
      final unsigned = laterLine(writer, noteId, 2, 'Unsigned', {
        'later': 'x',
      }, signExtra: false);
      final report = ImportReport();
      final applied = middle.applyEntries([
        for (final changed in [
          {...json, 'later': 'y'},
          {...json, 'v': 3},
          {...json, 'added': true},
          {
            for (final MapEntry(:key, :value) in json.entries)
              if (key != 'later') key: value,
          },
        ])
          entryOf(canonicalEntry(changed)),
        entryOf(unsigned),
      ], report: report);
      expect(applied, isEmpty);
      expect(report.refusedCount, 5);
      expect(middle.holds(writer.deviceId, entryOf(line).dseq), isFalse);
    });

    test('a version that is no whole number from 1 is no entry', () {
      for (final v in [0, '2', 1.5, null]) {
        final line = laterLine(writer, noteId, 1, 'Odd', {'v': v});
        expect(() => entryOf(line), throwsFormatException, reason: '$v');
      }
    });
  });

  group('key records', () {
    test('a record is valid only for the device its key names', () {
      final key = SigningKey.generate();
      final record = KeyRecord.make(key);
      expect(record.device, deviceIdFromKey(key.publicKey));
      expect(record.valid, isTrue);
      final other = SigningKey.generate();
      final claimed = KeyRecord(
        record.device,
        other.publicKey,
        record.signature,
      );
      expect(claimed.valid, isFalse);
      final moved = KeyRecord(
        deviceIdFromKey(other.publicKey),
        key.publicKey,
        record.signature,
      );
      expect(moved.valid, isFalse);
      expect(KeyRecord.fromJson(record.toJson())?.valid, isTrue);
      expect(KeyRecord.fromJson({'device': 1}), isNull);
    });

    test('keys met in passing are on trust, met in person verified', () {
      final clock = ticking();
      final a = device(clock, 'Ada');
      final b = device(clock, 'Bob');
      final c = device(clock, 'Cy');
      b.learnKeys([a.ownKeyRecord, c.ownKeyRecord], verifiedDevice: a.deviceId);
      expect(b.pinnedKey(a.deviceId)?.trust, KeyTrust.verified);
      expect(b.pinnedKey(c.deviceId)?.trust, KeyTrust.tofu);
      b.learnKeys([c.ownKeyRecord], verifiedDevice: c.deviceId);
      expect(b.pinnedKey(c.deviceId)?.trust, KeyTrust.verified);
      expect(b.pinnedKey(b.deviceId), isNull, reason: 'never its own');
      expect([
        for (final k in b.keyRecords()) k.device,
      ], [a.deviceId, b.deviceId, c.deviceId]..sort());
    });
  });

  group('delta exchange', () {
    test('only what the other lacks travels, and both end the same', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      Notes(phone).capture('Buy milk');
      Notes(desk).capture('Call the dentist');
      final request = syncRequest(phone, desk.versionVector(), name: 'Phone');
      expect(request['entries'], hasLength(phone.allEntries().length));
      exchange(phone, desk);
      expect(texts(phone), ['Buy milk', 'Call the dentist']);
      expect(texts(desk), texts(phone));
      expect(phone.versionVector(), desk.versionVector());
      final again = syncRequest(phone, desk.versionVector(), name: 'Phone');
      expect(again['entries'], isEmpty);
      final held = phone.allEntries().length;
      Notes(phone).capture('One more');
      final next = syncRequest(phone, desk.versionVector(), name: 'Phone');
      expect(
        next['entries'],
        hasLength(phone.allEntries().length - held),
        reason: 'only the new Note',
      );
    });

    test('a second exchange brings nothing and records no Moment', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      Notes(phone).capture('Buy milk');
      exchange(phone, desk);
      expect(desk.moments(), hasLength(1));
      expect(phone.moments(), isEmpty, reason: 'nothing came to the phone');
      exchange(phone, desk);
      expect(desk.moments(), hasLength(1));
    });

    test('a Device in between carries entries on, checked by key', () {
      final clock = ticking();
      final a = device(clock, 'Ada');
      final hub = device(clock, 'Ada');
      final c = device(clock, 'Ada');
      Notes(a).capture('From A');
      exchange(a, hub);
      exchange(c, hub);
      expect(texts(c), ['From A']);
      expect(c.pinnedKey(a.deviceId)?.trust, KeyTrust.tofu);
      expect(c.pinnedKey(hub.deviceId)?.trust, KeyTrust.verified);
    });

    test('edits on both sides converge, latest wins', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final note = Notes(phone).capture('nees mor sauce');
      exchange(phone, desk);
      Notes(phone).edit(note.id, 'Needs more sauce');
      Notes(desk).edit(note.id, 'Needs more hot sauce');
      exchange(phone, desk);
      expect(Notes(phone).byId(note.id)?.text, 'Needs more hot sauce');
      expect(Notes(desk).byId(note.id)?.text, 'Needs more hot sauce');
    });
  });

  group('signatures', () {
    test('a forged or unsigned entry is refused and reported', () {
      final clock = ticking();
      final a = device(clock, 'Ada');
      final b = device(clock, 'Bob');
      final real = Notes(a).capture('Real');
      final e = a.history(real.id).first;
      final forged = Entry(
        seq: -1,
        device: e.device,
        dseq: 99,
        entity: real.id,
        field: 'text',
        value: 'Forged',
        date: e.date,
        author: 'Ada',
        recorded: e.recorded,
        sig: e.sig,
      );
      final unsigned = Entry(
        seq: -1,
        device: 'stranger',
        dseq: 1,
        entity: 'note:x',
        field: r'$type',
        value: 'note',
        date: e.date,
        author: 'Eve',
        recorded: e.recorded,
      );
      final report = ImportReport();
      final applied = b.applyEntries(
        [...a.entriesSince(const {}), forged, unsigned],
        keys: [a.ownKeyRecord],
        report: report,
      );
      expect(applied, hasLength(a.allEntries().length));
      expect(report.refused, {('Ada', a.deviceId): 1, ('Eve', 'stranger'): 1});
      expect(report.refusedCount, 2);
      expect(report.newKeys.single.record.device, a.deviceId);
      expect(Notes(b).byId(real.id)?.text, 'Real');
      expect(b.versionVector()[a.deviceId], a.versionVector()[a.deviceId]);
    });

    test('entries without the key of their Device are refused', () {
      final clock = ticking();
      final a = device(clock, 'Ada');
      final b = device(clock, 'Bob');
      Notes(a).capture('Real');
      final report = ImportReport();
      expect(b.applyEntries(a.entriesSince(const {}), report: report), isEmpty);
      expect(report.refusedCount, a.allEntries().length);
    });

    test("this Device's own id beyond its counter is forged", () {
      final clock = ticking();
      final a = device(clock, 'Ada');
      Notes(a).capture('Mine');
      final mine = a.allEntries().last;
      final ahead = Entry(
        seq: -1,
        device: mine.device,
        dseq: mine.dseq + 1,
        entity: mine.entity,
        field: mine.field,
        value: mine.value,
        date: mine.date,
        author: mine.author,
        recorded: mine.recorded,
        sig: mine.sig,
      );
      final report = ImportReport();
      expect(a.applyEntries([ahead], report: report), isEmpty);
      expect(report.refusedCount, 1);
    });
  });

  group('blobs', () {
    test('photos and recordings follow the entries, checked by hash', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final photo = Uint8List.fromList([1, 2, 3]);
      final voice = Uint8List.fromList([4, 5, 6]);
      final note = Notes(phone)
          .capture('With a photo', photos: [photo], recordings: [voice]);
      final p = Notes(phone).byId(note.id)!.photos.single;
      final r = Notes(phone).byId(note.id)!.recordings.single;
      desk.applyEntries(
        phone.entriesSince(const {}),
        keys: [phone.ownKeyRecord],
      );
      expect(desk.missingBlobs(), ['audio/$r', 'photo/$p']..sort());
      expect(desk.knowsBlob('photo/$p'), isTrue);
      expect(desk.knowsBlob('photo/${'0' * 64}'), isFalse);
      expect(desk.knowsBlob('audio/$p'), isFalse);
      expect(desk.putBlob('photo', p, voice), isFalse, reason: 'wrong bytes');
      expect(desk.putBlob('photo', p, photo), isTrue);
      expect(desk.putBlob('audio', r, voice), isTrue);
      expect(desk.missingBlobs(), isEmpty);
      expect(desk.blob('photo', p), photo);
      expect(Notes(desk).recordingBytes(r), voice);
    });

    test('an exchange carries the blobs both ways', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      Notes(phone).capture(
        'P',
        photos: [
          Uint8List.fromList([7]),
        ],
      );
      Notes(desk).capture(
        'D',
        photos: [
          Uint8List.fromList([8]),
        ],
      );
      exchange(phone, desk);
      expect(phone.missingBlobs(), isEmpty);
      expect(desk.missingBlobs(), isEmpty);
    });
  });

  group('Moments', () {
    test('a sync that brought something records the mark before it', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      Notes(desk).capture('Old');
      final before = desk.currentSeq();
      Notes(phone).capture('New');
      exchange(phone, desk);
      final moment = desk.moments().single;
      expect(moment.seq, before);
      expect(moment.cause, MomentCause.sync);
      expect(moment.label, 'Phone');
      expect(
        {for (final e in desk.entriesAfter(moment.seq)) e.device},
        {phone.deviceId},
      );
      expect(phone.moments().single.label, 'Desk');
    });
  });

  test('settings stay on their Device', () {
    final clock = ticking();
    final phone = device(clock, 'Ada');
    final desk = device(clock, 'Ada');
    phone.themeChoice = ThemeChoice.dark;
    phone.starterSetOffered = true;
    phone.quietHoursOn = false;
    Notes(phone).capture('Hello');
    final request = jsonEncode(
      syncRequest(phone, desk.versionVector(), name: 'Phone'),
    );
    for (final secret in [
      phone.localSetting('signseed')!,
      'starterSetOffered',
    ]) {
      expect(request, isNot(contains(secret)));
    }
    exchange(phone, desk);
    expect(desk.themeChoice, ThemeChoice.system);
    expect(desk.starterSetOffered, isFalse);
    expect(desk.quietHoursOn, isTrue);
    expect(
      desk.localSetting('signseed'),
      isNot(phone.localSetting('signseed')),
    );
  });

  group('the Starter Set on two Devices', () {
    test('planted on both, one of each after a sync, renames kept', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      final all = {
        for (final a in starterSet) ...{
          a.slug,
          for (final t in a.topics) t.slug,
        },
      };
      Places(phone).plantStarterSet(all);
      final home = Places.starterId(PlaceKeys.typeArea, 'household');
      Places(phone).rename(home, 'Home');
      Places(phone).deleteArea(
        Places.starterId(PlaceKeys.typeArea, 'car'),
        NotesGo.unsorted,
      );
      Places(desk).plantStarterSet(all);
      exchange(phone, desk);
      for (final store in [phone, desk]) {
        final areas = Places(store).areas();
        expect([for (final a in areas) a.name], ['Health', 'Home', 'Work']);
        expect(Places(store).topics(home), hasLength(3));
      }
    });

    test('planting again skips what is there and is not due once Areas '
        'arrived', () {
      final clock = ticking();
      final phone = device(clock, 'Ada');
      final desk = device(clock, 'Ada');
      Places(phone).plantStarterSet({'work', 'meetings'});
      expect(Places(desk).starterSetDue, isTrue);
      exchange(phone, desk);
      expect(Places(desk).starterSetDue, isFalse);
      final held = desk.allEntries().length;
      Places(desk).plantStarterSet({'work', 'meetings', 'to-sort-out'});
      expect(desk.allEntries().length, held + 4, reason: 'only the new Topic');
      expect(Places(desk).areas(), hasLength(1));
    });
  });

  group('paired Devices', () {
    test('are remembered, updated and forgotten on this Device only', () {
      final store = Store.inMemory();
      final at = DateTime.utc(2026, 3, 1);
      store.rememberDevice(
        PairedDevice(
          id: 'desk',
          name: 'Desk',
          fingerprint: 'ab' * 32,
          host: '192.168.1.5',
          port: 4000,
          lastSync: at,
        ),
      );
      store.rememberDevice(
        PairedDevice(
          id: 'desk',
          name: 'Desk',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: at.add(const Duration(hours: 1)),
        ),
      );
      final desk = store.pairedDevice('desk')!;
      expect(desk.fingerprint, 'ab' * 32, reason: 'kept from hosting');
      expect(desk.port, 4000);
      expect(desk.lastSync, at.add(const Duration(hours: 1)));
      expect(store.pairedDevices(), hasLength(1));
      store.forgetDevice('desk');
      expect(store.pairedDevices(), isEmpty);
      expect(store.allEntries(), isEmpty);
    });
  });
}

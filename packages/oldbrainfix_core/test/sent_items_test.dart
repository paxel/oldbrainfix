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

      final id = acceptSentItem(ben, opened.item)!;
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
      final reopened = openSentItem(ben, file);
      expect(reopened.accepted, id);
      expect(reopened.state, SentItemState.upToDate);
      expect(acceptSentItem(ben, reopened.item), id);
      expect(ben.allEntries(), hasLength(count));
      // Sent again unchanged: only the version moves on.
      final resent = openSentItem(ben, sendNote(ann, note.id));
      expect(resent.state, SentItemState.update);
      expect(resent.changes!.isEmpty, isTrue);
      expect(acceptSentItem(ben, resent.item), id);
      expect(ben.allEntries(), hasLength(count + 1));
    });

    test('the same id from another key is refused', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final eve = device(clock, 'Eve');
      final ben = device(clock, 'Ben');
      final item = openSentItem(
        ben,
        sendNote(ann, Notes(ann).capture('Real').id),
      ).item;
      acceptSentItem(ben, item);
      final count = ben.allEntries().length;
      for (final cancel in [false, true]) {
        final copy = SentItem(
          id: item.id,
          version: 2,
          cancel: cancel,
          from: eve.ownKeyRecord,
          name: 'Ann',
          text: 'Fake',
        );
        expect(
          () => openSentItem(ben, encodeSentItem(copy, eve.signingKey)),
          throwsA(isA<SentItemNotFromSender>()),
        );
        expect(
          () => acceptSentItem(ben, copy),
          throwsA(isA<SentItemNotFromSender>()),
        );
      }
      expect(ben.allEntries(), hasLength(count));
    });
  });

  group('updates and cancellations', () {
    /// Ann's Note with a Reminder on Tuesday at 10:00 and a daily
    /// Routine, sent to Ben, who accepted it.
    ({Store ann, Store ben, String sent, String got}) accepted() {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final note = Notes(ann).capture('Dentist');
      Routines(ann).spawn(
        note.id,
        Schedule(start: Day(2026, 3, 3), times: [ClockTime(10, 0)]),
      );
      Routines(ann).spawn(
        note.id,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      final got = acceptSentItem(
        ben,
        openSentItem(ben, sendNote(ann, note.id)).item,
      )!;
      return (ann: ann, ben: ben, sent: note.id, got: got);
    }

    test('an update shows what changes and keeps what the receiver added', () {
      final (:ann, :ben, :sent, :got) = accepted();
      expect(wasSent(ann, sent), isTrue);
      expect(wasSent(ann, Notes(ann).capture('Other').id), isFalse);
      // Ben's own additions.
      Notes(ben).amend(got, 'Ask about the bill');
      final area = Places(ben).addArea('Health');
      Notes(ben).file(got, area.id);
      Notes(ben).setImportance(got, Importance.important);
      final daily = Routines(ben).ofNote(got).firstWhere((r) => !r.isReminder);
      final tuesday = Routines(ben).ofNote(got).firstWhere((r) => r.isReminder);
      Routines(ben)
          .markDone(tuesday.id, Occurrence(Day(2026, 3, 3), ClockTime(10, 0)));
      Routines(ben)
          .markDone(daily.id, Occurrence(Day(2026, 3, 1), ClockTime(8, 0)));
      final own = Routines(
        ben,
      ).spawn(got, Schedule(start: Day(2026, 3, 2), times: [ClockTime(18, 0)]));
      // Ann's changes: text, a Section, the Reminder moved to Wednesday
      // 14:00, the daily Routine taken away, a weekly one added.
      Notes(ann).edit(sent, 'Dentist, room 3');
      Notes(ann).amend(sent, 'Bring the card');
      final annRoutines = Routines(ann).ofNote(sent);
      final reminder = annRoutines.firstWhere((r) => r.isReminder);
      Routines(ann).reschedule(
        reminder.id,
        Schedule(start: Day(2026, 3, 4), times: [ClockTime(14, 0)]),
      );
      Routines(ann).trash(annRoutines.firstWhere((r) => !r.isReminder).id);
      Routines(ann).spawn(
        sent,
        Schedule(
          start: Day(2026, 3, 6),
          times: [ClockTime(9, 0)],
          rule: Recurrence.parse('FREQ=WEEKLY'),
        ),
      );

      final file = sendNote(ann, sent);
      final count = ben.allEntries().length;
      final opened = openSentItem(ben, file);
      expect(ben.allEntries(), hasLength(count), reason: 'preview only');
      expect(opened.state, SentItemState.update);
      expect(opened.accepted, got);
      final changes = opened.changes!;
      expect(changes.textBefore, 'Dentist');
      expect(changes.textEdited, isFalse);
      expect(
        [for (final s in changes.sectionsAdded) s.text],
        ['Bring the card'],
      );
      expect(changes.rescheduled, hasLength(1));
      expect(changes.rescheduled.single.before.start, Day(2026, 3, 3));
      expect(changes.rescheduled.single.after.start, Day(2026, 3, 4));
      expect(changes.rescheduled.single.after.times, [ClockTime(14, 0)]);
      expect(
        [for (final r in changes.routinesAdded) r.schedule.rule?.format()],
        ['FREQ=WEEKLY'],
      );
      expect([for (final r in changes.routinesRemoved) r.id], [daily.id]);
      expect(changes.restores, isFalse);
      expect(changes.isEmpty, isFalse);

      expect(acceptSentItem(ben, opened.item), got);
      final note = Notes(ben).byId(got)!;
      expect(note.text, 'Dentist, room 3');
      expect(note.place, area.id, reason: 'filing kept');
      expect(note.importance, Importance.important);
      expect(
        [for (final s in Notes(ben).sections(got)) s.text],
        ['Ask about the bill', 'Bring the card'],
      );
      final now = Routines(ben).ofNote(got);
      expect(now, hasLength(3));
      expect(now.map((r) => r.id), contains(own.id), reason: 'his own stays');
      final moved = now.firstWhere((r) => r.isReminder && r.id != own.id);
      expect(moved.schedule.start, Day(2026, 3, 4));
      expect(moved.schedule.times, [ClockTime(14, 0)]);
      expect(moved.id, tuesday.id);
      expect(moved.done, isNotEmpty, reason: 'Done marks kept');
      final gone = Routines(ben).byId(daily.id)!;
      expect(gone.isTrashed, isTrue, reason: 'restorable');
      expect(gone.done, isNotEmpty, reason: 'Done marks kept');
      final link = jsonDecode(ben.current(got, SentKeys.received)!);
      expect(link['version'], 2);

      // The same file again, or an older one, changes nothing.
      final after = ben.allEntries().length;
      final again = openSentItem(ben, file);
      expect(again.state, SentItemState.upToDate);
      expect(acceptSentItem(ben, again.item), got);
      expect(ben.allEntries(), hasLength(after));
    });

    test('a text the receiver changed stays unless the sender changes it', () {
      final (:ann, :ben, :sent, :got) = accepted();
      Notes(ben).edit(got, 'Dentist (my words)');
      final same = openSentItem(ben, sendNote(ann, sent));
      expect(same.state, SentItemState.update);
      expect(same.changes!.textBefore, isNull);
      expect(same.changes!.isEmpty, isTrue);
      acceptSentItem(ben, same.item);
      expect(Notes(ben).byId(got)!.text, 'Dentist (my words)');

      Notes(ann).edit(sent, 'Dentist moved');
      final changed = openSentItem(ben, sendNote(ann, sent));
      expect(changed.changes!.textBefore, 'Dentist (my words)');
      expect(changed.changes!.textEdited, isTrue);
      acceptSentItem(ben, changed.item);
      expect(Notes(ben).byId(got)!.text, 'Dentist moved');
      // Taken from the sender: no edit of his any more.
      Notes(ann).edit(sent, 'Dentist moved again');
      final next = openSentItem(ben, sendNote(ann, sent));
      expect(next.changes!.textEdited, isFalse);
    });

    test('an older version is ignored', () {
      final (:ann, :ben, :sent, :got) = accepted();
      final second = sendNote(ann, sent);
      final third = sendNote(ann, sent);
      acceptSentItem(ben, openSentItem(ben, third).item);
      final count = ben.allEntries().length;
      final old = openSentItem(ben, second);
      expect(old.state, SentItemState.upToDate);
      expect(old.accepted, got);
      expect(acceptSentItem(ben, old.item), got);
      expect(ben.allEntries(), hasLength(count));
    });

    test('a cancellation trashes the Note with its alarms, restorable', () {
      final (:ann, :ben, :sent, :got) = accepted();
      final own = Routines(
        ben,
      ).spawn(got, Schedule(start: Day(2026, 3, 2), times: [ClockTime(18, 0)]));
      final file = sendNote(ann, sent, cancel: true);
      final opened = openSentItem(ben, file);
      expect(opened.item.cancel, isTrue);
      expect(opened.item.text, 'Dentist');
      expect(opened.state, SentItemState.cancel);
      expect(opened.changes, isNull);
      expect(acceptSentItem(ben, opened.item), got);
      expect(Notes(ben).byId(got)!.isTrashed, isTrue);
      expect(Routines(ben).all(), isEmpty, reason: 'alarms off');

      final again = openSentItem(ben, file);
      expect(again.state, SentItemState.upToDate);

      Routines(ben).restoreNote(got);
      expect(Notes(ben).byId(got)!.isTrashed, isFalse);
      expect(Routines(ben).ofNote(got), hasLength(3));
      expect(Routines(ben).byId(own.id)!.isTrashed, isFalse);
    });

    test('a cancellation reaches a Note merged into another', () {
      final (:ann, :ben, :sent, :got) = accepted();
      final mine = Notes(ben).capture('My dentist notes');
      Notes(ben).merge(got, mine.id);
      final opened = openSentItem(ben, sendNote(ann, sent, cancel: true));
      expect(opened.state, SentItemState.cancel);
      acceptSentItem(ben, opened.item);
      for (final r in ben.entitiesOfType(RoutineKeys.typeRoutine)) {
        expect(Routines(ben).byId(r)!.isTrashed, isTrue);
      }
      expect(Routines(ben).all(), isEmpty, reason: 'alarms off');
    });

    test('an update to a Note in the trash brings it back', () {
      final (:ann, :ben, :sent, :got) = accepted();
      Routines(ben).trashNote(got, withRoutines: true);
      final cancel = openSentItem(ben, sendNote(ann, sent, cancel: true));
      expect(cancel.state, SentItemState.nothingToCancel);
      expect(acceptSentItem(ben, cancel.item), isNull);
      Notes(ann).edit(sent, 'Dentist after all');
      final update = openSentItem(ben, sendNote(ann, sent));
      expect(update.state, SentItemState.update);
      expect(update.changes!.restores, isTrue);
      expect(update.changes!.textBefore, 'Dentist');
      acceptSentItem(ben, update.item);
      expect(Notes(ben).byId(got)!.isTrashed, isFalse);
      expect(Notes(ben).byId(got)!.text, 'Dentist after all');
      expect(Routines(ben).ofNote(got), hasLength(2));
    });

    test('a cancellation of something never accepted cancels nothing', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final ben = device(clock, 'Ben');
      final note = Notes(ann).capture('Party');
      sendNote(ann, note.id);
      final count = ben.allEntries().length;
      final opened = openSentItem(ben, sendNote(ann, note.id, cancel: true));
      expect(opened.state, SentItemState.nothingToCancel);
      expect(opened.accepted, isNull);
      expect(acceptSentItem(ben, opened.item), isNull);
      expect(ben.allEntries(), hasLength(count));
    });
  });
}

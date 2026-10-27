import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_alarms.dart';
import 'fake_profile.dart' show halvesPng;

void main() {
  final now = DateTime.utc(2026, 10, 24, 9);
  late Store phone;
  late Store ann;
  late Directory dir;
  late FakeBundles files;
  late Announcements announcements;
  late FakeAlarms alarms;

  setUp(() {
    phone = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..setLocalSetting('author', 'Ben')
      ..applyProfileDefaults(name: 'Ben')
      ..renameOwnProfile('Ben');
    ann = Store.inMemory(clock: () => now)
      ..setLocalSetting('author', 'Ann')
      ..applyProfileDefaults(name: 'Ann')
      ..setOwnProfilePicture(squareProfilePicture(halvesPng));
    dir = Directory.systemTemp.createTempSync('sent_widget_');
    files = FakeBundles(dir);
    announcements = Announcements();
    alarms = FakeAlarms();
  });
  tearDown(() {
    phone.close();
    ann.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  /// [other] is another person's Device paired by invite with [store],
  /// its key met in person.
  void meet(Store store, Store other, String name) {
    store.rememberDevice(
      PairedDevice(
        id: other.deviceId,
        name: name,
        fingerprint: '',
        host: null,
        port: null,
        lastSync: now,
        shared: true,
      ),
    );
    store.learnKeys([other.ownKeyRecord], verifiedDevice: other.deviceId);
  }

  Future<void> app(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: phone,
        alarms: alarms,
        announcements: announcements,
        bundles: files,
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Opens the editor of [note] from the browser.
  Future<void> edit(WidgetTester tester, Note note) async {
    await app(tester);
    for (final step in [
      find.byTooltip('Search and browse'),
      find.byTooltip('All Notes'),
      find.text(note.text),
    ]) {
      await tester.tap(step);
      await tester.pumpAndSettle();
    }
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  /// [bytes] as a file other apps hand over.
  String fileOf(List<int> bytes) {
    final path = '${dir.path}/in-${DateTime.now().microsecondsSinceEpoch}';
    File(path).writeAsBytesSync(bytes);
    return path;
  }

  /// A Sent Item from Ann, as a file other apps hand over.
  String fileFrom(Note note, {Recipient? to, bool cancel = false}) =>
      fileOf(sendNote(ann, note.id, to: to, cancel: cancel));

  /// Ann's Note with a Reminder on Tuesday 3 November at 10:00, which
  /// the phone accepted.
  (Note, String) acceptedFromAnn() {
    final note = Notes(ann).capture('Dentist');
    Routines(ann).spawn(
      note.id,
      Schedule(start: Day(2026, 11, 3), times: [ClockTime(10, 0)]),
    );
    final got = acceptSentItem(
      phone,
      openSentItem(phone, sendNote(ann, note.id)).item,
    )!;
    return (note, got);
  }

  group('sending', () {
    testWidgets('to nobody known: signed only, with a hint', (tester) async {
      final note = Notes(phone).capture('Dentist on Friday');
      await edit(tester, note);
      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      expect(find.text('Anyone, not encrypted'), findsOneWidget);
      expect(find.textContaining('Not encrypted: whoever'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('send-note')));
      await tester.pumpAndSettle();

      final sent = files.shared.single;
      expect(sent, endsWith('oldbrainfix-note-2026-10-24.oldbrain'));
      expect(files.types.single, sentItemMimeType);
      final opened = openSentItem(ann, File(sent).readAsBytesSync());
      expect(opened.encrypted, isFalse);
      expect(opened.item.text, 'Dentist on Friday');
      expect(opened.sender.name, 'Ben');
    });

    testWidgets('to a person met: encrypted to them', (tester) async {
      meet(phone, ann, 'Ann');
      final note = Notes(phone).capture('Surprise party');
      await edit(tester, note);
      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      expect(find.text('Ann, encrypted'), findsOneWidget);
      expect(find.textContaining('Not encrypted: whoever'), findsNothing);
      await tester.tap(find.byKey(const ValueKey('send-note')));
      await tester.pumpAndSettle();
      final bytes = File(files.shared.single).readAsBytesSync();
      expect(String.fromCharCodes(bytes), isNot(contains('Surprise')));
      expect(openSentItem(ann, bytes).encrypted, isTrue);
    });

    testWidgets('to a person without a name: "No name", encrypted', (
      tester,
    ) async {
      meet(phone, ann, '');
      final note = Notes(phone).capture('Surprise party');
      await edit(tester, note);
      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      expect(find.text('No name, encrypted'), findsOneWidget);
    });

    testWidgets('sent before: an update, or a cancellation instead', (
      tester,
    ) async {
      final note = Notes(phone).capture('Dentist on Friday');
      sendNote(phone, note.id);
      await edit(tester, note);
      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      expect(find.text('Send an update'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('send-note')));
      await tester.pumpAndSettle();
      final update = openSentItem(
        ann,
        File(files.shared.last).readAsBytesSync(),
      );
      expect(update.item.version, 2);
      expect(update.item.cancel, isFalse);

      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('send-cancel')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('send-note')));
      await tester.pumpAndSettle();
      final cancel = openSentItem(
        ann,
        File(files.shared.last).readAsBytesSync(),
      );
      expect(cancel.item.version, 3);
      expect(cancel.item.cancel, isTrue);
    });

    testWidgets('a Note never sent offers no cancellation', (tester) async {
      final note = Notes(phone).capture('Milk');
      await edit(tester, note);
      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('send-cancel')), findsNothing);
    });

    testWidgets('no app to take it is said', (tester) async {
      files.shares = false;
      final note = Notes(phone).capture('Milk');
      await edit(tester, note);
      await tester.tap(find.byTooltip('Send this Note'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('send-note')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'No app here can take the file.');
    });
  });

  group('receiving', () {
    testWidgets('the preview shows who sent what; accepting lands it in '
        'Unsorted with its alarms', (tester) async {
      final note = Notes(ann).capture('Dentist on Friday');
      Routines(ann).spawn(
        note.id,
        Schedule(start: Day(2026, 10, 30), times: [ClockTime(9, 30)]),
      );
      files.waiting.add(fileFrom(note));
      await app(tester);

      expect(find.byType(SentItemScreen), findsOneWidget);
      expect(find.text('From Ann'), findsOneWidget);
      // Her picture, not the placeholder.
      final picture = find.byKey(const ValueKey('sent-from-picture'));
      expect(
        find.descendant(of: picture, matching: find.byType(Image)),
        findsOneWidget,
      );
      expect(
        find.text(
          'Not verified: this Profile is new here. '
          'Code ${ann.ownProfileCode}',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('came unencrypted'), findsOneWidget);
      expect(find.text('Dentist on Friday'), findsOneWidget);
      expect(find.textContaining('09:30'), findsOneWidget);
      expect(Notes(phone).all(), isEmpty, reason: 'nothing before Accept');

      await tester.tap(find.byKey(const ValueKey('accept-sent')));
      await tester.pumpAndSettle();
      expect(find.byType(SentItemScreen), findsNothing);
      expect(announcements.visible, 'Added to Unsorted, with its alarms on.');
      final got = Notes(phone).unsorted().single;
      expect(got.text, 'Dentist on Friday');
      expect(Routines(phone).ofNote(got.id).single.schedule.times, [
        ClockTime(9, 30),
      ]);
      // Her Profile is kept, and her Note shows her picture.
      final annId = ann.ownProfile!.id;
      expect(phone.keptPeople.map((p) => p.id), [annId]);
      expect(phone.personVerified(annId), isFalse);
      for (final step in ['Search and browse', 'All Notes']) {
        await tester.tap(find.byTooltip(step));
        await tester.pumpAndSettle();
      }
      final author = find.byKey(const ValueKey('author'));
      expect(author, findsOneWidget);
      expect(
        find.descendant(of: author, matching: find.byType(Image)),
        findsOneWidget,
      );
      expect(find.bySemanticsLabel(RegExp('Ann')), findsWidgets);
    });

    testWidgets('declining keeps nothing, not even codes compared', (
      tester,
    ) async {
      meet(ann, phone, 'Ben');
      final to = sentItemRecipients(ann).single;
      await app(tester);
      files.open(fileFrom(Notes(ann).capture('Secret'), to: to));
      await tester.pumpAndSettle();
      expect(find.text('It came encrypted to this Device.'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('compare-codes')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('codes-match')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('decline-sent')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'Declined: nothing was kept.');
      expect(Notes(phone).all(), isEmpty);
      expect(phone.localSettingsWithPrefix('person:'), isEmpty);
      expect(dir.listSync().whereType<File>(), isEmpty, reason: 'copy gone');

      // Her Profile kept before stays as it was.
      acceptedFromAnn();
      final kept = phone.localSettingsWithPrefix('person:').toList();
      announcements.dismiss();
      files.open(fileFrom(Notes(ann).capture('Party')));
      await tester.pumpAndSettle();
      expect(find.textContaining('Not verified: known here'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('compare-codes')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('codes-match')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('decline-sent')));
      await tester.pumpAndSettle();
      expect(phone.localSettingsWithPrefix('person:').toList(), kept);
      expect(phone.personVerified(ann.ownProfile!.id), isFalse);
    });

    testWidgets('codes compared in person verify the Profile on accept', (
      tester,
    ) async {
      final code = ann.ownProfileCode!;
      files.waiting.add(fileFrom(Notes(ann).capture('Dentist')));
      await app(tester);
      await tester.tap(find.byKey(const ValueKey('compare-codes')));
      await tester.pumpAndSettle();
      expect(
        find.text(
          'Ask Ann, in person, to read out the code on their Profile page. '
          'It must be $code.',
        ),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('codes-differ')));
      await tester.pumpAndSettle();
      expect(find.textContaining('The codes differ'), findsOneWidget);
      expect(find.textContaining('Not verified'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('compare-codes')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('codes-match')));
      await tester.pumpAndSettle();
      expect(
        find.text('Verified: code compared in person. Code $code'),
        findsOneWidget,
      );
      expect(find.text('Marked as verified when you accept.'), findsOneWidget);
      expect(find.byKey(const ValueKey('compare-codes')), findsNothing);
      expect(phone.localSettingsWithPrefix('person:'), isEmpty);
      await tester.tap(find.byKey(const ValueKey('accept-sent')));
      await tester.pumpAndSettle();
      expect(phone.personVerified(ann.ownProfile!.id), isTrue);

      // The next one from her says so, with nothing left to compare.
      files.open(fileFrom(Notes(ann).capture('Party')));
      await tester.pumpAndSettle();
      expect(find.textContaining('Verified: code compared'), findsOneWidget);
      expect(find.byKey(const ValueKey('compare-codes')), findsNothing);
    });

    testWidgets('a file for someone else or a damaged one is said', (
      tester,
    ) async {
      final eve = Store.inMemory(clock: () => now);
      addTearDown(eve.close);
      meet(ann, eve, 'Eve');
      final note = Notes(ann).capture('For Eve');
      await app(tester);
      files.open(fileFrom(note, to: sentItemRecipients(ann).single));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'This file is encrypted for someone else.');

      announcements.dismiss();
      final damaged = '${dir.path}/damaged';
      File(damaged).writeAsStringSync('{"format":1,"item":"x","sig":"y"}');
      files.open(damaged);
      await tester.pumpAndSettle();
      expect(announcements.visible, contains('cannot be read'));
      expect(find.byType(SentItemScreen), findsNothing);
    });

    testWidgets('an update shows what changes; applying keeps the own '
        'additions', (tester) async {
      final (note, got) = acceptedFromAnn();
      Notes(phone).amend(got, 'Ask about the bill');
      Notes(phone).setImportance(got, Importance.important);
      Notes(ann).edit(note.id, 'Dentist, room 3');
      Routines(ann).reschedule(
        Routines(ann).ofNote(note.id).single.id,
        Schedule(start: Day(2026, 11, 4), times: [ClockTime(14, 0)]),
      );
      files.waiting.add(fileFrom(note));
      await app(tester);

      expect(
        find.text('An update to a Note you accepted before.'),
        findsOneWidget,
      );
      expect(find.text('Text so far: Dentist'), findsOneWidget);
      expect(find.text('New text: Dentist, room 3'), findsOneWidget);
      final moved = find.textContaining('→');
      expect(moved, findsOneWidget);
      final line = tester.widget<Text>(moved).data!;
      expect(line, allOf(contains('10:00'), contains('14:00')));
      expect(line.indexOf('10:00'), lessThan(line.indexOf('14:00')));
      expect(find.text('Apply update'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('accept-sent')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'Updated. Your own additions stayed.');
      final held = Notes(phone).byId(got)!;
      expect(held.text, 'Dentist, room 3');
      expect(held.importance, Importance.important);
      expect(
        [for (final s in Notes(phone).sections(got)) s.text],
        ['Ask about the bill'],
      );
      expect(
        Routines(phone).ofNote(got).single.schedule.start,
        Day(2026, 11, 4),
      );
      expect(Notes(phone).all(), hasLength(1), reason: 'no second Note');
    });

    testWidgets('a cancellation trashes the Note and its alarms', (
      tester,
    ) async {
      final (note, got) = acceptedFromAnn();
      final routine = Routines(phone).ofNote(got).single.id;
      files.waiting.add(fileFrom(note, cancel: true));
      await app(tester);
      expect(alarms.armed.map((a) => a.alarm.routineId), contains(routine));

      expect(find.text('Cancelled: Dentist'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('accept-sent')));
      await tester.pumpAndSettle();
      expect(announcements.visible, contains('Moved to the trash'));
      expect(Notes(phone).byId(got)!.isTrashed, isTrue);
      expect(Routines(phone).all(), isEmpty);
      expect(
        alarms.armed.map((a) => a.alarm.routineId),
        isNot(contains(routine)),
      );
    });

    testWidgets('an old version or nothing to cancel only closes', (
      tester,
    ) async {
      final (note, _) = acceptedFromAnn();
      final older = sendNote(ann, note.id);
      acceptSentItem(phone, openSentItem(phone, sendNote(ann, note.id)).item);
      final after = phone.allEntries().length;
      files.waiting.add(fileOf(older));
      await app(tester);
      expect(find.textContaining('Already up to date'), findsOneWidget);
      expect(find.byKey(const ValueKey('accept-sent')), findsNothing);
      await tester.tap(find.byKey(const ValueKey('decline-sent')));
      await tester.pumpAndSettle();
      expect(find.byType(SentItemScreen), findsNothing);
      expect(phone.allEntries(), hasLength(after));

      files.open(fileFrom(Notes(ann).capture('Party'), cancel: true));
      await tester.pumpAndSettle();
      expect(find.textContaining('Nothing to cancel'), findsOneWidget);
      expect(find.text('Close'), findsOneWidget);
    });

    testWidgets('a change signed by another key is refused', (tester) async {
      final (note, _) = acceptedFromAnn();
      final eve = Store.inMemory(clock: () => now)
        ..applyProfileDefaults(name: 'Eve');
      addTearDown(eve.close);
      final id = openSentItem(phone, sendNote(ann, note.id)).item.id;
      final forged = encodeSentItem(
        SentItem(
          id: id,
          version: 9,
          cancel: true,
          from: eve.ownKeyRecord,
          profile: SignedProfile.fromJson(eve.ownProfileForOthers)!,
          text: 'Dentist',
        ),
        eve.signingKey,
      );
      await app(tester);
      files.open(fileOf(forged));
      await tester.pumpAndSettle();
      expect(announcements.visible, startsWith('Refused:'));
      expect(find.byType(SentItemScreen), findsNothing);
      expect(Notes(phone).all().single.isTrashed, isFalse);
    });

    testWidgets('behind the App Lock the preview waits for the unlock', (
      tester,
    ) async {
      phone
        ..setPin('2468', iterations: 10)
        ..appLockOn = true;
      files.waiting.add(fileFrom(Notes(ann).capture('Private')));
      await app(tester);
      expect(find.byType(SentItemScreen), findsNothing);
      expect(find.text('Private'), findsNothing);

      await tester.enterText(find.byKey(const ValueKey('lockPin')), '2468');
      await tester.tap(find.byKey(const ValueKey('unlockPin')));
      await tester.pumpAndSettle();
      expect(find.byType(SentItemScreen), findsOneWidget);
      expect(find.text('Private'), findsOneWidget);
    });

    group('blocking', () {
      testWidgets('"Decline and block" keeps nothing and blocks the sender', (
        tester,
      ) async {
        final annId = ann.ownProfile!.id;
        files.waiting.add(fileFrom(Notes(ann).capture('Spam')));
        await app(tester);
        await tester.tap(find.byKey(const ValueKey('block-sent')));
        await tester.pumpAndSettle();
        expect(find.text('Block Ann?'), findsOneWidget);
        await tester.tap(find.text('Cancel'));
        await tester.pumpAndSettle();
        expect(phone.isBlocked(annId), isFalse);
        expect(find.byType(SentItemScreen), findsOneWidget);

        await tester.tap(find.byKey(const ValueKey('block-sent')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('block-confirm')));
        await tester.pumpAndSettle();
        expect(find.byType(SentItemScreen), findsNothing);
        expect(
          announcements.visible,
          'Declined and blocked: nothing was kept.',
        );
        expect(Notes(phone).all(), isEmpty);
        expect(phone.isBlocked(annId), isTrue);
        // Kept for the "Blocked" list, without her picture.
        final blocked = phone.blockedPeople.single;
        expect(blocked.name, 'Ann');
        expect(blocked.picture, isNull);
      });

      testWidgets('a file from a blocked person shows nothing of it', (
        tester,
      ) async {
        acceptedFromAnn();
        phone.blockPerson(ann.ownProfile!.id);
        await app(tester);
        files.open(fileFrom(Notes(ann).capture('Party at mine')));
        await tester.pumpAndSettle();
        expect(
          announcements.visible,
          'Refused: this file comes from a person you blocked.',
        );
        expect(find.byType(SentItemScreen), findsNothing);
        expect(find.textContaining('Party'), findsNothing);
        expect(Notes(phone).all().map((n) => n.text), ['Dentist']);
        expect(dir.listSync().whereType<File>(), isEmpty, reason: 'copy gone');
      });

      testWidgets('a block that comes while the preview is open refuses '
          'Accept', (tester) async {
        files.waiting.add(fileFrom(Notes(ann).capture('Party')));
        await app(tester);
        // As if another of the user's Devices blocked her, by sync.
        phone.blockPerson(ann.ownProfile!.id);
        await tester.tap(find.byKey(const ValueKey('accept-sent')));
        await tester.pumpAndSettle();
        expect(find.byType(SentItemScreen), findsNothing);
        expect(
          announcements.visible,
          'Refused: this file comes from a person you blocked.',
        );
        expect(Notes(phone).all(), isEmpty);
      });

      testWidgets('blocking on the sender\'s Profile closes the preview', (
        tester,
      ) async {
        acceptedFromAnn();
        files.waiting.add(fileFrom(Notes(ann).capture('Party')));
        await app(tester);
        await tester.tap(find.byKey(const ValueKey('sent-from-picture')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('block-person')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('block-confirm')));
        await tester.pumpAndSettle();
        await tester.binding.handlePopRoute();
        await tester.pumpAndSettle();
        expect(find.byType(SentItemScreen), findsNothing);
        expect(find.text('Party'), findsNothing);
        expect(
          announcements.visible,
          'Refused: this file comes from a person you blocked.',
        );
      });
    });
  });
}

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

void main() {
  final now = DateTime.utc(2026, 10, 24, 9);
  late Store phone;
  late Store ann;
  late Directory dir;
  late FakeBundles files;
  late Announcements announcements;

  setUp(() {
    phone = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..setLocalSetting('author', 'Ben');
    ann = Store.inMemory(clock: () => now)..setLocalSetting('author', 'Ann');
    dir = Directory.systemTemp.createTempSync('sent_widget_');
    files = FakeBundles(dir);
    announcements = Announcements();
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
        alarms: FakeAlarms(),
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

  /// A Sent Item from Ann, as a file other apps hand over.
  String fileFrom(Note note, {Recipient? to}) {
    final path = '${dir.path}/in-${DateTime.now().microsecondsSinceEpoch}';
    File(path).writeAsBytesSync(sendNote(ann, note.id, to: to));
    return path;
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
      expect(opened.item.name, 'Ben');
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
      expect(
        find.text('Not verified: never met. Key ${ann.signingKey.code}'),
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
    });

    testWidgets('declining keeps nothing; a verified sender shows so', (
      tester,
    ) async {
      meet(phone, ann, 'Ann');
      meet(ann, phone, 'Ben');
      final note = Notes(ann).capture('Secret');
      final to = sentItemRecipients(ann).single;
      await app(tester);
      files.open(fileFrom(note, to: to));
      await tester.pumpAndSettle();
      expect(find.textContaining('Verified: met in person'), findsOneWidget);
      expect(find.text('It came encrypted to this Device.'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('decline-sent')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'Declined: nothing was kept.');
      expect(Notes(phone).all(), isEmpty);
      expect(dir.listSync().whereType<File>(), isEmpty, reason: 'copy gone');
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
  });
}

import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/photo_out.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/photo_views.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announced.dart';
import 'bundle_test.dart' show FakeBundles;
import 'fake_photos.dart';

/// Bundles with no place to write a file.
class _Unwritable extends FakeBundles {
  _Unwritable(super.dir);

  @override
  Future<String?> exportPath(String name) async => null;
}

void main() {
  late Store store;
  late Notes notes;
  late Directory dir;
  late FakePhotos photos;
  late FakeBundles files;
  late Announcements announcements;

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 10, 31, 9))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('photo_out_');
    photos = FakePhotos();
    files = FakeBundles(dir);
    announcements = Announcements();
  });
  tearDown(() {
    store.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  /// Opens the photo [hash] full screen from the Agenda.
  Future<void> view(WidgetTester tester, String hash) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        photos: photos,
        bundles: files,
        announcements: announcements,
      ),
    );
    await tester.pumpAndSettle();
    unawaited(viewPhoto(tester.element(find.byType(AgendaScreen)), hash));
    await tester.pumpAndSettle();
    expect(find.byType(PhotoViewerScreen), findsOneWidget);
  }

  /// Picks [out] in the viewer's ⋮ and waits until [done] or an
  /// announcement: the file is written outside the test's fake time.
  Future<void> pick(
    WidgetTester tester,
    PhotoOut out,
    bool Function() done,
  ) async {
    await tester.tap(find.byKey(const ValueKey('photo-menu')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(ValueKey('photo-${out.name}')));
    for (var i = 0; i < 50 && !done() && announcements.visible == null; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 20)),
      );
      await tester.pump();
    }
    await tester.pumpAndSettle();
  }

  String photoOf(Note note) => note.photos.single;

  testWidgets('the ⋮ offers Save, Copy and Share, each with its icon', (
    tester,
  ) async {
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await tester.tap(find.byKey(const ValueKey('photo-menu')));
    await tester.pumpAndSettle();
    for (final (key, icon, label) in [
      ('photo-save', Icons.download_outlined, 'Save'),
      ('photo-copy', Icons.copy_outlined, 'Copy'),
      ('photo-share', Icons.share_outlined, 'Share'),
    ]) {
      final item = find.byKey(ValueKey(key));
      expect(
        find.descendant(of: item, matching: find.text(label)),
        findsOneWidget,
      );
      expect(
        find.descendant(of: item, matching: find.byIcon(icon)),
        findsOneWidget,
      );
    }
  });

  testWidgets('a photo not on this Device offers no ⋮', (tester) async {
    await view(tester, 'not-here-0123456789');
    expect(find.byKey(const ValueKey('photo-menu')), findsNothing);
  });

  testWidgets('Save puts the photo into Pictures under its name and says '
      'nothing', (tester) async {
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.save, () => photos.saved.isNotEmpty);

    final name = 'photo-${hash.substring(0, 12)}.png';
    expect(photos.saved.keys, [name]);
    expect(photos.saved[name], onePixelPng);
    expect(announcements.visible, isNull);
    expect(find.byType(PhotoViewerScreen), findsOneWidget);
  });

  testWidgets('Save that fails is announced, why behind the tap', (
    tester,
  ) async {
    photos.saves = PhotoSaved.failed;
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.save, () => false);

    await expectFailureAnnounced(
      tester,
      announcements,
      'The photo was not saved.',
      title: 'Photo not saved',
      cause: "The photo could not be written into the phone's Pictures.",
      fix: 'Nothing was lost; free some space on the phone and save it again.',
    );
  });

  testWidgets('Save before Android 10 says it needs a newer one and points '
      'to Share', (tester) async {
    photos.saves = PhotoSaved.tooOld;
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.save, () => false);

    await expectFailureAnnounced(
      tester,
      announcements,
      'The photo was not saved.',
      cause: "Saving into the phone's Pictures needs Android 10 or later.",
      fix: 'Use Share and pick an app that keeps files.',
    );
  });

  testWidgets('Copy puts the photo file on the clipboard and says nothing', (
    tester,
  ) async {
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.copy, () => photos.copied.isNotEmpty);

    final copied = photos.copied.single;
    expect(copied.type, 'image/png');
    expect(copied.path, '${dir.path}/photo-${hash.substring(0, 12)}.png');
    expect(File(copied.path).readAsBytesSync(), onePixelPng);
    expect(announcements.visible, isNull);
  });

  testWidgets('Copy the clipboard refuses is announced', (tester) async {
    photos.copies = false;
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.copy, () => false);

    await expectFailureAnnounced(
      tester,
      announcements,
      'The photo was not copied.',
      title: 'Photo not copied',
      cause: "The phone's clipboard did not take the picture.",
      fix: 'Use Share to hand it to the other app instead.',
    );
  });

  testWidgets('Share offers the photo file as a picture and says nothing', (
    tester,
  ) async {
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.share, () => files.shared.isNotEmpty);

    expect(files.shared, ['${dir.path}/photo-${hash.substring(0, 12)}.png']);
    expect(files.types, ['image/png']);
    expect(File(files.shared.single).readAsBytesSync(), onePixelPng);
    expect(announcements.visible, isNull);
  });

  testWidgets('a share sheet left without an app says nothing', (tester) async {
    files.dismisses = true;
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.share, () => files.shared.isNotEmpty);

    expect(files.shared, hasLength(1));
    expect(announcements.visible, isNull);
  });

  testWidgets('Share with no app to take it is announced', (tester) async {
    files.shares = false;
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.share, () => false);

    await expectFailureAnnounced(
      tester,
      announcements,
      'The photo was not shared.',
      title: 'Photo not shared',
      cause: 'No app here can take the photo.',
      fix: 'Install a messenger or a gallery app, then share it again.',
    );
  });

  testWidgets('a file that cannot be written is announced, nothing handed '
      'on', (tester) async {
    files = _Unwritable(dir);
    final hash = photoOf(notes.capture('receipt', photos: [onePixelPng]));
    await view(tester, hash);
    await pick(tester, PhotoOut.share, () => false);

    expect(files.shared, isEmpty);
    await expectFailureAnnounced(
      tester,
      announcements,
      'The photo was not shared.',
      cause: "The photo's file could not be written on this phone.",
      fix: 'Nothing was lost; try again.',
    );
  });

  test('a photo goes out as photo- and its hash\'s first 12 characters', () {
    expect(photoOutName('0123456789abcdef', 'jpg'), 'photo-0123456789ab.jpg');
    expect(photoOutName('abc', 'png'), 'photo-abc.png');
  });
}

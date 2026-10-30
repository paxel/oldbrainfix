import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_photos.dart';

void main() {
  var now = DateTime.utc(2026, 10, 24, 9);
  late Store store;
  late Notes notes;
  late Directory dir;
  late FakeBundles files;
  late Announcements announcements;
  final voice = Uint8List.fromList(const [0x52, 0x49, 0x46, 0x46, 7]);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('share_note_');
    files = FakeBundles(dir);
    announcements = Announcements();
  });
  tearDown(() {
    store.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  /// Taps [finder] after scrolling it on screen: the card's ⋮ menu
  /// scrolls.
  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the app on All Notes, where the one Note's card shows.
  Future<void> allNotes(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        bundles: files,
        announcements: announcements,
      ),
    );
    await tester.pumpAndSettle();
    for (final step in [
      find.byTooltip('Search and browse'),
      find.byTooltip('All Notes'),
    ]) {
      await tester.tap(step);
      await tester.pumpAndSettle();
    }
  }

  /// Shares the one Note from its card's ⋮.
  Future<void> shareFromCard(WidgetTester tester) async {
    await allNotes(tester);
    await tap(tester, find.byKey(const ValueKey('note-menu')).first);
    expect(find.text('Send to a person'), findsOneWidget);
    await tap(tester, find.byKey(const ValueKey('note-share')));
  }

  testWidgets('Share to an app hands the text, its Sections under their '
      'dates and the photos and recordings', (tester) async {
    final note = notes.capture(
      'Dentist',
      photos: [onePixelPng],
      recordings: [voice],
    );
    notes.setTranscript(note.id, note.recordings.single, 'not shared');
    now = now.add(const Duration(days: 1));
    final section = notes.amend(note.id, 'Moved to Friday');
    await shareFromCard(tester);

    final shared = files.texts.single;
    final date = DateFormat.yMd('en')
        .add_Hm()
        .format(section.context.captured.toLocal());
    expect(shared.text, 'Dentist\n\n$date\nMoved to Friday');
    expect(shared.html, isNull);
    expect([for (final f in shared.files) f.type], ['image/png', 'audio/mp4']);
    expect(shared.files.first.path, startsWith(dir.path));
    expect(files.fileBytes[shared.files.first.path], onePixelPng);
    expect(files.fileBytes[shared.files.last.path], voice);
    expect(announcements.visible, isNull);
  });

  testWidgets('a formatted text goes with HTML beside its Markdown', (
    tester,
  ) async {
    notes.capture(
      '**Milk** and [x](javascript:y)',
      format: TextFormats.markdown,
    );
    await shareFromCard(tester);

    final shared = files.texts.single;
    expect(shared.text, '**Milk** and [x](javascript:y)');
    expect(shared.html, '<p><strong>Milk</strong> and x</p>\n');
    expect(shared.files, isEmpty);
  });

  testWidgets('photos and recordings go as the files they are', (tester) async {
    final wav = Uint8List.fromList([
      ...'RIFF'.codeUnits,
      0,
      0,
      0,
      0,
      ...'WAVE'.codeUnits,
    ]);
    notes.capture('Desk', photos: [onePixelPng], recordings: [wav]);
    await shareFromCard(tester);

    final [photo, recording] = files.texts.single.files;
    expect(photo.path, endsWith('.png'));
    expect(photo.type, 'image/png');
    expect(recording.path, endsWith('.wav'));
    expect(recording.type, 'audio/wav');
  });

  testWidgets('a Note without words hands its Transcript', (tester) async {
    final note = notes.capture('', recordings: [voice]);
    notes.setTranscript(note.id, note.recordings.single, 'call the plumber');
    await shareFromCard(tester);

    final shared = files.texts.single;
    expect(shared.text, 'call the plumber');
    expect(shared.files.single.type, 'audio/mp4');
  });

  testWidgets('the editor shares beside Send to a person', (tester) async {
    notes.capture('Bike lock code');
    await allNotes(tester);
    await tester.longPress(find.text('Bike lock code'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(find.byTooltip('Send to a person'), findsOneWidget);
    await tester.tap(find.byTooltip('Share to an app'));
    await tester.pumpAndSettle();

    expect(files.texts.single.text, 'Bike lock code');
  });

  testWidgets('no app to take it is announced at the top', (tester) async {
    notes.capture('Bike lock code');
    files.shares = false;
    await shareFromCard(tester);

    expect(files.texts, hasLength(1));
    expect(announcements.visible, 'The Note was not shared.');
    final action = announcements.visibleAction;
    announcements.dismiss();
    action?.call();
    await tester.pumpAndSettle();
    expect(
      find.textContaining('No app here can take the Note.'),
      findsOneWidget,
    );
  });

  testWidgets('a share sheet left without an app says nothing', (tester) async {
    notes.capture('Bike lock code');
    files.dismisses = true;
    await shareFromCard(tester);

    expect(files.texts, hasLength(1));
    expect(announcements.visible, isNull);
  });
}

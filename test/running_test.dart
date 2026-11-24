import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/area_deletion.dart';
import 'package:oldbrainfix/src/folder_import.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/import_run.dart';
import 'package:oldbrainfix/src/running.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announced.dart';
import 'fake_alarms.dart';
import 'fake_import.dart';
import 'fake_ocr.dart';
import 'fake_photos.dart';
import 'open_settings.dart';

/// Runs a long job inline once [gate] opens, as the isolate would, its
/// stop read from its file.
class _GatedRunner implements JobRunner {
  final FolderPlatform folders;
  final gate = Completer<void>();

  _GatedRunner(this.folders);

  @override
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  ) async {
    await gate.future;
    return work(store, folders, progress, () => File(stop.path).existsSync());
  }
}

void main() {
  final march = DateTime.utc(2025, 3, 1, 8);

  FakeImportFolders threeFiles() => FakeImportFolders()
    ..put('Dentist.txt', march, utf8.encode('Tuesday 9:00'))
    ..put('Plan.md', march, utf8.encode('Ship it'))
    ..put('Milk.txt', march, utf8.encode('Oat milk'));

  group('Abort (obf-1078)', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    test('a raised stop reaches the Import in its isolate, before its '
        'first Note', () async {
      final dir = Directory.systemTemp.createTempSync('running_test_');
      final path = '${dir.path}/oldbrainfix.db';
      final app = Store.open(path);
      addTearDown(() {
        app.close();
        dir.deleteSync(recursive: true);
      });
      final stop = JobStop()..raise();
      addTearDown(stop.clear);
      final summary = await IsolateJobRunner(path: path, folders: threeFiles())
          .run(
            app,
            folderImport('content://tree/Notes', 'From Notes'),
            (_, _) {},
            stop,
          );
      expect(summary.stopped, 3);
      expect(summary.notes, 0);
      expect(summary.areaId, isNull);
      expect(Notes(app).all(), isEmpty);
    });

    test('stops an Import before its next Note, keeping those in', () async {
      final jobs = ImportJobs(
        store: store,
        runner: InlineJobRunner(threeFiles()),
      );
      addTearDown(jobs.dispose);
      final job = ImportJob(ImportKind.folder, name: 'Notes');
      job.addListener(() {
        if (job.written == 1) job.abort();
      });
      final summary = await jobs.run(
        job,
        folderImport('content://tree/Notes', 'From Notes'),
      );
      expect(job.aborting, isTrue);
      expect(summary.notes, 1);
      expect(summary.stopped, 3);
      expect(job.stopped, (done: 1, of: 3));
      expect(Notes(store).inside(summary.areaId!), hasLength(1));
      expect(jobs.running.value, isNull);
    });

    test('stops deleting an Area before its next Note; the Area stays with '
        'the others', () async {
      final places = Places(store);
      final notes = Notes(store);
      final home = places.addArea('Home');
      for (final text in ['pills', 'sauce', 'keys']) {
        notes.capture(text, place: home.id);
      }
      final deletions = AreaDeletions(
        store: store,
        runner: InlineJobRunner(FakeImportFolders()),
      );
      addTearDown(deletions.dispose);
      deletions.addListener(() {
        if (deletions.of(home.id)?.moved == 1) deletions.abort(home.id);
      });
      final deleted = await deletions.run(home.id, NotesGo.unsorted);
      expect(deleted, isFalse);
      expect(places.areaById(home.id), isNotNull);
      expect(notes.inside(home.id), hasLength(2));
      expect(deletions.takeStopped(), [(areaId: home.id, moved: 1, of: 3)]);
      expect(deletions.takeStopped(), isEmpty);
      expect(deletions.running, isFalse);
    });
  });

  group('the page Running (obf-1078)', () {
    late Store store;
    late Announcements announcements;

    setUp(() {
      store = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      announcements = Announcements();
    });
    tearDown(() {
      announcements.dispose();
      store.close();
    });

    /// A few frames: the line under the bar moves, so nothing settles.
    Future<void> frames(WidgetTester tester) async {
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
    }

    Future<void> openFromMenu(WidgetTester tester) async {
      await tester.tap(find.byKey(const ValueKey('bar-more')));
      await frames(tester);
      await tester.tap(find.byKey(const ValueKey('running')));
      await frames(tester);
    }

    testWidgets('opens from the Agenda\'s ⋮ and the Areas list\'s, and says '
        'when nothing runs', (tester) async {
      Places(store)
        ..addArea('Home')
        ..addArea('Work');
      await tester.pumpWidget(
        OldBrainFixApp(store: store, alarms: FakeAlarms()),
      );
      await tester.pumpAndSettle();
      await openFromMenu(tester);
      expect(find.byType(RunningScreen), findsOneWidget);
      expect(find.text('Running'), findsWidgets);
      expect(find.text('Nothing runs in the background.'), findsOneWidget);

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('browse')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('areas-more')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('areas-running')));
      await tester.pumpAndSettle();
      expect(find.byType(RunningScreen), findsOneWidget);
    });

    testWidgets('a tap on the line opens it; Abort stops the Import before '
        'its first Note, and the top says so', (tester) async {
      final folders = threeFiles();
      final runner = _GatedRunner(folders);
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: FakeAlarms(),
          folders: folders,
          announcements: announcements,
          jobRunner: runner,
        ),
      );
      await tester.pumpAndSettle();
      await openDeviceSettings(tester);
      final row = find.byKey(const ValueKey('importFolder'));
      await tester.scrollUntilVisible(
        row,
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(row);
      await frames(tester);

      // The line under the Import page's bar.
      await tester.tap(find.byKey(const Key('syncLineTap')));
      await frames(tester);
      expect(find.byType(RunningScreen), findsOneWidget);
      expect(find.text('Import of Notes'), findsOneWidget);
      expect(find.textContaining('Since '), findsOneWidget);
      expect(find.text('The Notes imported so far stay.'), findsOneWidget);
      // Its own line opens no second page.
      await tester.tap(find.byKey(const Key('syncLineTap')));
      await frames(tester);
      expect(find.byType(RunningScreen), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('running-abort')));
      await frames(tester);
      expect(find.text('Stopping…'), findsOneWidget);
      expect(find.byKey(const ValueKey('running-abort')), findsNothing);

      runner.gate.complete();
      await tester.pumpAndSettle();
      expect(find.text('Nothing runs in the background.'), findsOneWidget);
      expect(find.text('Stopped'), findsOneWidget);
      expect(find.text('Stopped: 0 of 3 imported'), findsOneWidget);
      expect(Notes(store).all(), isEmpty);
      expect(announcements.visible, 'Import stopped.');
      await tapVisible(tester, announcements);
      expect(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.text('Stopped: 0 of 3 imported'),
        ),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('importOpenArea')), findsNothing);
    });

    testWidgets('Abort on the photos lets the one being read end and keeps '
        'the others waiting until Go on', (tester) async {
      final notes = Notes(store);
      notes.capture('', photos: [onePixelPng]);
      notes.capture('', photos: [otherPng]);
      final ocr = FakeOcr()
        ..texts[onePixelPng.length] = 'Level 3'
        ..texts[otherPng.length] = 'Row F'
        ..readGate = Completer<void>();
      await tester.pumpWidget(
        OldBrainFixApp(store: store, alarms: FakeAlarms(), ocr: ocr),
      );
      await tester.pumpAndSettle();
      await openFromMenu(tester);
      expect(find.text('Reading the text in photos'), findsOneWidget);
      expect(find.text('0 of 2'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('running-abort')));
      await tester.pumpAndSettle();
      expect(find.text('Stopping…'), findsOneWidget);
      ocr.readGate?.complete();
      await tester.pumpAndSettle();
      expect(ocr.readPhotos, hasLength(1));
      expect(notes.unreadPhotos(), hasLength(1));
      expect(
        find.text('Stopped: the others wait until Go on or the next start.'),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const ValueKey('running-go-on-photoTexts')));
      await tester.pumpAndSettle();
      expect(ocr.readPhotos, hasLength(2));
      expect(notes.unreadPhotos(), isEmpty);
      expect(find.text('Nothing runs in the background.'), findsOneWidget);
      expect(find.text('Stopped'), findsNothing);
    });
  });
}

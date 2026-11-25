import 'dart:async';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/area_deletion.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/import_run.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_import.dart';

/// Runs a long job inline once [gate] opens, telling how far it is
/// through [told] meanwhile, as the isolate's port does; or throws when
/// [fails].
class _GatedRunner implements JobRunner {
  final FolderPlatform folders;
  final gate = Completer<void>();
  JobProgress? told;
  var fails = false;

  _GatedRunner(this.folders);

  @override
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  ) async {
    told = progress;
    await gate.future;
    if (fails) throw const FileSystemException('disk full');
    return work(store, folders, progress, () => File(stop.path).existsSync());
  }
}

void main() {
  group('in an isolate of its own (obf-1093)', () {
    late Directory dir;
    late Store app;
    var heard = 0;

    setUp(() {
      dir = Directory.systemTemp.createTempSync('area_deletion_test_');
      app = Store.open('${dir.path}/oldbrainfix.db');
      heard = 0;
      app.addListener(() => heard++);
    });
    tearDown(() {
      app.close();
      dir.deleteSync(recursive: true);
    });

    test('trashes the Area\'s Notes there, tells how far it is, and the '
        'store here hears of it once', () async {
      final places = Places(app);
      final notes = Notes(app);
      final home = places.addArea('Home');
      final pills = notes.capture('pills', place: home.id);
      Routines(app).spawn(
        pills.id,
        Schedule(
          start: Day(2026, 10, 24),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      notes.capture('sauce', place: home.id);
      heard = 0;
      final deletions = AreaDeletions(
        store: app,
        runner: IsolateJobRunner(
          path: '${dir.path}/oldbrainfix.db',
          folders: FakeImportFolders(),
        ),
      );
      addTearDown(deletions.dispose);
      final seen = <AreaDeleting?>[];
      deletions.addListener(() => seen.add(deletions.of(home.id)));
      await deletions.run(home.id, NotesGo.trash);
      expect(seen, [
        (moved: 0, of: null),
        (moved: 0, of: 2),
        (moved: 1, of: 2),
        (moved: 2, of: 2),
        null,
      ]);
      expect(deletions.running, isFalse);
      expect(heard, 1);
      expect(places.areaById(home.id), isNull);
      expect(notes.trashed(), hasLength(2));
      expect(Routines(app).all(), isEmpty);
    });
  });

  group('beside the app (obf-1093)', () {
    late Store store;
    late Announcements announcements;
    late _GatedRunner runner;
    late Area home;
    late Area work;

    setUp(() {
      store = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      final places = Places(store);
      home = places.addArea('Home');
      work = places.addArea('Work');
      for (final text in ['pills', 'sauce', 'plants']) {
        Notes(store).capture(text, place: home.id);
      }
      Notes(store).capture('report', place: work.id);
      announcements = Announcements();
    });
    tearDown(() {
      announcements.dispose();
      store.close();
    });

    /// The app with deleting an Area gated by [runner], made here: a gate
    /// made outside the test's clock would never open in it.
    Future<void> pumpApp(WidgetTester tester) async {
      runner = _GatedRunner(FakeImportFolders());
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: FakeAlarms(),
          folders: FakeImportFolders(),
          announcements: announcements,
          jobRunner: runner,
        ),
      );
      await tester.pumpAndSettle();
    }

    /// A few frames: the line under the bar moves, so nothing settles.
    Future<void> frames(WidgetTester tester) async {
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
    }

    /// Opens the Areas list and moves Home to the trash with its Notes.
    Future<void> trashHome(WidgetTester tester) async {
      await tester.tap(find.byKey(const ValueKey('browse')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey('area-menu-${home.id}')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();
      expect(find.textContaining('3 Notes inside.'), findsOneWidget);
      await tester.tap(find.text('Move to trash'));
      await frames(tester);
    }

    testWidgets('its row shows it is busy and how many Notes have gone, '
        'and the app stays usable meanwhile', (tester) async {
      await pumpApp(tester);
      await trashHome(tester);
      final row = find.byKey(ValueKey('area-row-${home.id}'));
      expect(
        find.descendant(
          of: row,
          matching: find.byKey(ValueKey('area-deleting-${home.id}')),
        ),
        findsOneWidget,
      );
      expect(find.byKey(ValueKey('area-menu-${home.id}')), findsNothing);
      expect(
        find.descendant(of: row, matching: find.text('Being deleted…')),
        findsOneWidget,
      );
      expect(find.byKey(const Key('syncLineMoving')), findsOneWidget);

      runner.told?.call(0, 3);
      await tester.pump();
      expect(find.text('Being deleted: 0 of 3 Notes moved'), findsOneWidget);
      runner.told?.call(2, 3);
      await tester.pump();
      expect(find.text('Being deleted: 2 of 3 Notes moved'), findsOneWidget);

      // Its row takes no tap; the other Area opens, and its Notes show.
      await tester.tap(row, warnIfMissed: false);
      await frames(tester);
      expect(find.byType(AreaScreen), findsNothing);
      await tester.tap(find.byKey(ValueKey('area-row-${work.id}')));
      await frames(tester);
      expect(find.byType(AreaScreen), findsOneWidget);
      expect(find.textContaining('report'), findsWidgets);
      expect(find.byKey(const Key('syncLineMoving')), findsOneWidget);
      await tester.binding.handlePopRoute();
      await frames(tester);

      runner.gate.complete();
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('syncLineMoving')), findsNothing);
      expect(find.byKey(ValueKey('area-row-${home.id}')), findsNothing);
      expect(find.byKey(ValueKey('area-row-${work.id}')), findsOneWidget);
      expect(Places(store).areaById(home.id), isNull);
      expect(Notes(store).trashed(), hasLength(3));
      expect(announcements.visible, isNull);
    });

    testWidgets('a failure is announced at the top, and the Area stays', (
      tester,
    ) async {
      await pumpApp(tester);
      runner.fails = true;
      await trashHome(tester);
      runner.gate.complete();
      await tester.pumpAndSettle();
      expect(announcements.visible, 'The Area could not be deleted.');
      expect(find.byKey(ValueKey('area-deleting-${home.id}')), findsNothing);
      expect(find.byKey(ValueKey('area-menu-${home.id}')), findsOneWidget);
      expect(Places(store).areaById(home.id), isNotNull);
    });
  });
}

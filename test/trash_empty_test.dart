import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/import_run.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/running.dart';
import 'package:oldbrainfix/src/trash_emptying.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_import.dart';

/// Runs a long job inline, the one after [hold] is set only once [gate]
/// opens.
class _GatedRunner implements JobRunner {
  final FolderPlatform folders;
  final gate = Completer<void>();
  var hold = false;

  /// Whether the job after [hold] is set throws.
  var fails = false;

  _GatedRunner(this.folders);

  @override
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  ) async {
    if (hold) {
      await gate.future;
      if (fails) throw const FileSystemException('disk full');
    }
    return work(store, folders, progress, () => false);
  }
}

Schedule _daily() => Schedule(
  start: Day(2026, 10, 1),
  times: [ClockTime(8, 0)],
  rule: Recurrence.parse('FREQ=DAILY'),
);

void main() {
  group('in an isolate of its own (obf-1095)', () {
    late Directory dir;
    late Store app;

    setUp(() {
      dir = Directory.systemTemp.createTempSync('trash_empty_test_');
      app = Store.open('${dir.path}/oldbrainfix.db');
    });
    tearDown(() {
      app.close();
      dir.deleteSync(recursive: true);
    });

    test('counts what goes, empties the trash there, and the store here '
        'hears of it', () async {
      final notes = Notes(app);
      final routines = Routines(app);
      final pills = notes.capture('pills');
      routines.spawn(pills.id, _daily());
      routines.trashNote(pills.id);
      notes.trash(notes.capture('sauce').id);
      final kept = notes.capture('plants');
      final emptying = TrashEmptying(
        store: app,
        runner: IsolateJobRunner(
          path: '${dir.path}/oldbrainfix.db',
          folders: FakeImportFolders(),
        ),
      );
      addTearDown(emptying.dispose);
      final toEmpty = await emptying.toEmpty();
      expect(toEmpty.notes, hasLength(2));
      expect(toEmpty.routines, hasLength(1));

      var heard = 0;
      app.addListener(() => heard++);
      final seen = <bool>[];
      emptying.addListener(() => seen.add(emptying.running));
      final emptied = await emptying.run();
      expect(seen, [true, false]);
      expect(emptied.notes, unorderedEquals(toEmpty.notes));
      expect(heard, 1);
      expect(notes.trashed(), isEmpty);
      expect(Trash(app).toEmpty().routines, isEmpty);
      expect(notes.byId(kept.id), isNotNull);
      expect((await emptying.toEmpty()).isEmpty, isTrue);
    });
  });

  group('Empty the trash now (obf-1095)', () {
    late Store store;
    late Notes notes;
    late Routines routines;

    setUp(() {
      store = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      notes = Notes(store);
      routines = Routines(store);
    });
    tearDown(() => store.close());

    final action = find.byKey(const ValueKey('trash-empty'));
    final question = find.byKey(const ValueKey('trash-empty-question'));

    Future<void> openTrash(WidgetTester tester, {JobRunner? runner}) async {
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: FakeAlarms(),
          folders: FakeImportFolders(),
          jobRunner: runner,
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('browse')));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Trash'));
      await tester.pumpAndSettle();
    }

    bool enabled(WidgetTester tester) =>
        tester
            .widget<IconButton>(
              find.descendant(of: action, matching: find.byType(IconButton)),
            )
            .onPressed !=
        null;

    /// Two Notes in the trash, one with its Routine, and one kept.
    String trashSome() {
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, _daily());
      routines.trashNote(pills.id);
      notes.trash(notes.capture('sauce').id);
      notes.capture('plants');
      return routine.id;
    }

    testWidgets('sits on the bar, off while nothing is in the trash', (
      tester,
    ) async {
      await openTrash(tester);
      expect(find.text('The trash is empty.'), findsOneWidget);
      expect(action, findsOneWidget);
      expect(find.byTooltip('Empty the trash now'), findsOneWidget);
      expect(enabled(tester), isFalse);
      await tester.tap(action);
      await tester.pumpAndSettle();
      expect(question, findsNothing);
    });

    testWidgets('asks once with the counts; Cancel changes nothing', (
      tester,
    ) async {
      trashSome();
      await openTrash(tester);
      expect(enabled(tester), isTrue);
      await tester.tap(action);
      await tester.pumpAndSettle();
      expect(question, findsOneWidget);
      expect(
        find.text(
          "Delete 2 Notes and 1 Routine for good? They can't come back.",
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byType(DestructiveButton),
          matching: find.text('Delete for good'),
        ),
        findsOneWidget,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(question, findsNothing);
      expect(notes.trashed(), hasLength(2));
      expect(Trash(store).toEmpty().routines, hasLength(1));
      expect(find.textContaining('pills'), findsWidgets);
    });

    testWidgets('names only the Notes where no Routine goes', (tester) async {
      notes.trash(notes.capture('sauce').id);
      await openTrash(tester);
      await tester.tap(action);
      await tester.pumpAndSettle();
      expect(
        find.text("Delete 1 Note for good? It can't come back."),
        findsOneWidget,
      );
    });

    testWidgets('Delete for good empties the trash and turns the action off', (
      tester,
    ) async {
      final routine = trashSome();
      await openTrash(tester);
      await tester.tap(action);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete for good'));
      await tester.pumpAndSettle();
      expect(notes.trashed(), isEmpty);
      expect(Trash(store).toEmpty().isEmpty, isTrue);
      expect(routines.byId(routine), isNull);
      expect(notes.all().map((note) => note.text), contains('plants'));
      expect(find.text('The trash is empty.'), findsOneWidget);
      expect(enabled(tester), isFalse);
    });

    testWidgets('runs beside the app: the line moves, the page Running '
        'lists it without Abort, and the action is off meanwhile', (
      tester,
    ) async {
      trashSome();
      final runner = _GatedRunner(FakeImportFolders());
      await openTrash(tester, runner: runner);
      await tester.tap(action);
      await tester.pumpAndSettle();
      runner.hold = true;
      await tester.tap(find.text('Delete for good'));
      // The line under the bar moves, so nothing settles.
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(find.byKey(const Key('syncLineMoving')), findsOneWidget);
      expect(enabled(tester), isFalse);
      expect(notes.trashed(), hasLength(2));

      final context = tester.element(action);
      openRunning(context);
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(find.text('Emptying the trash'), findsOneWidget);
      expect(find.byKey(const ValueKey('running-abort')), findsNothing);

      runner
        ..hold = false
        ..gate.complete();
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(find.text('Emptying the trash'), findsNothing);
      expect(find.byKey(const ValueKey('running-none')), findsOneWidget);
      expect(notes.trashed(), isEmpty);
    });

    testWidgets('a failure is announced at the top and the trash stays', (
      tester,
    ) async {
      trashSome();
      final runner = _GatedRunner(FakeImportFolders());
      await openTrash(tester, runner: runner);
      await tester.tap(action);
      await tester.pumpAndSettle();
      runner
        ..hold = true
        ..fails = true;
      await tester.tap(find.text('Delete for good'));
      await tester.pump();
      runner.gate.complete();
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(find.text('The trash could not be emptied.'), findsOneWidget);
      expect(notes.trashed(), hasLength(2));
      runner
        ..hold = false
        ..fails = false;
      expect(enabled(tester), isTrue);
    });
  });
}

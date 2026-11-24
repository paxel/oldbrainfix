import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/folder_import.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/import_run.dart';
import 'package:oldbrainfix/src/import_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announced.dart';
import 'fake_alarms.dart';
import 'fake_import.dart';
import 'open_settings.dart';

/// An Import that writes a Note named after the isolate it runs in.
Future<ImportSummary> _namesItsIsolate(
  Store store,
  FolderPlatform folders,
  ImportProgress progress,
  Stopping stop,
) async {
  Notes(store).capture(Isolate.current.debugName ?? '');
  return const ImportSummary(areaId: null, notes: 0, notText: 0, tooLarge: 0);
}

/// Runs an Import inline once [gate] opens, telling the page how far it
/// is through [told] meanwhile, as the isolate's port does.
class _GatedRunner implements JobRunner {
  final FolderPlatform folders;
  final gate = Completer<void>();
  JobProgress? told;

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
    return work(store, folders, progress, () => File(stop.path).existsSync());
  }
}

void main() {
  final march = DateTime.utc(2025, 3, 1, 8);

  group('in an isolate of its own (obf-1061)', () {
    late Directory dir;
    late String path;
    late Store app;
    var heard = 0;

    setUp(() {
      dir = Directory.systemTemp.createTempSync('import_run_test_');
      path = '${dir.path}/oldbrainfix.db';
      app = Store.open(path);
      heard = 0;
      app.addListener(() => heard++);
    });
    tearDown(() {
      app.close();
      dir.deleteSync(recursive: true);
    });

    test('runs off the app\'s isolate', () async {
      final runner = IsolateJobRunner(path: path, folders: FakeImportFolders());
      await runner.run(app, _namesItsIsolate, (_, _) {}, JobStop());
      final where = Notes(app).all().single.text;
      expect(where, isNot(Isolate.current.debugName));
    });

    test('tells how far it is as it goes, and the store here hears of the '
        'new Notes', () async {
      final folders = FakeImportFolders()
        ..put('Dentist.txt', march, utf8.encode('Tuesday 9:00'))
        ..put('Plan.md', march, utf8.encode('Ship it'));
      final told = <(int, int)>[];
      final summary = await IsolateJobRunner(path: path, folders: folders).run(
        app,
        folderImport('content://tree/Notes', 'From Notes'),
        (written, of) => told.add((written, of)),
        JobStop(),
      );
      expect(summary.notes, 2);
      expect(told, [(0, 2), (1, 2), (2, 2)]);
      expect(heard, 1);
      expect(Notes(app).inside(summary.areaId!), hasLength(2));
    });
  });

  group('beside the app (obf-1061)', () {
    late Store store;
    late FakeImportFolders folders;
    late Announcements announcements;
    late _GatedRunner runner;

    setUp(() {
      store = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      folders = FakeImportFolders()
        ..put('Dentist.txt', march, utf8.encode('Tuesday 9:00'))
        ..put('Plan.md', march, utf8.encode('Ship it'));
      announcements = Announcements();
    });
    tearDown(() {
      announcements.dispose();
      store.close();
    });

    /// The app with the Import gated by [runner], made here: a gate made
    /// outside the test's clock would never open in it.
    Future<void> pumpApp(WidgetTester tester) async {
      runner = _GatedRunner(folders);
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
    }

    /// A few frames: the line under the bar moves, so nothing settles.
    Future<void> frames(WidgetTester tester) async {
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
    }

    testWidgets('counts its Notes on a page of its own, leaves the app '
        'usable meanwhile, and opens the new Area at the end', (tester) async {
      await pumpApp(tester);
      await openImports(tester);
      await tester.tap(find.byKey(const ValueKey('importFolder')));
      await frames(tester);
      expect(find.byType(ImportScreen), findsOneWidget);
      expect(find.text('Reading the folder…'), findsOneWidget);
      expect(
        find.text(
          'You can leave this page. The Import goes on beside the app, and '
          'the top of the screen says when it is done.',
        ),
        findsOneWidget,
      );

      runner.told?.call(0, 340);
      await tester.pump();
      expect(find.text('0 of 340 Notes'), findsOneWidget);
      runner.told?.call(120, 340);
      await tester.pump();
      expect(find.text('120 of 340 Notes'), findsOneWidget);

      // Back on the Agenda, the app goes on, the line under its bar.
      for (var i = 0; i < 3; i++) {
        await tester.binding.handlePopRoute();
        await frames(tester);
      }
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byKey(const Key('syncLineMoving')), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('browse')));
      await frames(tester);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byKey(const Key('syncLineMoving')), findsOneWidget);
      Notes(store).capture('Meanwhile');

      runner.gate.complete();
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('syncLineMoving')), findsNothing);
      expect(announcements.visible, 'Import finished.');
      final area = Places(store)
          .areas()
          .singleWhere((a) => a.name == 'From Notes');
      expect(Notes(store).inside(area.id), hasLength(2));

      await tapVisible(tester, announcements);
      expect(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.text('2 Notes went into the new Area From Notes.'),
        ),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('importOpenArea')));
      await tester.pumpAndSettle();
      expect(find.byType(AreaScreen), findsOneWidget);
      expect(find.textContaining('Dentist'), findsWidgets);
    });

    testWidgets('says on its page when it failed', (tester) async {
      folders.fails = true;
      await pumpApp(tester);
      runner.gate.complete();
      await openImports(tester);
      await tester.tap(find.byKey(const ValueKey('importFolder')));
      await tester.pumpAndSettle();
      expect(
        find.text(
          'The folder could not be imported.\n\n'
          'The folder or one of its files could not be read. Nothing was '
          'imported. Copy the folder onto the phone and import it from '
          'there.',
        ),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('importPageOpenArea')), findsNothing);
      expect(announcements.visible, 'The folder could not be imported.');
    });
  });
}

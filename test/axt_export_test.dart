import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/axt_export.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/import_run.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'editor_actions.dart';

/// "Export as AXT file…" (obf-1145, ADR 0056): beside every "Export as
/// Pack…", the Notes go as an AXT file to the share sheet, nothing asked,
/// and the file opens again in the app.
void main() {
  var now = DateTime.utc(2026, 11, 10, 8);
  late Store store;
  late Notes notes;
  late Places places;
  late Directory dir;
  late FakeBundles files;
  late Announcements announcements;
  late Area health;
  late Topic running;

  setUp(() {
    now = DateTime.utc(2026, 11, 10, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
    health = places.addArea('Health');
    running = places.addTopic(health.id, 'Running');
    dir = Directory.systemTemp.createTempSync('axt_export_');
    files = FakeBundles(dir);
    announcements = Announcements();
  });
  tearDown(() {
    store.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  void tick() => now = now.add(const Duration(minutes: 1));

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(
    WidgetTester tester, {
    bool platform = true,
    JobRunner? runner,
  }) async {
    tester.view.physicalSize = const Size(1080, 2600);
    tester.view.devicePixelRatio = 2.625;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        bundles: platform ? files : const NoBundles(),
        announcements: announcements,
        jobRunner: runner,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> browse(WidgetTester tester) =>
      tap(tester, find.byTooltip('Search and browse'));

  Future<void> allNotes(WidgetTester tester) async {
    await browse(tester);
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// The texts of the records of the one file shared, read as the app
  /// reads an AXT file it is handed, in the file's order.
  Future<List<String>> sharedTexts(WidgetTester tester) async {
    final bytes = File(files.shared.single).readAsBytesSync();
    final reader = Store.inMemory();
    addTearDown(reader.close);
    final file = await tester.runAsync(
      () => AxtFiles(reader).read(bytes, wallClock: (utc) => utc),
    );
    return [for (final record in file!.taken) record.draft!.text];
  }

  String shared() => files.shared.single.split('/').last;

  group('the file name', () {
    final day = DateTime.utc(2026, 11, 25);

    test('is the title, what no file name holds made _', () {
      expect(axtFileName('Buy milk', day), 'Buy milk.axt');
      expect(
        axtFileName('Bills 2026/11: paid?', day),
        'Bills 2026_11_ paid_.axt',
      );
      expect(axtFileName('Trip\nsecond line', day), 'Trip.axt');
      expect(axtFileName('  ..Plan.. ', day), 'Plan.axt');
      expect(axtFileName('con', day), '_con.axt');
      expect(axtFileName('a' * 80, day), '${'a' * 60}.axt');
    });

    test('is the day\'s without a title', () {
      expect(axtFileName(null, day), 'oldbrainfix-notes-2026-11-25.axt');
      expect(axtFileName(' / ', day), '_.axt');
      expect(axtFileName('...', day), 'oldbrainfix-notes-2026-11-25.axt');
    });
  });

  testWidgets('a card\'s ⋮ ▸ Share ▸ writes the Note and shares it as '
      '<title>.axt', (tester) async {
    notes.capture('Bike lock code\n4711', place: health.id);
    await start(tester);
    await allNotes(tester);
    await tap(tester, key('note-menu'));
    await tap(tester, key('note-group-share'));
    await tap(tester, key('note-axt'));

    expect(shared(), 'Bike lock code.axt');
    expect(files.types, [axtMimeType]);
    expect(await sharedTexts(tester), ['Bike lock code\n4711']);
  });

  testWidgets('the Note editor\'s Share group writes it too', (tester) async {
    notes.capture('Pills: blue one', place: health.id);
    await start(tester);
    await allNotes(tester);
    await tester.longPress(find.text('Pills: blue one'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    await tapEditorAction(tester, key('axt-note'));

    expect(shared(), 'Pills_ blue one.axt');
    expect(await sharedTexts(tester), ['Pills: blue one']);
  });

  testWidgets('an Area\'s menu writes all its Notes, its Topics\' too, '
      'named after it', (tester) async {
    notes.capture('Drink water', place: health.id);
    tick();
    notes.capture('Laps', place: running.id);
    notes.capture('Elsewhere');
    await start(tester);
    await browse(tester);
    await tap(tester, key('area-menu-${health.id}'));
    await tap(tester, key('area-axt-${health.id}'));

    expect(shared(), 'Health.axt');
    expect(await sharedTexts(tester), unorderedEquals(['Drink water', 'Laps']));
  });

  testWidgets('a Topic\'s menu writes its Notes, named after it', (
    tester,
  ) async {
    notes.capture('Drink water', place: health.id);
    notes.capture('Laps', place: running.id);
    await start(tester);
    await browse(tester);
    await tap(tester, find.text('Health'));
    await tap(tester, key('topic-menu-${running.id}'));
    await tap(tester, key('topic-axt-${running.id}'));

    expect(shared(), 'Running.axt');
    expect(await sharedTexts(tester), ['Laps']);
  });

  testWidgets('a selection\'s bar writes the ticked Notes and ends the '
      'selection', (tester) async {
    final a = notes.capture('apples');
    tick();
    final b = notes.capture('bread');
    tick();
    notes.capture('cheese');
    await start(tester);
    await allNotes(tester);
    await tap(tester, key('selectNotes'));
    expect(
      tester
          .widget<IconButton>(
            find.descendant(
              of: key('selection-axt'),
              matching: find.byType(IconButton),
            ),
          )
          .onPressed,
      isNull,
    );
    await tap(tester, key('select-${a.id}'));
    await tap(tester, key('select-${b.id}'));
    await tap(tester, key('selection-axt'));

    expect(shared(), 'oldbrainfix-notes-2026-11-10.axt');
    expect(await sharedTexts(tester), unorderedEquals(['apples', 'bread']));
    expect(key('select-${a.id}'), findsNothing);
  });

  testWidgets('an Area without Notes says so at the top and shares nothing', (
    tester,
  ) async {
    await start(tester);
    await browse(tester);
    await tap(tester, key('area-menu-${health.id}'));
    await tap(tester, key('area-axt-${health.id}'));

    expect(files.shared, isEmpty);
    expect(
      announcements.visible,
      'There are no Notes here to put in an AXT file.',
    );
  });

  testWidgets('no app to take it is announced at the top', (tester) async {
    notes.capture('Bike lock code', place: health.id);
    files.shares = false;
    await start(tester);
    await allNotes(tester);
    await tap(tester, key('note-menu'));
    await tap(tester, key('note-group-share'));
    await tap(tester, key('note-axt'));

    expect(announcements.visible, 'The AXT file was not shared.');
    final action = announcements.visibleAction;
    announcements.dismiss();
    action?.call();
    await tester.pumpAndSettle();
    expect(
      find.textContaining('No app here can take the file.'),
      findsOneWidget,
    );
  });

  testWidgets('a card of the list offers it in Share, after Export as Pack', (
    tester,
  ) async {
    notes.capture('Bike lock code', place: health.id);
    await start(tester);
    await allNotes(tester);
    await tap(tester, key('note-menu'));
    await tap(tester, key('note-group-share'));
    final pack = tester.getTopLeft(key('note-pack')).dy;
    expect(tester.getTopLeft(key('note-axt')).dy, greaterThan(pack));
    expect(find.byType(NoteCard), findsWidgets);
  });
  testWidgets('the file is written as a long job, on the app\'s runner', (
    tester,
  ) async {
    notes.capture('Bike lock code', place: health.id);
    final runner = _CountingRunner();
    await start(tester, runner: runner);
    await allNotes(tester);
    await tap(tester, key('note-menu'));
    await tap(tester, key('note-group-share'));
    await tap(tester, key('note-axt'));

    expect(runner.runs, 1);
    expect(await sharedTexts(tester), ['Bike lock code']);
  });

  testWidgets('no place to write it is announced at the top', (tester) async {
    notes.capture('Bike lock code', place: health.id);
    await start(tester, platform: false);
    await allNotes(tester);
    await tap(tester, key('note-menu'));
    await tap(tester, key('note-group-share'));
    await tap(tester, key('note-axt'));

    expect(announcements.visible, 'The AXT file was not shared.');
    final action = announcements.visibleAction;
    announcements.dismiss();
    action?.call();
    await tester.pumpAndSettle();
    expect(
      find.textContaining('The AXT file could not be written.'),
      findsOneWidget,
    );
  });

  group('in an isolate of its own', () {
    late Directory home;
    late Store app;

    setUp(() {
      home = Directory.systemTemp.createTempSync('axt_job_');
      app = Store.open('${home.path}/oldbrainfix.db');
    });
    tearDown(() {
      app.close();
      home.deleteSync(recursive: true);
    });

    test('writes the file on a store of its own, which opens again', () async {
      final note = Notes(app).capture('Pills at 8');
      final path = '${home.path}/Pills at 8.axt';
      await IsolateJobRunner(
        path: '${home.path}/oldbrainfix.db',
        folders: const NoFolders(),
      ).run(app, axtFileJob([note.id], path), (_, _) {}, JobStop());

      final file = await AxtFiles(app)
          .read(File(path).readAsBytesSync(), wallClock: (utc) => utc);
      expect(file.fromOwnDevice, isTrue);
      expect([for (final r in file.taken) r.draft?.text], ['Pills at 8']);
    });
  });
}

/// The app's runner, inline, counting the jobs it is given.
class _CountingRunner implements JobRunner {
  var runs = 0;

  @override
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  ) {
    runs++;
    return const InlineJobRunner(NoFolders()).run(store, work, progress, stop);
  }
}

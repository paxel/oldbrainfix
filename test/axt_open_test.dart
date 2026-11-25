import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/axt_open_screen.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/import_run.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;

/// An AXT file opened (obf-1146, ADR 0056): its records on a page, all
/// ticked, those not taken named and greyed, who it is from, a place
/// picked at the finger, Unsorted at first, and "Take it".
void main() {
  var now = DateTime.utc(2026, 11, 25, 8);
  late Store store;
  late Store ann;
  late Directory dir;
  late FakeBundles files;
  late Announcements announcements;
  late Area health;
  late Topic running;

  setUp(() {
    now = DateTime.utc(2026, 11, 25, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Ben');
    ann = Store.inMemory(clock: () => now);
    final places = Places(store);
    health = places.addArea('Health');
    running = places.addTopic(health.id, 'Running');
    dir = Directory.systemTemp.createTempSync('axt_open_');
    files = FakeBundles(dir);
    announcements = Announcements();
  });
  tearDown(() {
    store.close();
    ann.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1080, 2600);
    tester.view.devicePixelRatio = 2.625;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        bundles: files,
        announcements: announcements,
      ),
    );
    await tester.pumpAndSettle();
  }

  /// [bytes] as a file another app handed over, named [name].
  String copyOf(List<int> bytes, [String name = 'in.axt']) {
    final path = '${dir.path}/$name';
    File(path).writeAsBytesSync(bytes);
    return path;
  }

  /// Ann's AXT file of a plain Note and one with a Reminder.
  Future<Uint8List> annsFile(WidgetTester tester) async {
    final notes = Notes(ann);
    final milk = notes.capture('Buy milk\nand bread');
    final dentist = notes.capture('Dentist');
    Routines(ann).spawn(
      dentist.id,
      Schedule(start: Day(2026, 12, 3), times: [ClockTime(9, 30)]),
    );
    return (await tester.runAsync(
      () => AxtFiles(ann).write([milk.id, dentist.id]),
    ))!;
  }

  Uint8List catalog() =>
      File('desktop/crates/oldbrainfix-core/tests/axt/catalog.axt')
          .readAsBytesSync();

  bool ticked(WidgetTester tester, String id) =>
      tester.widget<CheckboxListTile>(key('axt-record-$id')).value ?? false;

  testWidgets('opened with the app: every record ticked, with its type and '
      'Reminder; taken into a Topic picked at the finger', (tester) async {
    final bytes = await annsFile(tester);
    await start(tester);
    final path = copyOf(bytes);
    files.open(path);
    await tester.pumpAndSettle();

    expect(find.byType(AxtOpenScreen), findsOneWidget);
    expect(File(path).existsSync(), isFalse);
    expect(find.text('From Old Brain Fix'), findsOneWidget);
    final file = await tester.runAsync(
      () => AxtFiles(store).read(bytes, wallClock: (utc) => utc),
    );
    expect(find.text('Key ${file!.shortFingerprint}'), findsOneWidget);
    for (final record in file.taken) {
      expect(ticked(tester, record.id), isTrue);
    }
    expect(find.text('Buy milk'), findsOneWidget);
    expect(find.text('Note'), findsOneWidget);
    expect(find.textContaining('Task · Reminder: '), findsOneWidget);
    expect(find.text('Unsorted'), findsOneWidget);

    await tap(tester, key('axt-place'));
    // Unsorted first, then the Area and its Topic.
    final unsorted = tester.getTopLeft(key('axt-place-')).dy;
    expect(
      tester.getTopLeft(key('axt-place-${health.id}')).dy,
      greaterThan(unsorted),
    );
    await tap(tester, key('axt-place-${running.id}'));
    expect(find.text('Health ▸ Running'), findsOneWidget);
    await tap(tester, key('axt-take'));

    expect(find.byType(AxtOpenScreen), findsNothing);
    expect(announcements.visible, '2 Notes taken');
    final taken = Notes(store).inside(running.id);
    expect(
      taken.map((n) => n.text),
      unorderedEquals(['Buy milk\nand bread', 'Dentist']),
    );
    expect(Routines(store).all(), hasLength(1));
  });

  testWidgets('another app\'s file: its name, a type not taken greyed, and '
      'only the ticked ones taken, into Unsorted', (tester) async {
    await start(tester);
    files.open(copyOf(catalog()));
    await tester.pumpAndSettle();

    expect(find.text('From cat-a-log'), findsOneWidget);
    expect(find.text('Not taken'), findsOneWidget);
    expect(find.text('org.example.Memo'), findsOneWidget);
    final memo = tester.widget<CheckboxListTile>(
      find.ancestor(
        of: find.text('org.example.Memo'),
        matching: find.byType(CheckboxListTile),
      ),
    );
    expect(memo.onChanged, isNull);
    expect(memo.value, isFalse);
    expect(find.textContaining('Appointment · Reminder: '), findsOneWidget);
    expect(find.textContaining('Task · Routine: '), findsOneWidget);

    final vet = find.ancestor(
      of: find.textContaining('Appointment · '),
      matching: find.byType(CheckboxListTile),
    );
    await tap(tester, vet);
    await tap(tester, key('axt-take'));

    expect(announcements.visible, '1 Note taken');
    final unsorted = Notes(store).unsorted();
    expect(unsorted, hasLength(1));
    expect(unsorted.single.text, isNot(startsWith('Vet')));
  });

  testWidgets('Untick all greys Take it and says why', (tester) async {
    final bytes = await annsFile(tester);
    await start(tester);
    files.open(copyOf(bytes));
    await tester.pumpAndSettle();
    await tap(tester, key('pack-untick-all'));

    expect(tester.widget<FilledButton>(key('axt-take')).onPressed, isNull);
    expect(find.text('Tick at least one Note.'), findsOneWidget);
    await tap(tester, key('pack-tick-all'));
    expect(tester.widget<FilledButton>(key('axt-take')).onPressed, isNotNull);
  });

  testWidgets('a file of this Device says it is from your own Device', (
    tester,
  ) async {
    final note = Notes(store).capture('Pills at 8');
    final bytes = (await tester.runAsync(
      () => AxtFiles(store).write([note.id]),
    ))!;
    await start(tester);
    files.open(copyOf(bytes, 'mine.bin'));
    await tester.pumpAndSettle();

    expect(find.text('From your own Device'), findsOneWidget);
  });

  testWidgets('a damaged file says at the top it did not open', (tester) async {
    await start(tester);
    files.open(copyOf([0x4F, 0x62, 0x6A, 0x01, 1, 2, 3, 4]));
    await tester.pumpAndSettle();

    expect(find.byType(AxtOpenScreen), findsNothing);
    expect(announcements.visible, 'The AXT file was not opened.');
  });

  testWidgets('a file picked opens the same page', (tester) async {
    final bytes = await annsFile(tester);
    await start(tester);
    final context = tester.element(find.byType(Navigator).first);
    final path = copyOf(bytes, 'picked');
    final opening = openReceivedFile(context, store, path);
    await tester.pumpAndSettle();

    expect(find.byType(AxtOpenScreen), findsOneWidget);
    await tap(tester, key('axt-take'));
    expect(await opening, 0);
    expect(Notes(store).unsorted(), hasLength(2));
  });

  test('only a file that starts as an Avro container is taken for one', () {
    expect(startsLikeAxt([0x4F, 0x62, 0x6A, 0x01]), isTrue);
    expect(startsLikeAxt([0x50, 0x4B, 0x03, 0x04]), isFalse);
    expect(startsLikeAxt('{"pack"'.codeUnits), isFalse);
    expect(startsLikeAxt([0x4F, 0x62]), isFalse);
  });

  group('in an isolate of its own', () {
    late Directory home;
    late Store app;

    setUp(() {
      home = Directory.systemTemp.createTempSync('axt_read_');
      app = Store.open('${home.path}/oldbrainfix.db');
    });
    tearDown(() {
      app.close();
      home.deleteSync(recursive: true);
    });

    test('reads a repeating Task on a store of its own', () async {
      final pills = Notes(ann).capture('Pills');
      Routines(ann).spawn(
        pills.id,
        Schedule(
          start: Day(2026, 12, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      final bytes = await AxtFiles(ann).write([pills.id]);
      final file = await IsolateJobRunner(
        path: '${home.path}/oldbrainfix.db',
        folders: const NoFolders(),
      ).run(app, axtReadJob(bytes), (_, _) {}, JobStop());

      final draft = file.taken.single.draft!;
      expect(draft.text, 'Pills');
      expect(draft.schedule?.rule?.format(), 'FREQ=DAILY');
      expect(file.fromOwnDevice, isFalse);
    });
  });
}

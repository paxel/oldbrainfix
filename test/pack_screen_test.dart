import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/pack_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_profile.dart';

void main() {
  late Store store;
  late Store ann;
  late Directory dir;
  late FakeBundles bundles;
  late Area health;
  late Topic running;
  late String laps;
  late String training;

  setUp(() {
    // The Pack's work runs inline but where a test puts the isolate back.
    packWork = <R>(work) async => work();
    packWorkRuns = 0;
    store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Ben');
    ann = Store.inMemory()..applyProfileDefaults(name: 'Ann');
    ann.renameOwnProfile('Ann');
    dir = Directory.systemTemp.createTempSync('pack_screen_');
    bundles = FakeBundles(dir);
    // Ann's Health: a Note of the Area, and in Running a Tally, a Session
    // picking it and a Note with a photo and its Photo Text.
    final places = Places(ann);
    final notes = Notes(ann);
    health = places.addArea('Health');
    running = places.addTopic(health.id, 'Running');
    notes.capture('Drink water', place: health.id);
    laps = notes.capture('Laps', place: running.id).id;
    Tallies(ann).addField(laps, name: 'Laps');
    training = notes.capture('Training', place: running.id).id;
    Sessions(ann)
        .make(training, pool: [laps], count: 1, rule: SessionRule.rolling);
    final route = notes.capture(
      'Route',
      place: running.id,
      photos: [Uint8List.fromList(List<int>.generate(3000, (i) => i % 251))],
    );
    notes.setPhotoText(
      route.id,
      notes.byId(route.id)!.photos.single,
      'Park map',
    );
  });
  tearDown(() {
    packWork = Isolate.run;
    store.close();
    ann.close();
    dir.deleteSync(recursive: true);
  });

  Future<void> start(WidgetTester tester, Store on) async {
    tester.view.physicalSize = const Size(1080, 2600);
    tester.view.devicePixelRatio = 2;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: on,
        profile: FakeProfilePlatform(name: 'Ben'),
        bundles: bundles,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Finder item(String key) => find.byKey(ValueKey('pack-item-$key'));

  bool ticked(WidgetTester tester, String key) =>
      tester.widget<CheckboxListTile>(item(key)).value!;

  String sizeShown(WidgetTester tester) =>
      tester.widget<Text>(find.byKey(const ValueKey('pack-size'))).data!;

  /// Ann's Pack of [ticks], with her Profile when [profile].
  List<int> annsPack({bool profile = false}) {
    final pack = ann.packOf(health.id);
    return ann.writePack(pack, pack.allKeys(), addProfile: profile);
  }

  String copyOf(List<int> bytes) {
    final path = '${dir.path}/in-${bytes.length}.obfsync';
    File(path).writeAsBytesSync(bytes);
    return path;
  }

  /// A photo larger than a Sent Item can be, on a new Note of Ann's
  /// Health.
  void bigPhoto() => Notes(ann).capture(
    'Big map',
    place: health.id,
    photos: [
      Uint8List.fromList(
        List<int>.generate(maxSentItemBytes + 1000, (i) => i % 241),
      ),
    ],
  );

  /// Waits, on the real clock, until [done], up to a minute.
  /// The work's results arrive on the real clock, what awaits them runs
  /// on the test's, with each pump.
  Future<void> until(WidgetTester tester, bool Function() done) async {
    for (var i = 0; i < 600 && !done(); i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
    }
    await tester.pumpAndSettle();
  }

  group('export', () {
    testWidgets('from an Area: the tree ticked but files, the size live, '
        'and the file to the share sheet', (tester) async {
      store.close();
      store = ann
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      ann = Store.inMemory();
      await start(tester, store);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byKey(ValueKey('area-menu-${health.id}')));
      await tap(tester, find.byKey(ValueKey('area-pack-${health.id}')));
      expect(find.byType(PackExportScreen), findsOneWidget);
      expect(find.text('Health'), findsOneWidget);
      expect(find.text('Running'), findsOneWidget);
      final pack = store.packOf(health.id);
      final route = pack.notes.indexWhere((n) => n.text == 'Route');
      expect(ticked(tester, PackItemKeys.note(route)), isTrue);
      expect(ticked(tester, PackItemKeys.photo(route, 0)), isFalse);
      expect(ticked(tester, PackItemKeys.photoText(route, 0)), isTrue);
      expect(find.text('Photo · 3 KB'), findsOneWidget);
      final before = sizeShown(tester);
      await tap(tester, item(PackItemKeys.photo(route, 0)));
      expect(sizeShown(tester), isNot(before));
      await tap(tester, find.byKey(const ValueKey('pack-untick-all')));
      expect(
        tester
            .widget<FilledButton>(find.byKey(const ValueKey('pack-share')))
            .onPressed,
        isNull,
      );
      await tap(tester, find.byKey(const ValueKey('pack-tick-all')));
      await tap(tester, find.byKey(const ValueKey('pack-add-profile')));
      await tap(tester, find.byKey(const ValueKey('pack-share')));
      expect(bundles.types, [packMimeType]);
      final file = File(bundles.shared.single);
      expect(file.path, endsWith('.$packExtension'));
      final json = jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
      expect(json.keys, ['pack', 'sig', 'type']);
    });

    testWidgets('from a Topic and a Note; unticking the last Tally of a '
        'Session unticks it', (tester) async {
      store.close();
      store = ann
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      ann = Store.inMemory();
      await start(tester, store);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.text('Health'));
      await tap(tester, find.byKey(ValueKey('topic-menu-${running.id}')));
      await tap(tester, find.byKey(ValueKey('topic-pack-${running.id}')));
      final pack = store.packOf(running.id);
      final session = PackItemKeys.note(
        pack.notes.indexWhere((n) => n.text == 'Training'),
      );
      final tally = PackItemKeys.note(
        pack.notes.indexWhere((n) => n.text == 'Laps'),
      );
      expect(find.text('Drink water'), findsNothing);
      expect(ticked(tester, session), isTrue);
      await tap(tester, item(tally));
      expect(ticked(tester, session), isFalse);
      await tap(tester, item(session));
      expect(ticked(tester, tally), isTrue);
      await tap(tester, find.byTooltip('Back'));
      // From a Note's ⋮: the Note and the Tally its Session picks.
      await tap(tester, find.text('Running'));
      final card = find.ancestor(
        of: find.text('Training'),
        matching: find.byType(NoteCard),
      );
      await tap(
        tester,
        find.descendant(
          of: card.first,
          matching: find.byKey(const ValueKey('note-menu')),
        ),
      );
      await tap(tester, find.byKey(const ValueKey('note-pack')));
      expect(find.byType(PackExportScreen), findsOneWidget);
      expect(find.text('Training'), findsWidgets);
      expect(find.text('Laps'), findsOneWidget);
      expect(find.text('Route'), findsNothing);
    });
  });

  group('off the UI thread', () {
    testWidgets('what the ticks pick and the size are worked out when the '
        'ticks change, not on each build', (tester) async {
      store.close();
      store = ann
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      ann = Store.inMemory();
      await start(tester, store);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byKey(ValueKey('area-menu-${health.id}')));
      packTicksWorkedOut = 0;
      await tap(tester, find.byKey(ValueKey('area-pack-${health.id}')));
      expect(packTicksWorkedOut, 1);
      // A build that changes no tick works out nothing again.
      await tap(tester, find.byKey(const ValueKey('pack-add-profile')));
      expect(packTicksWorkedOut, 1);
      final before = sizeShown(tester);
      await tap(tester, find.byKey(const ValueKey('pack-tick-all')));
      expect(packTicksWorkedOut, 2);
      expect(sizeShown(tester), isNot(before));
      await tap(tester, find.byKey(const ValueKey('pack-untick-all')));
      expect(packTicksWorkedOut, 3);
    });

    testWidgets('a large Pack is written on another isolate, Share… '
        'waiting meanwhile', (tester) async {
      bigPhoto();
      store.close();
      store = ann
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      ann = Store.inMemory();
      await start(tester, store);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.byKey(ValueKey('area-menu-${health.id}')));
      await tap(tester, find.byKey(ValueKey('area-pack-${health.id}')));
      await tap(tester, find.byKey(const ValueKey('pack-tick-all')));
      packWork = Isolate.run;
      final share = find.byKey(const ValueKey('pack-share'));
      await tester.ensureVisible(share);
      await tester.pumpAndSettle();
      await tester.tap(share);
      await tester.pump();
      await tester.pump();
      // The page keeps drawing, Share… greyed with its spinner.
      expect(tester.widget<FilledButton>(share).onPressed, isNull);
      expect(
        find.descendant(
          of: share,
          matching: find.byType(CircularProgressIndicator),
        ),
        findsOneWidget,
      );
      expect(bundles.shared, isEmpty);
      await until(tester, () => bundles.shared.isNotEmpty);
      expect(packWorkRuns, 1);
      expect(tester.widget<FilledButton>(share).onPressed, isNotNull);
      final bytes = File(bundles.shared.single).readAsBytesSync();
      expect(bytes.length, greaterThan(maxSentItemBytes));
      final opened = Store.inMemory().openPack(bytes);
      expect(opened.pack.notes.map((n) => n.text), contains('Big map'));
    });

    testWidgets('a large Pack is read and opened on another isolate, a '
        'small one read here', (tester) async {
      bigPhoto();
      final pack = ann.packOf(health.id);
      final big = ann.writePack(pack, pack.allKeys(), addProfile: true);
      expect(big.length, greaterThan(maxSentItemBytes));
      await start(tester, store);
      packWork = Isolate.run;
      bundles.open(copyOf(big));
      await until(
        tester,
        () => find.byType(PackImportScreen).evaluate().isNotEmpty,
      );
      expect(find.byType(PackImportScreen), findsOneWidget);
      expect(find.text('From Ann'), findsOneWidget);
      // Read when asked whether it is a Pack and when it is opened, then
      // checked and its Notes read.
      expect(packWorkRuns, 4);
      await tap(tester, find.byTooltip('Back'));

      packWork = <R>(work) async => work();
      packWorkRuns = 0;
      final small = ann.packOf(running.id);
      bundles.open(copyOf(ann.writePack(small, small.allKeys())));
      await tester.pumpAndSettle();
      expect(find.byType(PackImportScreen), findsOneWidget);
      // Only the checks and the reading of its Notes.
      expect(packWorkRuns, 2);
    });
  });

  group('import', () {
    testWidgets('opens with the app, all ticked, from Ann; unclaimed each '
        'Note shows "From Ann" until removed', (tester) async {
      await start(tester, store);
      bundles.open(copyOf(annsPack(profile: true)));
      await tester.pumpAndSettle();
      expect(find.byType(PackImportScreen), findsOneWidget);
      expect(find.text('From Ann'), findsOneWidget);
      expect(find.byKey(const ValueKey('pack-add-person')), findsOneWidget);
      final claim = find.byKey(const ValueKey('pack-claim'));
      expect(tester.widget<Checkbox>(claim).value, isTrue);
      final pack = store.openPack(annsPack(profile: true)).pack;
      for (final key in pack.allKeys()) {
        expect(ticked(tester, key), isTrue, reason: key);
      }
      await tap(tester, claim);
      await tap(tester, find.byKey(const ValueKey('pack-add')));
      expect(find.byType(PackImportScreen), findsNothing);
      expect(find.text('4 Notes added'), findsOneWidget);
      final areas = Places(store).areas();
      expect(areas.map((a) => a.name), contains('Health'));
      final water = Notes(store)
          .all()
          .firstWhere((n) => n.text == 'Drink water');
      expect(store.packFrom(water.id)?.name, 'Ann');
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.text('Health'));
      expect(find.byKey(ValueKey('pack-from-${water.id}')), findsOneWidget);
      final card = find.ancestor(
        of: find.text('Drink water'),
        matching: find.byType(NoteCard),
      );
      await tap(
        tester,
        find.descendant(
          of: card.first,
          matching: find.byKey(const ValueKey('note-menu')),
        ),
      );
      await tap(tester, find.byKey(const ValueKey('note-remove-pack-from')));
      expect(store.packFrom(water.id), isNull);
      expect(find.byKey(ValueKey('pack-from-${water.id}')), findsNothing);
    });

    testWidgets('a name taken gets the suffix; without a Profile nothing '
        'is asked', (tester) async {
      Places(store).addArea('Health');
      await start(tester, store);
      bundles.open(copyOf(annsPack()));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pack-claim')), findsNothing);
      expect(find.byKey(const ValueKey('pack-from')), findsNothing);
      await tap(tester, find.byKey(const ValueKey('pack-add')));
      expect(
        Places(store).areas().map((a) => a.name),
        containsAll(['Health', 'Health (Pack)']),
      );
      expect(
        Notes(store).all().map((n) => store.packFrom(n.id)),
        everyElement(isNull),
      );
    });

    testWidgets('a blocked sender\'s Pack and a changed one do not open', (
      tester,
    ) async {
      final signed = annsPack(profile: true);
      store.openProfileCard(ann.profileCard()!);
      store.blockPerson(ann.ownProfile!.id);
      await start(tester, store);
      bundles.open(copyOf(signed));
      await tester.pumpAndSettle();
      expect(find.byType(PackImportScreen), findsNothing);
      expect(find.text('The Pack was not opened.'), findsOneWidget);
      await tap(tester, find.text('The Pack was not opened.'));
      expect(
        find.textContaining('This Pack is from a person you blocked.'),
        findsOneWidget,
      );
      expect(Notes(store).all(), isEmpty);
      await tap(tester, find.text('Close'));

      final changed = utf8.encode(
        utf8.decode(signed).replaceFirst('Drink water', 'Drink wine'),
      );
      bundles.open(copyOf(changed));
      await tester.pumpAndSettle();
      expect(find.byType(PackImportScreen), findsNothing);
      await tap(tester, find.text('The Pack was not opened.').last);
      expect(find.textContaining('it was changed'), findsOneWidget);
      expect(Notes(store).all(), isEmpty);
    });
  });
}

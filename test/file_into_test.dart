import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// File into…: its filter, the places filed into last and the new
/// Areas and Topics made from it (#272).
void main() {
  late Store store;
  late Places places;
  late Area home;
  late Topic kitchen;
  late Area work;
  late Topic meetings;
  final en = lookupAppLocalizations(const Locale('en'));

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
    places = Places(store);
    home = places.addArea('Household');
    kitchen = places.addTopic(home.id, 'Kitchen');
    places.addTopic(home.id, 'Garden');
    work = places.addArea('Work');
    meetings = places.addTopic(work.id, 'Meetings');
  });
  tearDown(() => store.close());

  Finder key(String k) => find.byKey(ValueKey(k));

  /// File into… pushed over a page, as the app opens it; [picked] holds
  /// what it popped with once it closed.
  Future<List<String?>> open(WidgetTester tester, {String? current}) async {
    tester.view.physicalSize = const Size(2400, 3600);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    final picked = <String?>[];
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () async => picked.add(
              await Navigator.of(context).push(
                MaterialPageRoute<String>(
                  builder: (_) =>
                      FileIntoScreen(places: places, current: current),
                ),
              ),
            ),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    return picked;
  }

  Future<void> tap(WidgetTester tester, Finder target) async {
    await tester.ensureVisible(target);
    await tester.pumpAndSettle();
    await tester.tap(target);
    await tester.pumpAndSettle();
  }

  Finder inScreen(Finder finder) =>
      find.descendant(of: find.byType(FileIntoScreen), matching: finder);

  testWidgets('a pick is kept as filed into last, the latest of three on top', (
    tester,
  ) async {
    var picked = await open(tester);
    expect(find.text(en.fileIntoRecent), findsNothing);
    await tap(tester, find.text('Kitchen'));
    expect(picked, [kitchen.id]);
    expect(store.recentlyFiledInto, [kitchen.id]);

    store
      ..filedInto(meetings.id)
      ..filedInto(work.id);
    picked = await open(tester);
    expect(find.text(en.fileIntoRecent), findsOneWidget);
    final rows = [
      for (final id in [work.id, meetings.id, kitchen.id])
        tester.getTopLeft(key('file-into-recent-$id')).dy,
    ];
    expect(rows, orderedEquals([...rows]..sort()), reason: 'latest first');
    // A recent Topic names its Area under it.
    expect(
      find.descendant(
        of: key('file-into-recent-${kitchen.id}'),
        matching: find.text('Household'),
      ),
      findsOneWidget,
    );
    await tap(tester, key('file-into-recent-${meetings.id}'));
    expect(picked, [meetings.id]);
    expect(store.recentlyFiledInto, [meetings.id, work.id, kitchen.id]);
  });

  testWidgets('a recent place deleted since is left out', (tester) async {
    store
      ..filedInto(kitchen.id)
      ..filedInto('gone');
    places.deleteTopic(kitchen.id, NotesGo.unsorted);
    await open(tester);
    expect(find.text(en.fileIntoRecent), findsNothing);
    expect(find.text('Kitchen'), findsNothing);
  });

  testWidgets('the filter shows the Areas and Topics whose names match', (
    tester,
  ) async {
    store.filedInto(meetings.id);
    await open(tester);
    expect(find.byIcon(Icons.search), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'gar');
    await tester.pumpAndSettle();
    expect(find.text('Garden'), findsOneWidget);
    expect(find.text('Household'), findsOneWidget, reason: 'its Area');
    expect(find.text('Kitchen'), findsNothing);
    expect(find.text('Work'), findsNothing);
    expect(find.text(en.fileIntoRecent), findsNothing);

    // An Area that matches shows with all its Topics.
    await tester.enterText(find.byType(TextField), 'WORK');
    await tester.pumpAndSettle();
    expect(find.text('Meetings'), findsOneWidget);
    expect(find.text('Household'), findsNothing);

    await tester.enterText(find.byType(TextField), 'dentist');
    await tester.pumpAndSettle();
    expect(find.text(en.fileIntoNoMatch), findsOneWidget);
    expect(key('file-into-new-area'), findsOneWidget);

    await tap(tester, find.byTooltip(en.clearSearch));
    expect(find.text('Kitchen'), findsOneWidget);
    expect(find.text(en.fileIntoRecent), findsOneWidget);
  });

  testWidgets('New Area… makes the Area with the filter\'s text and picks it', (
    tester,
  ) async {
    final picked = await open(tester);
    await tester.enterText(find.byType(TextField), 'Dentist');
    await tester.pumpAndSettle();
    await tap(tester, key('file-into-new-area'));
    expect(find.text(en.newArea), findsOneWidget);
    expect(find.text(en.create), findsOneWidget, reason: 'not Rename');
    await tap(tester, find.text(en.create));

    final dentist = places.areas().singleWhere((a) => a.name == 'Dentist');
    expect(picked, [dentist.id]);
    expect(store.recentlyFiledInto, [dentist.id]);
  });

  testWidgets('New Topic… makes the Topic in its Area and picks it', (
    tester,
  ) async {
    final picked = await open(tester);
    // Filtered to Work: the filter's text matched Work, so the name
    // starts empty.
    await tester.enterText(find.byType(TextField), 'work');
    await tester.pumpAndSettle();
    await tap(tester, key('file-into-new-topic-${work.id}'));
    expect(find.text(en.newTopic), findsOneWidget);
    final field = find.descendant(
      of: find.byType(AlertDialog),
      matching: find.byType(TextField),
    );
    expect(tester.widget<TextField>(field).controller?.text, isEmpty);
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      'Travel',
    );
    await tap(tester, find.text(en.create));

    final travel = places
        .topics(work.id)
        .singleWhere((t) => t.name == 'Travel');
    expect(picked, [travel.id]);
    expect(store.recentlyFiledInto, [travel.id]);
  });

  testWidgets('a name dialog left with Cancel makes nothing and stays', (
    tester,
  ) async {
    final picked = await open(tester);
    await tap(tester, key('file-into-new-area'));
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      'Car',
    );
    await tap(tester, find.text(en.cancel));
    await tap(tester, key('file-into-new-topic-${home.id}'));
    await tap(tester, find.text(en.cancel));

    expect(places.areas(), hasLength(2));
    expect(places.topics(home.id), hasLength(2));
    expect(picked, isEmpty);
    expect(inScreen(find.text('Household')), findsOneWidget);
    expect(store.recentlyFiledInto, isEmpty);
  });

  testWidgets('with no Area yet, New Area… is there', (tester) async {
    for (final area in places.areas()) {
      places.deleteArea(area.id, NotesGo.unsorted);
    }
    final picked = await open(tester);
    await tap(tester, key('file-into-new-area'));
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      'Car',
    );
    await tap(tester, find.text(en.create));
    expect(picked, [places.areas().single.id]);
  });

  testWidgets('in Simple, New Topic… is not offered; New Area… is', (
    tester,
  ) async {
    store.advanced = false;
    await open(tester);
    expect(find.text(en.fileIntoNewTopic), findsNothing);
    expect(key('file-into-new-area'), findsOneWidget);
  });

  testWidgets('a Member of a Shared Area gets no New Topic… there', (
    tester,
  ) async {
    store
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    final ben = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Pixel')
      ..renameOwnProfile('Ben');
    addTearDown(ben.close);
    final area = store.share(home.id);
    // Ann's computer invites Ben's phone as a Member, as a LAN session
    // would.
    final done = answerSync(
      store,
      wire(
        syncRequest(
          ben,
          store.versionVector(),
          name: 'Phone',
          hostDevice: store.deviceId,
          invited: true,
        ),
      ),
      name: 'Desk',
      inviting: Invite(area, role: AreaRole.member),
    )!;
    applyAnswer(ben, wire(done.answer), shared: true);
    final own = Places(ben).addArea('Mine');
    places = Places(ben);

    await open(tester);
    expect(inScreen(find.text('Household')), findsOneWidget);
    expect(key('file-into-new-topic-${home.id}'), findsNothing);
    expect(key('file-into-new-topic-${own.id}'), findsOneWidget);
  });
}

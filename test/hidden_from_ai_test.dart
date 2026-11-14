import 'dart:io';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/pack_screen.dart';
import 'package:oldbrainfix/src/setting_badges.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_photos.dart';
import 'open_capture.dart';

/// Hidden from AI on the phone (#267, ADR 0049): the switch in a card's
/// ⋮, an Area's menu and the Note editor's badge, a shield on what is
/// hidden, not offered in a Shared Area, and every way of giving
/// something to other people asks to show it to the AI first.
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  late Directory dir;

  setUp(() {
    packWork = <R>(work) async => work();
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 12, 9))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
    dir = Directory.systemTemp.createTempSync('hidden_from_ai_');
  });
  tearDown(() {
    packWork = Isolate.run;
    store.close();
    dir.deleteSync(recursive: true);
  });

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(store: store, bundles: FakeBundles(dir)),
    );
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
  }

  Future<void> allNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// The card of the Note whose text holds [text].
  Finder card(String text) => find
      .ancestor(
        of: find.textContaining(text, findRichText: true),
        matching: find.byType(NoteCard),
      )
      .first;

  Future<void> openMenu(WidgetTester tester, String text) =>
      tap(tester, find.descendant(of: card(text), matching: key('note-menu')));

  Future<void> fileFromCard(WidgetTester tester, String text) => tap(
    tester,
    find.descendant(of: card(text), matching: find.byTooltip('File into…')),
  );

  Finder shieldOn(String text) => find.descendant(
    of: card(text),
    matching: key('note-hidden-from-ai-mark'),
  );

  testWidgets('the card\'s ⋮ marks a Note and takes the mark off, a shield '
      'shows it; in a Shared Area it is not offered', (tester) async {
    final diary = notes.capture('Diary of today').id;
    final family = places.addArea('Family');
    final shared = store.share(family.id);
    notes.capture('Shopping for all', place: shared);
    await allNotes(tester);
    expect(shieldOn('Diary'), findsNothing);

    await openMenu(tester, 'Diary');
    await tap(tester, key('note-hidden-from-ai'));
    expect(store.isMarkedHiddenFromAi(diary), isTrue);
    expect(shieldOn('Diary'), findsOneWidget);
    expect(
      tester
          .widget<Icon>(
            find.descendant(of: shieldOn('Diary'), matching: find.byType(Icon)),
          )
          .semanticLabel,
      'Hidden from AI',
    );

    await openMenu(tester, 'Diary');
    final item = tester.widget<Switch>(
      find.descendant(
        of: key('note-hidden-from-ai'),
        matching: find.byType(Switch),
      ),
    );
    expect(item.value, isTrue);
    await tap(tester, key('note-hidden-from-ai'));
    expect(store.isMarkedHiddenFromAi(diary), isFalse);
    expect(shieldOn('Diary'), findsNothing);

    await openMenu(tester, 'Shopping for all');
    expect(key('note-hidden-from-ai'), findsNothing);
  });

  testWidgets('an Area\'s menu marks it: a shield on its row and on the '
      'cards of its Notes', (tester) async {
    final health = places.addArea('Health');
    notes.capture('Back pain since Monday', place: health.id);
    await start(tester);
    expect(
      find.descendant(
        of: key('area-row-${health.id}'),
        matching: find.byIcon(Icons.shield_outlined),
      ),
      findsNothing,
    );
    await tap(tester, key('area-menu-${health.id}'));
    await tap(tester, key('area-hidden-from-ai-${health.id}'));
    expect(store.isMarkedHiddenFromAi(health.id), isTrue);
    expect(
      find.descendant(
        of: key('area-row-${health.id}'),
        matching: find.byIcon(Icons.shield_outlined),
      ),
      findsOneWidget,
    );
    await tap(tester, find.byTooltip('All Notes'));
    expect(shieldOn('Back pain'), findsOneWidget);
  });

  testWidgets('filing a hidden Note into a Shared Area asks first: Cancel '
      'keeps it as it was, Show to the AI takes the mark off and files it', (
    tester,
  ) async {
    final diary = notes.capture('Diary of today').id;
    store.setHiddenFromAi(diary, true);
    final family = places.addArea('Family');
    final shared = store.share(family.id);
    await allNotes(tester);

    await fileFromCard(tester, 'Diary');
    await tap(tester, find.text('Family'));
    expect(key('hidden-from-ai-refused'), findsOneWidget);
    expect(find.textContaining('Diary of today: hidden from AI'), findsOne);
    await tap(tester, find.text('Cancel'));
    expect(notes.byId(diary)?.place, isNull);
    expect(store.isMarkedHiddenFromAi(diary), isTrue);

    await fileFromCard(tester, 'Diary');
    // Also among those filed into recently now.
    await tap(tester, find.text('Family').first);
    await tap(tester, key('hidden-from-ai-show'));
    expect(store.isMarkedHiddenFromAi(diary), isFalse);
    expect(notes.byId(diary)?.place, shared);
  });

  testWidgets('a Note hidden by its Area is not filed into a Shared Area, '
      'and the Area\'s mark is only named, never taken off from there', (
    tester,
  ) async {
    final health = places.addArea('Health');
    final back = notes.capture('Back pain', place: health.id).id;
    store.setHiddenFromAi(health.id, true);
    final family = places.addArea('Family');
    store.share(family.id);
    await allNotes(tester);

    await fileFromCard(tester, 'Back pain');
    await tap(tester, find.text('Family'));
    expect(key('hidden-from-ai-area-first'), findsOneWidget);
    expect(find.textContaining('Health: hidden from AI'), findsOne);
    expect(key('hidden-from-ai-show'), findsNothing);
    await tap(tester, find.text('Close'));
    expect(store.isMarkedHiddenFromAi(health.id), isTrue);
    expect(notes.byId(back)?.place, health.id);
  });

  testWidgets('merging a hidden Note into a Note of a Shared Area asks '
      'first, as does merging one of a Shared Area into a hidden Note', (
    tester,
  ) async {
    final diary = notes.capture('Diary of today').id;
    store.setHiddenFromAi(diary, true);
    final shared = store.share(places.addArea('Family').id);
    final milk = notes.capture('Milk for the family', place: shared).id;
    await allNotes(tester);

    Future<void> mergeFromCard(String text, String into) async {
      await tap(
        tester,
        find.descendant(
          of: card(text),
          matching: find.byTooltip('Merge into…'),
        ),
      );
      await tap(
        tester,
        find
            .descendant(
              of: find.byType(MergeIntoScreen),
              matching: find.textContaining(into, findRichText: true),
            )
            .first,
      );
    }

    await mergeFromCard('Diary', 'Milk');
    expect(key('hidden-from-ai-refused'), findsOneWidget);
    expect(find.textContaining('Diary of today: hidden from AI'), findsOne);
    await tap(tester, find.text('Cancel'));
    expect(notes.byId(diary)?.isMerged, isFalse);
    expect(store.isMarkedHiddenFromAi(diary), isTrue);

    await mergeFromCard('Milk', 'Diary');
    expect(key('hidden-from-ai-refused'), findsOneWidget);
    expect(find.textContaining('Diary of today: hidden from AI'), findsOne);
    await tap(tester, key('hidden-from-ai-show'));
    expect(store.isMarkedHiddenFromAi(diary), isFalse);
    expect(notes.byId(milk)?.mergedInto, diary);
  });

  testWidgets('merging the ticked Notes asks once for all of them; Cancel '
      'keeps them ticked and apart, Show to the AI merges them', (
    tester,
  ) async {
    final diary = notes.capture('Diary of today').id;
    final dream = notes.capture('Dream of a boat').id;
    store
      ..setHiddenFromAi(diary, true)
      ..setHiddenFromAi(dream, true);
    final shared = store.share(places.addArea('Family').id);
    final milk = notes.capture('Milk for the family', place: shared).id;
    await allNotes(tester);

    Future<void> mergeTicked() async {
      await tap(tester, key('selection-merge'));
      await tap(
        tester,
        find
            .descendant(
              of: find.byType(MergeIntoScreen),
              matching: find.textContaining('Milk', findRichText: true),
            )
            .first,
      );
    }

    await tap(tester, key('selectNotes'));
    await tap(tester, key('select-$diary'));
    await tap(tester, key('select-$dream'));
    await mergeTicked();
    expect(key('hidden-from-ai-refused'), findsOneWidget);
    expect(find.textContaining('Diary of today'), findsWidgets);
    await tap(tester, find.text('Cancel'));
    expect(notes.byId(diary)?.isMerged, isFalse);
    expect(notes.byId(dream)?.isMerged, isFalse);
    expect(tester.widget<Checkbox>(key('select-$diary')).value, isTrue);

    await mergeTicked();
    await tap(tester, key('hidden-from-ai-show'));
    expect(store.hiddenFromAiMarks(), isEmpty);
    expect(notes.byId(diary)?.mergedInto, milk);
    expect(notes.byId(dream)?.mergedInto, milk);
  });

  testWidgets('filing into an Area of the user\'s own asks nothing', (
    tester,
  ) async {
    final diary = notes.capture('Diary of today').id;
    store.setHiddenFromAi(diary, true);
    final home = places.addArea('Home');
    await allNotes(tester);
    await fileFromCard(tester, 'Diary');
    await tap(tester, find.text('Home'));
    expect(key('hidden-from-ai-refused'), findsNothing);
    expect(notes.byId(diary)?.place, home.id);
    expect(store.isMarkedHiddenFromAi(diary), isTrue);
  });

  testWidgets('a Pack and a Sent Item of a hidden Note ask first', (
    tester,
  ) async {
    final diary = notes.capture('Diary of today').id;
    store.setHiddenFromAi(diary, true);
    await allNotes(tester);

    await openMenu(tester, 'Diary');
    await tap(tester, key('note-share-menu'));
    await tap(tester, key('note-pack'));
    expect(key('hidden-from-ai-refused'), findsOneWidget);
    await tap(tester, find.text('Cancel'));
    expect(find.byType(PackExportScreen), findsNothing);

    await openMenu(tester, 'Diary');
    await tap(tester, key('note-share-menu'));
    await tap(tester, key('note-send'));
    expect(key('hidden-from-ai-refused'), findsOneWidget);
    await tap(tester, find.text('Cancel'));
    expect(find.byType(AlertDialog), findsNothing);
    expect(store.isMarkedHiddenFromAi(diary), isTrue);

    await openMenu(tester, 'Diary');
    await tap(tester, key('note-share-menu'));
    await tap(tester, key('note-pack'));
    await tap(tester, key('hidden-from-ai-show'));
    expect(find.byType(PackExportScreen), findsOneWidget);
    expect(store.isMarkedHiddenFromAi(diary), isFalse);
  });

  testWidgets('sharing an Area holding a hidden Note asks first, naming the '
      'Note', (tester) async {
    final health = places.addArea('Health');
    final back = notes.capture('Back pain', place: health.id).id;
    store.setHiddenFromAi(back, true);
    await start(tester);
    await tap(tester, key('area-menu-${health.id}'));
    await tap(tester, find.text('Share…'));
    expect(find.textContaining('Back pain: hidden from AI'), findsOne);
    await tap(tester, find.text('Cancel'));
    expect(find.byType(ShareAreaScreen), findsNothing);
    expect(store.isShared(health.id), isFalse);

    await tap(tester, key('area-menu-${health.id}'));
    await tap(tester, find.text('Share…'));
    await tap(tester, key('hidden-from-ai-show'));
    expect(find.byType(ShareAreaScreen), findsOneWidget);
    expect(store.isMarkedHiddenFromAi(back), isFalse);
  });

  testWidgets('the Note editor\'s badge switches the mark, and says when its '
      'Area hides it', (tester) async {
    final health = places.addArea('Health');
    final back = notes.capture('Back pain', place: health.id).id;
    await allNotes(tester);
    await tester.longPress(card('Back pain'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    final badge = key('badge-hidden-from-ai');
    String? shown() => tester.widget<SettingBadge>(badge).value;
    expect(shown(), 'Off');
    await tap(tester, badge);
    await tap(tester, key('badge-switch'));
    expect(store.isMarkedHiddenFromAi(back), isTrue);
    // Twice in the same dialog: back off.
    await tap(tester, key('badge-switch'));
    expect(store.isMarkedHiddenFromAi(back), isFalse);
    await tap(tester, key('badge-close'));

    store.setHiddenFromAi(health.id, true);
    await tester.pageBack();
    await tester.pumpAndSettle();
    await tester.longPress(card('Back pain'));
    await tester.pumpAndSettle();
    expect(shown(), 'Off, hidden by its Area');
  });

  testWidgets('Capture asks nothing: a Note hidden from AI picked for a '
      'Shared Area stays where it was, said at the top', (tester) async {
    store.setQuickNoteShows(QuickNoteControl.fileInto, true);
    final family = places.addArea('Family');
    store.share(family.id);
    final announcements = Announcements();
    addTearDown(announcements.dispose);
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        photos: FakePhotos(nextPhoto: onePixelPng),
        announcements: announcements,
      ),
    );
    await tester.pumpAndSettle();
    await openTextCapture(tester);
    await tap(tester, find.byKey(const ValueKey('capturePhoto')));
    // The photo wrote the Note; it is marked meanwhile, as on another
    // Device.
    final id = notes.all().single.id;
    store.setHiddenFromAi(id, true);
    await tap(
      tester,
      find.descendant(
        of: find.byType(CaptureScreen),
        matching: find.byTooltip('File into…'),
      ),
    );
    await tap(tester, find.text('Family'));
    expect(key('hidden-from-ai-refused'), findsNothing);
    await tap(tester, find.byKey(const ValueKey('captureSave')));

    expect(tester.takeException(), isNull);
    expect(find.byType(CaptureScreen), findsNothing);
    expect(notes.byId(id)?.place, isNull);
    expect(store.isMarkedHiddenFromAi(id), isTrue);
    expect(announcements.visible, 'A Note was not filed.');
  });
}

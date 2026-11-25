import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/lock_screen.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/big_screen.dart';
import 'package:oldbrainfix/src/frogs_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/notes_screen.dart';
import 'package:oldbrainfix/src/spotlight.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/tag_tree_screen.dart';
import 'package:oldbrainfix/src/trash_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';

/// The text field of the Note editor.
final editorText = find.descendant(
  of: find.byKey(const ValueKey('note-text')),
  matching: find.byType(TextField),
);

/// The big screen's layout (obf-1148): from 700 dp wide a sidebar beside
/// the list and a Note as a dialog over it; the phone's layout below.
void main() {
  late Store store;
  late Places places;
  late Notes notes;
  late Area home;
  late Topic kitchen;
  late Note bags;

  setUp(() {
    store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    places = Places(store);
    notes = Notes(store);
    home = places.addArea('Household');
    kitchen = places.addTopic(home.id, 'Kitchen');
    bags = notes.capture('Buy bin bags', place: home.id);
  });
  tearDown(() => store.close());

  const ipadLandscape = Size(1024, 768);
  const ipadMini = Size(744, 1133);
  const phone = Size(390, 844);

  Future<void> start(
    WidgetTester tester,
    Size size, {
    bool bigScreen = true,
    bool asMain = false,
  }) async {
    tester.view
      ..physicalSize = size
      ..devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        bigScreen: bigScreen,
        // As main.dart starts it.
        spotlights: asMain,
        tapMarks: asMain,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> turn(WidgetTester tester, Size size) async {
    tester.view.physicalSize = size;
    await tester.pumpAndSettle();
  }

  final sidebar = find.byType(Sidebar);

  Finder entry(String key) => find.byKey(ValueKey('sidebar-$key'));

  Future<void> go(WidgetTester tester, String key) async {
    // The sidebar builds what comes into view.
    await tester.scrollUntilVisible(
      entry(key),
      100,
      scrollable: find.descendant(
        of: sidebar,
        matching: find.byType(Scrollable),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(entry(key));
    await tester.pumpAndSettle();
  }

  /// Whether the sidebar marks the entry [key].
  bool marked(WidgetTester tester, String key) =>
      tester.widget<ListTile>(entry(key)).selected;

  Future<void> openBags(WidgetTester tester) async {
    await tester.longPress(find.text('Buy bin bags').first);
    await tester.pumpAndSettle();
  }

  /// Skips the tips showing, as a page's first visit brings them.
  Future<void> skipTips(WidgetTester tester) async {
    if (find.byType(SpotlightTip).evaluate().isEmpty) return;
    await tester.tap(find.byKey(const ValueKey('spotSkip')));
    await tester.pumpAndSettle();
  }

  final noteDialog = find.ancestor(
    of: find.byType(NoteEditorScreen),
    matching: find.byType(Dialog),
  );

  for (final (name, size) in [
    ('1024x768', ipadLandscape),
    ('744x1133', ipadMini),
  ]) {
    group('at $name', () {
      testWidgets('the sidebar stands beside the list and leads to every '
          'place, marking it', (tester) async {
        await start(tester, size);
        expect(sidebar, findsOneWidget);
        expect(tester.getTopLeft(sidebar).dx, 0);
        expect(tester.getSize(sidebar).width, sidebarWidth);
        expect(find.byType(AgendaScreen), findsOneWidget);
        expect(
          tester.getTopLeft(find.byType(AgendaScreen)).dx,
          sidebarWidth,
          reason: 'the list beside it',
        );
        expect(marked(tester, 'agenda'), isTrue);
        expect(
          find.byKey(const ValueKey('browse')),
          findsNothing,
          reason: 'the sidebar leads to the Areas',
        );

        for (final (key, page) in [
          ('area-${home.id}', AreaScreen),
          ('topic-${kitchen.id}', TopicScreen),
          ('allNotes', AllNotesScreen),
          ('unsorted', UnsortedScreen),
          ('frogs', FrogsScreen),
          ('areas', AreasScreen),
          ('tags', TagTreeScreen),
          ('trash', TrashScreen),
          ('agenda', AgendaScreen),
        ]) {
          await go(tester, key);
          expect(find.byType(page), findsOneWidget, reason: key);
          expect(sidebar, findsOneWidget, reason: key);
          expect(marked(tester, key), isTrue, reason: key);
          expect(
            tester.getTopLeft(find.byType(page)).dx,
            sidebarWidth,
            reason: key,
          );
        }
      });

      testWidgets('a Note opens as a dialog over the list and the sidebar, '
          'and a tap beside it keeps what was typed', (tester) async {
        await start(tester, size);
        await go(tester, 'area-${home.id}');
        await openBags(tester);
        expect(noteDialog, findsOneWidget);
        expect(sidebar, findsOneWidget, reason: 'under the dialog');

        await tester.enterText(editorText, 'Buy bin bags and soap');
        await tester.pump();
        // Beside the dialog, over the sidebar.
        await tester.tapAt(const Offset(4, 4));
        await tester.pumpAndSettle();
        expect(noteDialog, findsNothing);
        expect(notes.byId(bags.id)?.text, 'Buy bin bags and soap');
        expect(find.byType(AreaScreen), findsOneWidget);
      });
    });
  }

  testWidgets('at 390x844 the phone keeps its layout', (tester) async {
    await start(tester, phone);
    expect(sidebar, findsNothing);
    expect(find.byKey(const ValueKey('browse')), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('browse')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Household'));
    await tester.pumpAndSettle();
    expect(sidebar, findsNothing);
    await openBags(tester);
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(noteDialog, findsNothing, reason: 'a page of its own');
  });

  testWidgets('without the big screen switched on, a wide window keeps the '
      "phone's layout", (tester) async {
    await start(tester, ipadLandscape, bigScreen: false);
    expect(sidebar, findsNothing);
    expect(find.byKey(const ValueKey('browse')), findsOneWidget);
  });

  testWidgets('turning across 700 dp keeps the place shown', (tester) async {
    await start(tester, ipadLandscape);
    await go(tester, 'topic-${kitchen.id}');
    expect(find.byType(TopicScreen), findsOneWidget);

    await turn(tester, phone);
    expect(sidebar, findsNothing);
    expect(find.byType(TopicScreen), findsOneWidget);

    await turn(tester, ipadMini);
    expect(find.byType(TopicScreen), findsOneWidget);
    expect(marked(tester, 'topic-${kitchen.id}'), isTrue);

    // Back leads up the levels, as on the phone.
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.byType(AreaScreen), findsOneWidget);
    expect(marked(tester, 'area-${home.id}'), isTrue);
  });

  testWidgets('turning across 700 dp keeps an open Note and what was typed '
      'in it', (tester) async {
    await start(tester, ipadLandscape);
    await go(tester, 'area-${home.id}');
    await openBags(tester);
    await tester.enterText(editorText, 'Buy bin bags and soap');
    await tester.pump();
    final editor = tester.state(find.byType(NoteEditorScreen));

    await turn(tester, phone);
    expect(tester.state(find.byType(NoteEditorScreen)), same(editor));
    expect(
      tester.widget<TextField>(editorText).controller?.text,
      'Buy bin bags and soap',
    );
    expect(tester.takeException(), isNull);

    await turn(tester, ipadLandscape);
    expect(tester.state(find.byType(NoteEditorScreen)), same(editor));
    expect(noteDialog, findsOneWidget);
  });

  testWidgets('a closed locked Area lists no Topics and asks for its PIN', (
    tester,
  ) async {
    store.setPin('2468', iterations: 10);
    final doctor = places.addArea('Doctor');
    places.addTopic(doctor.id, 'Dentist');
    store.setAreaLocked(doctor.id, true);
    await start(tester, ipadLandscape);
    expect(entry('area-${doctor.id}'), findsOneWidget);
    expect(find.text('Dentist'), findsNothing);
    expect(
      find.descendant(
        of: entry('area-${doctor.id}'),
        matching: find.byIcon(Icons.lock_outline),
      ),
      findsOneWidget,
    );

    await go(tester, 'area-${doctor.id}');
    expect(find.byKey(const ValueKey('pinField')), findsOneWidget);
    expect(find.byType(AreaScreen), findsNothing);
  });

  testWidgets('Unsorted counts its Notes in the sidebar', (tester) async {
    notes.capture('Call the garage');
    notes.capture('Pick up the parcel');
    await start(tester, ipadLandscape);
    expect(
      find.descendant(of: entry('unsorted'), matching: find.text('2')),
      findsOneWidget,
    );
  });

  testWidgets('a phone held sideways takes the sidebar, and a Note leaves '
      'room for its text over the keyboard', (tester) async {
    await start(tester, const Size(844, 390));
    expect(sidebar, findsOneWidget);
    await go(tester, 'trash');
    await go(tester, 'area-${home.id}');
    await openBags(tester);
    expect(noteDialog, findsOneWidget);

    tester.view.viewInsets = const FakeViewPadding(bottom: 200);
    addTearDown(tester.view.resetViewInsets);
    await tester.showKeyboard(editorText);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    final dialog = tester.getRect(
      find.descendant(of: noteDialog, matching: find.byType(Material)).first,
    );
    expect(dialog.bottom, lessThanOrEqualTo(390 - 200), reason: 'above it');
    expect(dialog.height, greaterThan(150), reason: 'little lost to its edges');
    expect(
      tester.getTopLeft(editorText).dy,
      lessThan(dialog.bottom - 24),
      reason: 'its text shows in it',
    );
  });

  testWidgets('while the App Lock is locked neither the sidebar nor an open '
      'Note shows', (tester) async {
    store
      ..setPin('2468', iterations: 10)
      ..appLockOn = true
      ..relockAfter = Duration.zero;
    await start(tester, ipadLandscape);
    expect(find.byType(LockScreen), findsOneWidget);
    expect(sidebar, findsNothing);
    expect(find.text('Household'), findsNothing);

    await tester.enterText(find.byKey(const ValueKey('lockPin')), '2468');
    await tester.tap(find.byKey(const ValueKey('unlockPin')));
    await tester.pumpAndSettle();
    await go(tester, 'area-${home.id}');
    await openBags(tester);
    expect(noteDialog, findsOneWidget);

    final binding = tester.binding;
    for (final state in [
      AppLifecycleState.inactive,
      AppLifecycleState.hidden,
      AppLifecycleState.paused,
      AppLifecycleState.hidden,
      AppLifecycleState.inactive,
      AppLifecycleState.resumed,
    ]) {
      binding.handleAppLifecycleStateChanged(state);
    }
    await tester.pumpAndSettle();
    expect(find.byType(LockScreen), findsOneWidget);
    expect(find.byType(NoteEditorScreen, skipOffstage: false), findsNothing);
    expect(sidebar, findsNothing);
    expect(find.text('Household'), findsNothing);
    expect(find.text('Buy bin bags'), findsNothing);
  });

  testWidgets("Capture's way into the editor opens the Note as a dialog", (
    tester,
  ) async {
    await start(tester, ipadLandscape);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'ring the plumber');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('captureToEditor')));
    await tester.pumpAndSettle();
    expect(noteDialog, findsOneWidget);
    expect(sidebar, findsOneWidget, reason: 'the Agenda under it');
  });

  testWidgets('as main.dart starts it on the iPad, with tips and tap marks, '
      'the sidebar and the Note dialog work', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
    try {
      await start(tester, ipadMini, asMain: true);
      // The Agenda's tips come first, as on the phone; the folder's
      // waits, its button not on the bar here.
      expect(find.byType(SpotlightTip), findsOneWidget);
      await skipTips(tester);
      expect(sidebar, findsOneWidget);
      await go(tester, 'area-${home.id}');
      await skipTips(tester);
      await openBags(tester);
      await skipTips(tester);
      expect(noteDialog, findsOneWidget);
      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      expect(noteDialog, findsNothing);
      await go(tester, 'agenda');
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(tester.takeException(), isNull);
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });
}

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_group.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/note_history_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_capture.dart';
import 'open_settings.dart';

/// "Fat fingers" (#224): text 1.3 times on top of the system's font size,
/// icons and tap targets 1.3 times, the Agenda's rows with them, and
/// nothing overflowing on a phone in German.
void main() {
  late Store store;
  late Notes notes;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        voice: FakeVoice(),
        photos: FakePhotos(nextPhoto: onePixelPng),
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

  /// The narrowest phone Android supports, 320 dp wide, in German.
  void germanPhone(WidgetTester tester) {
    tester.view.physicalSize = const Size(960, 2340);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
  }

  /// The system's font size, 1.5 times its usual.
  void bigSystemFont(WidgetTester tester) {
    tester.platformDispatcher.textScaleFactorTestValue = 1.5;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
  }

  final fatFingers = find.byKey(const ValueKey('fatFingers'));

  double textOf(WidgetTester tester, Finder at) =>
      MediaQuery.textScalerOf(tester.element(at)).scale(10);

  double iconOf(WidgetTester tester, Finder at) =>
      IconTheme.of(tester.element(at)).size!;

  /// The buttons of the bar on top, the way back among them, side by side
  /// on the screen, none over another.
  void expectBarClear(WidgetTester tester) {
    final bar = find.byType(AppBar).last;
    final rects = [
      for (final button
          in find
              .descendant(of: bar, matching: find.byType(IconButton))
              .evaluate())
        tester.getRect(find.byWidget(button.widget)),
    ]..sort((a, b) => a.left.compareTo(b.left));
    expect(rects, isNotEmpty);
    for (var i = 1; i < rects.length; i++) {
      expect(rects[i].left, greaterThanOrEqualTo(rects[i - 1].right - 0.01));
    }
    final width = tester.view.physicalSize.width / tester.view.devicePixelRatio;
    expect(rects.last.right, lessThanOrEqualTo(width + 0.01));
  }

  testWidgets('off by default: text follows the system, icons and the '
      "Agenda's boxes keep their size", (tester) async {
    bigSystemFont(tester);
    Routines(store).create(
      'pills',
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(10, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    await start(tester);
    expect(store.fatFingers, isFalse);
    final agenda = find.byType(AgendaScreen);
    expect(textOf(tester, agenda), 15);
    expect(iconOf(tester, agenda), 24);
    expect(tester.getSize(find.byType(AgendaBox).first), const Size(40, 40));
  });

  testWidgets('the switch in Settings ▸ Look grows text 1.3 times on top of '
      "the system's font size, icons, tap targets and the Agenda's rows "
      'at once, and keeps it on this Device', (tester) async {
    bigSystemFont(tester);
    Routines(store).create(
      'pills',
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(10, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    await start(tester);
    await openDeviceSettings(tester);
    final settings = find.byType(SettingsScreen);
    expect(tester.widget<Switch>(fatFingers).value, isFalse);
    final small = tester.getRect(fatFingers).size;

    await tap(tester, fatFingers);
    expect(store.fatFingers, isTrue);
    expect(tester.widget<Switch>(fatFingers).value, isTrue);
    expect(textOf(tester, settings), closeTo(10 * 1.5 * 1.3, 0.001));
    expect(iconOf(tester, settings), closeTo(24 * 1.3, 0.001));
    // The switch's target: 48 high, now 1.3 times.
    expect(small.height, 48);
    expect(tester.getRect(fatFingers).height, closeTo(48 * 1.3, 0.001));
    final back = find.byType(BackButton);
    expect(
      tester
          .widget<Icon>(find.descendant(of: back, matching: find.byType(Icon)))
          .size,
      isNull,
    );
    expect(
      tester.getSize(find.descendant(of: back, matching: find.byType(Icon))),
      const Size.square(24 * 1.3),
    );

    await tap(tester, back);
    await tap(tester, find.byType(BackButton));
    final box = tester.getSize(find.byType(AgendaBox).first);
    expect(box.width, closeTo(40 * 1.3, 0.001));
    expect(box.height, closeTo(40 * 1.3, 0.001));

    await openDeviceSettings(tester);
    await tap(tester, fatFingers);
    expect(store.fatFingers, isFalse);
    expect(textOf(tester, settings), 15);
    expect(iconOf(tester, settings), 24);
  });

  testWidgets('the switch hands the widget its text grown at once', (
    tester,
  ) async {
    final alarms = FakeAlarms();
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
    await openDeviceSettings(tester);
    await tap(tester, fatFingers);
    expect(alarms.agendas.last.fatFingers, isTrue);
    await tap(tester, fatFingers);
    expect(alarms.agendas.last.fatFingers, isFalse);
  });

  testWidgets('a new start shows the text grown as kept', (tester) async {
    store.fatFingers = true;
    await start(tester);
    final agenda = find.byType(AgendaScreen);
    expect(textOf(tester, agenda), closeTo(13, 0.001));
    expect(iconOf(tester, agenda), closeTo(24 * 1.3, 0.001));
  });

  group('with Fat fingers on, a phone in German fits', () {
    setUp(() => store.fatFingers = true);

    testWidgets("the Agenda's rows of every kind", (tester) async {
      germanPhone(tester);
      final routines = Routines(store);
      Schedule daily(List<ClockTime> times) => Schedule(
        start: Day(2026, 10, 1),
        times: times,
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      routines.create('Tabletten nehmen', daily([ClockTime(10, 0)]));
      routines.create('Aufräumen', daily([]));
      final trip = notes.capture('Vor der Reise\nReisepass\nTickets').id;
      Checklists(store).make(trip);
      final shopping = notes.capture('Einkauf\nMilch\nEier').id;
      ShoppingLists(store).make(shopping);
      Frogs(store).spawn(notes.capture('Steuererklärung').id);
      notes.capture('unsortiert');
      await start(tester);
      expect(find.byType(AgendaGroup), findsWidgets);
      expectBarClear(tester);
      expect(tester.takeException(), isNull);
      await tester.drag(find.byType(Scrollable).first, const Offset(0, -3000));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });

    testWidgets('the Note editor and its bar', (tester) async {
      germanPhone(tester);
      final note = notes.capture('Soße kaufen');
      notes.amend(note.id, 'Gekauft');
      await start(tester);
      await tester.tap(find.byTooltip('Suchen und stöbern'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Alle Notizen'));
      await tester.pumpAndSettle();
      await tester.longPress(find.text('Soße kaufen'));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expectBarClear(tester);
      // What does not fit goes into ⋮, last, with its icon and words.
      expect(find.byKey(const ValueKey('note-history')), findsNothing);
      await tester.tap(find.byKey(const ValueKey('bar-more')));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('note-history')), findsOneWidget);
      expect(find.text('In den Papierkorb'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('note-history')));
      await tester.pumpAndSettle();
      expect(find.byType(NoteHistoryScreen), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('the capture screen with all of it on, the keyboard up', (
      tester,
    ) async {
      for (final control in QuickNoteControl.values) {
        store.setQuickNoteShows(control, true);
      }
      ShoppingLists(store).make(notes.capture('Einkauf').id);
      germanPhone(tester);
      const keyboard = 300.0;
      await start(tester);
      await openTextCapture(tester);
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: keyboard * 3);
      await tester.pumpAndSettle();
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(find.byType(TwoRowTextField), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('the Settings, all the way down', (tester) async {
      germanPhone(tester);
      await start(tester);
      await openDeviceSettings(tester);
      expect(tester.widget<Switch>(fatFingers).value, isTrue);
      expectBarClear(tester);
      await tester.drag(find.byType(Scrollable).first, const Offset(0, -6000));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });
  });
}

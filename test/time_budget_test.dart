import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/l10n/app_localizations_en.dart';
import 'package:oldbrainfix/src/agenda_reader.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

/// The time budget (#339, Q753): every tap shows its effect within
/// 100 ms, every page change within 500 ms. A test machine's clock says
/// little of a phone's, so these hold what the budget rests on: a
/// notification opens its Note without waiting on the alarms armed
/// anew, a page out of sight reads nothing on a change and reads once
/// as it shows, and a burst of changes arms the alarms once more, not
/// once each.
void main() {
  setUpAll(initializeDateFormatting);

  group('a notification tapped while the app runs', () {
    late Store store;
    var now = DateTime(2026, 10, 24, 9);

    setUp(() {
      now = DateTime(2026, 10, 24, 9);
      store = Store.inMemory(clock: () => now)
        ..starterSetOffered = true
        ..alarmSetupDone = true;
    });
    tearDown(() => store.close());

    testWidgets('marks it on the Agenda within half a second of frames, '
        'while the alarms are still being armed anew', (tester) async {
      now = DateTime(2026, 10, 24, 6);
      final note = Notes(store).capture('Packing\nTent\nPegs').id;
      Checklists(store).make(note);
      final routine = Routines(store).spawn(
        note,
        Schedule(
          start: Day(2026, 10, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      now = DateTime(2026, 10, 24, 9);
      final alarms = _HeldAlarms();
      await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
      await tester.pumpAndSettle();
      // Away on the Areas list, as a tap can come from anywhere.
      await tester.tap(find.byKey(const ValueKey('browse')));
      await tester.pumpAndSettle();
      final marked = find.byKey(const ValueKey('agenda-marked'));

      // Arming hands every alarm to the platform, which takes its time.
      final held = alarms.hold();
      final arms = alarms.arms;
      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      var waited = Duration.zero;
      while (marked.evaluate().isEmpty &&
          waited < const Duration(milliseconds: 500)) {
        await tester.pump(const Duration(milliseconds: 16));
        waited += const Duration(milliseconds: 16);
      }

      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(marked.hitTestable(), findsOneWidget);
      expect(find.text('Tent'), findsWidgets);
      expect(alarms.arms, arms, reason: 'opened before the arming ended');
      held.complete();
      await tester.pumpAndSettle();
      expect(alarms.arms, arms + 1);
    });
  });

  group('a page out of sight', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    Future<NavigatorState> start(WidgetTester tester) async {
      final navigator = GlobalKey<NavigatorState>();
      await tester.pumpWidget(
        MaterialApp(
          navigatorKey: navigator,
          home: _CountingPage(store: store),
        ),
      );
      return navigator.currentState!;
    }

    _CountingPageState page(WidgetTester tester) =>
        tester.state<_CountingPageState>(
          find.byType(_CountingPage, skipOffstage: false),
        );

    testWidgets('reads nothing on a change under another page, and reads '
        'once as it shows again, in the first frame of the way back', (
      tester,
    ) async {
      final navigator = await start(tester);
      Notes(store).capture('one');
      await tester.pump();
      expect(page(tester).reads, 2);
      await tester.pump();
      expect(find.text('Notes: 1'), findsOneWidget);

      unawaited(
        navigator.push(
          MaterialPageRoute<void>(builder: (_) => const Text('on top')),
        ),
      );
      await tester.pumpAndSettle();
      Notes(store).capture('two');
      await tester.pump();
      Notes(store).capture('three');
      await tester.pump();
      expect(page(tester).reads, 2);

      navigator.pop();
      await tester.pump();
      expect(page(tester).reads, 3);
      expect(find.text('Notes: 3'), findsOneWidget);
      await tester.pumpAndSettle();
      expect(page(tester).reads, 3);
    });

    testWidgets('under a dialog it is in sight and reads as before', (
      tester,
    ) async {
      final navigator = await start(tester);
      unawaited(
        showDialog<void>(
          context: navigator.context,
          builder: (_) => const AlertDialog(content: Text('asking')),
        ),
      );
      await tester.pumpAndSettle();
      Notes(store).capture('one');
      await tester.pump();
      expect(page(tester).reads, 2);
      await tester.pump();
      expect(find.text('Notes: 1'), findsOneWidget);
    });
  });

  group('the alarms', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    test('arm once more for every change made while they arm, not once '
        'each', () async {
      final reader = _HeldReader();
      final alarms = AlarmSync(
        store: store,
        platform: FakeAlarms(),
        wallClock: deviceWallClock,
        reader: reader,
      )..setTexts(AppLocalizationsEn(), 'en');
      addTearDown(alarms.dispose);
      await pumpEventQueue();
      expect(reader.plans, 1);

      for (var i = 0; i < 5; i++) {
        Notes(store).capture('Change $i');
        await pumpEventQueue();
      }
      reader.release();
      await pumpEventQueue();

      expect(reader.plans, 2);
    });
  });
}

/// A platform whose arming waits until the test lets it go.
class _HeldAlarms extends FakeAlarms {
  Completer<void>? _held;

  /// Holds every arming until the completer returned completes.
  Completer<void> hold() => _held = Completer<void>();

  @override
  Future<void> arm(
    List<ArmedAlarm> alarms,
    AlarmLabels labels, {
    List<(String, Occurrence)> done = const [],
    List<ArmedHeadsUp> headsUps = const [],
    ArmedNudge? nudge,
    ArmedChangesNudge? changes,
    String? sound,
  }) async {
    await _held?.future;
    return super.arm(
      alarms,
      labels,
      done: done,
      headsUps: headsUps,
      nudge: nudge,
      changes: changes,
      sound: sound,
    );
  }
}

/// Reads at once, but its first plan arrives only when released.
class _HeldReader implements AgendaReader {
  final _inline = const InlineAgendaReader();
  final _first = Completer<void>();
  var plans = 0;

  void release() => _first.complete();

  @override
  AgendaLoad load(Store store, WallClock wallClock, UnitSystem system) =>
      _inline.load(store, wallClock, system);

  @override
  AgendaLoad read(Store store, WallClock wallClock, UnitSystem system) =>
      _inline.read(store, wallClock, system);

  @override
  Future<AlarmPlanRead> plan(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) async {
    plans++;
    if (plans == 1) await _first.future;
    return _inline.plan(store, wallClock, t, locale);
  }
}

/// A page that counts how often it read the log.
class _CountingPage extends StatefulWidget {
  final Store store;

  const _CountingPage({required this.store});

  @override
  State<_CountingPage> createState() => _CountingPageState();
}

class _CountingPageState extends State<_CountingPage> with ReloadsOnLogChange {
  var reads = 0;
  late var _count = _read();

  int _read() {
    reads++;
    return Notes(widget.store).all().length;
  }

  @override
  Store get log => widget.store;

  @override
  void reloadLog() => setState(() => _count = _read());

  @override
  Widget build(BuildContext context) => Scaffold(body: Text('Notes: $_count'));
}

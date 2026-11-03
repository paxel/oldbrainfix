import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/l10n/app_localizations_de.dart';
import 'package:oldbrainfix/l10n/app_localizations_en.dart';
import 'package:oldbrainfix/src/agenda_reader.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/agenda_widget.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/l10n.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/schedule_text.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

/// Reads on the store at once, as [InlineAgendaReader] does, but hands
/// the Agenda's loads over only once the test lets them go, as an
/// isolate's read arrives later.
class _HeldReader implements AgendaReader {
  /// Whether the Agenda's loads wait for [release].
  var holding = false;

  /// Whether a load says it wrote on a connection of its own.
  var wroteElsewhere = false;

  /// Whether the alarms' plans wait for [releasePlan].
  var holdingPlans = false;

  final _held = <(Completer<AgendaLoad>, AgendaLoad)>[];
  final _heldPlans = <(Completer<AlarmPlanRead>, AlarmPlanRead)>[];

  int get waiting => _held.length;
  int get plansWaiting => _heldPlans.length;

  /// Hands the oldest load waiting over.
  void release() {
    final (completer, load) = _held.removeAt(0);
    completer.complete(load);
  }

  void releasePlan() {
    final (completer, plan) = _heldPlans.removeAt(0);
    completer.complete(plan);
  }

  @override
  FutureOr<AgendaLoad> load(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) {
    final read = loadAgenda(store, wallClock, system);
    final load = AgendaLoad(
      days: read.days,
      frogs: read.frogs,
      missed: read.missed,
      unsorted: read.unsorted,
      seq: read.seq,
      wroteElsewhere: wroteElsewhere,
    );
    if (!holding) return load;
    final completer = Completer<AgendaLoad>();
    _held.add((completer, load));
    return completer.future;
  }

  @override
  FutureOr<AgendaLoad> read(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) => readAgenda(store, wallClock, system);

  @override
  FutureOr<AlarmPlanRead> plan(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) {
    final plan = planAlarms(store, wallClock, t, locale);
    if (!holdingPlans) return plan;
    final completer = Completer<AlarmPlanRead>();
    _heldPlans.add((completer, plan));
    return completer.future;
  }
}

/// Half a year of daily use: [routines] daily Routines checked off most
/// days, a Session with its picks and entries, Notes, a Checklist, a
/// Shopping List and Frogs; ending today on the real clock, as the
/// isolate's store reads it.
void _halfYear(Store store, {int routines = 20, int days = 180}) {
  final notes = Notes(store);
  final now = DateTime.now().toUtc();
  final first = DateTime.utc(
    now.year,
    now.month,
    now.day,
  ).subtract(Duration(days: days));
  Day dayAt(int k) {
    final d = first.add(Duration(days: k));
    return Day(d.year, d.month, d.day);
  }

  final daily = Schedule(
    start: dayAt(0),
    times: [ClockTime(8, 0)],
    rule: Recurrence(frequency: Frequency.daily),
  );
  store.batch(() {
    for (var i = 0; i < 100; i++) {
      notes.capture('Thought $i\nline two');
    }
    Checklists(store).make(notes.capture('Pack\nsocks\nshirt').id);
    ShoppingLists(store).make(notes.capture('Shop\nmilk\neggs').id);
    for (var i = 0; i < 3; i++) {
      Frogs(store).spawn(notes.capture('Frog $i').id);
    }
    final pool = [
      for (var i = 0; i < 4; i++)
        () {
          final id = notes.capture('Exercise $i').id;
          Tallies(store).addField(id, name: 'Times', unit: 'times');
          return id;
        }(),
    ];
    Sessions(store).spawn(
      notes.capture('Training').id,
      daily,
      pool: pool,
      count: 2,
      rule: SessionRule.rolling,
    );
    final ids = [
      for (var i = 0; i < routines; i++)
        Routines(store).create('Routine $i', daily).id,
    ];
    for (var k = 0; k < days - 1; k++) {
      final occurrence = Occurrence(dayAt(k), ClockTime(8, 0));
      for (final (j, id) in ids.indexed) {
        if ((k + j) % 5 != 0) Routines(store).markDone(id, occurrence);
      }
    }
  });
}

void main() {
  setUpAll(initializeDateFormatting);

  group('the isolate\'s reader (#207)', () {
    late Directory dir;
    late String path;
    late Store store;

    setUp(() {
      dir = Directory.systemTemp.createTempSync('agenda_reader_test_');
      path = '${dir.path}/oldbrainfix.db';
      store = Store.open(path)..setLocalSetting('author', 'Ann');
    });
    tearDown(() {
      store.close();
      dir.deleteSync(recursive: true);
    });

    /// The words the widget would show of [load], all its rows in them.
    String shown(AgendaLoad load) => jsonEncode(
      AgendaSnapshot.from(
        store,
        AgendaRead.of(load, deviceWallClock),
        AppLocalizationsEn(),
        'en',
      ).toMap(),
    );

    test('loads the Agenda in an isolate of its own while this one keeps '
        'running, and hands back the rows a load here reads', () async {
      _halfYear(store);
      // This isolate's event loop, as the app's frames ride on it.
      var longest = Duration.zero;
      var ticks = 0;
      final gap = Stopwatch()..start();
      final ticker = Timer.periodic(const Duration(milliseconds: 2), (_) {
        if (gap.elapsed > longest) longest = gap.elapsed;
        ticks++;
        gap.reset();
      });
      final took = Stopwatch()..start();
      final load = await IsolateAgendaReader(path)
          .load(store, deviceWallClock, UnitSystem.metric);
      took.stop();
      ticker.cancel();
      if (gap.elapsed > longest) longest = gap.elapsed;
      // Today's picks were made there, on its connection.
      expect(load.wroteElsewhere, isTrue);
      store.changedElsewhere();
      expect(load.seq, store.currentSeq());
      final here = loadAgenda(store, deviceWallClock, UnitSystem.metric);
      expect(here.seq, load.seq, reason: 'nothing more written');
      expect(shown(load), shown(here));
      expect(load.days.todayRows, hasLength(23));
      // Drawing goes on: no stall here near the time the read took.
      expect(took.elapsed, greaterThan(const Duration(milliseconds: 200)));
      expect(longest, lessThan(took.elapsed ~/ 3));
      expect(ticks, greaterThan(10));
    });

    test('reads for the widget there writing nothing, and plans the '
        'alarms with the texts a plan here has, in the reader\'s '
        'language', () async {
      _halfYear(store, routines: 3, days: 20);
      final reader = IsolateAgendaReader(path);
      final seq = store.currentSeq();
      final read = await reader.read(store, deviceWallClock, UnitSystem.metric);
      expect(read.wroteElsewhere, isFalse);
      expect(store.currentSeq(), seq);
      expect(
        shown(read),
        shown(readAgenda(store, deviceWallClock, UnitSystem.metric)),
      );
      for (final (t, locale) in [
        (AppLocalizationsEn(), 'en'),
        (AppLocalizationsDe(), 'de'),
      ]) {
        final there = await reader.plan(store, deviceWallClock, t, locale);
        final here = planAlarms(store, deviceWallClock, t, locale);
        String words(AlarmPlanRead plan) => [
          for (final a in plan.armed) '${a.alarm}|${a.title}|${a.body}',
          for (final d in plan.done) '$d',
          '${plan.nudge?.title} ${plan.escalating} ${plan.seq}',
        ].join('\n');
        expect(there.armed, isNotEmpty);
        expect(words(there), words(here));
      }
    });
  });

  group('the Agenda while it reads (#207)', () {
    late Store store;
    late Routines routines;
    late _HeldReader reader;
    var now = DateTime(2026, 10, 24, 9);

    setUp(() {
      now = DateTime(2026, 10, 24, 9);
      store = Store.inMemory(clock: () => now)..starterSetOffered = true;
      routines = Routines(store);
      reader = _HeldReader();
    });
    tearDown(() => store.close());

    Schedule daily() => Schedule(
      start: Day(2026, 10, 1),
      times: [ClockTime(20, 0)],
      rule: Recurrence.parse('FREQ=DAILY'),
    );

    Future<void> start(WidgetTester tester) async {
      await tester.pumpWidget(
        OldBrainFixApp(store: store, agendaReader: reader),
      );
      await tester.pumpAndSettle();
    }

    const nothingToday = 'Nothing due today. Tap + to capture a thought.';

    testWidgets('shows the last rows until the new read is in', (tester) async {
      routines.create('pills', daily());
      await start(tester);
      expect(find.text('pills'), findsWidgets);

      reader.holding = true;
      routines.create('tea', daily());
      await tester.pumpAndSettle();
      expect(reader.waiting, 1);
      expect(find.text('pills'), findsWidgets);
      expect(find.text('tea'), findsNothing);

      reader.release();
      await tester.pumpAndSettle();
      expect(find.text('pills'), findsWidgets);
      expect(find.text('tea'), findsWidgets);
    });

    testWidgets('shows no empty day before its first read is in', (
      tester,
    ) async {
      routines.create('pills', daily());
      reader.holding = true;
      await start(tester);
      expect(find.text(nothingToday), findsNothing);
      expect(find.text('Today'), findsNothing);
      expect(find.text('pills'), findsNothing);

      reader.release();
      await tester.pumpAndSettle();
      expect(find.text('Today'), findsWidgets);
      expect(find.text('pills'), findsWidgets);
    });

    testWidgets('drops a read the log has changed since, and reads once '
        'more', (tester) async {
      routines.create('pills', daily());
      await start(tester);

      reader.holding = true;
      routines.create('tea', daily());
      await tester.pumpAndSettle();
      expect(reader.waiting, 1);
      // Changed while that read runs: one more read waits for it.
      routines.create('water', daily());
      await tester.pumpAndSettle();
      expect(reader.waiting, 1);
      AgendaScreen.loads = 0;

      reader.release();
      await tester.pumpAndSettle();
      expect(find.text('tea'), findsNothing, reason: 'read before "water"');
      expect(AgendaScreen.loads, 1);
      expect(reader.waiting, 1);

      reader.release();
      await tester.pumpAndSettle();
      expect(find.text('tea'), findsWidgets);
      expect(find.text('water'), findsWidgets);
      expect(AgendaScreen.loads, 1);
    });

    testWidgets('a box tapped shows at once, and a second tap undoes it, '
        'while the Agenda reads again', (tester) async {
      final pills = routines.create('pills', daily());
      await start(tester);
      bool box() => tester.widget<Checkbox>(find.byType(Checkbox).first).value!;
      final today = Occurrence(Day(2026, 10, 24), ClockTime(20, 0));
      expect(box(), isFalse);

      reader.holding = true;
      await tester.tap(find.byType(Checkbox).first);
      await tester.pump();
      expect(reader.waiting, 1);
      expect(box(), isTrue);
      expect(routines.byId(pills.id)!.doneAt(today), isNotNull);

      await tester.tap(find.byType(Checkbox).first);
      await tester.pump();
      expect(box(), isFalse);
      expect(routines.byId(pills.id)!.doneAt(today), isNull);

      reader
        ..release()
        ..holding = false;
      await tester.pumpAndSettle();
      expect(box(), isFalse);
    });

    testWidgets('picks written on a connection of its own are no reason to '
        'read again (#205)', (tester) async {
      routines.create('pills', daily());
      await start(tester);
      reader
        ..holding = true
        ..wroteElsewhere = true;
      var told = 0;
      store.addListener(() => told++);
      AgendaScreen.loads = 0;

      routines.create('tea', daily());
      await tester.pumpAndSettle();
      final changes = told;
      reader.release();
      await tester.pumpAndSettle();
      expect(told, changes + 1, reason: 'the picks written there');
      expect(find.text('tea'), findsWidgets);
      expect(AgendaScreen.loads, 1);
      expect(reader.waiting, 0);
    });
  });

  group('the alarms while their plan is read (#207)', () {
    testWidgets('arm no plan the log has changed since, and arm the one '
        'read after', (tester) async {
      final now = DateTime(2026, 10, 24, 9);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      final platform = FakeAlarms();
      final reader = _HeldReader();
      final sync = AlarmSync(
        store: store,
        platform: platform,
        wallClock: deviceWallClock,
        reader: reader,
      );
      addTearDown(sync.dispose);
      Schedule daily() => Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(20, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      Routines(store).create('pills', daily());
      sync.setTexts(AppLocalizationsEn(), 'en');
      await tester.pump(AlarmSync.showDelay);
      expect(platform.armed.map((a) => a.title).toSet(), {'pills'});
      final arms = platform.arms;

      reader.holdingPlans = true;
      Routines(store).create('tea', daily());
      await tester.pump();
      expect(reader.plansWaiting, 1);
      Routines(store).create('water', daily());
      reader.releasePlan();
      await tester.pump();
      expect(platform.arms, arms, reason: 'read before "water"');

      expect(reader.plansWaiting, 1);
      reader.releasePlan();
      await tester.pump(AlarmSync.showDelay);
      expect(platform.arms, arms + 1);
      expect(platform.armed.map((a) => a.title).toSet(), {
        'pills',
        'tea',
        'water',
      });
    });
  });

  group('the alarms\' plan (#228)', () {
    late Store store;
    final now = DateTime(2026, 10, 24, 9);

    setUp(() {
      store = Store.inMemory(clock: () => now)..setPin('1234', iterations: 10);
    });
    tearDown(() => store.close());

    Schedule daily(List<int> hours) => Schedule(
      start: Day(2026, 10, 1),
      times: [for (final h in hours) ClockTime(h, 0)],
      rule: Recurrence.parse('FREQ=DAILY'),
    );

    /// A locked Masked Area with a Routine and a Reminder, an open one,
    /// a Note merged into another, a snooze, a check-off yesterday.
    void seed() {
      final notes = Notes(store);
      final places = Places(store);
      final routines = Routines(store);
      final doctor = places.addArea('Doctor');
      final home = places.addArea('Home');
      final pills = routines.spawn(
        notes.capture('Pills\nwith water', place: doctor.id).id,
        daily([8, 12, 20]),
      );
      routines.spawn(
        notes.capture('Call the clinic', place: doctor.id).id,
        Schedule(start: Day(2026, 10, 26), times: [ClockTime(10, 0)]),
      );
      store
        ..setAreaLocked(doctor.id, true)
        ..setAreaMasked(doctor.id, true);
      routines.spawn(
        notes.capture('Water plants', place: home.id).id,
        daily([7, 19]),
      );
      final tea = routines.create('Tea', daily([7]));
      final old = notes.capture('Old walk');
      routines.spawn(old.id, daily([17]));
      notes.merge(old.id, notes.capture('Walk\nin the park').id);
      routines.markDone(tea.id, Occurrence(Day(2026, 10, 23), ClockTime(7, 0)));
      Alarms(store).applySnooze(
        pills.id,
        '2026-10-24T12:00',
        DateTime(2026, 10, 24, 12, 10),
      );
    }

    test('arms the alarms and texts a read of each Routine per alarm '
        'gave, in English and German', () {
      seed();
      for (final (t, locale) in [
        (AppLocalizationsEn(), 'en'),
        (AppLocalizationsDe(), 'de'),
      ]) {
        // As the plan read before #228: each alarm's Routine on its own.
        String title(PlannedAlarm alarm) {
          final routine = Routines(store).byId(alarm.routineId);
          if (routine != null && AreaLocks(store).masks(routine)) {
            return maskedText(t, locale, routine, alarm.occurrence);
          }
          final line = alarm.text?.trim().split('\n').first.trim();
          if (line != null && line.isNotEmpty) return line;
          return alarm.text == null
              ? t.noteGone
              : wordlessTitle(t, alarm.wordless);
        }

        final before = [
          for (final a in Alarms(store).plan(deviceWallClock))
            '$a|${title(a)}|${occurrenceTime(t, locale, a.occurrence)}',
        ];
        final plan = planAlarms(store, deviceWallClock, t, locale);
        expect([
          for (final a in plan.armed) '${a.alarm}|${a.title}|${a.body}',
        ], before);
        expect(plan.done, Alarms(store).recentlyDone(deviceWallClock));
        expect(plan.done, isNotEmpty);
        expect(
          plan.escalating,
          Routines(store).all().any((r) => r.escalation != null),
        );
        final titles = {for (final a in plan.armed) a.title};
        expect(
          titles,
          containsAll([
            t.maskedRoutineAt(
              occurrenceTime(
                t,
                locale,
                Occurrence(Day(2026, 10, 24), ClockTime(12, 0)),
              ),
            ),
            t.maskedReminder,
            'Water plants',
            'Tea',
            'Walk',
          ]),
        );
        expect(titles, isNot(contains('Pills')));
        expect(
          plan.armed.where((a) => a.alarm.kind == AlarmKind.snooze),
          isNotEmpty,
        );
      }
    });

    test('reads each Routine once per plan, not once per alarm', () {
      seed();
      final reads = store.historyReads;
      final plan = planAlarms(
        store,
        deviceWallClock,
        AppLocalizationsEn(),
        'en',
      );
      final read = store.historyReads - reads;
      expect(plan.armed.length, greaterThan(150));
      expect(read, lessThan(plan.armed.length ~/ 2), reason: '$read reads');
    });
  });
}

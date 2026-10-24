import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Central European time in 2026: summer time (+2) from 29 March 01:00
/// UTC to 25 October 01:00 UTC, winter time (+1) around it.
DateTime europe(DateTime utc) {
  final summer =
      !utc.isBefore(DateTime.utc(2026, 3, 29, 1)) &&
      utc.isBefore(DateTime.utc(2026, 10, 25, 1));
  final wall = utc.add(Duration(hours: summer ? 2 : 1));
  return DateTime.utc(
    wall.year,
    wall.month,
    wall.day,
    wall.hour,
    wall.minute,
    wall.second,
    wall.millisecond,
  );
}

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

const min10 = Duration(minutes: 10);
const min30 = Duration(minutes: 30);

void main() {
  late DateTime now;
  late Store store;
  late Notes notes;
  late Alarms alarms;

  setUp(() {
    // 09:00 on Saturday, 24 October 2026, in Europe (summer time).
    now = DateTime.utc(2026, 10, 24, 7);
    store = Store.inMemory(clock: () => now);
    notes = Notes(store);
    alarms = Alarms(store);
  });
  tearDown(() => store.close());

  /// The instant [hour]:[minute] shows on the European wall clock on
  /// [day] of October 2026.
  DateTime local(int day, int hour, [int minute = 0]) =>
      instantOf(Day(2026, 10, day), at(hour, minute), europe);

  /// The Nudge's firings after now, as wall-clock times.
  List<String> firings(int count) {
    final nudge = alarms.nudge(europe);
    if (nudge == null) return const [];
    final out = <String>[];
    var after = now;
    for (var i = 0; i < count; i++) {
      after = nudge.firesAfter(after, europe);
      final wall = europe(after);
      out.add('${wall.day}. ${at(wall.hour, wall.minute)}');
    }
    return out;
  }

  group('a Note and its Nudge time', () {
    test('keeps the time picked at Capture, in the log', () {
      final note = notes.capture('nees mor sauce', nudgeIn: min10);
      expect(note.nudge, local(24, 9, 10));
      expect(note.nudgeAt, local(24, 9, 10));
      expect(
        store.history(note.id).map((e) => e.field),
        contains(NoteKeys.nudge),
        reason: 'part of the Note, so every Device nudges for it',
      );
    });

    test('its Nudge time can be picked again later', () {
      final note = notes.capture('nees mor sauce');
      now = local(24, 9, 5);
      notes.setNudge(note.id, min10);
      expect(notes.byId(note.id)?.nudgeAt, local(24, 9, 15));
    });

    test('without a pick is due an hour after Capture', () {
      final note = notes.capture('call the plumber');
      expect(note.nudge, isNull);
      expect(note.nudgeAt, local(24, 10));
    });

    test('the Unsorted Notes are listed oldest first', () {
      final area = Places(store).addArea('Household');
      final first = notes.capture('first');
      now = now.add(const Duration(minutes: 1));
      final filed = notes.capture('filed', place: area.id);
      now = now.add(const Duration(minutes: 1));
      final trashed = notes.capture('trashed');
      notes.trash(trashed.id);
      now = now.add(const Duration(minutes: 1));
      final last = notes.capture('last');
      expect([for (final n in notes.unsorted()) n.id], [first.id, last.id]);
      expect(notes.unsorted().map((n) => n.id), isNot(contains(filed.id)));
    });
  });

  group('the Nudge settings of this Device', () {
    test('Quiet Hours are 22:00 to 08:00 and on until changed', () {
      expect(store.quietHours, QuietHours(at(22), at(8)));
      expect(store.quietHoursOn, isTrue);
      expect(store.nudgeRepeat, const Duration(hours: 1));
    });

    test('are kept per Device, never logged', () {
      store.quietHours = QuietHours(at(23, 30), at(6, 45));
      store.quietHoursOn = false;
      store.nudgeRepeat = min30;
      expect(store.quietHours, QuietHours(at(23, 30), at(6, 45)));
      expect(store.quietHoursOn, isFalse);
      expect(store.nudgeRepeat, min30);
      store.quietHoursOn = true;
      expect(
        store.quietHours,
        QuietHours(at(23, 30), at(6, 45)),
        reason: 'switching them off keeps the times',
      );
      expect(store.allEntries(), isEmpty);
    });

    test('anything unreadable reads as the default', () {
      store.setLocalSetting('quietHours', 'late-early');
      store.setLocalSetting('nudgeRepeatMinutes', 'often');
      expect(store.quietHours, QuietHours(at(22), at(8)));
      expect(store.nudgeRepeat, const Duration(hours: 1));
      expect(() => store.nudgeRepeat = Duration.zero, throwsArgumentError);
    });
  });

  group('Quiet Hours', () {
    test('run over midnight when they start later than they end', () {
      final quiet = QuietHours(at(22), at(8));
      expect(quiet.contains(at(22)), isTrue);
      expect(quiet.contains(at(3)), isTrue);
      expect(quiet.contains(at(7, 59)), isTrue);
      expect(quiet.contains(at(8)), isFalse);
      expect(quiet.contains(at(21, 59)), isFalse);
      expect(quiet.contains(at(12)), isFalse);
    });

    test('a window within a day, and an empty one', () {
      final noon = QuietHours(at(13), at(15));
      expect(noon.contains(at(13)), isTrue);
      expect(noon.contains(at(14, 59)), isTrue);
      expect(noon.contains(at(15)), isFalse);
      expect(noon.contains(at(23)), isFalse);
      expect(QuietHours(at(8), at(8)).contains(at(8)), isFalse);
      expect(noon.defer(local(24, 13, 30), europe), local(24, 15));
      expect(noon.defer(local(24, 16), europe), local(24, 16));
    });

    test('before midnight wait until the next morning, after it until '
        'the same one', () {
      final quiet = QuietHours(at(22), at(8));
      expect(quiet.defer(local(23, 22, 30), europe), local(24, 8));
      expect(quiet.defer(local(24, 3), europe), local(24, 8));
    });

    test('end at the same wall time across the switches to and from '
        'summer time', () {
      final quiet = QuietHours(at(22), at(8));
      // 25 October: the clock goes from 03:00 back to 02:00.
      expect(quiet.defer(local(24, 23), europe), DateTime.utc(2026, 10, 25, 7));
      // 29 March: the clock jumps from 02:00 to 03:00.
      expect(
        quiet.defer(instantOf(Day(2026, 3, 28), at(23), europe), europe),
        DateTime.utc(2026, 3, 29, 6),
      );
      // An end the clock skips comes when it jumps past it.
      expect(
        QuietHours(
          at(1),
          at(2, 30),
        ).defer(instantOf(Day(2026, 3, 29), at(1, 30), europe), europe),
        DateTime.utc(2026, 3, 29, 1),
      );
    });
  });

  group('the Nudge', () {
    test('is not there while nothing is Unsorted', () {
      expect(alarms.nudge(europe), isNull);
      final area = Places(store).addArea('Household');
      notes.capture('filed', place: area.id);
      expect(alarms.nudge(europe), isNull);
    });

    test('fires once at the earliest own time and covers every Unsorted '
        'Note', () {
      notes.capture('in an hour');
      now = now.add(const Duration(minutes: 5));
      notes.capture('in 10 min', nudgeIn: min10);
      now = now.add(const Duration(minutes: 1));
      notes.capture('in half an hour', nudgeIn: min30);
      final nudge = alarms.nudge(europe);
      expect(nudge?.at, local(24, 9, 15));
      expect(nudge?.count, 3);
      expect(nudge?.notes.first.text, 'in an hour');
      // Covered at 09:15: the others' own times bring no second Nudge.
      expect(firings(3), ['24. 09:15', '24. 10:15', '24. 11:15']);
    });

    test('a Note captured after it fired brings the next one forward', () {
      notes.capture('first', nudgeIn: min10);
      now = local(24, 9, 30);
      expect(firings(1), ['24. 10:10']);
      notes.capture('second', nudgeIn: min10);
      expect(firings(3), ['24. 09:40', '24. 10:40', '24. 11:40']);
      // A later own time than the next repeat changes nothing.
      notes.capture('third');
      expect(firings(1), ['24. 09:40']);
    });

    test('repeats while anything is Unsorted, at the Device interval', () {
      final note = notes.capture('first', nudgeIn: min10);
      store.nudgeRepeat = min30;
      expect(firings(3), ['24. 09:10', '24. 09:40', '24. 10:10']);
      now = local(24, 15, 5);
      expect(firings(1), ['24. 15:10']);
      expect(alarms.nudge(europe)?.since, local(24, 14, 40));
      Places(store).addArea('Household');
      notes.file(note.id, Places(store).areas().single.id);
      expect(alarms.nudge(europe), isNull);
    });

    test('a snooze holds back the Notes there were and repeats from its '
        'end', () {
      notes.capture('first', nudgeIn: min10);
      now = local(24, 9, 11);
      notes.capture('second', nudgeIn: min30);
      now = local(24, 9, 12);
      alarms.applyNudgeSnooze(now, local(24, 10, 12));
      expect(alarms.nudgeSnooze, (at: now, until: local(24, 10, 12)));
      // "second" would have been due at 09:41.
      expect(firings(2), ['24. 10:12', '24. 11:12']);
      expect(
        store.allEntries().where((e) => e.field == 'nudgeSnooze'),
        isEmpty,
      );
    });

    test('a Note captured during a snooze keeps its own time', () {
      notes.capture('first', nudgeIn: min10);
      now = local(24, 9, 12);
      alarms.applyNudgeSnooze(now, local(24, 9, 42));
      now = local(24, 9, 20);
      notes.capture('new thought', nudgeIn: min10);
      expect(firings(3), ['24. 09:30', '24. 09:42', '24. 10:42']);
    });

    test('waits out Quiet Hours and repeats from their end, across the '
        'switch from summer time', () {
      now = local(24, 21, 55);
      notes.capture('late thought', nudgeIn: min10);
      // 25 October 08:00 is 07:00 UTC, after the clock went back.
      expect(alarms.nudge(europe)?.at, DateTime.utc(2026, 10, 25, 7));
      expect(firings(2), ['25. 08:00', '25. 09:00']);
    });

    test('waits out Quiet Hours across the switch to summer time', () {
      now = instantOf(Day(2026, 3, 28), at(21, 30), europe);
      notes.capture('late thought');
      expect(alarms.nudge(europe)?.at, DateTime.utc(2026, 3, 29, 6));
    });

    test('a snooze ending in Quiet Hours waits for their end', () {
      now = local(24, 21, 0);
      notes.capture('evening', nudgeIn: min10);
      now = local(24, 21, 15);
      alarms.applyNudgeSnooze(now, local(24, 22, 15));
      expect(firings(1), ['25. 08:00']);
    });

    test('fires through the night with Quiet Hours off', () {
      store.quietHoursOn = false;
      now = local(24, 21, 55);
      notes.capture('late thought', nudgeIn: min10);
      expect(firings(2), ['24. 22:05', '24. 23:05']);
    });

    test('Quiet Hours never hold back Routines and Reminders', () {
      final pills = Routines(store)
          .create('pills', Schedule(start: Day(2026, 10, 24), times: [at(23)]));
      notes.capture('late thought');
      expect(store.quietHoursOn, isTrue);
      final alarm = alarms
          .plan(europe)
          .singleWhere((a) => a.routineId == pills.id);
      expect(alarm.at, local(24, 23));
    });

    test('later firings follow the same rule from where it left off', () {
      notes.capture('first', nudgeIn: min10);
      now = local(24, 9, 20);
      notes.capture('second', nudgeIn: min30);
      now = local(24, 9, 30);
      alarms.applyNudgeSnooze(now, local(24, 10, 0));
      store.nudgeRepeat = const Duration(minutes: 45);
      final planned = alarms.nudge(europe);
      expect(planned, isNotNull);
      var after = now;
      for (var i = 0; i < 40; i++) {
        final next = planned?.firesAfter(after, europe);
        now = after;
        expect(next, alarms.nudge(europe)?.at, reason: 'after $after');
        after = after.add(const Duration(minutes: 37));
      }
    });

    test('a Note Unsorted for a year still plans at once', () {
      now = DateTime.utc(2025, 10, 24, 7);
      notes.capture('old');
      store.nudgeRepeat = min10;
      now = DateTime.utc(2026, 10, 24, 7);
      final watch = Stopwatch()..start();
      expect(alarms.nudge(europe), isNotNull);
      expect(watch.elapsed, lessThan(const Duration(seconds: 2)));
    });
  });
}

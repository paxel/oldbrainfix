import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Central European time, as a fixed zone for tests: summer time (+2)
/// ends on 2026-10-25 at 01:00 UTC, winter time (+1) after it.
DateTime europe(DateTime utc) => utc.add(
  Duration(hours: utc.isBefore(DateTime.utc(2026, 10, 25, 1)) ? 2 : 1),
);

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

/// The keys of [schedule]'s Occurrences from [from] to [to].
List<String> keys(Schedule schedule, Day from, Day to) => [
  for (final o in schedule.between(from, to)) o.key,
];

/// The days of [rule] from [start] up to [to], as keys.
List<String> ruleDays(String rule, Day start, Day to) => [
  for (final o in Schedule(
    start: start,
    rule: Recurrence.parse(rule),
  ).between(start, to))
    o.key,
];

void main() {
  group('wall clock', () {
    test('today is the day the Device\'s wall clock shows', () {
      expect(
        Day.today(DateTime.utc(2026, 10, 24, 21, 59), europe),
        Day(2026, 10, 24),
      );
      expect(
        Day.today(DateTime.utc(2026, 10, 24, 22, 30), europe),
        Day(2026, 10, 25),
      );
      // After the switch to winter time midnight is an hour later in UTC.
      expect(
        Day.today(DateTime.utc(2026, 10, 25, 22, 30), europe),
        Day(2026, 10, 25),
      );
      expect(
        Day.today(DateTime.utc(2026, 10, 25, 23, 30), europe),
        Day(2026, 10, 26),
      );
    });

    test('an 08:00 Routine stays at 08:00 across the DST switch', () {
      final daily = Schedule(
        start: Day(2026, 10, 20),
        times: [at(8)],
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      expect(keys(daily, Day(2026, 10, 24), Day(2026, 10, 26)), [
        '2026-10-24T08:00',
        '2026-10-25T08:00',
        '2026-10-26T08:00',
      ]);
      expect(keys(daily, Day(2026, 3, 28), Day(2026, 3, 30)), isEmpty);
      final spring = Schedule(
        start: Day(2026, 3, 1),
        times: [at(2, 30)],
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      expect(keys(spring, Day(2026, 3, 29), Day(2026, 3, 29)), [
        '2026-03-29T02:30',
      ]);
    });

    test('days step by the calendar over month and year ends', () {
      expect(Day(2026, 12, 31).addDays(1), Day(2027, 1, 1));
      expect(Day(2028, 2, 28).addDays(1), Day(2028, 2, 29));
      expect(Day(2026, 10, 24).daysUntil(Day(2026, 10, 26)), 2);
      expect(Day.tryParse('2026-02-30'), isNull);
      expect(Day.tryParse('2026-02-28'), Day(2026, 2, 28));
      expect(
        Occurrence.tryParse('2026-10-25T08:00'),
        Occurrence(Day(2026, 10, 25), at(8)),
      );
      expect(Occurrence.tryParse('2026-10-25'), Occurrence(Day(2026, 10, 25)));
      expect(Occurrence.tryParse('2026-10-25T8'), isNull);
    });
  });

  group('simple schedules', () {
    final start = Day(2026, 10, 24); // a Saturday

    test('daily, at several times a day, each its own Occurrence', () {
      final pills = Schedule(
        start: start,
        times: [at(20), at(8), at(14), at(8)],
        rule: const SimpleRepeat.every(1, RepeatUnit.days).toRule(),
      );
      expect(pills.times, [at(8), at(14), at(20)]);
      expect(keys(pills, Day(2026, 10, 25), Day(2026, 10, 25)), [
        '2026-10-25T08:00',
        '2026-10-25T14:00',
        '2026-10-25T20:00',
      ]);
    });

    test('nothing before the start', () {
      final daily = Schedule(
        start: start,
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      expect(keys(daily, Day(2026, 10, 22), Day(2026, 10, 25)), [
        '2026-10-24',
        '2026-10-25',
      ]);
    });

    test('every N days, weeks, months and years', () {
      Schedule every(int n, RepeatUnit unit, Day from) =>
          Schedule(start: from, rule: SimpleRepeat.every(n, unit).toRule());
      expect(keys(every(3, RepeatUnit.days, start), start, Day(2026, 11, 2)), [
        '2026-10-24',
        '2026-10-27',
        '2026-10-30',
        '2026-11-02',
      ]);
      expect(
        keys(every(2, RepeatUnit.weeks, start), start, Day(2026, 11, 21)),
        ['2026-10-24', '2026-11-07', '2026-11-21'],
      );
      expect(
        keys(
          every(1, RepeatUnit.months, Day(2026, 1, 31)),
          Day(2026, 1, 1),
          Day(2026, 5, 31),
        ),
        ['2026-01-31', '2026-03-31', '2026-05-31'],
        reason: 'a month without the 31st is skipped, as RFC 5545 says',
      );
      expect(
        keys(
          every(1, RepeatUnit.years, Day(2028, 2, 29)),
          Day(2028, 1, 1),
          Day(2033, 1, 1),
        ),
        ['2028-02-29', '2032-02-29'],
      );
    });

    test('chosen weekdays', () {
      final gym = Schedule(
        start: start,
        times: [at(18)],
        rule: const SimpleRepeat.weekdays({
          DateTime.friday,
          DateTime.monday,
          DateTime.wednesday,
        }).toRule(),
      );
      expect(gym.rule?.format(), 'FREQ=WEEKLY;BYDAY=MO,WE,FR');
      expect(keys(gym, start, Day(2026, 10, 31)), [
        '2026-10-26T18:00',
        '2026-10-28T18:00',
        '2026-10-30T18:00',
      ]);
    });

    test('once is one Occurrence on its day; it holds at most one time', () {
      final dentist = Schedule(start: Day(2026, 10, 27), times: [at(10)]);
      expect(dentist.once, isTrue);
      expect(keys(dentist, Day(2026, 1, 1), Day(2027, 1, 1)), [
        '2026-10-27T10:00',
      ]);
      expect(keys(dentist, Day(2026, 10, 28), Day(2027, 1, 1)), isEmpty);
      expect(Schedule(start: start).on(start).single.allDay, isTrue);
      expect(
        () => Schedule(start: start, times: [at(8), at(9)]),
        throwsArgumentError,
      );
    });

    test('the simple editor reads back what it wrote', () {
      final newYear = Day(2026, 12, 31);
      for (final (repeat, rule) in [
        (const SimpleRepeat.once(), null),
        (const SimpleRepeat.every(1, RepeatUnit.days), 'FREQ=DAILY'),
        (const SimpleRepeat.every(3, RepeatUnit.days), 'FREQ=DAILY;INTERVAL=3'),
        (
          const SimpleRepeat.every(2, RepeatUnit.weeks),
          'FREQ=WEEKLY;INTERVAL=2',
        ),
        (
          const SimpleRepeat.every(6, RepeatUnit.months),
          'FREQ=MONTHLY;INTERVAL=6',
        ),
        (const SimpleRepeat.every(1, RepeatUnit.years), 'FREQ=YEARLY'),
        (
          const SimpleRepeat.weekdays({DateTime.tuesday, DateTime.sunday}),
          'FREQ=WEEKLY;BYDAY=TU,SU',
        ),
        (const SimpleRepeat.monthDay(15), 'FREQ=MONTHLY;BYMONTHDAY=15'),
        (
          const SimpleRepeat.monthDay(31, every: 3),
          'FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=31',
        ),
        (
          const SimpleRepeat.monthWeekday(WeekdayRule(DateTime.tuesday, 2)),
          'FREQ=MONTHLY;BYDAY=2TU',
        ),
        (
          const SimpleRepeat.monthWeekday(
            WeekdayRule(DateTime.friday, -1),
            every: 2,
          ),
          'FREQ=MONTHLY;INTERVAL=2;BYDAY=-1FR',
        ),
        (
          const SimpleRepeat.every(1, RepeatUnit.days, count: 10),
          'FREQ=DAILY;COUNT=10',
        ),
        (
          SimpleRepeat.weekdays(const {DateTime.monday}, until: newYear),
          'FREQ=WEEKLY;UNTIL=20261231;BYDAY=MO',
        ),
        (
          const SimpleRepeat.monthDay(1, count: 12),
          'FREQ=MONTHLY;COUNT=12;BYMONTHDAY=1',
        ),
        (
          SimpleRepeat.monthWeekday(
            const WeekdayRule(DateTime.thursday, 4),
            until: newYear,
          ),
          'FREQ=MONTHLY;UNTIL=20261231;BYDAY=4TH',
        ),
      ]) {
        final text = repeat.toRule()?.format();
        expect(text, rule);
        final back = SimpleRepeat.of(
          text == null ? null : Recurrence.parse(text),
          start,
        );
        expect(back, repeat, reason: '$text');
        expect(back?.toRule()?.format(), text);
      }
    });

    test('its ends: never, on a day, after a number', () {
      expect(const SimpleRepeat.monthDay(3).end, RepeatEnd.never);
      expect(
        SimpleRepeat.every(1, RepeatUnit.days, until: start).end,
        RepeatEnd.until,
      );
      expect(
        const SimpleRepeat.every(1, RepeatUnit.days, count: 2).end,
        RepeatEnd.count,
      );
    });

    test('its monthly rules fall where they say', () {
      final schedule = Schedule(
        start: Day(2026, 10, 1),
        rule: SimpleRepeat.monthWeekday(
          const WeekdayRule(DateTime.friday, -1),
          until: Day(2026, 12, 25),
        ).toRule(),
      );
      expect(keys(schedule, Day(2026, 10, 1), Day(2027, 3, 1)), [
        '2026-10-30',
        '2026-11-27',
        '2026-12-25',
      ]);
      final paid = Schedule(
        start: Day(2026, 10, 1),
        rule: const SimpleRepeat.monthDay(15, count: 2).toRule(),
      );
      expect(keys(paid, Day(2026, 10, 1), Day(2027, 3, 1)), [
        '2026-10-15',
        '2026-11-15',
      ]);
    });

    test('rules the simple editor cannot show need the advanced mode', () {
      for (final text in [
        'FREQ=MONTHLY;BYDAY=1MO,-1FR',
        'FREQ=MONTHLY;BYDAY=5MO',
        'FREQ=MONTHLY;BYDAY=-2MO',
        'FREQ=YEARLY;BYDAY=1MO',
        'FREQ=MONTHLY;BYMONTHDAY=1,15',
        'FREQ=MONTHLY;BYMONTHDAY=13;BYDAY=FR',
        'FREQ=YEARLY;BYMONTHDAY=1',
        'FREQ=DAILY;UNTIL=20261231T120000',
        'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH',
        'FREQ=YEARLY;BYMONTH=3',
        'FREQ=DAILY;BYHOUR=8,20',
        'FREQ=MONTHLY;BYMONTHDAY=-1',
      ]) {
        expect(
          SimpleRepeat.of(Recurrence.parse(text), start),
          isNull,
          reason: text,
        );
      }
    });
  });

  group('advanced RRULEs', () {
    test('the first Monday and the last Friday of the month', () {
      expect(
        ruleDays('FREQ=MONTHLY;BYDAY=1MO', Day(2026, 10, 1), Day(2026, 12, 31)),
        ['2026-10-05', '2026-11-02', '2026-12-07'],
      );
      expect(
        ruleDays(
          'RRULE:freq=monthly;byday=-1fr',
          Day(2026, 10, 1),
          Day(2026, 12, 31),
        ),
        ['2026-10-30', '2026-11-27', '2026-12-25'],
      );
    });

    test('the last day of every month', () {
      expect(
        ruleDays(
          'FREQ=MONTHLY;BYMONTHDAY=-1',
          Day(2026, 1, 1),
          Day(2026, 4, 30),
        ),
        ['2026-01-31', '2026-02-28', '2026-03-31', '2026-04-30'],
      );
    });

    test('a weekday that is also a month day: Friday the 13th', () {
      expect(
        ruleDays(
          'FREQ=MONTHLY;BYDAY=FR;BYMONTHDAY=13',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        ['2026-02-13', '2026-03-13', '2026-11-13'],
      );
    });

    test('yearly by month and ordinal weekday (RFC 5545 examples)', () {
      // US Thanksgiving: the fourth Thursday in November.
      expect(
        ruleDays(
          'FREQ=YEARLY;BYMONTH=11;BYDAY=4TH',
          Day(2026, 1, 1),
          Day(2027, 12, 31),
        ),
        ['2026-11-26', '2027-11-25'],
      );
      // "Monday of week number 20" in RFC terms: the 20th Monday.
      expect(
        ruleDays('FREQ=YEARLY;BYDAY=20MO', Day(1997, 1, 1), Day(1997, 12, 31)),
        ['1997-05-19'],
      );
      expect(
        ruleDays('FREQ=YEARLY;BYMONTH=1,7', Day(2026, 1, 15), Day(2027, 1, 31)),
        ['2026-01-15', '2026-07-15', '2027-01-15'],
      );
    });

    test('daily limited to some months and weekdays', () {
      expect(
        ruleDays(
          'FREQ=DAILY;BYMONTH=12;BYDAY=SA,SU',
          Day(2026, 11, 28),
          Day(2026, 12, 13),
        ),
        ['2026-12-05', '2026-12-06', '2026-12-12', '2026-12-13'],
      );
    });

    test('WKST matters for every other week (RFC 5545 example)', () {
      final start = Day(1997, 8, 5);
      final to = Day(1997, 12, 31);
      expect(
        ruleDays(
          'FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU;WKST=MO',
          start,
          to,
        ),
        ['1997-08-05', '1997-08-10', '1997-08-19', '1997-08-24'],
      );
      expect(
        ruleDays(
          'FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU;WKST=SU',
          start,
          to,
        ),
        ['1997-08-05', '1997-08-17', '1997-08-19', '1997-08-31'],
      );
    });

    test('BYHOUR and BYMINUTE are the times of day, each pair', () {
      final schedule = Schedule(
        start: Day(2026, 10, 24),
        times: [at(12)],
        rule: Recurrence.parse('FREQ=DAILY;BYHOUR=20,8;BYMINUTE=30,0'),
      );
      expect(keys(schedule, Day(2026, 10, 24), Day(2026, 10, 24)), [
        '2026-10-24T08:00',
        '2026-10-24T08:30',
        '2026-10-24T20:00',
        '2026-10-24T20:30',
      ]);
    });

    test('COUNT counts every Occurrence, each time of day on its own', () {
      final schedule = Schedule(
        start: Day(2026, 10, 24),
        times: [at(8), at(20)],
        rule: Recurrence.parse('FREQ=DAILY;COUNT=3'),
      );
      expect(keys(schedule, Day(2026, 10, 1), Day(2026, 12, 31)), [
        '2026-10-24T08:00',
        '2026-10-24T20:00',
        '2026-10-25T08:00',
      ]);
      expect(keys(schedule, Day(2026, 10, 25), Day(2026, 12, 31)), [
        '2026-10-25T08:00',
      ]);
    });

    test('UNTIL is inclusive, to the day or to the minute', () {
      expect(
        ruleDays(
          'FREQ=DAILY;UNTIL=20261026',
          Day(2026, 10, 24),
          Day(2026, 12, 31),
        ),
        ['2026-10-24', '2026-10-25', '2026-10-26'],
      );
      final schedule = Schedule(
        start: Day(2026, 10, 24),
        times: [at(8), at(20)],
        rule: Recurrence.parse('FREQ=DAILY;UNTIL=20261025T120000Z'),
      );
      expect(keys(schedule, Day(2026, 10, 1), Day(2026, 12, 31)), [
        '2026-10-24T08:00',
        '2026-10-24T20:00',
        '2026-10-25T08:00',
      ]);
    });

    test('skipping ahead gives the same days as walking from the start', () {
      for (final text in [
        'FREQ=DAILY;INTERVAL=3',
        'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH',
        'FREQ=MONTHLY;INTERVAL=5;BYDAY=-1SU',
        'FREQ=YEARLY;INTERVAL=3;BYMONTH=2;BYMONTHDAY=29',
      ]) {
        final schedule = Schedule(
          start: Day(2001, 3, 7),
          rule: Recurrence.parse(text),
        );
        final from = Day(2026, 1, 1);
        final to = Day(2045, 12, 31);
        expect(
          schedule.between(from, to),
          schedule.between(Day(2001, 3, 7), to).where((o) => !(o.day < from)),
          reason: text,
        );
      }
    });

    test('next looks ahead, and gives up on a rule that never falls', () {
      final leap = Schedule(
        start: Day(2026, 1, 1),
        rule: Recurrence.parse('FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=29'),
      );
      expect(
        [for (final o in leap.next(Day(2026, 10, 24), 2)) o.key],
        ['2028-02-29', '2032-02-29'],
      );
      final never = Schedule(
        start: Day(2026, 1, 1),
        rule: Recurrence.parse('FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=30'),
      );
      expect(never.next(Day(2026, 1, 1), 1), isEmpty);
    });

    test('formats back to a rule that means the same', () {
      for (final text in [
        'FREQ=DAILY',
        'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SU;WKST=SU',
        'FREQ=MONTHLY;COUNT=10;BYDAY=1MO,-1FR',
        'FREQ=YEARLY;UNTIL=20301231;BYMONTH=11;BYDAY=4TH',
        'FREQ=DAILY;UNTIL=20261025T120000;BYHOUR=8,20;BYMINUTE=0',
      ]) {
        final rule = Recurrence.parse(text);
        final again = Recurrence.parse(rule.format());
        expect(again.format(), rule.format(), reason: text);
        final start = Day(2026, 1, 1);
        expect(
          Schedule(start: start, rule: again).between(start, Day(2031, 1, 1)),
          Schedule(start: start, rule: rule).between(start, Day(2031, 1, 1)),
          reason: text,
        );
      }
    });

    test('refuses what it does not support instead of guessing', () {
      for (final text in [
        '',
        'BYDAY=MO',
        'FREQ=HOURLY',
        'FREQ=MINUTELY',
        'FREQ=DAILY;BYSETPOS=-1',
        'FREQ=YEARLY;BYWEEKNO=20',
        'FREQ=YEARLY;BYYEARDAY=100',
        'FREQ=DAILY;BYSECOND=30',
        'FREQ=DAILY;COUNT=3;UNTIL=20261231',
        'FREQ=WEEKLY;BYDAY=1MO',
        'FREQ=WEEKLY;BYMONTHDAY=3',
        'FREQ=MONTHLY;BYMONTHDAY=0',
        'FREQ=MONTHLY;BYMONTH=13',
        'FREQ=DAILY;INTERVAL=0',
        'FREQ=DAILY;FREQ=WEEKLY',
        'FREQ=DAILY;UNTIL=tomorrow',
        'FREQ=DAILY;BYDAY=XX',
        'nonsense',
      ]) {
        expect(
          () => Recurrence.parse(text),
          throwsFormatException,
          reason: text,
        );
        expect(Recurrence.tryParse(text), isNull, reason: text);
      }
    });
  });
}

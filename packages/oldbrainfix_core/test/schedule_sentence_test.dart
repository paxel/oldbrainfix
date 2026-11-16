import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

/// The day the sentences are written and read on.
final today = Day(2026, 11, 3);

/// Every kind of the simple editor, with each end, start and times.
final schedules = [
  SimpleSchedule(const SimpleRepeat.once(), today, [at(9)]),
  SimpleSchedule(const SimpleRepeat.once(), Day(2027, 1, 5), const []),
  SimpleSchedule(const SimpleRepeat.every(1, RepeatUnit.days), today, const []),
  SimpleSchedule(const SimpleRepeat.every(3, RepeatUnit.days), today, [
    at(8),
    at(20, 30),
  ]),
  for (final unit in RepeatUnit.values) ...[
    SimpleSchedule(SimpleRepeat.every(1, unit), Day(2026, 12, 1), const []),
    SimpleSchedule(SimpleRepeat.every(2, unit), today, [at(7, 15)]),
  ],
  SimpleSchedule(
    SimpleRepeat.weekdays(const {1, 4}, until: Day(2026, 12, 31)),
    today,
    [at(8), at(15)],
  ),
  SimpleSchedule(const SimpleRepeat.weekdays({}), today, const []),
  SimpleSchedule(
    const SimpleRepeat.weekdays({1, 2, 3, 4, 5, 6, 7}, count: 10),
    Day(2026, 3, 2),
    const [],
  ),
  SimpleSchedule(const SimpleRepeat.monthDay(15), today, [at(10)]),
  SimpleSchedule(
    SimpleRepeat.monthDay(31, every: 2, until: Day(2027, 6, 30)),
    Day(2026, 1, 31),
    const [],
  ),
  SimpleSchedule(
    const SimpleRepeat.monthWeekday(WeekdayRule(1, 1)),
    today,
    const [],
  ),
  SimpleSchedule(
    const SimpleRepeat.monthWeekday(WeekdayRule(5, -1), every: 3, count: 1),
    today,
    [at(18)],
  ),
  SimpleSchedule(
    const SimpleRepeat.every(1, RepeatUnit.weeks, count: 4),
    Day(2026, 11, 10),
    [at(0), at(23, 59)],
  ),
];

/// The unknown parts of [text] as the text they cover.
List<String> unknown(String text, String locale) => [
  for (final r in readSchedule(text, locale, today).unknown)
    text.substring(r.start, r.end),
];

/// What the suggestions at the end of [text] would put in.
List<String> suggested(String text, String locale) => [
  for (final s in suggestSchedule(text, text.length, locale)) s.text,
];

void main() {
  group('writing', () {
    test('a schedule reads as a sentence in English', () {
      String en(SimpleSchedule s) => scheduleSentence(s, 'en', today);
      expect(
        en(schedules[0]),
        'Once on 11/3 at 09:00',
        reason: 'once always names its day',
      );
      expect(en(schedules[1]), 'Once on 1/5/2027');
      expect(en(schedules[2]), 'Every day');
      expect(en(schedules[3]), 'Every 3 days at 08:00, 20:30');
      expect(en(schedules[4]), 'Every day, from 12/1');
      expect(en(schedules[7]), 'Every 2 weeks at 07:15');
      expect(en(schedules[10]), 'Every year, from 12/1');
      expect(
        en(schedules[12]),
        'Weekdays Mon, Thu at 08:00, 15:00, until 12/31',
      );
      expect(en(schedules[13]), 'Weekdays');
      expect(
        en(schedules[14]),
        'Weekdays Mon, Tue, Wed, Thu, Fri, Sat, Sun, from 3/2, 10 times',
      );
      expect(en(schedules[15]), 'Every month on day 15 at 10:00');
      expect(
        en(schedules[16]),
        'Every 2 months on day 31, from 1/31, until 6/30/2027',
      );
      expect(en(schedules[17]), 'Every month on the first Monday');
      expect(
        en(schedules[18]),
        'Every 3 months on the last Friday at 18:00, once',
      );
    });

    test('and in German', () {
      String de(SimpleSchedule s) => scheduleSentence(s, 'de', today);
      expect(de(schedules[0]), 'Einmal am 3.11. um 09:00');
      expect(de(schedules[1]), 'Einmal am 5.1.2027');
      expect(de(schedules[2]), 'Jeden Tag');
      expect(de(schedules[3]), 'Alle 3 Tage um 08:00, 20:30');
      expect(de(schedules[6]), 'Jede Woche, ab 1.12.');
      expect(de(schedules[8]), 'Jeden Monat, ab 1.12.');
      expect(de(schedules[10]), 'Jedes Jahr, ab 1.12.');
      expect(de(schedules[11]), 'Alle 2 Jahre um 07:15');
      expect(
        de(schedules[12]),
        'Wochentags Mo, Do um 08:00, 15:00, bis 31.12.',
      );
      expect(de(schedules[15]), 'Jeden Monat am 15. um 10:00');
      expect(
        de(schedules[16]),
        'Alle 2 Monate am 31., ab 31.1., bis 30.6.2027',
      );
      expect(de(schedules[17]), 'Jeden Monat am ersten Montag');
      expect(
        de(schedules[18]),
        'Alle 3 Monate am letzten Freitag um 18:00, einmal',
      );
      expect(de(schedules[19]), 'Jede Woche um 00:00, 23:59, ab 10.11., 4-mal');
    });

    test('a schedule the simple editor cannot show has no sentence', () {
      final advanced = Schedule(
        start: today,
        rule: Recurrence.parse('FREQ=MONTHLY;BYDAY=1MO,2TU'),
      );
      expect(SimpleSchedule.of(advanced), isNull);
      final daily = Schedule(
        start: today,
        times: [at(8)],
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      expect(
        SimpleSchedule.of(daily),
        SimpleSchedule(const SimpleRepeat.every(1, RepeatUnit.days), today, [
          at(8),
        ]),
      );
    });
  });

  group('reading', () {
    test('every sentence written reads back as its schedule', () {
      for (final locale in ['en', 'de']) {
        for (final schedule in schedules) {
          final text = scheduleSentence(schedule, locale, today);
          final read = readSchedule(text, locale, today);
          expect(read.schedule, schedule, reason: '$locale: $text');
          expect(read.unknown, isEmpty, reason: '$locale: $text');
        }
      }
    });

    test('what is left out is all day, from today, never ending', () {
      expect(
        readSchedule('every week', 'en', today).schedule,
        SimpleSchedule(
          const SimpleRepeat.every(1, RepeatUnit.weeks),
          today,
          const [],
        ),
      );
      expect(
        readSchedule('Einmal', 'de', today).schedule,
        SimpleSchedule(const SimpleRepeat.once(), today, const []),
      );
    });

    test('it reads more than it writes', () {
      SimpleSchedule? read(String text, String locale) =>
          readSchedule(text, locale, today).schedule;
      expect(
        read('Every mon and thursday at 8:00, until 12/31/27', 'en'),
        SimpleSchedule(
          SimpleRepeat.weekdays(const {1, 4}, until: Day(2027, 12, 31)),
          today,
          [at(8)],
        ),
      );
      expect(
        read('immer Mo Do um 08:00 bis 31.12.2027', 'de'),
        SimpleSchedule(
          SimpleRepeat.weekdays(const {1, 4}, until: Day(2027, 12, 31)),
          today,
          [at(8)],
        ),
      );
      expect(
        read('alle 2 wochen, 10mal', 'de'),
        SimpleSchedule(
          const SimpleRepeat.every(2, RepeatUnit.weeks, count: 10),
          today,
          const [],
        ),
      );
      expect(
        read('every 2 month on day 3', 'en'),
        SimpleSchedule(
          const SimpleRepeat.monthDay(3, every: 2),
          today,
          const [],
        ),
      );
      // A day written with dots is day and month, with a slash month and
      // day, in both languages.
      expect(read('Every day from 24.12.', 'en')?.start, Day(2026, 12, 24));
      expect(read('Jeden Tag ab 12/24', 'de')?.start, Day(2026, 12, 24));
    });

    test('unknown parts are marked and change nothing', () {
      expect(
        readSchedule('Every day at 08:00 please', 'en', today).schedule,
        SimpleSchedule(const SimpleRepeat.every(1, RepeatUnit.days), today, [
          at(8),
        ]),
      );
      expect(unknown('Every day at 08:00 please', 'en'), ['please']);
      // A part left unfinished is marked as a whole.
      expect(unknown('Every day, until', 'en'), ['until']);
      expect(unknown('Every day at 25:00', 'en'), ['at', '25:00']);
      expect(unknown('Every 2 wekks at 08:00', 'en'), [
        'Every',
        '2',
        'wekks',
        'at',
        '08:00',
      ]);
      expect(readSchedule('Every 2 wekks', 'en', today).schedule, isNull);
      expect(unknown('Every month on day 32', 'en'), ['on', 'day', '32']);
      expect(
        readSchedule('Every month on day 32', 'en', today).schedule,
        SimpleSchedule(
          const SimpleRepeat.every(1, RepeatUnit.months),
          today,
          const [],
        ),
      );
      // Once has a single time and no end.
      expect(unknown('Once at 08:00, 09:00, until 12/1', 'en'), [
        '09:00',
        'until',
        '12/1',
      ]);
      expect(
        readSchedule('Once at 08:00, 09:00', 'en', today).schedule,
        SimpleSchedule(const SimpleRepeat.once(), today, [at(8)]),
      );
      expect(unknown('Jeden Tag ab 31.2.', 'de'), ['ab', '31.2.']);
      expect(unknown('Jeden Tag 0-mal', 'de'), ['0', 'mal']);
      expect(unknown('', 'en'), isEmpty);
      expect(readSchedule('', 'en', today).schedule, isNull);
      expect(readSchedule('  hello', 'en', today).unknown, [
        (start: 2, end: 7),
      ]);
    });
  });

  group('suggestions', () {
    test('the next word', () {
      expect(suggested('', 'en'), [
        'Once',
        'Every',
        'Weekdays',
        'Today',
        'Tonight',
        'Tomorrow',
        'In',
      ]);
      expect(suggested('Ev', 'en'), ['Every']);
      expect(suggested('Every ', 'en'), [
        'day',
        'week',
        'month',
        'year',
        'Mon',
        'Tue',
        'Wed',
        'Thu',
        'Fri',
        'Sat',
        'Sun',
      ]);
      expect(suggested('Every 2 w', 'en'), ['weeks']);
      expect(suggested('Every day ', 'en'), ['at', 'from', 'until']);
      expect(suggested('Every month o', 'en'), ['on']);
      expect(suggested('Every month on the ', 'en'), [
        'first',
        'second',
        'third',
        'fourth',
        'last',
      ]);
      expect(suggested('Alle 3 W', 'de'), ['Wochen']);
      expect(suggested('Jeden Monat am l', 'de'), ['letzten']);
      expect(suggested('Jeden Tag, 3 ', 'de'), ['mal']);
    });

    test('weekday names, those not picked yet', () {
      expect(suggested('Weekdays Tue, T', 'en'), ['Thu']);
      expect(suggested('Wochentags Mo., D', 'de'), ['Di', 'Do']);
      expect(suggested('Jeden Monat am ersten M', 'de'), [
        'Montag',
        'Mittwoch',
      ]);
    });

    test('times', () {
      expect(suggested('Every day at ', 'en'), ['08:00', '12:00', '18:00']);
      expect(suggested('Every day at 8', 'en'), ['08:00', '08:30']);
      expect(suggested('Jeden Tag um 08:00, 13:3', 'de'), ['13:30']);
    });

    test('they replace the word at the cursor', () {
      const text = 'Every 2 we at 08:00';
      final [weeks] = suggestSchedule(text, 10, 'en');
      expect((weeks.start, weeks.end, weeks.text), (8, 10, 'weeks'));
    });
  });

  group('relative words (#271)', () {
    // A Tuesday afternoon; the day starts at 07:30.
    final now = SentenceNow(today, at(14, 20), at(7, 30));
    SimpleSchedule? read(String text, String locale, [SentenceNow? at]) =>
        readScheduleAt(text, locale, at ?? now).schedule;
    SimpleSchedule once(Day day, [ClockTime? time]) =>
        SimpleSchedule(const SimpleRepeat.once(), day, [?time]);
    final tomorrow = today.addDays(1);

    test('a day, at the day start, 19:00 or the time typed', () {
      expect(read('tomorrow', 'en'), once(tomorrow, at(7, 30)));
      expect(read('Tomorrow 7 pm', 'en'), once(tomorrow, at(19)));
      expect(read('tomorrow at 7:30pm', 'en'), once(tomorrow, at(19, 30)));
      expect(read('tomorrow evening', 'en'), once(tomorrow, at(19)));
      expect(read('tonight', 'en'), once(today, at(19)));
      expect(read('Tonight 8 PM', 'en'), once(today, at(20)));
      expect(read('today', 'en'), once(today));
      expect(read('today at 17:00', 'en'), once(today, at(17)));
      expect(read('morgen', 'de'), once(tomorrow, at(7, 30)));
      expect(read('morgen 19 Uhr', 'de'), once(tomorrow, at(19)));
      expect(read('Morgen um 8 Uhr', 'de'), once(tomorrow, at(8)));
      expect(read('heute Abend', 'de'), once(today, at(19)));
      expect(read('heute Abend um 20:30', 'de'), once(today, at(20, 30)));
      expect(read('heute', 'de'), once(today));
      // As the day of Once, as a new Reminder's line shows it.
      expect(read('Once tomorrow at 15:00', 'en'), once(tomorrow, at(15)));
      expect(read('Once on tomorrow', 'en'), once(tomorrow, at(7, 30)));
      expect(read('Einmal am morgen um 15:00', 'de'), once(tomorrow, at(15)));
    });

    test('in minutes and hours from now, over midnight', () {
      expect(read('in 30 minutes', 'en'), once(today, at(14, 50)));
      expect(read('in 2 hours', 'en'), once(today, at(16, 20)));
      expect(read('in an hour', 'en'), once(today, at(15, 20)));
      expect(read('In 1 h', 'en'), once(today, at(15, 20)));
      expect(read('in 2 Stunden', 'de'), once(today, at(16, 20)));
      expect(read('in einer Minute', 'de'), once(today, at(14, 21)));
      final late = SentenceNow(Day(2026, 12, 31), at(23, 30), at(7, 30));
      expect(read('in 1 hour', 'en', late), once(Day(2027, 1, 1), at(0, 30)));
      expect(read('in 90 min', 'en', late), once(Day(2027, 1, 1), at(1)));
    });

    test('in days and weeks, at the day start or the time typed', () {
      expect(read('in 3 days', 'en'), once(today.addDays(3), at(7, 30)));
      expect(read('in a week', 'en'), once(today.addDays(7), at(7, 30)));
      expect(read('in 2 weeks 9 am', 'en'), once(today.addDays(14), at(9)));
      expect(read('in 3 Tagen', 'de'), once(today.addDays(3), at(7, 30)));
      expect(read('in einer Woche', 'de'), once(today.addDays(7), at(7, 30)));
    });

    test('times with am, pm and Uhr read in every sentence', () {
      expect(
        readSchedule('Every day at 7 pm', 'en', today).schedule,
        SimpleSchedule(const SimpleRepeat.every(1, RepeatUnit.days), today, [
          at(19),
        ]),
      );
      expect(read('today 12 am', 'en'), once(today, at(0)));
      expect(read('today 12 pm', 'en'), once(today, at(12)));
      expect(
        readSchedule('Jeden Tag um 8 Uhr', 'de', today).schedule,
        SimpleSchedule(const SimpleRepeat.every(1, RepeatUnit.days), today, [
          at(8),
        ]),
      );
    });

    List<String> unknownAt(String text, String locale) => [
      for (final r in readScheduleAt(text, locale, now).unknown)
        text.substring(r.start, r.end),
    ];

    test('what does not fit is marked and changes nothing', () {
      expect(unknownAt('today 13 pm', 'en'), ['13 pm']);
      expect(unknownAt('today 0 am', 'en'), ['0 am']);
      expect(unknownAt('heute 25 Uhr', 'de'), ['25 Uhr']);
      expect(unknownAt('in 0 minutes', 'en'), ['in', '0', 'minutes']);
      expect(read('in 0 minutes', 'en'), isNull);
      expect(unknownAt('in 3', 'en'), ['in', '3']);
      expect(unknownAt('in 2 hours at 08:00', 'en'), ['08:00']);
      expect(unknownAt('tomorrow, 3 times', 'en'), ['3', 'times']);
      // "am" is German for "on".
      expect(unknownAt('Einmal am 5.11.', 'de'), isEmpty);
    });

    test('they are read only at a now', () {
      expect(readSchedule('tomorrow', 'en', today).schedule, isNull);
      expect(unknown('tomorrow', 'en'), ['tomorrow']);
      expect(unknown('in 2 hours', 'en'), ['in', '2', 'hours']);
    });

    test('they are suggested', () {
      expect(suggested('T', 'en'), ['Today', 'Tonight', 'Tomorrow']);
      expect(suggested('Tomorrow ', 'en'), ['evening', 'at']);
      expect(suggested('Tonight ', 'en'), ['at']);
      expect(suggested('In 2 ', 'en'), ['minutes', 'hours', 'days', 'weeks']);
      expect(suggested('in a ', 'en'), ['minute', 'hour', 'day', 'week']);
      expect(suggested('H', 'de'), ['Heute']);
      expect(suggested('Heute ', 'de'), ['Abend', 'um']);
      expect(suggested('In 3 T', 'de'), ['Tagen']);
      expect(suggested('Tomorrow 7 ', 'en'), isEmpty);
    });

    test('the chips are their words read at now', () {
      expect(quickReminders(now), [
        QuickReminder(QuickChip.inAnHour, today, at(15, 20)),
        QuickReminder(QuickChip.tonight, today, at(19)),
        QuickReminder(QuickChip.tomorrow, tomorrow, at(7, 30)),
        QuickReminder(QuickChip.inAWeek, today.addDays(7), at(7, 30)),
      ]);
      const words = {
        'en': {
          QuickChip.inAnHour: 'in 1 hour',
          QuickChip.tonight: 'tonight',
          QuickChip.tomorrow: 'tomorrow',
          QuickChip.inAWeek: 'in 1 week',
        },
        'de': {
          QuickChip.inAnHour: 'in 1 Stunde',
          QuickChip.tonight: 'heute Abend',
          QuickChip.tomorrow: 'morgen',
          QuickChip.inAWeek: 'in 1 Woche',
        },
      };
      for (final moment in [
        now,
        SentenceNow(Day(2026, 12, 31), at(23, 30), at(9)),
        SentenceNow(today, at(0), at(6, 15)),
      ]) {
        for (final chip in quickReminders(moment)) {
          for (final MapEntry(key: locale, value: word) in words.entries) {
            expect(
              read(word[chip.chip] ?? '', locale, moment),
              chip.schedule,
              reason: '$moment $locale',
            );
          }
        }
      }
    });

    test('"Tonight" is offered while 19:00 is still to come', () {
      final evening = SentenceNow(today, at(19), at(7, 30));
      expect(
        [for (final c in quickReminders(evening)) c.chip],
        [QuickChip.inAnHour, QuickChip.tomorrow, QuickChip.inAWeek],
      );
      expect(
        quickReminders(SentenceNow(today, at(18, 59), at(7, 30)))[1].chip,
        QuickChip.tonight,
      );
    });
  });
}

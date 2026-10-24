/// When a Routine happens (ADR 0006): a start day, the times of day, and
/// an iCalendar RRULE for the days, or none for a Reminder that happens
/// once.
///
/// Everything here is the Device's local wall clock with no zone: a day
/// is a calendar day and a time is what the clock on the wall shows. An
/// 08:00 Routine stays at 08:00 across a daylight-saving switch because
/// nothing is ever counted in hours. Days are stepped with calendar
/// fields on a UTC carrier, never by adding a [Duration] to a local
/// time. Turning a wall-clock Occurrence into an instant (and what a
/// skipped or doubled hour means for it) is the alarm layer's business.
library;

/// Turns an instant into the Device's local wall clock, as a [DateTime]
/// whose fields are read and whose zone is ignored. The app hands in
/// `(utc) => utc.toLocal()`; tests hand in a fixed zone.
typedef WallClock = DateTime Function(DateTime utc);

/// A day on the wall calendar.
class Day implements Comparable<Day> {
  final int year;
  final int month;
  final int day;

  /// [year], [month] and [day] as a calendar normalises them: the 32nd
  /// of January is the 1st of February.
  factory Day(int year, int month, int day) =>
      Day._of(DateTime.utc(year, month, day));

  const Day._(this.year, this.month, this.day);

  Day._of(DateTime utc) : this._(utc.year, utc.month, utc.day);

  /// The day [wall] shows, its zone ignored.
  factory Day.of(DateTime wall) => Day(wall.year, wall.month, wall.day);

  /// The day of [now] on the wall clock [wallClock].
  factory Day.today(DateTime now, WallClock wallClock) =>
      Day.of(wallClock(now.toUtc()));

  /// Parses `YYYY-MM-DD`; null when it is not a valid day.
  static Day? tryParse(String text) {
    final match = RegExp(r'^(\d{4})-(\d{2})-(\d{2})$').firstMatch(text);
    if (match == null) return null;
    final [y, m, d] = [for (var i = 1; i <= 3; i++) int.parse(match[i]!)];
    final day = Day(y, m, d);
    return day.year == y && day.month == m && day.day == d ? day : null;
  }

  DateTime get _utc => DateTime.utc(year, month, day);

  /// `DateTime.monday` to `DateTime.sunday`.
  int get weekday => _utc.weekday;

  Day addDays(int days) => Day(year, month, day + days);

  /// How many days lie from this day to [other]; negative when before.
  int daysUntil(Day other) => other._utc.difference(_utc).inDays;

  /// A [DateTime] carrying this day's fields at midnight, for formatting.
  DateTime get wall => DateTime(year, month, day);

  @override
  int compareTo(Day other) => _utc.compareTo(other._utc);

  bool operator <(Day other) => compareTo(other) < 0;
  bool operator >(Day other) => compareTo(other) > 0;
  bool operator <=(Day other) => compareTo(other) <= 0;

  @override
  bool operator ==(Object other) =>
      other is Day &&
      other.year == year &&
      other.month == month &&
      other.day == day;

  @override
  int get hashCode => Object.hash(year, month, day);

  @override
  String toString() =>
      '${year.toString().padLeft(4, '0')}-${_two(month)}-${_two(day)}';
}

/// A time of day on the wall clock, to the minute.
class ClockTime implements Comparable<ClockTime> {
  final int hour;
  final int minute;

  ClockTime(this.hour, this.minute) {
    if (hour < 0 || hour > 23 || minute < 0 || minute > 59) {
      throw ArgumentError('$hour:$minute is no time of day');
    }
  }

  /// Parses `HH:MM`; null when it is not a time of day.
  static ClockTime? tryParse(String text) {
    final match = RegExp(r'^(\d{2}):(\d{2})$').firstMatch(text);
    if (match == null) return null;
    final hour = int.parse(match[1]!);
    final minute = int.parse(match[2]!);
    if (hour > 23 || minute > 59) return null;
    return ClockTime(hour, minute);
  }

  @override
  int compareTo(ClockTime other) =>
      (hour * 60 + minute).compareTo(other.hour * 60 + other.minute);

  @override
  bool operator ==(Object other) =>
      other is ClockTime && other.hour == hour && other.minute == minute;

  @override
  int get hashCode => Object.hash(hour, minute);

  @override
  String toString() => '${_two(hour)}:${_two(minute)}';
}

/// One due moment of a Routine (CONTEXT.md): a day and a time of day, or
/// the whole day.
class Occurrence implements Comparable<Occurrence> {
  final Day day;

  /// The time of day; null for an all-day Occurrence.
  final ClockTime? time;

  const Occurrence(this.day, [this.time]);

  bool get allDay => time == null;

  /// Names the Occurrence for good: `2026-10-25` or `2026-10-25T08:00`.
  /// Wall-clock, so a zone change or a daylight-saving switch never
  /// renames it.
  String get key => switch (time) {
    null => '$day',
    final time => '${day}T$time',
  };

  /// Parses [key]; null when it names no Occurrence.
  static Occurrence? tryParse(String key) {
    final [dayText, ...rest] = key.split('T');
    final day = Day.tryParse(dayText);
    if (day == null || rest.length > 1) return null;
    if (rest.isEmpty) return Occurrence(day);
    final time = ClockTime.tryParse(rest.single);
    return time == null ? null : Occurrence(day, time);
  }

  /// By day; within a day all-day first, then by time.
  @override
  int compareTo(Occurrence other) {
    final byDay = day.compareTo(other.day);
    if (byDay != 0) return byDay;
    final a = time;
    final b = other.time;
    if (a == null || b == null) {
      return (a == null ? 0 : 1).compareTo(b == null ? 0 : 1);
    }
    return a.compareTo(b);
  }

  @override
  bool operator ==(Object other) => other is Occurrence && other.key == key;

  @override
  int get hashCode => key.hashCode;

  @override
  String toString() => key;
}

/// How often an RRULE's periods come round. HOURLY, MINUTELY and
/// SECONDLY are not supported: several times per day are times of day.
enum Frequency { daily, weekly, monthly, yearly }

/// One BYDAY item: a weekday, with an ordinal for "the first Monday"
/// (1MO) or "the last Friday" (-1FR) of the month or year.
class WeekdayRule {
  /// `DateTime.monday` to `DateTime.sunday`.
  final int weekday;

  /// 1 for the first, -1 for the last; null for every such weekday.
  final int? ordinal;

  const WeekdayRule(this.weekday, [this.ordinal]);

  @override
  bool operator ==(Object other) =>
      other is WeekdayRule &&
      other.weekday == weekday &&
      other.ordinal == ordinal;

  @override
  int get hashCode => Object.hash(weekday, ordinal);

  @override
  String toString() => '${ordinal ?? ''}${_weekdayCodes[weekday - 1]}';
}

/// The end an RRULE's UNTIL sets, inclusive, read as wall clock.
typedef Until = ({Day day, ClockTime? time});

/// An iCalendar recurrence rule (RFC 5545, 3.3.10), the subset a Routine
/// needs: FREQ (DAILY, WEEKLY, MONTHLY, YEARLY), INTERVAL, BYDAY with
/// ordinals, BYMONTHDAY (negative from the month's end), BYMONTH,
/// BYHOUR, BYMINUTE, COUNT, UNTIL and WKST. BYSETPOS, BYYEARDAY,
/// BYWEEKNO and BYSECOND other than 0 are refused, not guessed.
class Recurrence {
  final Frequency frequency;
  final int interval;
  final List<WeekdayRule> byDay;
  final List<int> byMonthDay;
  final List<int> byMonth;

  /// Times of day as RRULE spells them: every BYHOUR with every BYMINUTE.
  final List<int> byHour;
  final List<int> byMinute;

  /// How many Occurrences there are in all, counted from the start.
  final int? count;

  /// The last moment an Occurrence may fall on, inclusive. A trailing
  /// `Z` is read as wall clock too.
  final Until? until;

  /// The day a week starts on, for WEEKLY with an INTERVAL above 1.
  final int weekStart;

  Recurrence({
    required this.frequency,
    this.interval = 1,
    this.byDay = const [],
    this.byMonthDay = const [],
    this.byMonth = const [],
    this.byHour = const [],
    this.byMinute = const [],
    this.count,
    this.until,
    this.weekStart = DateTime.monday,
  }) {
    if (interval < 1) throw ArgumentError.value(interval, 'interval');
    if (count != null && count! < 1) throw ArgumentError.value(count, 'count');
  }

  /// Parses an RRULE value, with or without the `RRULE:` prefix, in any
  /// case. Throws a [FormatException] saying what is wrong, with the
  /// offending part as its source.
  static Recurrence parse(String text) {
    var body = text.trim().toUpperCase();
    if (body.startsWith('RRULE:')) body = body.substring(6);
    if (body.isEmpty) throw const FormatException('empty rule', '');
    final parts = <String, String>{};
    for (final part in body.split(';')) {
      if (part.isEmpty) continue;
      final eq = part.indexOf('=');
      if (eq <= 0) throw FormatException('not KEY=VALUE', part);
      final key = part.substring(0, eq);
      if (parts.containsKey(key)) throw FormatException('twice', key);
      parts[key] = part.substring(eq + 1);
    }
    for (final key in parts.keys) {
      if (!_supportedKeys.contains(key)) {
        throw FormatException('not supported', key);
      }
    }
    final frequency = switch (parts['FREQ']) {
      'DAILY' => Frequency.daily,
      'WEEKLY' => Frequency.weekly,
      'MONTHLY' => Frequency.monthly,
      'YEARLY' => Frequency.yearly,
      null => throw const FormatException('FREQ missing', 'FREQ'),
      final other => throw FormatException('FREQ not supported', other),
    };
    final byDay = [
      for (final item in _list(parts['BYDAY'])) _weekdayRule(item, frequency),
    ];
    final byMonthDay = _numbers(parts['BYMONTHDAY'], 'BYMONTHDAY', -31, 31);
    if (byMonthDay.contains(0)) {
      throw const FormatException('BYMONTHDAY 0', 'BYMONTHDAY');
    }
    if (byMonthDay.isNotEmpty && frequency == Frequency.weekly) {
      throw const FormatException('BYMONTHDAY with WEEKLY', 'BYMONTHDAY');
    }
    if (parts['BYSECOND'] case final seconds? when seconds != '0') {
      throw FormatException('BYSECOND not supported', seconds);
    }
    if (parts.containsKey('COUNT') && parts.containsKey('UNTIL')) {
      throw const FormatException('COUNT and UNTIL together', 'COUNT, UNTIL');
    }
    final weekStart = switch (parts['WKST']) {
      null => DateTime.monday,
      final code => _weekday(code),
    };
    return Recurrence(
      frequency: frequency,
      interval: _number(parts['INTERVAL'], 'INTERVAL', 1, 9999) ?? 1,
      byDay: byDay,
      byMonthDay: byMonthDay,
      byMonth: _numbers(parts['BYMONTH'], 'BYMONTH', 1, 12),
      byHour: _numbers(parts['BYHOUR'], 'BYHOUR', 0, 23),
      byMinute: _numbers(parts['BYMINUTE'], 'BYMINUTE', 0, 59),
      count: _number(parts['COUNT'], 'COUNT', 1, 99999),
      until: switch (parts['UNTIL']) {
        null => null,
        final text => _until(text),
      },
      weekStart: weekStart,
    );
  }

  /// [parse], or null when [text] is no supported rule.
  static Recurrence? tryParse(String text) {
    try {
      return parse(text);
    } on FormatException {
      return null;
    }
  }

  /// The RRULE value, without the `RRULE:` prefix, in RFC order.
  String format() {
    final until = this.until;
    return [
      'FREQ=${frequency.name.toUpperCase()}',
      if (interval != 1) 'INTERVAL=$interval',
      if (count != null) 'COUNT=$count',
      if (until != null)
        'UNTIL=${_compact(until.day)}'
            '${until.time == null ? '' : 'T${_two(until.time!.hour)}${_two(until.time!.minute)}00'}',
      if (byMonth.isNotEmpty) 'BYMONTH=${byMonth.join(',')}',
      if (byMonthDay.isNotEmpty) 'BYMONTHDAY=${byMonthDay.join(',')}',
      if (byDay.isNotEmpty) 'BYDAY=${byDay.join(',')}',
      if (byHour.isNotEmpty) 'BYHOUR=${byHour.join(',')}',
      if (byMinute.isNotEmpty) 'BYMINUTE=${byMinute.join(',')}',
      if (weekStart != DateTime.monday) 'WKST=${_weekdayCodes[weekStart - 1]}',
    ].join(';');
  }

  /// The times of day the rule itself names, or null when it names none
  /// and the Schedule's own times count. A BYMINUTE without BYHOUR is
  /// taken at every hour, as RFC 5545 expands it.
  List<ClockTime>? get times {
    if (byHour.isEmpty && byMinute.isEmpty) return null;
    final hours = byHour.isEmpty ? [for (var h = 0; h < 24; h++) h] : byHour;
    final minutes = byMinute.isEmpty ? [0] : byMinute;
    return {
      for (final h in hours)
        for (final m in minutes) ClockTime(h, m),
    }.toList()..sort();
  }

  /// The days the rule falls on, from [start] (which counts only when the
  /// rule falls on it) up to [to], in order. With [from], periods wholly
  /// before it are skipped; COUNT needs every day, so it never skips.
  Iterable<Day> days(Day start, {Day? from, required Day to}) sync* {
    final weekZero = start.addDays(-((start.weekday - weekStart) % 7));
    var k = from == null || count != null
        ? 0
        : switch (frequency) {
            Frequency.daily => start.daysUntil(from) ~/ interval,
            Frequency.weekly => weekZero.daysUntil(from) ~/ 7 ~/ interval,
            Frequency.monthly =>
              ((from.year - start.year) * 12 + from.month - start.month) ~/
                  interval,
            Frequency.yearly => (from.year - start.year) ~/ interval,
          };
    if (k < 0) k = 0;
    for (; ; k++) {
      final step = k * interval;
      final Day periodStart;
      final List<Day> candidates;
      switch (frequency) {
        case Frequency.daily:
          periodStart = start.addDays(step);
          candidates = _dailyKeeps(periodStart) ? [periodStart] : const [];
        case Frequency.weekly:
          periodStart = weekZero.addDays(7 * step);
          candidates = _weekDays(periodStart, start);
        case Frequency.monthly:
          periodStart = Day(start.year, start.month + step, 1);
          candidates = byMonth.isEmpty || byMonth.contains(periodStart.month)
              ? _monthDays(periodStart.year, periodStart.month, start.day)
              : const [];
        case Frequency.yearly:
          periodStart = Day(start.year + step, 1, 1);
          candidates = _yearDays(periodStart.year, start);
      }
      if (periodStart > to) return;
      for (final day in candidates) {
        if (day < start || day > to) continue;
        yield day;
      }
    }
  }

  bool _dailyKeeps(Day day) {
    if (byMonth.isNotEmpty && !byMonth.contains(day.month)) return false;
    if (byMonthDay.isNotEmpty &&
        !_resolveMonthDays(day.year, day.month).contains(day.day)) {
      return false;
    }
    if (byDay.isNotEmpty && !byDay.any((r) => r.weekday == day.weekday)) {
      return false;
    }
    return true;
  }

  List<Day> _weekDays(Day weekBegins, Day start) {
    final weekdays = byDay.isEmpty
        ? {start.weekday}
        : {for (final rule in byDay) rule.weekday};
    return [
        for (final weekday in weekdays)
          weekBegins.addDays((weekday - weekStart) % 7),
      ].where((d) => byMonth.isEmpty || byMonth.contains(d.month)).toList()
      ..sort();
  }

  List<Day> _yearDays(int year, Day start) {
    if (byMonth.isEmpty && byMonthDay.isEmpty && byDay.isNotEmpty) {
      // BYDAY over the whole year: 1MO is the year's first Monday.
      final first = Day(year, 1, 1);
      final length = first.daysUntil(Day(year + 1, 1, 1));
      return _weekdayPicks([for (var i = 0; i < length; i++) first.addDays(i)]);
    }
    final months = byMonth.isNotEmpty
        ? byMonth
        : byMonthDay.isNotEmpty
        ? [for (var m = 1; m <= 12; m++) m]
        : [start.month];
    return [
      for (final month in [...months]..sort())
        ..._monthDays(year, month, start.day),
    ];
  }

  /// The days of [month] the rule picks; [defaultDay] when it names no
  /// day, skipped where the month is too short (RFC 5545).
  List<Day> _monthDays(int year, int month, int defaultDay) {
    final length = _monthLength(year, month);
    if (byMonthDay.isEmpty && byDay.isEmpty) {
      return defaultDay <= length ? [Day(year, month, defaultDay)] : const [];
    }
    final byDate = byMonthDay.isEmpty ? null : _resolveMonthDays(year, month);
    final byWeekday = byDay.isEmpty
        ? null
        : {
            for (final day in _weekdayPicks([
              for (var d = 1; d <= length; d++) Day(year, month, d),
            ]))
              day.day,
          };
    return [
      for (var d = 1; d <= length; d++)
        if ((byDate?.contains(d) ?? true) && (byWeekday?.contains(d) ?? true))
          Day(year, month, d),
    ];
  }

  /// The days of [span] each BYDAY item picks: every such weekday, or
  /// the nth one from the start or the end.
  List<Day> _weekdayPicks(List<Day> span) {
    final picked = <Day>{};
    for (final rule in byDay) {
      final matching = [
        for (final day in span)
          if (day.weekday == rule.weekday) day,
      ];
      final ordinal = rule.ordinal;
      if (ordinal == null) {
        picked.addAll(matching);
      } else {
        final index = ordinal > 0 ? ordinal - 1 : matching.length + ordinal;
        if (index >= 0 && index < matching.length) picked.add(matching[index]);
      }
    }
    return picked.toList()..sort();
  }

  Set<int> _resolveMonthDays(int year, int month) {
    final length = _monthLength(year, month);
    return {
      for (final d in byMonthDay)
        if (d > 0 && d <= length)
          d
        else if (d < 0 && -d <= length)
          length + 1 + d,
    };
  }

  static const _supportedKeys = {
    'FREQ',
    'INTERVAL',
    'BYDAY',
    'BYMONTHDAY',
    'BYMONTH',
    'BYHOUR',
    'BYMINUTE',
    'BYSECOND',
    'COUNT',
    'UNTIL',
    'WKST',
  };

  static List<String> _list(String? value) =>
      value == null ? const [] : value.split(',');

  static int? _number(String? value, String key, int min, int max) {
    if (value == null) return null;
    final n = int.tryParse(value);
    if (n == null || n < min || n > max) {
      throw FormatException('out of range', '$key=$value');
    }
    return n;
  }

  static List<int> _numbers(String? value, String key, int min, int max) => [
    for (final item in _list(value)) _number(item, key, min, max)!,
  ];

  static WeekdayRule _weekdayRule(String item, Frequency frequency) {
    final match = RegExp(r'^([+-]?\d{1,2})?([A-Z]{2})$').firstMatch(item);
    if (match == null) throw FormatException('not a weekday', item);
    final weekday = _weekday(match[2]!);
    final ordinalText = match[1];
    if (ordinalText == null) return WeekdayRule(weekday);
    final ordinal = int.parse(ordinalText);
    if (ordinal == 0 || ordinal.abs() > 53) {
      throw FormatException('ordinal out of range', item);
    }
    if (frequency != Frequency.monthly && frequency != Frequency.yearly) {
      throw FormatException('ordinal only with MONTHLY or YEARLY', item);
    }
    return WeekdayRule(weekday, ordinal);
  }

  static int _weekday(String code) {
    final index = _weekdayCodes.indexOf(code);
    if (index < 0) throw FormatException('not a weekday', code);
    return index + 1;
  }

  static Until _until(String text) {
    final match = RegExp(
      r'^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2})Z?)?$',
    ).firstMatch(text);
    final day = match == null
        ? null
        : Day.tryParse('${match[1]}-${match[2]}-${match[3]}');
    if (match == null || day == null) {
      throw FormatException('not a date', 'UNTIL=$text');
    }
    if (match[4] == null) return (day: day, time: null);
    final time = ClockTime.tryParse('${match[4]}:${match[5]}');
    if (time == null) throw FormatException('not a time', 'UNTIL=$text');
    return (day: day, time: time);
  }
}

/// When a Routine happens: from [start], at [times] (all day when there
/// are none), on the days [rule] picks, or only on [start] when there is
/// no rule. A Reminder is a Schedule without a rule (ADR 0005).
///
/// The Schedule is fixed (ADR 0006): it never reads what was Done, so a
/// Done early or late cannot move the next Occurrence.
class Schedule {
  /// The first day; for a Reminder its only one.
  final Day start;

  /// The times of day, ascending; empty for all day. When [rule] names
  /// times itself (BYHOUR, BYMINUTE), those count instead.
  final List<ClockTime> times;

  /// The repeat rule; null for one time only.
  final Recurrence? rule;

  /// How far ahead [next] looks before it gives up on a rule that
  /// hardly ever falls.
  static const lookAheadYears = 50;

  Schedule({required this.start, List<ClockTime> times = const [], this.rule})
    : times = ({...times}.toList()..sort()) {
    if (rule == null && this.times.length > 1) {
      throw ArgumentError.value(times, 'times', 'a Reminder happens once');
    }
  }

  /// Happens only once: a Reminder.
  bool get once => rule == null;

  /// The times of day each day gets; empty for all day.
  List<ClockTime> get dayTimes => rule?.times ?? times;

  /// The Occurrences from [from] to [to], both days included, in order.
  List<Occurrence> between(Day from, Day to) =>
      _occurrences(from: from, to: to).toList();

  /// The Occurrences on [day], all-day first, then by time.
  List<Occurrence> on(Day day) => between(day, day);

  /// The next [n] Occurrences from [from] on, within [lookAheadYears].
  List<Occurrence> next(Day from, int n) => _occurrences(
    from: from,
    to: Day(from.year + lookAheadYears, from.month, from.day),
  ).take(n).toList();

  /// Whether [occurrence] is one of this Schedule's.
  bool has(Occurrence occurrence) => on(occurrence.day).contains(occurrence);

  Iterable<Occurrence> _occurrences({
    required Day from,
    required Day to,
  }) sync* {
    final rule = this.rule;
    final times = dayTimes;
    final days = rule == null
        ? (start <= to ? [start] : const <Day>[])
        : rule.days(start, from: from, to: to);
    var counted = 0;
    for (final day in days) {
      for (final time in times.isEmpty ? const <ClockTime?>[null] : times) {
        final occurrence = Occurrence(day, time);
        if (rule != null && _beyond(occurrence, rule, counted)) return;
        counted++;
        if (!(day < from)) yield occurrence;
      }
    }
  }

  /// Whether [occurrence], with [counted] Occurrences before it, lies
  /// past the rule's COUNT or UNTIL. COUNT counts every Occurrence, each
  /// time of day on its own, as RFC 5545 counts BYHOUR.
  bool _beyond(Occurrence occurrence, Recurrence rule, int counted) {
    final count = rule.count;
    if (count != null && counted >= count) return true;
    final until = rule.until;
    if (until == null) return false;
    if (occurrence.day != until.day) return occurrence.day > until.day;
    final time = occurrence.time;
    final last = until.time;
    return time != null && last != null && time.compareTo(last) > 0;
  }
}

/// The ways the simple editor repeats (ADR 0006).
enum RepeatKind {
  /// Once: a Reminder.
  once,

  /// Every N days, weeks, months or years.
  every,

  /// On chosen weekdays, every week.
  weekdays,
}

/// The unit of [RepeatKind.every].
enum RepeatUnit { days, weeks, months, years }

/// A schedule the simple editor can show: once, every N days, weeks,
/// months or years, or chosen weekdays. Months and years fall on the
/// start's day; a month or year without it is skipped (RFC 5545), so
/// "every month from the 31st" skips the shorter months.
class SimpleRepeat {
  final RepeatKind kind;
  final int every;
  final RepeatUnit unit;

  /// `DateTime.monday` to `DateTime.sunday`, for [RepeatKind.weekdays].
  final Set<int> weekdays;

  const SimpleRepeat.once()
    : kind = RepeatKind.once,
      every = 1,
      unit = RepeatUnit.days,
      weekdays = const {};

  const SimpleRepeat.every(this.every, this.unit)
    : kind = RepeatKind.every,
      weekdays = const {};

  const SimpleRepeat.weekdays(this.weekdays)
    : kind = RepeatKind.weekdays,
      every = 1,
      unit = RepeatUnit.weeks;

  /// The RRULE for this repeat; null for once.
  Recurrence? toRule() => switch (kind) {
    RepeatKind.once => null,
    RepeatKind.every => Recurrence(
      frequency: switch (unit) {
        RepeatUnit.days => Frequency.daily,
        RepeatUnit.weeks => Frequency.weekly,
        RepeatUnit.months => Frequency.monthly,
        RepeatUnit.years => Frequency.yearly,
      },
      interval: every,
    ),
    RepeatKind.weekdays => Recurrence(
      frequency: Frequency.weekly,
      byDay: [for (final d in weekdays.toList()..sort()) WeekdayRule(d)],
    ),
  };

  /// What the simple editor shows for [rule] starting on [start], or null
  /// when only the advanced mode can show it.
  static SimpleRepeat? of(Recurrence? rule, Day start) {
    if (rule == null) return const SimpleRepeat.once();
    if (rule.byMonthDay.isNotEmpty ||
        rule.byMonth.isNotEmpty ||
        rule.byHour.isNotEmpty ||
        rule.byMinute.isNotEmpty ||
        rule.count != null ||
        rule.until != null ||
        rule.weekStart != DateTime.monday ||
        rule.byDay.any((r) => r.ordinal != null)) {
      return null;
    }
    final unit = switch (rule.frequency) {
      Frequency.daily => RepeatUnit.days,
      Frequency.weekly => RepeatUnit.weeks,
      Frequency.monthly => RepeatUnit.months,
      Frequency.yearly => RepeatUnit.years,
    };
    if (rule.byDay.isEmpty) return SimpleRepeat.every(rule.interval, unit);
    if (rule.frequency != Frequency.weekly || rule.interval != 1) return null;
    final weekdays = {for (final r in rule.byDay) r.weekday};
    return SimpleRepeat.weekdays(weekdays);
  }
}

const _weekdayCodes = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU'];

int _monthLength(int year, int month) => DateTime.utc(year, month + 1, 0).day;

String _two(int n) => n.toString().padLeft(2, '0');

String _compact(Day day) => day.toString().replaceAll('-', '');

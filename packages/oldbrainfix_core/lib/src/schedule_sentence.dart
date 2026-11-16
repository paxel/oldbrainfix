/// A schedule of the simple editor as a sentence (ADR 0044): written in
/// the app's language, read back as typed, with the next words to offer
/// while typing. The sentence is never stored; the schedule's fields are.
///
/// The Rust core's `schedule_sentence.rs` writes, reads and suggests the
/// same (fixture `schedule_sentences.json`). Offsets here are UTF-16
/// code units, as a Flutter text field counts them.
library;

import 'schedule.dart';

/// A schedule the simple editor shows: how it repeats, with its end, its
/// first day and its times of day, ascending, none for all day. Unlike a
/// [Schedule] it may be one the editor cannot save yet: weekdays with
/// none picked.
class SimpleSchedule {
  final SimpleRepeat repeat;
  final Day start;
  final List<ClockTime> times;

  SimpleSchedule(this.repeat, this.start, List<ClockTime> times)
    : times = List.unmodifiable({...times}.toList()..sort());

  /// [schedule] as the simple editor shows it; null when only the
  /// advanced mode can.
  static SimpleSchedule? of(Schedule schedule) {
    final repeat = SimpleRepeat.of(schedule.rule, schedule.start);
    if (repeat == null) return null;
    return SimpleSchedule(repeat, schedule.start, schedule.times);
  }

  /// The [Schedule] to store; null while weekdays have none picked or
  /// once has more than one time.
  Schedule? toSchedule() {
    if (repeat.kind == RepeatKind.weekdays && repeat.weekdays.isEmpty) {
      return null;
    }
    if (repeat.kind == RepeatKind.once && times.length > 1) return null;
    return Schedule(start: start, times: times, rule: repeat.toRule());
  }

  @override
  bool operator ==(Object other) =>
      other is SimpleSchedule &&
      other.repeat == repeat &&
      other.start == start &&
      _sameTimes(other.times, times);

  static bool _sameTimes(List<ClockTime> a, List<ClockTime> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i] != b[i]) return false;
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(repeat, start, Object.hashAll(times));

  @override
  String toString() =>
      'SimpleSchedule(${repeat.toRule()?.format() ?? 'once'}, $start, $times)';
}

/// What a sentence read as: the schedule, or null when it names no
/// repeat; and the parts not understood, which changed nothing.
class SentenceRead {
  final SimpleSchedule? schedule;
  final List<({int start, int end})> unknown;

  const SentenceRead(this.schedule, this.unknown);
}

/// The moment a sentence is read at, for its relative words (#271): the
/// day and the time on this Device's wall clock, and when its day starts
/// (`DeviceSettings.dayStart`), at which "tomorrow" rings.
class SentenceNow {
  final Day today;
  final ClockTime time;
  final ClockTime dayStart;

  const SentenceNow(this.today, this.time, this.dayStart);

  /// At [wall], a time on the wall clock, to the minute.
  factory SentenceNow.at(DateTime wall, ClockTime dayStart) =>
      SentenceNow(Day.of(wall), ClockTime(wall.hour, wall.minute), dayStart);

  @override
  bool operator ==(Object other) =>
      other is SentenceNow &&
      other.today == today &&
      other.time == time &&
      other.dayStart == dayStart;

  @override
  int get hashCode => Object.hash(today, time, dayStart);

  @override
  String toString() => 'SentenceNow($today $time, day from $dayStart)';
}

/// The chips over a new Reminder's schedule (#271), in their order.
enum QuickChip { inAnHour, tonight, tomorrow, inAWeek }

/// A chip with the day and time it sets.
class QuickReminder {
  final QuickChip chip;
  final Day day;
  final ClockTime time;

  const QuickReminder(this.chip, this.day, this.time);

  /// The Reminder the chip sets.
  SimpleSchedule get schedule =>
      SimpleSchedule(const SimpleRepeat.once(), day, [time]);

  @override
  bool operator ==(Object other) =>
      other is QuickReminder &&
      other.chip == chip &&
      other.day == day &&
      other.time == time;

  @override
  int get hashCode => Object.hash(chip, day, time);

  @override
  String toString() => 'QuickReminder(${chip.name}, $day $time)';
}

/// "Tonight", and "this evening" after a day.
final _tonight = ClockTime(19, 0);

/// [minutes] after [now], counted on the wall clock, the day carried.
({Day day, ClockTime time}) _later(SentenceNow now, int minutes) {
  final total = now.time.hour * 60 + now.time.minute + minutes;
  return (
    day: now.today.addDays(total ~/ (24 * 60)),
    time: ClockTime(total ~/ 60 % 24, total % 60),
  );
}

/// The chips' Reminders at [now], as the words read them: "in 1 hour",
/// "tonight" while 19:00 is still to come, "tomorrow" and "in 1 week",
/// these two at [SentenceNow.dayStart].
List<QuickReminder> quickReminders(SentenceNow now) {
  final hour = _later(now, 60);
  return [
    QuickReminder(QuickChip.inAnHour, hour.day, hour.time),
    if (now.time.compareTo(_tonight) < 0)
      QuickReminder(QuickChip.tonight, now.today, _tonight),
    QuickReminder(QuickChip.tomorrow, now.today.addDays(1), now.dayStart),
    QuickReminder(QuickChip.inAWeek, now.today.addDays(7), now.dayStart),
  ];
}

/// A word to put in for the text from [start] to [end].
class SentenceSuggestion {
  final int start;
  final int end;
  final String text;

  const SentenceSuggestion(this.start, this.end, this.text);

  @override
  String toString() => 'SentenceSuggestion($start, $end, $text)';
}

/// [schedule] as a sentence in [locale] (`en` or `de`, `de_DE` counts
/// as `de`), written on [today]: its days without the year when they lie
/// in [today]'s, its start left out when it is [today].
String scheduleSentence(SimpleSchedule schedule, String locale, Day today) {
  final w = _Words.of(locale);
  final repeat = schedule.repeat;
  String date(Day day) => w.date(day, today);
  final out = StringBuffer(switch (repeat.kind) {
    RepeatKind.once => '${w.onceHead} ${w.on} ${date(schedule.start)}',
    RepeatKind.every => w.every(repeat.every, repeat.unit),
    RepeatKind.weekdays => [
      w.weekdaysHead,
      if (repeat.weekdays.isNotEmpty)
        [for (final d in repeat.weekdays.toList()..sort()) w.weekdays[d - 1]]
            .join(', '),
    ].join(' '),
    RepeatKind.monthDay =>
      '${w.every(repeat.every, RepeatUnit.months)} '
          '${w.monthDay(repeat.monthDay ?? schedule.start.day)}',
    RepeatKind.monthWeekday =>
      '${w.every(repeat.every, RepeatUnit.months)} '
          '${w.nth(repeat.nth ?? WeekdayRule(schedule.start.weekday, 1))}',
  });
  if (schedule.times.isNotEmpty) {
    out.write(' ${w.at} ${schedule.times.join(', ')}');
  }
  if (repeat.kind != RepeatKind.once) {
    if (schedule.start != today) {
      out.write(', ${w.from} ${date(schedule.start)}');
    }
    if (repeat.count case final count?) {
      out.write(', ${count == 1 ? w.countOnce : w.count(count)}');
    } else if (repeat.until case final until?) {
      out.write(', ${w.until} ${date(until)}');
    }
  }
  return out.toString();
}

/// Reads [text] in [locale] on [today]: what is left out is all day,
/// from [today], never ending, and a day without its year lies in
/// [today]'s. Parts not understood are skipped. The relative words are
/// not understood here; [readScheduleAt] reads them.
SentenceRead readSchedule(String text, String locale, Day today) =>
    _read(text, locale, today, null);

/// Reads [text] in [locale] at [now] as [readSchedule] does on its day,
/// and also a one-time Reminder in relative words (#271): "today",
/// "tonight", "tomorrow", "in 2 hours" and in German "heute",
/// "heute Abend", "morgen", "in 2 Stunden".
SentenceRead readScheduleAt(String text, String locale, SentenceNow now) =>
    _read(text, locale, now.today, now);

SentenceRead _read(String text, String locale, Day today, SentenceNow? now) {
  final w = _Words.of(locale);
  final parser = _Parser(w, today, now);
  for (final token in _tokens(text, 0, w.german)) {
    parser.take(token);
  }
  parser.finish();
  return SentenceRead(parser.schedule(), [
    for (final t in parser.unknown..sort((a, b) => a.start - b.start))
      (start: t.start, end: t.end),
  ]);
}

/// The words that may come at [cursor] in [text] in [locale]: those that
/// start with the word being typed there, each replacing it.
List<SentenceSuggestion> suggestSchedule(
  String text,
  int cursor,
  String locale,
) {
  final w = _Words.of(locale);
  var start = cursor;
  while (start > 0 && _inWord(text.codeUnitAt(start - 1))) {
    start--;
  }
  final typed = text.substring(start, cursor);
  final noon = ClockTime(12, 0);
  final parser = _Parser(
    w,
    Day(2000, 1, 1),
    SentenceNow(Day(2000, 1, 1), noon, noon),
  );
  for (final token in _tokens(text.substring(0, start), 0, w.german)) {
    parser.take(token);
  }
  // "8:3" is typed for "08:30".
  final prefix = (RegExp(r'^\d(:|$)').hasMatch(typed) ? '0$typed' : typed)
      .toLowerCase();
  final seen = <String>{};
  return [
    for (final word in parser.expected(typed))
      if (word.toLowerCase().startsWith(prefix) &&
          word.toLowerCase() != prefix &&
          seen.add(word))
        SentenceSuggestion(start, cursor, word),
  ];
}

/// Whether [unit] continues a word being typed: a letter, a digit, or a
/// dot, colon or slash inside a day, a time or an abbreviation.
bool _inWord(int unit) =>
    _isLetter(unit) ||
    _isDigit(unit) ||
    unit == 0x2e ||
    unit == 0x3a ||
    unit == 0x2f;

bool _isDigit(int c) => c >= 0x30 && c <= 0x39;

/// A Latin letter, with the accented ones of Latin-1 and Latin
/// Extended-A and -B; other scripts are no words of the sentence.
bool _isLetter(int c) =>
    (c >= 0x41 && c <= 0x5a) ||
    (c >= 0x61 && c <= 0x7a) ||
    (c >= 0xc0 && c <= 0x24f && c != 0xd7 && c != 0xf7);

/// Whitespace and the hyphen of "10-mal" part words.
bool _isSpace(int c) => _isBlank(c) || c == 0x2d;

bool _isBlank(int c) => c <= 0x20 || c == 0xa0 || c == 0x2009 || c == 0x202f;

/// The end of the "am" or "pm" (English) or "Uhr" (German) that follows
/// a time or an hour ending at [i], blanks before it, with its dot; null
/// when none does.
int? _suffixEnd(String text, int i, bool german) {
  var j = i;
  while (j < text.length && _isBlank(text.codeUnitAt(j))) {
    j++;
  }
  final from = j;
  while (j < text.length && _isLetter(text.codeUnitAt(j))) {
    j++;
  }
  final word = text.substring(from, j).toLowerCase();
  if (!(german ? word == 'uhr' : word == 'am' || word == 'pm')) return null;
  if (j < text.length && text.codeUnitAt(j) == 0x2e) j++;
  return j;
}

enum _Kind { word, number, time, date, comma, other }

class _Token {
  final _Kind kind;
  final String text;
  final int start;
  final int end;

  const _Token(this.kind, this.text, this.start, this.end);
}

/// The tokens of [text], their offsets counted from [base]; an hour or a
/// time with its "pm" or "Uhr" is one time.
List<_Token> _tokens(String text, int base, bool german) {
  final tokens = <_Token>[];
  var i = 0;
  int at(int j) => j < text.length ? text.codeUnitAt(j) : -1;
  while (i < text.length) {
    final c = text.codeUnitAt(i);
    final from = i;
    _Kind kind;
    if (_isSpace(c)) {
      i++;
      continue;
    } else if (c == 0x2c || c == 0x3b) {
      i++;
      kind = _Kind.comma;
    } else if (_isLetter(c)) {
      while (_isLetter(at(i))) {
        i++;
      }
      if (at(i) == 0x2e) i++;
      kind = _Kind.word;
    } else if (_isDigit(c)) {
      while (_isDigit(at(i))) {
        i++;
      }
      kind = _Kind.number;
      final sep = at(i);
      if (sep == 0x3a && _isDigit(at(i + 1))) {
        i++;
        while (_isDigit(at(i))) {
          i++;
        }
        kind = _Kind.time;
      } else if ((sep == 0x2f || sep == 0x2e) && _isDigit(at(i + 1))) {
        i++;
        while (_isDigit(at(i))) {
          i++;
        }
        if (at(i) == sep && _isDigit(at(i + 1))) {
          i++;
          while (_isDigit(at(i))) {
            i++;
          }
        }
        if (sep == 0x2e && at(i) == 0x2e) i++;
        kind = _Kind.date;
      } else if (sep == 0x2e) {
        // "am 15." is the 15th.
        i++;
      }
      if (kind == _Kind.time || (kind == _Kind.number && sep != 0x2e)) {
        if (_suffixEnd(text, i, german) case final end?) {
          i = end;
          kind = _Kind.time;
        }
      }
    } else {
      // One code point that is no part of a sentence.
      i += c >= 0xd800 && c <= 0xdbff && i + 1 < text.length ? 2 : 1;
      kind = _Kind.other;
    }
    tokens.add(_Token(kind, text.substring(from, i), base + from, base + i));
  }
  return tokens;
}

/// The words of one language. Weekday names are the app's own: the
/// phone's short ones (`Mo`, as on its weekday chips) and its long ones;
/// a word is matched without case and without a trailing dot, so the
/// desktop's `Mo.` reads too.
class _Words {
  final bool german;

  const _Words(this.german);

  factory _Words.of(String locale) => _Words(
    locale == 'de' || locale.startsWith('de_') || locale.startsWith('de-'),
  );

  String get onceHead => german ? 'Einmal' : 'Once';
  String get weekdaysHead => german ? 'Wochentags' : 'Weekdays';
  String get on => german ? 'am' : 'on';
  String get at => german ? 'um' : 'at';
  String get from => german ? 'ab' : 'from';
  String get until => german ? 'bis' : 'until';
  String get countOnce => german ? 'einmal' : 'once';
  String get timesWord => german ? 'mal' : 'times';

  List<String> get weekdays => german
      ? const ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So']
      : const ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  List<String> get weekdaysLong => german
      ? const [
          'Montag',
          'Dienstag',
          'Mittwoch',
          'Donnerstag',
          'Freitag',
          'Samstag',
          'Sonntag',
        ]
      : const [
          'Monday',
          'Tuesday',
          'Wednesday',
          'Thursday',
          'Friday',
          'Saturday',
          'Sunday',
        ];

  /// The ordinals 1 to 4 and the last, in that order.
  List<String> get ordinals => german
      ? const ['ersten', 'zweiten', 'dritten', 'vierten', 'letzten']
      : const ['first', 'second', 'third', 'fourth', 'last'];

  /// Each unit's word for one, then for more.
  List<String> get units => german
      ? const ['Tag', 'Woche', 'Monat', 'Jahr']
      : const ['day', 'week', 'month', 'year'];
  List<String> get unitsMany => german
      ? const ['Tage', 'Wochen', 'Monate', 'Jahre']
      : const ['days', 'weeks', 'months', 'years'];

  /// The words that start every N units: "Every", or German's
  /// "Jeden", "Jede", "Jedes" and "Alle".
  List<String> get everyHeads =>
      german ? const ['Jeden', 'Jede', 'Jedes', 'Alle'] : const ['Every'];
  List<String> get weekdaysHeads =>
      german ? const ['Wochentags', 'Immer'] : const ['Weekdays'];
  List<String> get heads => [onceHead, ...everyHeads, weekdaysHead];

  /// The relative words a one-time Reminder starts with (#271).
  String get today => german ? 'heute' : 'today';
  String get tomorrow => german ? 'morgen' : 'tomorrow';
  List<String> get relativeHeads => german
      ? const ['Heute', 'Morgen', 'In']
      : const ['Today', 'Tonight', 'Tomorrow', 'In'];

  /// The words after a day for its evening, the first one suggested.
  List<String> get evenings =>
      german ? const ['Abend'] : const ['evening', 'night'];

  /// The words for one after "in".
  List<String> get articles =>
      german ? const ['eine', 'einer', 'einem', 'einen'] : const ['a', 'an'];

  /// The units after "in N", for one and for more, as suggested.
  List<String> get relativeUnits => german
      ? const ['Minute', 'Stunde', 'Tag', 'Woche']
      : const ['minute', 'hour', 'day', 'week'];
  List<String> get relativeUnitsMany => german
      ? const ['Minuten', 'Stunden', 'Tagen', 'Wochen']
      : const ['minutes', 'hours', 'days', 'weeks'];

  /// The unit after "in N" [word] names, 0 to 3 for minutes, hours,
  /// days and weeks, in the singular, the plural or short; null for
  /// none.
  int? relativeUnit(String word) => switch ((german, _plain(word))) {
    (false, 'minute' || 'minutes' || 'min' || 'mins') => 0,
    (false, 'hour' || 'hours' || 'h' || 'hr' || 'hrs') => 1,
    (false, 'day' || 'days') => 2,
    (false, 'week' || 'weeks') => 3,
    (true, 'minute' || 'minuten' || 'min') => 0,
    (true, 'stunde' || 'stunden' || 'std' || 'h') => 1,
    (true, 'tag' || 'tage' || 'tagen') => 2,
    (true, 'woche' || 'wochen') => 3,
    _ => null,
  };

  String every(int n, RepeatUnit unit) {
    final i = unit.index;
    if (!german) {
      return n == 1 ? 'Every ${units[i]}' : 'Every $n ${unitsMany[i]}';
    }
    if (n != 1) return 'Alle $n ${unitsMany[i]}';
    return '${const ['Jeden', 'Jede', 'Jeden', 'Jedes'][i]} ${units[i]}';
  }

  String monthDay(int day) => german ? 'am $day.' : 'on day $day';

  String nth(WeekdayRule rule) {
    final ordinal =
        ordinals[switch (rule.ordinal) {
          final o? when o >= 1 && o <= 4 => o - 1,
          _ => 4,
        }];
    final weekday = weekdaysLong[rule.weekday - 1];
    return german ? 'am $ordinal $weekday' : 'on the $ordinal $weekday';
  }

  String count(int n) => german ? '$n-mal' : '$n times';

  /// [day] in the locale's numbers, its year only when not [today]'s.
  String date(Day day, Day today) {
    final year = day.year == today.year ? null : day.year;
    if (german) return '${day.day}.${day.month}.${year ?? ''}';
    return '${day.month}/${day.day}${year == null ? '' : '/$year'}';
  }

  static String _plain(String word) {
    final lower = word.toLowerCase();
    return lower.endsWith('.') ? lower.substring(0, lower.length - 1) : lower;
  }

  /// The index of [word] among [words]; -1 when it is none of them.
  static int find(String word, List<String> words) {
    final plain = _plain(word);
    for (var i = 0; i < words.length; i++) {
      if (_plain(words[i]) == plain) return i;
    }
    return -1;
  }

  bool isWord(String word, String expected) => find(word, [expected]) == 0;

  /// The unit [word] names, for one or more; null for none.
  RepeatUnit? unit(String word) {
    var i = find(word, units);
    if (i < 0) i = find(word, unitsMany);
    return i < 0 ? null : RepeatUnit.values[i];
  }

  /// `DateTime.monday` to `DateTime.sunday` for a short or long name.
  int? weekday(String word) {
    var i = find(word, weekdays);
    if (i < 0) i = find(word, weekdaysLong);
    return i < 0 ? null : i + 1;
  }

  /// 1 to 4, or -1 for the last.
  int? ordinal(String word) {
    final i = find(word, ordinals);
    return i < 0 ? null : (i == 4 ? -1 : i + 1);
  }
}

/// Where the reader is in the sentence (ADR 0044).
enum _State {
  /// Before the repeat.
  start,

  /// After "Once"; "on" and a day may follow.
  once,
  onceOn,

  /// After "Every": a unit, a number or a weekday.
  every,
  everyN,

  /// After every N months: "on" a day or a weekday may follow.
  months,
  monthOn,
  monthOnDay,
  monthOnThe,
  monthOrd,

  /// After "today" or "tomorrow", "evening" or a time may follow; after
  /// the other relative words a time (#271).
  relativeDay,
  relative,

  /// After "in" and after its number: a number, then a unit.
  inRelative,
  inRelativeN,

  /// Weekdays, more of them may follow.
  weekdays,
  weekdayAnd,
  weekdayComma,

  /// After the repeat: times, the start and the end.
  body,
  times0,
  times,
  timesAnd,
  timesComma,
  fromDate,
  untilDate,
  countN,
}

/// Reads a sentence token by token. A state that waits for more
/// ([_pending]) falls back when the next token does not fit, its tokens
/// marked unknown, and that token is read again from the fallback.
class _Parser {
  final _Words w;
  final Day today;

  /// Now, for the relative words; without it they are not understood.
  final SentenceNow? now;

  _State state = _State.start;

  /// Whether a repeat was read.
  bool headDone = false;
  RepeatKind kind = RepeatKind.once;
  int every = 1;
  RepeatUnit unit = RepeatUnit.days;
  final weekdays = <int>{};
  int? monthDay;
  WeekdayRule? nth;
  int? count;
  Day? until;
  Day? start;
  final times = <ClockTime>[];

  /// The time of a relative day when none is typed: 19:00 "tonight",
  /// the day start "tomorrow"; none for all day.
  ClockTime? dayTime;

  /// The number before "times", and the ordinal before its weekday.
  int pendingNumber = 0;
  int pendingOrdinal = 0;

  /// The tokens of the part being read that is not finished yet.
  final pending = <_Token>[];
  final unknown = <_Token>[];

  _Parser(this.w, this.today, this.now);

  static const _waiting = {
    _State.onceOn,
    _State.every,
    _State.everyN,
    _State.monthOn,
    _State.monthOnDay,
    _State.monthOnThe,
    _State.monthOrd,
    _State.inRelative,
    _State.inRelativeN,
    _State.weekdayAnd,
    _State.times0,
    _State.timesAnd,
    _State.fromDate,
    _State.untilDate,
    _State.countN,
  };

  /// Where a waiting state falls back to.
  _State _fallback(_State s) => switch (s) {
    _State.every ||
    _State.everyN ||
    _State.inRelative ||
    _State.inRelativeN => _State.start,
    _State.monthOn ||
    _State.monthOnDay ||
    _State.monthOnThe ||
    _State.monthOrd => _State.months,
    _State.weekdayAnd => _State.weekdays,
    _State.onceOn => _State.once,
    _ => _State.body,
  };

  void take(_Token token) {
    if (_step(token)) return;
    if (_waiting.contains(state)) {
      unknown.addAll(pending);
      pending.clear();
      state = _fallback(state);
      take(token);
      return;
    }
    unknown.add(token);
  }

  /// Marks what is left unfinished.
  void finish() {
    if (_waiting.contains(state)) unknown.addAll(pending);
    pending.clear();
  }

  /// Waits for more in [next], [token] being part of it.
  bool _wait(_State next, _Token token) {
    pending.add(token);
    state = next;
    return true;
  }

  /// Done with the part being read, which ends with [token]; [to] next.
  bool _done(_State to) {
    pending.clear();
    state = to;
    return true;
  }

  int? _number(_Token t) {
    if (t.kind != _Kind.number) return null;
    final digits = t.text.endsWith('.')
        ? t.text.substring(0, t.text.length - 1)
        : t.text;
    if (digits.length > 5) return null;
    return int.parse(digits);
  }

  /// `08:00`, `8:00`, and `7 pm`, `7:30pm` or `19 Uhr`.
  ClockTime? _time(_Token t) {
    if (t.kind != _Kind.time) return null;
    final text = t.text;
    var i = 0;
    while (i < text.length && _isDigit(text.codeUnitAt(i))) {
      i++;
    }
    final h = text.substring(0, i);
    var m = '00';
    if (i < text.length && text.codeUnitAt(i) == 0x3a) {
      final from = ++i;
      while (i < text.length && _isDigit(text.codeUnitAt(i))) {
        i++;
      }
      m = text.substring(from, i);
    }
    final suffix = text
        .substring(i)
        .replaceAll(RegExp('[^A-Za-z]'), '')
        .toLowerCase();
    if (h.length > 2 || m.length != 2) return null;
    var hour = int.parse(h);
    final minute = int.parse(m);
    if (minute > 59) return null;
    if (suffix == 'am' || suffix == 'pm') {
      if (hour < 1 || hour > 12) return null;
      hour = hour % 12 + (suffix == 'pm' ? 12 : 0);
    }
    if (hour > 23) return null;
    return ClockTime(hour, minute);
  }

  Day? _date(_Token t) {
    if (t.kind != _Kind.date) return null;
    final dots = t.text.contains('.');
    final body = dots && t.text.endsWith('.')
        ? t.text.substring(0, t.text.length - 1)
        : t.text;
    final parts = body.split(dots ? '.' : '/');
    if (parts[0].length > 2 || parts[1].length > 2) return null;
    final a = int.parse(parts[0]);
    final b = int.parse(parts[1]);
    var year = today.year;
    if (parts.length == 3) {
      final y = parts[2];
      if (y.length == 2) {
        year = 2000 + int.parse(y);
      } else if (y.length == 4) {
        year = int.parse(y);
      } else {
        return null;
      }
    }
    final (month, day) = dots ? (b, a) : (a, b);
    final date = Day(year, month, day);
    return date.year == year && date.month == month && date.day == day
        ? date
        : null;
  }

  bool _word(_Token t, String expected) =>
      t.kind == _Kind.word && w.isWord(t.text, expected);

  bool _anyWord(_Token t, List<String> words) =>
      t.kind == _Kind.word && _Words.find(t.text, words) >= 0;

  int? _weekday(_Token t) => t.kind == _Kind.word ? w.weekday(t.text) : null;

  /// Takes [t] in the current state; false when it does not fit there.
  bool _step(_Token t) {
    switch (state) {
      case _State.start:
        if (_word(t, w.onceHead)) {
          headDone = true;
          kind = RepeatKind.once;
          return _done(_State.once);
        }
        if (_anyWord(t, w.everyHeads)) return _wait(_State.every, t);
        if (_anyWord(t, w.weekdaysHeads)) {
          headDone = true;
          kind = RepeatKind.weekdays;
          return _done(_State.weekdays);
        }
        return _relativeHead(t);
      case _State.relativeDay:
        if (_anyWord(t, w.evenings)) {
          dayTime = _tonight;
          return _done(_State.relative);
        }
        return _relative(t);
      case _State.relative:
        return _relative(t);
      case _State.inRelative:
        final n = _number(t) ?? (_anyWord(t, w.articles) ? 1 : null);
        if (n != null && n >= 1) {
          pendingNumber = n;
          return _wait(_State.inRelativeN, t);
        }
        return false;
      case _State.inRelativeN:
        final unit = t.kind == _Kind.word ? w.relativeUnit(t.text) : null;
        return unit != null && _inUnits(pendingNumber, unit);
      case _State.once:
        if (_word(t, w.on)) return _wait(_State.onceOn, t);
        if (_relativeDayWord(t)) return true;
        if (_date(t) case final day?) {
          start = day;
          return _done(_State.body);
        }
        return _body(t);
      case _State.onceOn:
        if (_relativeDayWord(t)) return true;
        if (_date(t) case final day?) {
          start = day;
          return _done(_State.body);
        }
        return false;
      case _State.every:
        if (t.kind == _Kind.word) {
          if (w.unit(t.text) case final u?) return _unit(1, u);
          if (w.weekday(t.text) case final d?) {
            headDone = true;
            kind = RepeatKind.weekdays;
            weekdays.add(d);
            return _done(_State.weekdays);
          }
        }
        if (_number(t) case final n? when n >= 1) {
          every = n;
          return _wait(_State.everyN, t);
        }
        return false;
      case _State.everyN:
        if (t.kind == _Kind.word) {
          if (w.unit(t.text) case final u?) return _unit(every, u);
        }
        return false;
      case _State.months:
        if (_word(t, w.on)) return _wait(_State.monthOn, t);
        return _body(t);
      case _State.monthOn:
        if (!w.german && _word(t, 'day')) return _wait(_State.monthOnDay, t);
        if (!w.german && _word(t, 'the')) return _wait(_State.monthOnThe, t);
        if (t.kind == _Kind.word) {
          if (w.ordinal(t.text) case final o?) {
            pendingOrdinal = o;
            return _wait(_State.monthOrd, t);
          }
        }
        if (w.german) return _monthDay(t);
        return false;
      case _State.monthOnDay:
        return _monthDay(t);
      case _State.monthOnThe:
        if (t.kind == _Kind.word) {
          if (w.ordinal(t.text) case final o?) {
            pendingOrdinal = o;
            return _wait(_State.monthOrd, t);
          }
        }
        return false;
      case _State.monthOrd:
        if (_weekday(t) case final d?) {
          kind = RepeatKind.monthWeekday;
          nth = WeekdayRule(d, pendingOrdinal);
          return _done(_State.body);
        }
        return false;
      case _State.weekdays:
        if (_weekday(t) case final d?) {
          weekdays.add(d);
          return _done(_State.weekdays);
        }
        if (_word(t, w.german ? 'und' : 'and')) {
          return _wait(_State.weekdayAnd, t);
        }
        if (t.kind == _Kind.comma) return _done(_State.weekdayComma);
        return _body(t);
      case _State.weekdayAnd:
      case _State.weekdayComma:
        if (_weekday(t) case final d?) {
          weekdays.add(d);
          return _done(_State.weekdays);
        }
        if (state == _State.weekdayAnd) return false;
        return _body(t);
      case _State.body:
        return _body(t);
      case _State.times0:
      case _State.timesAnd:
        if (_time(t) case final time?) {
          _addTime(t, time);
          return _done(_State.times);
        }
        return false;
      case _State.times:
        if (_time(t) case final time?) {
          _addTime(t, time);
          return _done(_State.times);
        }
        if (t.kind == _Kind.comma) return _done(_State.timesComma);
        if (_word(t, w.german ? 'und' : 'and')) {
          return _wait(_State.timesAnd, t);
        }
        return _body(t);
      case _State.timesComma:
        if (_time(t) case final time?) {
          _addTime(t, time);
          return _done(_State.times);
        }
        return _body(t);
      case _State.fromDate:
        if (_date(t) case final day?) {
          start = day;
          return _done(_State.body);
        }
        return false;
      case _State.untilDate:
        if (_date(t) case final day?) {
          if (kind == RepeatKind.once) return _refuse(t);
          until = day;
          count = null;
          return _done(_State.body);
        }
        return false;
      case _State.countN:
        if (_word(t, w.timesWord) || (!w.german && _word(t, 'time'))) {
          if (kind == RepeatKind.once) return _refuse(t);
          count = pendingNumber;
          until = null;
          return _done(_State.body);
        }
        return false;
    }
  }

  /// A relative word a Reminder starts with; false for none, and while
  /// there is no now to read it at.
  bool _relativeHead(_Token t) {
    if (_relativeDayWord(t)) return true;
    if (now != null && _word(t, 'in')) return _wait(_State.inRelative, t);
    return false;
  }

  /// "today", "tonight" or "tomorrow", at the start or as the day of
  /// Once; false for none, and while there is no now.
  bool _relativeDayWord(_Token t) {
    final now = this.now;
    if (now == null) return false;
    if (_word(t, w.today)) return _relativeDay(now.today, null, true);
    if (!w.german && _word(t, 'tonight')) {
      return _relativeDay(now.today, _tonight, false);
    }
    if (_word(t, w.tomorrow)) {
      return _relativeDay(now.today.addDays(1), now.dayStart, true);
    }
    return false;
  }

  /// Once on [day], at [time] unless one is typed; "evening" may follow
  /// when [evening].
  bool _relativeDay(Day day, ClockTime? time, bool evening) {
    headDone = true;
    kind = RepeatKind.once;
    start = day;
    dayTime = time;
    return _done(evening ? _State.relativeDay : _State.relative);
  }

  /// Once in [n] of the [unit]th of minutes, hours, days and weeks: at
  /// the moment for minutes and hours, at the day start for days and
  /// weeks unless a time is typed.
  bool _inUnits(int n, int unit) {
    final now = this.now;
    if (now == null) return false;
    if (unit < 2) {
      final later = _later(now, unit == 0 ? n : n * 60);
      times.add(later.time);
      return _relativeDay(later.day, null, false);
    }
    return _relativeDay(
      now.today.addDays(unit == 2 ? n : n * 7),
      now.dayStart,
      false,
    );
  }

  /// After a relative word: a time without "at", or what follows a
  /// repeat.
  bool _relative(_Token t) {
    if (_time(t) case final time?) {
      _addTime(t, time);
      return _done(_State.times);
    }
    return _body(t);
  }

  /// The repeat is every [n] [u]s.
  bool _unit(int n, RepeatUnit u) {
    headDone = true;
    kind = RepeatKind.every;
    every = n;
    unit = u;
    return _done(u == RepeatUnit.months ? _State.months : _State.body);
  }

  bool _monthDay(_Token t) {
    if (_number(t) case final d? when d >= 1 && d <= 31) {
      kind = RepeatKind.monthDay;
      monthDay = d;
      return _done(_State.body);
    }
    return false;
  }

  /// A part once cannot have: marked with its tokens, read past.
  bool _refuse(_Token t) {
    unknown
      ..addAll(pending)
      ..add(t);
    return _done(_State.body);
  }

  void _addTime(_Token t, ClockTime time) {
    if (kind == RepeatKind.once && times.isNotEmpty && !times.contains(time)) {
      unknown.add(t);
      return;
    }
    times.add(time);
  }

  /// What may follow the repeat.
  bool _body(_Token t) {
    if (!headDone) return false;
    if (t.kind == _Kind.comma) return _done(_State.body);
    if (_word(t, w.at)) return _wait(_State.times0, t);
    if (_word(t, w.from)) return _wait(_State.fromDate, t);
    if (_word(t, w.until)) return _wait(_State.untilDate, t);
    if (_word(t, w.countOnce)) {
      if (kind == RepeatKind.once) return _refuse(t);
      count = 1;
      until = null;
      return _done(_State.body);
    }
    if (_number(t) case final n? when n >= 1) {
      pendingNumber = n;
      return _wait(_State.countN, t);
    }
    return false;
  }

  /// The words that may come next, for [typed] being typed.
  List<String> expected(String typed) {
    // Once has no end; a second "at" adds to the times already read.
    final body = [
      if (times.isEmpty) w.at,
      if (kind != RepeatKind.once) ...[w.from, w.until],
    ];
    final picked = [
      for (var d = 1; d <= 7; d++)
        if (!weekdays.contains(d)) w.weekdays[d - 1],
    ];
    return switch (state) {
      _State.start => [...w.heads, ...w.relativeHeads],
      _State.relativeDay => [w.evenings.first, ...body],
      _State.relative => body,
      _State.inRelative => const [],
      _State.inRelativeN =>
        pendingNumber == 1 ? w.relativeUnits : w.relativeUnitsMany,
      _State.once => [w.on, w.at],
      _State.onceOn || _State.fromDate || _State.untilDate => const [],
      _State.monthOnDay => const [],
      _State.every => [...w.units, ...w.weekdays],
      _State.everyN => w.unitsMany,
      _State.months => [w.on, ...body],
      _State.monthOn => w.german ? w.ordinals : const ['day', 'the'],
      _State.monthOnThe => w.ordinals,
      _State.monthOrd => w.weekdaysLong,
      _State.weekdays || _State.weekdayComma => [...picked, ...body],
      _State.weekdayAnd => picked,
      _State.body || _State.times => body,
      _State.times0 || _State.timesAnd => _times(typed),
      _State.timesComma => [..._times(typed), ...body],
      // Once has no end.
      _State.countN => kind == RepeatKind.once ? const [] : [w.timesWord],
    };
  }

  /// Times for [typed]: its hour on the hour and the half, or three
  /// times of the day.
  static List<String> _times(String typed) {
    final match = RegExp(r'^(\d{1,2})(:\d{0,2})?$').firstMatch(typed);
    if (match == null) return const ['08:00', '12:00', '18:00'];
    final hour = int.tryParse(match[1] ?? '');
    if (hour == null || hour > 23) return const [];
    final hh = hour.toString().padLeft(2, '0');
    return ['$hh:00', '$hh:30'];
  }

  /// The schedule read; null when no repeat was.
  SimpleSchedule? schedule() {
    if (!headDone) return null;
    final repeat = switch (kind) {
      RepeatKind.once => const SimpleRepeat.once(),
      RepeatKind.every => SimpleRepeat.every(
        every,
        unit,
        count: count,
        until: until,
      ),
      RepeatKind.weekdays => SimpleRepeat.weekdays(
        {...weekdays},
        count: count,
        until: until,
      ),
      RepeatKind.monthDay => SimpleRepeat.monthDay(
        monthDay ?? 1,
        every: every,
        count: count,
        until: until,
      ),
      RepeatKind.monthWeekday => SimpleRepeat.monthWeekday(
        nth ?? const WeekdayRule(1, 1),
        every: every,
        count: count,
        until: until,
      ),
    };
    final dayTime = this.dayTime;
    return SimpleSchedule(
      repeat,
      start ?? today,
      times.isEmpty && dayTime != null ? [dayTime] : times,
    );
  }
}

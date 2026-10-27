import 'places.dart';

/// The lead-in Whisper reads before it transcribes a voice recording
/// (whisper.cpp's initial prompt), one per language it can hear: words
/// it should expect, so "Unsorted" or "Meeting" in German speech are not
/// written in German spelling. Whisper picks English or German by the
/// recording and takes the matching one; an empty one leads in nothing.
///
/// Whisper keeps only the last 224 tokens of a prompt and drops the
/// start, so a prompt stays under [maxLength] characters and lists at
/// most [maxNames] names.
///
/// On silent or very short recordings Whisper may write out its prompt
/// instead of what it heard; [guard] takes that out of a Transcript.
class TranscriptPrompts {
  /// For a recording in English: the Area and Topic names.
  final String english;

  /// For a recording in German: [germanLeadIn], the app's words and the
  /// Area and Topic names.
  final String german;

  /// The Area and Topic names both prompts list, as they stand there.
  final List<String> names;

  /// The app's words the German prompt lists, as they stand there.
  final List<String> terms;

  const TranscriptPrompts({
    this.english = '',
    this.german = '',
    this.names = const [],
    this.terms = const [],
  });

  /// No lead-in at all.
  static const none = TranscriptPrompts();

  /// German with the English words German speech is full of, as a
  /// sentence, since Whisper follows the style of what it reads.
  static const germanLeadIn =
      'Notiz auf Deutsch mit englischen Wörtern: Nach dem Meeting das '
      'Update ans Team, Feedback vor der Deadline, Call und Termin am '
      'Montag, Shopping, Download, Ticket, Event, Homeoffice.';

  /// The most characters (Unicode code points) a prompt has: well under
  /// Whisper's 224 tokens even for German, at about three characters a
  /// token.
  static const maxLength = 400;

  /// The most Area and Topic names a prompt lists.
  static const maxNames = 40;

  /// The prompts for [names] (Area and Topic names, the most wanted
  /// first) and [terms] (the app's own words, such as Unsorted, as the
  /// user reads them). Names are trimmed, empty ones and repeats (in any
  /// case) left out; a name that no longer fits is left out and the next
  /// one tried.
  factory TranscriptPrompts.of({
    required Iterable<String> names,
    Iterable<String> terms = const [],
  }) {
    final cleanTerms = [
      for (final term in terms)
        if (term.trim().isNotEmpty) term.trim(),
    ];
    final german = [
      germanLeadIn,
      if (cleanTerms.isNotEmpty) '${cleanTerms.join(', ')}.',
    ].join(' ');
    // The German prompt is the longer one: what fits there fits in both.
    final listed = _fit(names, maxLength - _length(german) - 2);
    if (listed.isEmpty) {
      return TranscriptPrompts(german: german, terms: cleanTerms);
    }
    final list = '${listed.join(', ')}.';
    return TranscriptPrompts(
      english: list,
      german: '$german $list',
      names: listed,
      terms: cleanTerms,
    );
  }

  /// The prompts for the Areas and Topics in [places]: the Areas first,
  /// then their Topics, each as [areaName] and [topicName] show it
  /// (default: the stored name).
  factory TranscriptPrompts.forPlaces(
    Places places, {
    Iterable<String> terms = const [],
    String Function(Area area)? areaName,
    String Function(Topic topic)? topicName,
  }) {
    final areas = places.areas();
    return TranscriptPrompts.of(
      names: [
        for (final area in areas) areaName?.call(area) ?? area.name,
        for (final area in areas)
          for (final topic in places.topics(area.id))
            topicName?.call(topic) ?? topic.name,
      ],
      terms: terms,
    );
  }

  /// [transcript] without what Whisper wrote out of these prompts rather
  /// than heard: every repeat of [germanLeadIn], whole or from its start
  /// at least through "Wörtern:", and every run of three or more of the
  /// listed [names] and [terms] one after another. A Transcript that is
  /// nothing but the listed names, two or more, in their order, goes
  /// whole. Words are compared in any case and without the punctuation
  /// around them, a name of several words as a whole. A lone name, or two
  /// amid speech, stays: real speech names places too.
  ///
  /// With nothing taken out, [transcript] comes back as it is; otherwise
  /// what is left of it, its words joined by single spaces, or empty when
  /// no word is left.
  String guard(String transcript) {
    // Each word with the punctuation alone after it; what comes before
    // the first word goes with that.
    final words = <String>[];
    final groups = <List<String>>[];
    final before = <String>[];
    for (final token in _tokens(transcript)) {
      final word = _word(token);
      if (word.isNotEmpty) {
        words.add(word);
        groups.add([...before, token]);
        before.clear();
      } else if (groups.isNotEmpty) {
        groups.last.add(token);
      } else {
        before.add(token);
      }
    }
    final dropped = List.filled(words.length, false);
    final leadIn = _words(germanLeadIn);
    final opening = _words(
      germanLeadIn.substring(0, germanLeadIn.indexOf(':')),
    );
    for (var i = 0; i < words.length;) {
      var n = 0;
      while (n < leadIn.length &&
          i + n < words.length &&
          words[i + n] == leadIn[n]) {
        n++;
      }
      if (n >= opening.length) dropped.fillRange(i, i + n, true);
      i += n >= opening.length ? n : 1;
    }
    final left = [
      for (var i = 0; i < words.length; i++)
        if (!dropped[i]) i,
    ];
    final rest = [for (final i in left) words[i]];
    final listed = [for (final name in names) ..._words(name)];
    if (names.length >= 2 && _same(rest, listed)) {
      for (final i in left) {
        dropped[i] = true;
      }
    } else {
      final vocabulary = [
        for (final item in [...names, ...terms])
          if (_words(item) case final words when words.isNotEmpty) words,
      ];
      // The length in words of the longest name or term at rest[at].
      int match(int at) {
        var longest = 0;
        for (final item in vocabulary) {
          if (item.length <= longest || at + item.length > left.length) {
            continue;
          }
          if (_same(rest.sublist(at, at + item.length), item)) {
            longest = item.length;
          }
        }
        return longest;
      }

      for (var i = 0; i < left.length;) {
        var end = i;
        var count = 0;
        for (var n = match(end); n > 0; n = match(end)) {
          end += n;
          count++;
        }
        if (count >= 3) {
          for (var k = i; k < end; k++) {
            dropped[left[k]] = true;
          }
        }
        i = end > i ? end : i + 1;
      }
    }
    if (!dropped.contains(true)) return transcript;
    return [
      for (var i = 0; i < words.length; i++)
        if (!dropped[i]) ...groups[i],
    ].join(' ');
  }

  /// The names of [names] that fit into [budget] characters, joined by
  /// ", ", at most [maxNames] of them.
  static List<String> _fit(Iterable<String> names, int budget) {
    final seen = <String>{};
    final fitting = <String>[];
    var used = 0;
    for (final raw in names) {
      if (fitting.length == maxNames) break;
      final name = raw.trim();
      if (name.isEmpty || !seen.add(name.toLowerCase())) continue;
      final cost = _length(name) + (fitting.isEmpty ? 0 : 2);
      if (used + cost > budget) continue;
      fitting.add(name);
      used += cost;
    }
    return fitting;
  }

  static int _length(String text) => text.runes.length;

  /// The parts of [text] between whitespace, as Dart's `trim` knows it.
  static List<String> _tokens(String text) {
    final tokens = <String>[];
    final token = StringBuffer();
    for (final rune in text.runes) {
      if (!_whitespace(rune)) {
        token.writeCharCode(rune);
      } else if (token.isNotEmpty) {
        tokens.add(token.toString());
        token.clear();
      }
    }
    if (token.isNotEmpty) tokens.add(token.toString());
    return tokens;
  }

  /// Unicode's White_Space and the byte order mark.
  static bool _whitespace(int rune) =>
      (rune >= 0x09 && rune <= 0x0d) ||
      rune == 0x20 ||
      rune == 0x85 ||
      rune == 0xa0 ||
      rune == 0x1680 ||
      (rune >= 0x2000 && rune <= 0x200a) ||
      rune == 0x2028 ||
      rune == 0x2029 ||
      rune == 0x202f ||
      rune == 0x205f ||
      rune == 0x3000 ||
      rune == 0xfeff;

  /// What a word may start or end with that is no part of it.
  static final _punctuation = '.,:;!?…"\'„“”‚‘’«»()[]–—-'.runes.toSet();

  /// [token] in lower case without the punctuation around it; empty when
  /// it is punctuation alone.
  static String _word(String token) {
    final runes = token.runes.toList();
    var start = 0;
    var end = runes.length;
    while (start < end && _punctuation.contains(runes[start])) {
      start++;
    }
    while (end > start && _punctuation.contains(runes[end - 1])) {
      end--;
    }
    return String.fromCharCodes(runes, start, end).toLowerCase();
  }

  static bool _same(List<String> a, List<String> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i] != b[i]) return false;
    }
    return true;
  }

  /// The words of [text], as [guard] compares them.
  static List<String> _words(String text) => [
    for (final token in _tokens(text))
      if (_word(token) case final word when word.isNotEmpty) word,
  ];
}

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
class TranscriptPrompts {
  /// For a recording in English: the Area and Topic names.
  final String english;

  /// For a recording in German: [germanLeadIn], the app's words and the
  /// Area and Topic names.
  final String german;

  const TranscriptPrompts({this.english = '', this.german = ''});

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
    if (listed.isEmpty) return TranscriptPrompts(german: german);
    final list = '${listed.join(', ')}.';
    return TranscriptPrompts(english: list, german: '$german $list');
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
}

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The places a search or the find bar found in one text (obf-1047), each
/// from its start to its end in that text as it is drawn, top first; the
/// one at [current] is the find bar's, drawn stronger. A table holding
/// [unfoldAt], the first place found in a table, shows all its rows.
class FoundMarks {
  final List<(int, int)> ranges;
  final int? current;
  final int? unfoldAt;

  const FoundMarks(this.ranges, [this.current, this.unfoldAt]);

  bool get isEmpty => ranges.isEmpty;

  /// Whether [start] to [end] touches a place: null where it touches
  /// none, true where it touches the current one.
  bool? at(int start, int end) {
    bool? found;
    for (final (i, (from, to)) in ranges.indexed) {
      if (from >= end) break;
      if (to <= start) continue;
      if (i == current) return true;
      found = false;
    }
    return found;
  }

  /// The places shifted by [by], as in a text that starts [by] later.
  FoundMarks shifted(int by) => by == 0
      ? this
      : FoundMarks(
          [for (final (a, b) in ranges) (a + by, b + by)],
          current,
          switch (unfoldAt) {
            final at? => at + by,
            null => null,
          },
        );
}

/// [style] drawn as a found place: on the tint of a find, the current one
/// on a stronger tint.
TextStyle foundStyle(
  BuildContext context,
  TextStyle? style, {
  required bool current,
}) {
  final scheme = Theme.of(context).colorScheme;
  return (style ?? const TextStyle()).copyWith(
    backgroundColor: current ? scheme.tertiary : scheme.tertiaryContainer,
    color: current ? scheme.onTertiary : scheme.onTertiaryContainer,
  );
}

/// [text] from [from] to [to], [style] where [marks] finds nothing and
/// [foundStyle] where it does, as spans; one span without marks.
List<InlineSpan> foundSpans(
  BuildContext context,
  String text,
  int from,
  int to,
  FoundMarks? marks, {
  TextStyle? style,
  GestureRecognizer? recognizer,
}) {
  if (from >= to) return const [];
  final ranges = marks?.ranges ?? const <(int, int)>[];
  final out = <InlineSpan>[];
  var at = from;
  for (final (i, (start, end)) in ranges.indexed) {
    if (end <= at) continue;
    if (start >= to) break;
    final a = start < at ? at : start;
    final b = end > to ? to : end;
    if (a > at) {
      out.add(
        TextSpan(
          text: text.substring(at, a),
          style: style,
          recognizer: recognizer,
        ),
      );
    }
    out.add(
      TextSpan(
        text: text.substring(a, b),
        style: foundStyle(context, style, current: i == marks?.current),
        recognizer: recognizer,
      ),
    );
    at = b;
  }
  if (to > at) {
    out.add(
      TextSpan(
        text: text.substring(at, to),
        style: style,
        recognizer: recognizer,
      ),
    );
  }
  return out;
}

/// [spans], flat spans of text one after the other that draw [from]
/// onward of a text, with the places of [marks] in that text drawn as
/// found: a span a place cuts is cut there.
List<InlineSpan> markSpans(
  BuildContext context,
  List<InlineSpan> spans,
  int from,
  FoundMarks? marks,
) {
  if (marks == null || marks.isEmpty) return spans;
  final out = <InlineSpan>[];
  var at = from;
  for (final span in spans) {
    if (span is! TextSpan || span.children != null || span.text == null) {
      out.add(span);
      at += 1;
      continue;
    }
    final text = span.text ?? '';
    final end = at + text.length;
    if (marks.at(at, end) == null) {
      out.add(span);
    } else {
      for (final piece in foundSpans(
        context,
        text,
        0,
        text.length,
        marks.shifted(-at),
        style: span.style,
      )) {
        out.add(piece);
      }
    }
    at = end;
  }
  return out;
}

/// The places found in the texts of the Notes and Sections below it
/// (obf-1047), by the Note or Section, the part and the media hash
/// ([Finding.key]): a Transcript, a Photo Text or an Attachment Text
/// drawn below it draws its own.
class FindScope extends InheritedWidget {
  final Map<(String, FindingPart, String), FoundMarks> marks;

  const FindScope({super.key, required this.marks, required super.child});

  /// The places found in the [part] of [key] of the Note or Section
  /// [entity]; null where none is.
  static FoundMarks? of(
    BuildContext context,
    String entity,
    FindingPart part,
    String key,
  ) => context
      .dependOnInheritedWidgetOfExactType<FindScope>()
      ?.marks[(entity, part, key)];

  /// The places found in the [part] of [key] of whichever Note or
  /// Section below holds it, for a text that does not know its own.
  static FoundMarks? ofKey(BuildContext context, FindingPart part, String key) {
    final marks = context
        .dependOnInheritedWidgetOfExactType<FindScope>()
        ?.marks;
    if (marks == null) return null;
    for (final MapEntry(key: (_, p, k), :value) in marks.entries) {
      if (p == part && k == key) return value;
    }
    return null;
  }

  @override
  bool updateShouldNotify(FindScope old) => !identical(marks, old.marks);
}

/// The places of [found] in each text, as [FindScope] holds them, of the
/// Note [noteId] and its Sections; the one at [current] among [found] is
/// the current one.
Map<(String, FindingPart, String), FoundMarks> foundMarksOf(
  String noteId,
  List<Finding> found, {
  int? current,
}) {
  final ranges = <(String, FindingPart, String), List<(int, int)>>{};
  final currents = <(String, FindingPart, String), int>{};
  final unfolds = <(String, FindingPart, String), int>{};
  for (final (i, f) in found.indexed) {
    final key = (f.section ?? noteId, f.part, f.key);
    final list = ranges.putIfAbsent(key, () => []);
    if (i == current) currents[key] = list.length;
    if (f.cell != null) unfolds.putIfAbsent(key, () => f.start);
    list.add((f.start, f.end));
  }
  return {
    for (final MapEntry(:key, :value) in ranges.entries)
      key: FoundMarks(value, currents[key], unfolds[key]),
  };
}

/// [text] as a [Text] in [style], the places of [marks] in it drawn as
/// found; those lying beyond its end, found in a text since changed,
/// left out.
Widget foundText(
  BuildContext context,
  String text,
  FoundMarks? marks, {
  Key? key,
  TextStyle? style,
  int? maxLines,
  TextOverflow? overflow,
}) {
  final fits = marks == null || marks.ranges.every((r) => r.$2 <= text.length);
  if (marks == null || marks.isEmpty || !fits) {
    return Text(
      text,
      key: key,
      style: style,
      maxLines: maxLines,
      overflow: overflow,
    );
  }
  return Text.rich(
    TextSpan(children: foundSpans(context, text, 0, text.length, marks)),
    key: key,
    style: style,
    maxLines: maxLines,
    overflow: overflow,
  );
}

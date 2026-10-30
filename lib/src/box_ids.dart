import 'package:flutter/services.dart';

/// The invisible ids of boxes in a text being edited (ADR 0035): a box is
/// its glyph, a variation selector perhaps, and four tag characters and a
/// cancel tag after it. In a text field the id is never seen and goes
/// with its glyph: the cursor and a selection never end inside one, and
/// an edit that takes part of a box takes all of it.

/// A tag character (U+E0000 to U+E007F) in UTF-16: this high surrogate,
/// then a low one from 0xDC00 to 0xDC7F.
const _tagHigh = 0xDB40;
const _cancelLow = 0xDC7F;

bool _tagLow(int c) => c >= 0xDC00 && c <= _cancelLow;

bool _idLow(int c) =>
    (c >= 0xDC30 && c <= 0xDC39) || (c >= 0xDC61 && c <= 0xDC7A);

bool _glyph(int c) => c == 0x2610 || c == 0x2705 || c == 0x274C;

/// Whether the tag characters from [start] to [end] of [text] are digits,
/// small letters and cancel tags, as an id's are.
bool _idRun(String text, int start, int end) {
  for (var i = start + 1; i < end; i += 2) {
    final c = text.codeUnitAt(i);
    if (!_idLow(c) && c != _cancelLow) return false;
  }
  return true;
}

/// Whether the code unit at [i] of [text] belongs to a tag character.
bool _isTagUnit(String text, int i) {
  final c = text.codeUnitAt(i);
  if (c == _tagHigh) {
    return i + 1 < text.length && _tagLow(text.codeUnitAt(i + 1));
  }
  return _tagLow(c) && i > 0 && text.codeUnitAt(i - 1) == _tagHigh;
}

/// A run of tag characters in a text: from [start] to [end], the
/// character it follows from [base], and whether it is the id of a box
/// ([box]): four digits or small letters and a cancel tag after a ☐, ✅
/// or ❌ and its variation selector.
typedef TagRun = ({int base, int start, int end, bool box});

/// Every run of tag characters in [text], in order.
List<TagRun> tagRuns(String text) {
  final runs = <TagRun>[];
  if (!text.codeUnits.contains(_tagHigh)) return runs;
  var i = 0;
  while (i < text.length) {
    if (!_isTagUnit(text, i)) {
      i++;
      continue;
    }
    final start = i;
    while (i + 1 < text.length && _isTagUnit(text, i)) {
      i += 2;
    }
    final end = i;
    // Its glyph, a variation selector after it perhaps.
    var at = start;
    if (at > 0) {
      final c = text.codeUnitAt(at - 1);
      if (c == 0xFE0E || c == 0xFE0F) at--;
    }
    var base = at;
    if (base > 1 &&
        text.codeUnitAt(base - 1) >= 0xDC00 &&
        text.codeUnitAt(base - 1) <= 0xDFFF &&
        text.codeUnitAt(base - 2) >= 0xD800 &&
        text.codeUnitAt(base - 2) <= 0xDBFF) {
      base -= 2;
    } else if (base > 0) {
      base--;
    }
    var box =
        end - start == 10 &&
        base + 1 == at &&
        _glyph(text.codeUnitAt(base)) &&
        text.codeUnitAt(end - 1) == _cancelLow;
    for (var k = 0; box && k < 4; k++) {
      if (!_idLow(text.codeUnitAt(start + 2 * k + 1))) box = false;
    }
    runs.add((base: base, start: start, end: end, box: box));
  }
  return runs;
}

/// How many code units of [text] before [offset] are no part of an id.
int idFreeCount(String text, int offset) {
  var count = 0;
  final stop = offset.clamp(0, text.length);
  for (var i = 0; i < stop; i++) {
    if (!_isTagUnit(text, i)) count++;
  }
  return count;
}

/// The offset in [text] after [count] code units that are no part of an
/// id, past the id that follows them: where a cursor goes when ids were
/// put into the text it was in ([idFreeCount]).
int idFreeOffset(String text, int count) {
  var seen = 0;
  var i = 0;
  while (i < text.length && seen < count) {
    if (!_isTagUnit(text, i)) seen++;
    i++;
  }
  while (i < text.length && _isTagUnit(text, i)) {
    i++;
  }
  return i;
}

/// [offset] of [text] moved out of a box or tag run it lies inside: to
/// the run's end, or with [back] to the start of its glyph.
int snapOutOfIds(String text, int offset, {bool back = false}) {
  for (final run in tagRuns(text)) {
    if (offset > run.base && offset < run.end) {
      return back ? run.base : run.end;
    }
  }
  return offset;
}

/// [selection] of [text] with its ends out of every id; an end moving
/// back from a box's end ([before]) goes to its start.
TextSelection snapSelection(
  String text,
  TextSelection selection, {
  TextSelection? before,
}) {
  if (!selection.isValid || !text.codeUnits.contains(_tagHigh)) {
    return selection;
  }
  int snap(int offset, int? was) {
    final back = was != null && was > offset;
    return snapOutOfIds(text, offset, back: back);
  }

  final base = snap(selection.baseOffset, before?.baseOffset);
  final extent = snap(selection.extentOffset, before?.extentOffset);
  if (base == selection.baseOffset && extent == selection.extentOffset) {
    return selection;
  }
  return selection.copyWith(baseOffset: base, extentOffset: extent);
}

/// Keeps the ids of the boxes whole while text is typed, pasted and
/// deleted: typing inside an id goes after its box, and what is left of
/// a box an edit took part of goes too, its glyph with it. Runs of tag
/// characters that were there before, such as a flag's, stay as they
/// are.
class BoxIdsFormatter extends TextInputFormatter {
  const BoxIdsFormatter();

  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue oldValue,
    TextEditingValue newValue,
  ) {
    final before = oldValue.text;
    final after = newValue.text;
    if (!before.codeUnits.contains(_tagHigh)) return newValue;
    final broken = _broken(before, after);
    if (broken.isEmpty) return newValue;
    // Typed inside an id: it goes after the box.
    if (_movedInsertion(before, after) case final moved?) return moved;
    return _without(newValue, broken);
  }

  /// The runs of tag characters in [after] that are no box's id and were
  /// not there in [before], each with the glyph it follows.
  static List<(int, int)> _broken(String before, String after) {
    final most = before.length < after.length ? before.length : after.length;
    var head = 0;
    while (head < most && before.codeUnitAt(head) == after.codeUnitAt(head)) {
      head++;
    }
    var tail = 0;
    while (tail < most - head &&
        before.codeUnitAt(before.length - 1 - tail) ==
            after.codeUnitAt(after.length - 1 - tail)) {
      tail++;
    }
    final grew = after.length - before.length;
    final kept = {
      for (final run in tagRuns(before))
        if (!run.box) (run.start, run.end),
    };
    final out = <(int, int)>[];
    for (final run in tagRuns(after)) {
      if (run.box) continue;
      final (start, end) = (run.start, run.end);
      final there = end <= head
          ? kept.contains((start, end))
          : start >= after.length - tail &&
                kept.contains((start - grew, end - grew));
      if (there) continue;
      final glyph = run.base < start && _glyph(after.codeUnitAt(run.base));
      // What is left of an id, not another emoji's tags such as a flag's.
      if (!glyph && (end - start > 10 || !_idRun(after, start, end))) continue;
      out.add((glyph ? run.base : start, end));
    }
    return out;
  }

  /// [after] as [before] with what was typed moved to after the box it
  /// went into, or null when it was no typing inside an id.
  static TextEditingValue? _movedInsertion(String before, String after) {
    final added = after.length - before.length;
    if (added <= 0) return null;
    var head = 0;
    while (head < before.length &&
        before.codeUnitAt(head) == after.codeUnitAt(head)) {
      head++;
    }
    if (before.substring(head) != after.substring(head + added)) return null;
    for (final run in tagRuns(before)) {
      if (!run.box || head <= run.base || head >= run.end) continue;
      final typed = after.substring(head, head + added);
      final text =
          before.substring(0, run.end) + typed + before.substring(run.end);
      if (_broken(before, text).isNotEmpty) return null;
      return TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: run.end + added),
      );
    }
    return null;
  }

  /// [value] without [ranges], its selection moved with the text.
  static TextEditingValue _without(
    TextEditingValue value,
    List<(int, int)> ranges,
  ) {
    final text = value.text;
    final out = StringBuffer();
    var from = 0;
    for (final (start, end) in ranges) {
      out.write(text.substring(from, start));
      from = end;
    }
    out.write(text.substring(from));
    int moved(int offset) {
      var shift = 0;
      for (final (start, end) in ranges) {
        if (offset >= end) {
          shift += end - start;
        } else if (offset > start) {
          shift += offset - start;
        }
      }
      return offset - shift;
    }

    final selection = value.selection;
    return TextEditingValue(
      text: out.toString(),
      selection: selection.isValid
          ? selection.copyWith(
              baseOffset: moved(selection.baseOffset),
              extentOffset: moved(selection.extentOffset),
            )
          : selection,
    );
  }
}

/// The id a box's [run] of tag characters in [text] spells, such as
/// `a7k2`.
String runId(String text, TagRun run) => String.fromCharCodes([
  for (var i = run.start + 1; i < run.end - 2; i += 2)
    text.codeUnitAt(i) - 0xDC00,
]);

/// [text] without the code units of its tag characters: what stays the
/// same when ids are put into a text.
String withoutIds(String text) {
  if (!text.codeUnits.contains(_tagHigh)) return text;
  final out = StringBuffer();
  for (var i = 0; i < text.length; i++) {
    if (!_isTagUnit(text, i)) out.writeCharCode(text.codeUnitAt(i));
  }
  return out.toString();
}

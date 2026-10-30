/// Boxes in user text (ADR 0035): a ☐, ✅ or ❌ anywhere in a Note's or
/// a Section's text that carries an id of its own, written right after
/// it in invisible tag characters, so a tick can name it. The glyph in
/// the text is the box's starting state; each tick is an entry of its
/// own beside the text ([Boxes]). Saving a text gives every bare glyph
/// an id ([TextBoxes.convert]).
library;

import 'dart:convert';

import 'package:crypto/crypto.dart';

import 'markdown.dart';
import 'text_format.dart';

/// The state a box shows, and the character it is written as.
enum BoxState {
  /// ☐: not done yet.
  open('☐'),

  /// ✅: done.
  done('✅'),

  /// ❌: failed, or not to be done.
  failed('❌');

  const BoxState(this.glyph);

  /// The character the box is written as, without a variation selector.
  final String glyph;

  /// The state named [name] as a tick stores it, or null.
  static BoxState? named(String? name) => switch (name) {
    'open' => open,
    'done' => done,
    'failed' => failed,
    _ => null,
  };
}

/// A box as the source text holds it: its [id], the [state] its glyph
/// shows (the starting state), and where it lies in UTF-16 code units:
/// the glyph from [start], a variation selector after it included, up
/// to [idStart]; the id from there up to [end].
typedef FoundBox = ({
  String id,
  BoxState state,
  int start,
  int idStart,
  int end,
});

/// One tick of a box: the [state] it set, when ([at], its entry's date),
/// the Device that wrote it, and whether that is one of the user's own.
class BoxTick {
  final BoxState state;
  final DateTime at;
  final String device;
  final bool own;

  const BoxTick({
    required this.state,
    required this.at,
    required this.device,
    required this.own,
  });
}

/// A box as it shows: its [id], the [state] it shows now and the one
/// its glyph started with ([starting]), its glyph from [start] to [end]
/// in the shown text ([TextBoxes.shown]), where the whole box lies in
/// the source text from [sourceStart] to [sourceEnd] with its id from
/// [idStart], and its latest tick, null while nobody ticked it. Offsets
/// in UTF-16 code units.
class ShownBox {
  final String id;
  final BoxState state;
  final BoxState starting;
  final int start;
  final int end;
  final int sourceStart;
  final int idStart;
  final int sourceEnd;
  final BoxTick? last;

  const ShownBox({
    required this.id,
    required this.state,
    required this.starting,
    required this.start,
    required this.end,
    required this.sourceStart,
    required this.idStart,
    required this.sourceEnd,
    this.last,
  });
}

/// A glyph [TextBoxes._scan] found, as it describes.
typedef _Scanned = ({
  String? id,
  BoxState state,
  int start,
  int idStart,
  int end,
  bool repeated,
});

/// A text as it shows: every box with the glyph of its current state,
/// without its id, and the boxes in it.
typedef ShownText = ({String text, List<ShownBox> boxes});

/// Finding, making and showing boxes in a text, the same in both cores
/// (`text_boxes.rs`).
///
/// A box is ☐, ✅ or ❌, one variation selector after it (U+FE0E or
/// U+FE0F) belonging to it, then [idLength] tag characters, each a digit
/// or a small letter (U+E0030–U+E0039, U+E0061–U+E007A), then a cancel
/// tag (U+E007F). A zero-width joiner after the glyph, or after its
/// selector, makes it part of another emoji, which is no box. An id
/// that shows up a second time in a text makes no second box.
abstract final class TextBoxes {
  /// How many characters an id has.
  static const idLength = 4;

  /// The characters of an id, as a tick names it.
  static const idAlphabet = '0123456789abcdefghijklmnopqrstuvwxyz';

  /// Whether [id] has the shape of a box id.
  static bool isId(String id) =>
      id.length == idLength && id.codeUnits.every(_idCodeUnit);

  /// Every box in [text], in the order they come.
  static List<FoundBox> find(String text) => [
    for (final b in _scan(text))
      if (b.id case final id? when !b.repeated)
        (
          id: id,
          state: b.state,
          start: b.start,
          idStart: b.idStart,
          end: b.end,
        ),
  ];

  /// [text] as it shows with [ticks], the latest tick of each box by id:
  /// every box with the bare glyph of its current state, the state of
  /// its latest tick or else its starting state, and no id. A second
  /// box of an id shows its own bare glyph and is no box.
  static ShownText shown(String text, [Map<String, BoxTick> ticks = const {}]) {
    final out = StringBuffer();
    final boxes = <ShownBox>[];
    var from = 0;
    for (final b in _scan(text)) {
      final id = b.id;
      if (id == null) continue;
      out.write(text.substring(from, b.start));
      from = b.end;
      final last = b.repeated ? null : ticks[id];
      final state = last?.state ?? b.state;
      if (!b.repeated) {
        boxes.add(
          ShownBox(
            id: id,
            state: state,
            starting: b.state,
            start: out.length,
            end: out.length + state.glyph.length,
            sourceStart: b.start,
            idStart: b.idStart,
            sourceEnd: b.end,
            last: last,
          ),
        );
      }
      out.write(state.glyph);
    }
    out.write(text.substring(from));
    return (text: out.toString(), boxes: boxes);
  }

  /// [text] without the ids of its boxes, each with the glyph it was
  /// written with: what search and the plain projections read when no
  /// ticks are at hand.
  static String strip(String text) => _hasTag(text) ? shown(text).text : text;

  /// [text] with every box written in the bare glyph of its current
  /// state by [ticks], its id kept: what a Sent Item carries, so its
  /// receiver starts from the sender's states and the ids stay the same.
  static String baked(String text, Map<String, BoxTick> ticks) {
    final out = StringBuffer();
    var from = 0;
    for (final b in _scan(text)) {
      final id = b.id;
      if (id == null || b.repeated) continue;
      final state = ticks[id]?.state ?? b.state;
      out
        ..write(text.substring(from, b.start))
        ..write(state.glyph);
      from = b.idStart;
    }
    out.write(text.substring(from));
    return out.toString();
  }

  /// [text] in [format] as it is saved: every bare ☐, ✅ or ❌, one not
  /// part of another emoji, becomes a box with a new id, and a box whose
  /// id came up before in the text gets a new one. In a formatted text
  /// a glyph in inline code or in a link's address stays as it is. Each
  /// id is drawn from [newId], the store's maker of entity ids, again
  /// while it is in the text already.
  static String convert(
    String text, {
    String format = TextFormats.plain,
    required String Function() newId,
  }) {
    final found = _scan(text);
    if (found.every((b) => b.id != null && !b.repeated)) return text;
    final used = {
      for (final b in found)
        if (b.id case final id? when !b.repeated) id,
    };
    final skipped = format == TextFormats.markdown
        ? _literalRanges(text)
        : const <(int, int)>[];
    final out = StringBuffer();
    var from = 0;
    for (final b in found) {
      if (b.id != null && !b.repeated) continue;
      if (b.id == null &&
          skipped.any((r) => b.start >= r.$1 && b.start < r.$2)) {
        continue;
      }
      var id = _drawn(newId);
      while (used.contains(id)) {
        id = _drawn(newId);
      }
      used.add(id);
      out
        ..write(text.substring(from, b.idStart))
        ..write(encodeId(id));
      from = b.end;
    }
    out.write(text.substring(from));
    return out.toString();
  }

  /// The id [id] as it is written after a glyph: its tag characters and
  /// the cancel tag.
  static String encodeId(String id) => String.fromCharCodes([
    for (final c in id.codeUnits) _tagBase + c,
    _cancelTag,
  ]);

  static const _textStyle = 0xFE0E;
  static const _emojiStyle = 0xFE0F;
  static const _joiner = 0x200D;
  static const _tagBase = 0xE0000;
  static const _cancelTag = 0xE007F;

  // A tag character in UTF-16: this high surrogate, then 0xDC00 plus the
  // ASCII code it stands for.
  static const _tagHigh = 0xDB40;
  static const _tagLow = 0xDC00;

  static bool _idCodeUnit(int c) =>
      (c >= 0x30 && c <= 0x39) || (c >= 0x61 && c <= 0x7A);

  static bool _hasTag(String text) => text.codeUnits.contains(_tagHigh);

  static BoxState? _glyph(int c) => switch (c) {
    0x2610 => BoxState.open,
    0x2705 => BoxState.done,
    0x274C => BoxState.failed,
    _ => null,
  };

  /// Every ☐, ✅ and ❌ of [text] that is no part of another emoji: from
  /// [start], to [idStart] after a variation selector, to [end] after its
  /// id when it has one, else [end] == [idStart]. [repeated] when its id
  /// came up before in the text.
  static List<_Scanned> _scan(String text) {
    final found = <_Scanned>[];
    final seen = <String>{};
    for (var i = 0; i < text.length; i++) {
      final state = _glyph(text.codeUnitAt(i));
      if (state == null) continue;
      var at = i + 1;
      if (at < text.length) {
        final next = text.codeUnitAt(at);
        if (next == _textStyle || next == _emojiStyle) at++;
      }
      if (at < text.length && text.codeUnitAt(at) == _joiner) continue;
      final id = _idAt(text, at);
      final end = id == null ? at : at + 2 * (idLength + 1);
      found.add((
        id: id,
        state: state,
        start: i,
        idStart: at,
        end: end,
        repeated: id != null && !seen.add(id),
      ));
      i = end - 1;
    }
    return found;
  }

  /// The id written at [at] in [text], or null when there is none.
  static String? _idAt(String text, int at) {
    if (at + 2 * (idLength + 1) > text.length) return null;
    final id = StringBuffer();
    for (var k = 0; k < idLength; k++) {
      if (text.codeUnitAt(at + 2 * k) != _tagHigh) return null;
      final c = text.codeUnitAt(at + 2 * k + 1) - _tagLow;
      if (!_idCodeUnit(c)) return null;
      id.writeCharCode(c);
    }
    final end = at + 2 * idLength;
    if (text.codeUnitAt(end) != _tagHigh ||
        text.codeUnitAt(end + 1) != _tagLow + (_cancelTag - _tagBase)) {
      return null;
    }
    return id.toString();
  }

  /// A new id from [newId]: the first bytes of the SHA-256 of a fresh
  /// entity id, each taken modulo the alphabet.
  static String _drawn(String Function() newId) {
    final digest = sha256.convert(utf8.encode(newId())).bytes;
    return String.fromCharCodes([
      for (var k = 0; k < idLength; k++)
        idAlphabet.codeUnitAt(digest[k] % idAlphabet.length),
    ]);
  }

  /// Where a glyph of the formatted text [text] is no box: inline code
  /// and links' addresses, as (start, end).
  static List<(int, int)> _literalRanges(String text) {
    final ranges = <(int, int)>[];
    void walk(List<MarkdownSpan> spans) {
      for (final s in spans) {
        switch (s.kind) {
          case MarkdownSpanKind.code:
            ranges.add((s.start, s.end));
          case MarkdownSpanKind.link:
            ranges.add((s.contentEnd, s.end));
            walk(s.children);
          default:
            walk(s.children);
        }
      }
    }

    for (final block in Markdown.parse(text)) {
      walk(block.spans);
    }
    return ranges;
  }
}

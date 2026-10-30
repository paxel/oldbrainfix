/// Tags in user text (ADR 0032): a `#word` in any text is drawn
/// highlighted, and a tap on it searches for it. Nothing is stored for a
/// tag and there is no list of them; it is part of the text.
library;

/// A tag found in a text: where it lies, from [start] to [end] in UTF-16
/// code units with its `#`, and the [tag] it is, `#` and its name in
/// lower case, which is also what a search for it is.
typedef FoundTag = ({int start, int end, String tag});

/// Finding tags in a text, the same in both cores (`text_tags.rs`).
///
/// A tag is a `#` followed by letters, digits, `_` and `-`, at least one
/// letter among them, the first no `-` and any `-` at its end left out.
/// Before the `#` comes the start of the text or a character that is no
/// letter, digit, `_` or `-` and no `#`, `/`, `\` or `&`, so neither
/// "C#" nor a web address's "/#top" is one. A letter is a character of
/// Unicode's Alphabetic property, a digit one of its numbers.
abstract final class TextTags {
  /// Every tag in [text], first one first.
  static List<FoundTag> find(String text) {
    final found = <FoundTag>[];
    var i = 0;
    while (i < text.length) {
      if (text.codeUnitAt(i) != _hash || !_mayPrecede(_before(text, i))) {
        i++;
        continue;
      }
      final from = i + 1;
      var end = from;
      var stop = from;
      var letters = false;
      while (end < text.length) {
        final c = _at(text, end);
        if (!_inTag(c) || (end == from && c == _dash)) break;
        letters = letters || _letter(c);
        end += c > 0xFFFF ? 2 : 1;
        if (c != _dash) stop = end;
      }
      if (letters) {
        found.add((
          start: i,
          end: stop,
          tag: '#${text.substring(from, stop).toLowerCase()}',
        ));
      }
      i = end;
    }
    return found;
  }

  static const _hash = 0x23;
  static const _dash = 0x2D;
  static const _underscore = 0x5F;
  static const _slash = 0x2F;
  static const _backslash = 0x5C;
  static const _ampersand = 0x26;

  static final _alphabetic = RegExp(r'^\p{Alphabetic}$', unicode: true);
  static final _number = RegExp(r'^\p{N}$', unicode: true);

  /// The code point at the UTF-16 offset [i] of [text]; a lone
  /// surrogate as it is.
  static int _at(String text, int i) {
    final c = text.codeUnitAt(i);
    if (c >= 0xD800 && c < 0xDC00 && i + 1 < text.length) {
      final low = text.codeUnitAt(i + 1);
      if (low >= 0xDC00 && low < 0xE000) {
        return 0x10000 + ((c - 0xD800) << 10) + (low - 0xDC00);
      }
    }
    return c;
  }

  /// The code point before the UTF-16 offset [i] of [text], or null at
  /// its start.
  static int? _before(String text, int i) {
    if (i == 0) return null;
    final c = text.codeUnitAt(i - 1);
    if (c >= 0xDC00 && c < 0xE000 && i >= 2) {
      final high = text.codeUnitAt(i - 2);
      if (high >= 0xD800 && high < 0xDC00) return _at(text, i - 2);
    }
    return c;
  }

  static bool _letter(int c) {
    if (c < 0x80) return (c | 0x20) >= 0x61 && (c | 0x20) <= 0x7A;
    return _alphabetic.hasMatch(String.fromCharCode(c));
  }

  static bool _inTag(int c) {
    if (c < 0x80) {
      return _letter(c) ||
          (c >= 0x30 && c <= 0x39) ||
          c == _underscore ||
          c == _dash;
    }
    return _letter(c) || _number.hasMatch(String.fromCharCode(c));
  }

  static bool _mayPrecede(int? c) =>
      c == null ||
      !(_inTag(c) ||
          c == _hash ||
          c == _slash ||
          c == _backslash ||
          c == _ampersand);
}

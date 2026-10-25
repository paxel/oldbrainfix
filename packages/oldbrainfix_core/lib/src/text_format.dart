import 'dart:convert';

/// The formats of user text (ADR 0018): the text of a Note or a Section
/// is stored with the format it is in. This version knows only [plain];
/// formatted text comes later (#57).
abstract final class TextFormats {
  /// Text as typed, without formatting.
  static const plain = 'plain';

  /// Every format this version reads and writes.
  static const known = {plain};
}

/// A user text as stored: its [format] and its [text], the source text
/// for a format this version does not know.
///
/// It is stored as one value, `{"format":…,"text":…}`, keys sorted and no
/// spaces, the same bytes from both cores, so text and format travel,
/// conflict and win together in one entry. A value that is no such
/// object was written before the format was recorded and reads as plain,
/// whole.
class StoredText {
  final String format;
  final String text;

  /// Whether this version may change it: its format is known and the
  /// value holds nothing else.
  final bool editable;

  const StoredText._(this.format, this.text, this.editable);

  /// [text] as plain text, the only format this version writes.
  const StoredText.plain(this.text)
    : format = TextFormats.plain,
      editable = true;

  /// [text] in [format] as it arrived from elsewhere, such as a Sent
  /// Item, to be stored as it is.
  StoredText.of(this.format, this.text)
    : editable = TextFormats.known.contains(format);

  /// The text a stored [value] holds; null reads as empty plain text.
  factory StoredText.read(String? value) {
    if (value == null) return const StoredText.plain('');
    if (value.startsWith('{')) {
      try {
        if (jsonDecode(value)
            case {'format': final String format, 'text': final String text} &&
                final Map<Object?, Object?> json) {
          final known = TextFormats.known.contains(format);
          return StoredText._(format, text, known && json.length == 2);
        }
      } on FormatException {
        // Plain text that happens to start with a brace.
      }
    }
    return StoredText.plain(value);
  }

  /// The value it is stored as.
  String encode() => jsonEncode({'format': format, 'text': text});

  @override
  bool operator ==(Object other) =>
      other is StoredText &&
      other.format == format &&
      other.text == text &&
      other.editable == editable;

  @override
  int get hashCode => Object.hash(format, text, editable);
}

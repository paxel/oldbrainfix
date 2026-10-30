import 'dart:convert';

import 'markdown.dart';
import 'text_boxes.dart';

/// The formats of user text (ADR 0018): the text of a Note or a Section
/// is stored with the format it is in. This version knows [plain] and
/// [markdown].
abstract final class TextFormats {
  /// Text as typed, without formatting.
  static const plain = 'plain';

  /// Formatted text, the subset of Markdown [Markdown] reads (ADR 0033).
  /// A text stays plain until it is first formatted, and is formatted
  /// from then on.
  static const markdown = 'markdown';

  /// Every format this version reads and writes.
  static const known = {plain, markdown};

  /// The newest structure of the stored value this version knows: `v` 1
  /// holds `format` and `text` and nothing else.
  static const version = 1;

  /// [text] in [format] as it reads without its marks and without the
  /// ids of its boxes (ADR 0035), for search and previews; a text in any
  /// other format as it is.
  static String visible(String format, String text) => switch (format) {
    markdown => Markdown.plain(TextBoxes.strip(text)),
    plain => TextBoxes.strip(text),
    _ => text,
  };

  /// The plain text [text] as it is written into a text in [format] to
  /// read as itself, such as a Transcript put into a Note's text.
  static String literal(String format, String text) =>
      format == markdown ? Markdown.escape(text) : text;
}

/// A user text as stored: its [format], its [text], the source text for a
/// format this version does not know, and the [version] of the value's
/// structure.
///
/// It is stored as one value, `{"format":…,"text":…}`, keys sorted and no
/// spaces, the same bytes from both cores, so text and format travel,
/// conflict and win together in one entry. `v` is written only when it
/// is more than 1, its default; a value without it is version 1. A value that is no such object was written before the
/// format was recorded and reads as plain, whole.
class StoredText {
  final String format;
  final String text;

  /// The version of the value's structure, `v`: which keys it has and
  /// what they mean.
  final int version;

  /// Whether this version may change it: its structure and its format are
  /// known and the value holds nothing else.
  final bool editable;

  const StoredText._(this.format, this.text, this.version, this.editable);

  /// [text] as plain text.
  const StoredText.plain(this.text)
    : format = TextFormats.plain,
      version = TextFormats.version,
      editable = true;

  /// [text] in [format] with structure [version] as it arrived from
  /// elsewhere, such as a Sent Item, to be stored as it is.
  StoredText.of(this.format, this.text, [this.version = TextFormats.version])
    : editable =
          version <= TextFormats.version && TextFormats.known.contains(format);

  /// The text a stored [value] holds; null reads as empty plain text.
  ///
  /// Up to the version known here, a value needs a string `format` and a
  /// string `text`. A later version may change or add keys: its value
  /// shows `text` when it is a string, nothing otherwise, and is never
  /// editable here.
  factory StoredText.read(String? value) {
    if (value == null) return const StoredText.plain('');
    if (value.startsWith('{')) {
      try {
        if (jsonDecode(value) case final Map<String, Object?> json) {
          if (_marked(json) case final marked?) return marked;
        }
      } on FormatException {
        // Plain text that happens to start with a brace.
      }
    }
    return StoredText.plain(value);
  }

  /// The text [json] holds when it is a marked value, null when not.
  static StoredText? _marked(Map<String, Object?> json) {
    final version = switch (json['v']) {
      null when !json.containsKey('v') => TextFormats.version,
      final int v when v >= 1 => v,
      _ => null,
    };
    if (version == null) return null;
    final format = json['format'];
    final text = json['text'];
    if (version > TextFormats.version) {
      return StoredText._(
        format is String ? format : '',
        text is String ? text : '',
        version,
        false,
      );
    }
    if (format is! String || text is! String) return null;
    final keys = json.containsKey('v') ? 3 : 2;
    final known = TextFormats.known.contains(format);
    return StoredText._(format, text, version, known && json.length == keys);
  }

  /// The text as it reads without its marks ([TextFormats.visible]).
  String get visible => TextFormats.visible(format, text);

  /// The value it is stored as; version 1, the default, says no `v`.
  String encode() => jsonEncode({
    'format': format,
    'text': text,
    if (version != TextFormats.version) 'v': version,
  });

  @override
  bool operator ==(Object other) =>
      other is StoredText &&
      other.format == format &&
      other.text == text &&
      other.version == version &&
      other.editable == editable;

  @override
  int get hashCode => Object.hash(format, text, version, editable);
}

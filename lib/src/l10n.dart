import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

extension L10nContext on BuildContext {
  /// The app's strings for the current locale.
  AppLocalizations get t => AppLocalizations.of(this);
}

/// [note] where only its words fit, as in a row or an alarm: its text
/// as it reads, without the marks of a formatted one, or what it holds
/// when it has no words ([Note.wordless]); a Note of files alone by its
/// first file's name ([fileTitle]).
String noteText(AppLocalizations t, Note note) => note.text.trim().isEmpty
    ? fileTitle(t, note) ?? wordlessTitle(t, note.wordless)
    : note.visibleText;

/// What a Note without words that holds files and no photo or recording,
/// removed ones included, goes by (#267): the name of its first file, or
/// the word for an Attachment when it has none; null for any other Note.
String? fileTitle(AppLocalizations t, Note note) {
  if (note.attachments.isEmpty ||
      note.photos.isNotEmpty ||
      note.removedPhotos.isNotEmpty ||
      note.recordings.isNotEmpty ||
      note.removedRecordings.isNotEmpty) {
    return null;
  }
  final name = note.attachments.first.name;
  return name.isEmpty ? t.attachment : name;
}

/// "Photo" or "Voice recording" for a Note without words.
String wordlessTitle(AppLocalizations t, Wordless wordless) =>
    switch (wordless) {
      Wordless.photo => t.photo,
      Wordless.voiceRecording => t.voiceRecording,
    };

/// [names] as a sentence lists them: "Bob", "Bob and Carol", "Bob, Carol
/// and Dave".
String namesText(AppLocalizations t, List<String> names) => switch (names) {
  [] => '',
  [final one] => one,
  [...final head, final last] => t.namesAnd(head.join(', '), last),
};

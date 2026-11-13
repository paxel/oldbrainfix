import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

extension L10nContext on BuildContext {
  /// The app's strings for the current locale.
  AppLocalizations get t => AppLocalizations.of(this);
}

/// [note] where only its words fit, as in a row or an alarm: its text
/// as it reads, without the marks of a formatted one, or what it holds
/// when it has no words ([Note.wordless]).
String noteText(AppLocalizations t, Note note) => note.text.trim().isEmpty
    ? wordlessTitle(t, note.wordless, note.wordlessName)
    : note.visibleText;

/// "Photo" or "Voice recording" for a Note without words; for one of
/// files by the [name] of its first file (#267, [Note.wordlessName]), or
/// the word for an Attachment when it has none.
String wordlessTitle(AppLocalizations t, Wordless wordless, String name) =>
    switch (wordless) {
      Wordless.photo => t.photo,
      Wordless.voiceRecording => t.voiceRecording,
      Wordless.file => name.isEmpty ? t.attachment : name,
    };

/// [names] as a sentence lists them: "Bob", "Bob and Carol", "Bob, Carol
/// and Dave".
String namesText(AppLocalizations t, List<String> names) => switch (names) {
  [] => '',
  [final one] => one,
  [...final head, final last] => t.namesAnd(head.join(', '), last),
};

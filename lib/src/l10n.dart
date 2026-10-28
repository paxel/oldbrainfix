import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

extension L10nContext on BuildContext {
  /// The app's strings for the current locale.
  AppLocalizations get t => AppLocalizations.of(this);
}

/// [note] where only its words fit, as in a row or an alarm: its text,
/// or what it holds when it has no words ([Note.wordless]).
String noteText(AppLocalizations t, Note note) =>
    note.text.trim().isEmpty ? wordlessTitle(t, note.wordless) : note.text;

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

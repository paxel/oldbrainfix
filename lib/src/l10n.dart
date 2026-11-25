import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

extension L10nContext on BuildContext {
  /// The app's strings for the current locale.
  AppLocalizations get t => AppLocalizations.of(this);
}

/// [note] where only its words fit, as in a row or an alarm: its text
/// as it reads, without the marks of a formatted one, else the first
/// line of its Transcript or Photo Text ([Note.shownName], obf-1049), or
/// what it holds when it has neither ([Note.wordless]).
String noteText(AppLocalizations t, Note note) => switch (note.shownName) {
  '' => wordlessTitle(t, note.wordless, note.wordlessName),
  final name => name,
};

/// What a calendar entry of [note] reads, its first line the entry's
/// title and the whole its description: its text as it reads, else its
/// whole Transcript or Photo Text ([Note.calendarText], obf-1049), or
/// what it holds when it has neither.
String noteCalendarText(AppLocalizations t, Note note) =>
    switch (note.calendarText) {
      '' => wordlessTitle(t, note.wordless, note.wordlessName),
      final text => text,
    };

/// "Photo" or "Voice recording" for a Note without words; for one of
/// files by the [name] of its first file (#267, [Note.wordlessName]), or
/// the word for an Attachment when it has none; "Checklist" or "Shopping
/// List" for a list without a Name (#309).
String wordlessTitle(AppLocalizations t, Wordless wordless, String name) =>
    switch (wordless) {
      Wordless.photo => t.photo,
      Wordless.voiceRecording => t.voiceRecording,
      Wordless.file => name.isEmpty ? t.attachment : name,
      Wordless.checklist => t.checklistHeading,
      Wordless.shoppingList => t.shoppingHeading,
    };

/// What a Note of the kind [kind] shows where its name would be while it
/// has none (#309): "Checklist" or "Shopping List" for a list; null for
/// any other kind, which shows nothing there.
String? untitledList(AppLocalizations t, NoteKind kind) => switch (kind) {
  NoteKind.checklist => t.checklistHeading,
  NoteKind.shoppingList => t.shoppingHeading,
  _ => null,
};

/// [names] as a sentence lists them: "Bob", "Bob and Carol", "Bob, Carol
/// and Dave".
String namesText(AppLocalizations t, List<String> names) => switch (names) {
  [] => '',
  [final one] => one,
  [...final head, final last] => t.namesAnd(head.join(', '), last),
};

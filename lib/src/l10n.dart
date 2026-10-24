import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

extension L10nContext on BuildContext {
  /// The app's strings for the current locale.
  AppLocalizations get t => AppLocalizations.of(this);
}

/// [note] where only its words fit, as in a row or an alarm: its text,
/// or "Photo" for a photo Note without words.
String noteText(AppLocalizations t, Note note) =>
    note.text.trim().isEmpty ? t.photo : note.text;

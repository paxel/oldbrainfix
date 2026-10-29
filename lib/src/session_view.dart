import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'l10n.dart';
import 'tally_entry_screen.dart';
import 'tally_view.dart';

/// A Tally a Session picked, as its row on the Agenda shows it: its
/// Note's text and the targets of its fields, each with the field's name
/// when it has more than one ("Push-ups 20 times").
String pickText(
  AppLocalizations t,
  SessionPick pick,
  UnitSystem system,
  String decimal,
) {
  final note = pick.note;
  final name = note == null ? t.noteGone : noteText(t, note);
  final aimed = [
    for (final field in pick.tally?.shown ?? const <TallyField>[])
      if (field.target != null) field,
  ];
  final targets = [
    for (final field in aimed)
      if (field.target case final target?)
        [
          if (aimed.length > 1) field.name,
          shownValue(target, field.unit, system, decimal),
        ].join(' '),
  ];
  return targets.isEmpty ? name : '$name ${targets.join(' · ')}';
}

/// Opens the entry form of [pick] of [today]: a new entry prefilled with
/// the targets, recorded for the Occurrence (ADR 0029), or, once it has
/// one, its latest entry to change. Should the pick have changed
/// meanwhile, such as by a sync, the entry still goes to its Tally, so
/// nothing typed is lost. Whether something was saved.
Future<bool> openPick(
  BuildContext context,
  Notes notes,
  SessionOccurrence today,
  SessionPick pick,
) {
  if (pick.entries.lastOrNull case final entry?) {
    return editTallyEntry(context, notes, pick.noteId, entry.id);
  }
  final store = notes.store;
  return recordTallyEntry(context, notes, pick.noteId, (values, line) {
    try {
      Sessions(store).record(
        today.session.id,
        today.occurrence,
        pick.noteId,
        values: values,
        line: line,
      );
    } on ArgumentError {
      Tallies(store).add(pick.noteId, values: values, line: line);
    }
  });
}

/// Whether this Device may open the entry form of [pick] of the Session
/// [sessionId]: record for it, or change the entry it has.
bool opensPick(Store store, String sessionId, SessionPick pick) {
  final note = pick.note;
  if (note == null || note.isTrashed || pick.tally == null) return false;
  if (pick.entries.lastOrNull case final entry?) {
    return store.mayChange(entry.id);
  }
  return store.mayCheckOff(sessionId) && store.mayWriteIn(pick.noteId);
}

/// The picks of a Session's Occurrence today, under its title on the
/// Agenda, in the order picked: each a line with its targets, struck
/// through once it has an entry, opening its entry form with [onOpen];
/// and under each without an entry "Another one" with [onAnother].
class SessionPicks extends StatelessWidget {
  final SessionOccurrence today;

  /// The text each pick shows, by its place: [pickText], or a masked
  /// one.
  final List<String> texts;

  /// Opens the entry form of the pick at a place; the line opens nothing
  /// when it returns null.
  final VoidCallback? Function(int index) onOpen;

  /// Swaps the pick at a place; no button when it returns null.
  final VoidCallback? Function(int index) onAnother;

  const SessionPicks({
    super.key,
    required this.today,
    required this.texts,
    required this.onOpen,
    required this.onAnother,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    if (today.picks.isEmpty) {
      return Text(t.sessionNothingToPick, style: text.bodyMedium);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final (i, pick) in today.picks.indexed) ...[
          InkWell(
            key: ValueKey('session-pick-${pick.noteId}'),
            borderRadius: BorderRadius.circular(8),
            onTap: onOpen(i),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 6),
              child: Row(
                children: [
                  Icon(
                    pick.done
                        ? Icons.check_circle_outline
                        : Icons.radio_button_unchecked,
                    size: 18,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      texts[i],
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: text.bodyMedium?.copyWith(
                        decoration: pick.done
                            ? TextDecoration.lineThrough
                            : null,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
          // The control goes below its text.
          if (onAnother(i) case final another? when !pick.done)
            TextButton.icon(
              key: ValueKey('session-another-${pick.noteId}'),
              onPressed: another,
              icon: const Icon(Icons.swap_horiz),
              label: Text(t.sessionAnother),
            ),
        ],
      ],
    );
  }
}

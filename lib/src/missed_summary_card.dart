import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'l10n.dart';
import 'look.dart';
import 'schedule_text.dart';

/// The Missed Summary (CONTEXT.md) on top of the Agenda: each Routine
/// that Missed something since the user last tapped "seen", how often,
/// and its latest Missed Occurrences to check off late. It stays until
/// "seen" is tapped; opening the app never clears it. A Routine from a
/// locked Area set to Masked shows its masked text, still to check off.
class MissedSummaryCard extends StatelessWidget {
  /// How many Missed Occurrences of one Routine show to check off.
  static const shown = 3;

  final MissedSummary summary;

  /// Whether a Routine shows Masked; none does when not given.
  final bool Function(Routine routine)? masks;

  /// Checks off [occurrence] of [routine] as Done, late.
  final void Function(Routine routine, Occurrence occurrence) onCheck;
  final VoidCallback onSeen;

  const MissedSummaryCard({
    super.key,
    required this.summary,
    this.masks,
    required this.onCheck,
    required this.onSeen,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: Embossed(
        strong: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(Icons.history, color: scheme.error),
                  const SizedBox(width: 8),
                  Expanded(child: Text(t.missedTitle, style: text.titleMedium)),
                  TextButton(onPressed: onSeen, child: Text(t.missedSeen)),
                ],
              ),
              for (final missed in summary.routines) ...[
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    '${switch (missed.note) {
                      _ when masks?.call(missed.routine) ?? false => maskedText(t, locale, missed.routine),
                      final note? => noteText(t, note).trim().split('\n').first,
                      null => t.noteGone,
                    }}'
                    ' · ${t.missedTimes(missed.count)}',
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: text.bodyLarge,
                  ),
                ),
                for (final (:occurrence, missedAt: _) in missed.missed.take(
                  shown,
                ))
                  Row(
                    children: [
                      Checkbox(
                        value: false,
                        semanticLabel: t.markDone,
                        onChanged: (_) => onCheck(missed.routine, occurrence),
                      ),
                      Text(
                        '${dayText(locale, occurrence.day)}  '
                        '${occurrenceTime(t, locale, occurrence)}',
                      ),
                    ],
                  ),
                if (missed.count > shown)
                  Padding(
                    padding: const EdgeInsets.only(left: 12, bottom: 4),
                    child: Text(
                      t.missedMore(missed.count - shown),
                      style: text.bodySmall,
                    ),
                  ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

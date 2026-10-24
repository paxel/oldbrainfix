import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'schedule_text.dart';

/// The Missed Summary (CONTEXT.md) on top of the Agenda: each Routine
/// that Missed something since the user last tapped "seen", how often,
/// and its latest Missed Occurrences to check off late. It stays until
/// "seen" is tapped; opening the app never clears it.
class MissedSummaryCard extends StatelessWidget {
  /// How many Missed Occurrences of one Routine show to check off.
  static const shown = 3;

  final MissedSummary summary;

  /// Checks off [occurrence] of [routine] as Done, late.
  final void Function(Routine routine, Occurrence occurrence) onCheck;
  final VoidCallback onSeen;

  const MissedSummaryCard({
    super.key,
    required this.summary,
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
                    '${missed.note?.text.trim().split('\n').first ?? t.noteGone}'
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

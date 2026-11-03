import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'agenda_group.dart';
import 'l10n.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';

/// What a Note's card knows of its Occurrences today (#215): [read] gives
/// them as [Routines.todayByNote] has them for the Note, read when the
/// card is, never while building; [routines] checks them off.
typedef NoteOccurrences = ({
  List<AgendaRow> Function() read,
  Routines routines,
});

/// The boxes of a Note's card, in either view, for today's Occurrences of
/// its Routines and Reminders (#215): of each its first one today not
/// Done, else its last one, Done and struck through. A box ticks it as
/// the Agenda row's does and opens nothing: it checks off and unchecks
/// again, a Session's only shows Done, and one Done from every Line
/// checked stays checked. Where this Device may not check it off, the box
/// only shows.
class CardOccurrences extends StatefulWidget {
  final String noteId;
  final NoteOccurrences occurrences;

  const CardOccurrences({
    super.key,
    required this.noteId,
    required this.occurrences,
  });

  @override
  State<CardOccurrences> createState() => _CardOccurrencesState();
}

class _CardOccurrencesState extends State<CardOccurrences> {
  List<AgendaRow> _rows = const [];

  /// Whether the box of each of [_rows] ticks, read with them.
  List<bool> _ticks = const [];

  @override
  void initState() {
    super.initState();
    _show(widget.occurrences.read());
  }

  @override
  void didUpdateWidget(CardOccurrences old) {
    super.didUpdateWidget(old);
    // A check-off changes the Routine, not the Note, so this reads anew.
    _show(widget.occurrences.read());
  }

  void _show(List<AgendaRow> rows) {
    _rows = rows;
    _ticks = [for (final row in rows) _checks(row)];
  }

  /// Whether the box of [row] ticks: as on the Agenda, not a Session's,
  /// not one its Lines made Done, and only where this Device checks off.
  bool _checks(AgendaRow row) {
    final store = widget.occurrences.routines.store;
    final id = row.routine.id;
    if (!store.mayCheckOff(id) || Sessions(store).byId(id) != null) {
      return false;
    }
    final lines = Checklists(store)
        .forOccurrence((routineId: id, occurrence: row.occurrence));
    return !(lines != null && lines.count > 0 && lines.allChecked);
  }

  void _toggle(AgendaRow row) {
    final routines = widget.occurrences.routines;
    if (row.done == null) {
      // Its Note may go to the trash with it (#211).
      routines.checkOff(row.routine.id, row.occurrence);
    } else {
      routines.unmarkDone(row.routine.id, row.occurrence);
    }
    // Read anew, not from the list of cards, which still holds the old.
    final rows = routines.todayByNote(deviceWallClock)[widget.noteId];
    setState(() => _show(rows ?? const []));
  }

  @override
  Widget build(BuildContext context) {
    if (_rows.isEmpty) return const SizedBox.shrink();
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme.bodyMedium;
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final (i, row) in _rows.indexed)
            Row(
              key: ValueKey(
                'card-occurrence-${row.routine.id}@${row.occurrence.key}',
              ),
              children: [
                AgendaBox(
                  value: row.done != null,
                  label:
                      '${t.markDone}: '
                      '${occurrenceTime(t, locale, row.occurrence)}',
                  onChanged: _ticks[i] ? (_) => _toggle(row) : null,
                ),
                const SizedBox(width: 4),
                Flexible(
                  child: Text(
                    occurrenceTime(t, locale, row.occurrence),
                    style: row.done == null
                        ? text
                        : text?.copyWith(
                            decoration: TextDecoration.lineThrough,
                          ),
                  ),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

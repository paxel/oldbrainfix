import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'browser.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'schedule_text.dart';
import 'system_bars.dart';

/// The deleted Notes, and the Routines and Reminders deleted on their
/// own, most recently deleted first, each with how long it stays and a
/// button to restore it. A Note comes back to where it was filed, with
/// the Routines and Reminders deleted along with it. Those of closed
/// locked Areas stay out.
class TrashScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const TrashScreen({super.key, required this.notes, required this.places});

  @override
  State<TrashScreen> createState() => _TrashScreenState();
}

/// One thing in the trash: a Note or a Routine, with when it was deleted.
typedef _Trashed = ({DateTime at, PlacedNote? note, Routine? routine});

class _TrashScreenState extends State<TrashScreen> with ReloadsOnLogChange {
  late final _routines = Routines(widget.notes.store);
  late List<_Trashed> _trashed = _load();

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => setState(() => _trashed = _load());

  List<_Trashed> _load() {
    final lock = LockScope.of(context);
    final trashed = <_Trashed>[
      for (final note in widget.notes.trashed())
        if (!(lock?.hides(note) ?? false))
          (
            at: note.trashed ?? note.captured,
            note: placeNote(widget.places, note),
            routine: null,
          ),
      for (final routine in _routines.trashed())
        if (!(lock?.hidesRoutine(routine) ?? false))
          (at: routine.trashed ?? DateTime(0), note: null, routine: routine),
    ];
    trashed.sort((a, b) => b.at.compareTo(a.at));
    return trashed;
  }

  void _restoreNote(Note note) {
    _routines.restoreNote(note.id);
    setState(() => _trashed = _load());
  }

  void _restoreRoutine(Routine routine) {
    _routines.restore(routine.id);
    setState(() => _trashed = _load());
  }

  /// Whole days until [at], by the store's clock.
  int _daysLeft(DateTime? at) {
    if (at == null) return 0;
    return at.difference(widget.notes.store.clock().toUtc()).inDays;
  }

  Widget _footer(DateTime? purgeAt, VoidCallback restore) => ControlBelow(
    padding: EdgeInsets.zero,
    title: Text(
      context.t.goneIn(_daysLeft(purgeAt)),
      style: Theme.of(context).textTheme.bodySmall,
    ),
    control: TextButton(onPressed: restore, child: Text(context.t.restore)),
  );

  /// A Routine or Reminder: what it reminds of and when.
  Widget _routineCard(Routine routine) {
    final t = context.t;
    final theme = Theme.of(context);
    final locale = Localizations.localeOf(context).toString();
    final note = _routines.note(routine);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Embossed(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 4, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(
                    routine.isReminder
                        ? Icons.notifications_outlined
                        : Icons.repeat,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      note == null ? t.noteDeleted : noteText(t, note),
                      style: theme.textTheme.bodyLarge,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(scheduleText(t, locale, routine.schedule)),
              // Only a Manager restores it in a Shared Area (ADR 0013).
              if (widget.notes.store.mayChangeRoutine(routine.id))
                _footer(
                  _routines.purgeAt(routine),
                  () => _restoreRoutine(routine),
                ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.trashTitle)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: _trashed.isEmpty
            ? [BrowserEmpty(text: t.trashEmpty)]
            : [
                for (final trashed in _trashed)
                  if (trashed.note case (
                    :final note,
                    :final area,
                    :final topic,
                    :final changes,
                    schedules: _,
                    amends: _,
                    :final author,
                  ))
                    NoteCard(
                      note: note,
                      when: when,
                      area: area,
                      topic: topic,
                      author: author,
                      showPlace: true,
                      // Only who may change it restores it (ADR 0013).
                      footer: changes
                          ? _footer(
                              widget.notes.purgeAt(note),
                              () => _restoreNote(note),
                            )
                          : null,
                    )
                  else if (trashed.routine case final routine?)
                    _routineCard(routine),
              ],
      ),
    );
  }
}

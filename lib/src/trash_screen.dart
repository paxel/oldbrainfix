import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'app_lock.dart';
import 'browser.dart';
import 'failure_dialog.dart';
import 'frogs_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'note_selection.dart';
import 'page_bar.dart';
import 'run_over.dart';
import 'schedule_text.dart';
import 'system_bars.dart';
import 'trash_emptying.dart';

/// The deleted Notes, and the Routines, Reminders and Frogs deleted on
/// their own, most recently deleted first, each with how long it stays
/// and a button to restore it; a Frog stays until restored (ADR 0027).
/// A Note comes back to where it was filed, with the Routines and
/// Reminders deleted along with it. Those of closed locked Areas stay
/// out. "Empty the trash now", last on the bar, deletes for good beside
/// the app what this Device may change, once asked (obf-1095).
class TrashScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const TrashScreen({super.key, required this.notes, required this.places});

  @override
  State<TrashScreen> createState() => _TrashScreenState();
}

/// One thing in the trash: a Note, a Routine or a Frog, with when it was
/// deleted.
typedef _Trashed = ({
  DateTime at,
  PlacedNote? note,
  Routine? routine,
  Frog? frog,
});

class _TrashScreenState extends State<TrashScreen>
    with ReloadsOnLogChange, SelectsNotes {
  late final _routines = Routines(widget.notes.store);
  late final _frogs = Frogs(widget.notes.store);
  late List<_Trashed> _trashed = _load();

  /// The app's emptying of the trash (obf-1095); none without the app,
  /// and then no "Empty the trash now".
  TrashEmptying? _emptying;

  /// What "Empty the trash now" removes, as last counted beside the app;
  /// null until it is.
  TrashEmptied? _toEmpty;

  /// Which count is the latest, so an older one ending later is dropped.
  var _counts = 0;

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => _reload();

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final emptying = TrashEmptyingScope.of(context);
    if (emptying == _emptying) return;
    _emptying?.removeListener(_emptyingChanged);
    _emptying = emptying?..addListener(_emptyingChanged);
    unawaited(_count());
  }

  @override
  void dispose() {
    _emptying?.removeListener(_emptyingChanged);
    super.dispose();
  }

  /// Emptying started or ended: the action is off meanwhile.
  void _emptyingChanged() {
    if (mounted) setState(() {});
  }

  void _reload() {
    setState(() {
      _trashed = _load();
      keepShown(_notes);
      // Off until counted again: the question never names an old count.
      _toEmpty = null;
    });
    unawaited(_count());
  }

  /// Counts what "Empty the trash now" removes, beside the app: it reads
  /// every Note in the trash, too slow for a frame on thousands.
  Future<void> _count() async {
    final emptying = _emptying;
    if (emptying == null) return;
    final count = ++_counts;
    TrashEmptied? toEmpty;
    try {
      toEmpty = await emptying.toEmpty();
    } on Object {
      // The store closed under it, as on a Restart: the action stays off.
      toEmpty = null;
    }
    if (!mounted || count != _counts) return;
    setState(() => _toEmpty = toEmpty);
  }

  /// Whether "Empty the trash now" has something to remove and nothing
  /// empties the trash already.
  bool get _mayEmpty =>
      !(_toEmpty?.isEmpty ?? true) && !(_emptying?.running ?? true);

  /// Asks once, naming how many Notes and Routines go, then empties the
  /// trash beside the app (obf-1095); it cannot be undone. A failure is
  /// announced at the top in [AppLocalizations] taken before the wait,
  /// even after this page closed; success is not.
  Future<void> _emptyNow() async {
    final emptying = _emptying;
    final toEmpty = _toEmpty;
    if (emptying == null || toEmpty == null || !_mayEmpty) return;
    final t = context.t;
    final (notes, routines) = (toEmpty.notes.length, toEmpty.routines.length);
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        key: const ValueKey('trash-empty-question'),
        content: Text(switch ((notes, routines)) {
          (_, 0) => t.trashEmptyNotes(notes),
          (0, _) => t.trashEmptyRoutines(routines),
          _ => t.trashEmptyBoth(notes, routines),
        }),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.cancel),
          ),
          DestructiveButton(
            key: const ValueKey('trash-empty-confirm'),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.deleteForGood),
          ),
        ],
      ),
    );
    if (!mounted || sure != true) return;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final store = widget.notes.store;
    try {
      await emptying.run();
    } on Object catch (e) {
      // A Restart ended this run of the app meanwhile: its store is
      // closed, and nobody is left to tell.
      if (runOver(store)) return;
      announceFailureWith(
        announcements,
        navigator,
        t.trashEmptyFailed,
        title: t.trashEmptyFailedTitle,
        cause: t.trashEmptyFailedCause,
        fix: t.trashEmptyFailedFix,
        details: '$e',
      );
    }
    // The page reads the trash again as the log tells it changed.
  }

  /// The Notes among [_trashed], which the selection takes (#273).
  Iterable<PlacedNote> get _notes => [
    for (final trashed in _trashed) ?trashed.note,
  ];

  List<_Trashed> _load() {
    final lock = LockScope.of(context);
    final notes = placeNotes(widget.places, [
      for (final note in widget.notes.trashed())
        if (!(lock?.hides(note) ?? false)) note,
    ]);
    final trashed = <_Trashed>[
      for (final placed in notes)
        (
          at: placed.note.trashed ?? placed.note.captured,
          note: placed,
          routine: null,
          frog: null,
        ),
      for (final routine in _routines.trashed())
        if (!(lock?.hidesRoutine(routine) ?? false))
          (
            at: routine.trashed ?? DateTime(0),
            note: null,
            routine: routine,
            frog: null,
          ),
      for (final id in widget.notes.store.entitiesOfType(FrogKeys.typeFrog))
        if (_frogs.byId(id) case final frog? when frog.isTrashed)
          if (!_hidesFrog(lock, frog))
            (
              at: frog.trashed ?? DateTime(0),
              note: null,
              routine: null,
              frog: frog,
            ),
    ];
    trashed.sort((a, b) => b.at.compareTo(a.at));
    return trashed;
  }

  /// Whether [frog] stays out, as its Note does.
  bool _hidesFrog(AppLock? lock, Frog frog) {
    final note = _frogs.note(frog);
    return note != null && (lock?.hides(note) ?? false);
  }

  void _restoreFrog(Frog frog) {
    _frogs.restore(frog.id);
    _reload();
  }

  void _restoreNote(Note note) {
    _routines.restoreNote(note.id);
    _reload();
  }

  void _restoreRoutine(Routine routine) {
    _routines.restore(routine.id);
    _reload();
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

  /// A Frog: what its Note says, and Restore; it is never purged.
  Widget _frogCard(Frog frog) {
    final t = context.t;
    final note = _frogs.note(frog);
    final store = widget.notes.store;
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
                  const FrogIcon(),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      note == null ? t.noteDeleted : noteText(t, note),
                      style: Theme.of(context).textTheme.bodyLarge,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(t.frog),
              // Only a Manager restores it in a Shared Area (ADR 0013).
              if (store.mayManage(store.areaOf(frog.id)))
                Align(
                  alignment: Alignment.centerLeft,
                  child: TextButton(
                    onPressed: () => _restoreFrog(frog),
                    child: Text(t.restore),
                  ),
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
    final notes = _notes;
    return selectionBack(
      Scaffold(
        appBar: pageBar(
          context,
          title: selecting ? t.notesSelected(selected.length) : t.trashTitle,
          actions: selecting
              ? selectionActions(
                  context,
                  onRestore: () {
                    restoreSelected(widget.notes, notes);
                    _reload();
                  },
                )
              : [
                  if (mayTick(notes)) selectButton(context),
                  if (_emptying != null)
                    BarAction(
                      key: const ValueKey('trash-empty'),
                      icon: Icons.delete_forever_outlined,
                      label: t.trashEmptyNow,
                      onPressed: _mayEmpty ? _emptyNow : null,
                    ),
                ],
        ),
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
                        // A Note in the trash is read before it is restored.
                        full: true,
                        sections: () => widget.notes.sections(note.id),
                        kind: () => NoteKinds(widget.notes.store).of(note.id),
                        // Only who may change it restores it (ADR 0013).
                        footer: changes
                            ? _footer(
                                widget.notes.purgeAt(note),
                                () => _restoreNote(note),
                              )
                            : null,
                        onSelect: selecting && changes
                            ? (on) => setSelected(note, on)
                            : null,
                        selected: selected.contains(note.id),
                      )
                    else if (trashed.routine case final routine?)
                      _routineCard(routine)
                    else if (trashed.frog case final frog?)
                      _frogCard(frog),
                ],
        ),
      ),
    );
  }
}

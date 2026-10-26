import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'areas_screen.dart';
import 'browser.dart';
import 'l10n.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'system_bars.dart';

/// How many of the oldest Unsorted Notes the Nudge screen shows.
const nudgeCards = 3;

/// What the Nudge's notification opens (UI laws, "Alarms and
/// notifications"): the three oldest Unsorted cards, each opening its
/// Note and offering to sort it right here, and "+x" for the rest, which
/// opens the list of all Unsorted Notes.
class NudgeScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const NudgeScreen({super.key, required this.notes, required this.places});

  @override
  State<NudgeScreen> createState() => _NudgeScreenState();
}

class _NudgeScreenState extends State<NudgeScreen> {
  late List<Note> _unsorted = widget.notes.unsorted();

  void _reload() {
    if (!mounted) return;
    setState(() => _unsorted = widget.notes.unsorted());
  }

  Future<void> _all() async {
    await openUnsorted(context, widget.notes, widget.places);
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final more = _unsorted.length - nudgeCards;
    return Scaffold(
      appBar: AppBar(title: Text(t.nudgeScreenTitle)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          if (_unsorted.isEmpty)
            BrowserEmpty(text: t.noUnsorted)
          else
            ..._sortingCards(
              context,
              widget.notes,
              widget.places,
              _unsorted.take(nudgeCards),
              _reload,
            ),
          if (more > 0)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
              child: Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonal(
                  key: const ValueKey('nudgeMore'),
                  onPressed: _all,
                  child: Tooltip(
                    message: t.nudgeMoreTooltip,
                    child: Text(t.nudgeMore(more)),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Opens the list of all Unsorted Notes.
Future<void> openUnsorted(BuildContext context, Notes notes, Places places) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => UnsortedScreen(notes: notes, places: places),
      ),
    );

/// Every Unsorted Note as a card, oldest first, as the Nudge screen
/// starts them, each offering to sort it; reached from the "x Unsorted"
/// banner on the Agenda and from "+x" on the Nudge screen.
class UnsortedScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const UnsortedScreen({super.key, required this.notes, required this.places});

  @override
  State<UnsortedScreen> createState() => _UnsortedScreenState();
}

class _UnsortedScreenState extends State<UnsortedScreen> {
  PersonFilter _filter = PersonFilter.everyone;
  late List<Note> _unsorted = _load();

  /// The Unsorted Notes of the people the filter keeps (#53).
  List<Note> _load() =>
      widget.notes.store.notesBy(widget.notes.unsorted(), _filter);

  void _reload() {
    if (!mounted) return;
    setState(() => _unsorted = _load());
  }

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _unsorted = _load();
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(
        title: Text(t.unsorted),
        actions: [
          PersonFilterButton(
            store: widget.notes.store,
            filter: _filter,
            onChanged: _filtered,
          ),
        ],
      ),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          PersonFilterChips(
            store: widget.notes.store,
            filter: _filter,
            onChanged: _filtered,
          ),
          if (_unsorted.isEmpty)
            BrowserEmpty(
              text: _filter.isEveryone ? t.noUnsorted : t.nothingFound,
            )
          else
            ..._sortingCards(
              context,
              widget.notes,
              widget.places,
              _unsorted,
              _reload,
            ),
        ],
      ),
    );
  }
}

/// [unsorted] as cards that open their Note and offer to file, merge or
/// schedule it; [changed] runs once the user is back from any of them.
List<Widget> _sortingCards(
  BuildContext context,
  Notes notes,
  Places places,
  Iterable<Note> unsorted,
  VoidCallback changed,
) {
  Future<void> then(Future<Object?> step) async {
    await step;
    changed();
  }

  return diaryCards(
    context,
    [for (final note in unsorted) placeNote(places, note)],
    (note) => then(fileNote(context, notes, places, note)),
    (note) => then(mergeNote(context, notes, places, note)),
    (note) => then(openNote(context, notes, places, note)),
    onSchedule: (note, reminder) =>
        then(spawnFromNote(context, notes, places, note, reminder: reminder)),
  );
}

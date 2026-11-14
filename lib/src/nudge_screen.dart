import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'areas_screen.dart';
import 'browser.dart';
import 'l10n.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'note_selection.dart';
import 'page_bar.dart';
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

class _NudgeScreenState extends State<NudgeScreen>
    with ReloadsOnLogChange, FoldsNoteCards, SelectsNotes {
  late List<Note> _unsorted = widget.notes.unsorted();

  /// The cards shown, the oldest of [_unsorted].
  late List<PlacedNote> _shown = _place();

  List<PlacedNote> _place() => [
    for (final note in _unsorted.take(nudgeCards))
      placeNote(widget.places, note),
  ];

  @override
  Store get log => widget.notes.store;

  /// The same Unsorted Notes as the list of all of them.
  @override
  String get cardsKey => 'unsorted';

  @override
  void reloadLog() => _reload();

  void _reload() {
    if (!mounted) return;
    setState(() {
      _unsorted = widget.notes.unsorted();
      _shown = _place();
      keepShown(_shown);
    });
  }

  Future<void> _all() async {
    await openUnsorted(context, widget.notes, widget.places);
    if (!mounted) return;
    // The list of all of them shares the compact/detailed switch.
    setState(readCompact);
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final more = _unsorted.length - nudgeCards;
    return selectionBack(
      Scaffold(
        appBar: pageBar(
          context,
          title: selecting
              ? t.notesSelected(selected.length)
              : t.nudgeScreenTitle,
          actions: selecting
              ? noteSelectionActions(
                  context,
                  widget.notes,
                  widget.places,
                  _shown,
                  _reload,
                )
              : [
                  CompactNotesButton(compact: compact, onChanged: setCompact),
                  if (mayTick(_shown)) selectButton(context),
                ],
        ),
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
                _shown,
                _reload,
                compact: compact,
                switched: switched,
                onSwitch: switchView,
                selected: selecting ? selected : null,
                onSelect: setSelected,
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

class _UnsortedScreenState extends State<UnsortedScreen>
    with ReloadsOnLogChange, FoldsNoteCards, SelectsNotes {
  PersonFilter _filter = PersonFilter.everyone;
  late List<PlacedNote> _unsorted = _load();

  @override
  Store get log => widget.notes.store;

  @override
  String get cardsKey => 'unsorted';

  @override
  void reloadLog() => _reload();

  /// The Unsorted Notes of the people the filter keeps (#53).
  List<PlacedNote> _load() => [
    for (final note in widget.notes.store.notesBy(
      widget.notes.unsorted(),
      _filter,
    ))
      placeNote(widget.places, note),
  ];

  void _reload() {
    if (!mounted) return;
    setState(() {
      _unsorted = _load();
      keepShown(_unsorted);
    });
  }

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _unsorted = _load();
    keepShown(_unsorted);
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return selectionBack(
      Scaffold(
        appBar: pageBar(
          context,
          title: selecting ? t.notesSelected(selected.length) : t.unsorted,
          actions: selecting
              ? noteSelectionActions(
                  context,
                  widget.notes,
                  widget.places,
                  _unsorted,
                  _reload,
                )
              : [
                  PersonFilterButton(
                    store: widget.notes.store,
                    filter: _filter,
                    onChanged: _filtered,
                  ),
                  CompactNotesButton(compact: compact, onChanged: setCompact),
                  if (mayTick(_unsorted)) selectButton(context),
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
                compact: compact,
                switched: switched,
                onSwitch: switchView,
                selected: selecting ? selected : null,
                onSelect: setSelected,
              ),
          ],
        ),
      ),
    );
  }
}

/// [unsorted] as cards, detailed unless [compact] (#189), that open their
/// Note and offer to file, merge or schedule it; [changed] runs once the
/// user is back from any of them. With [selected] they have their boxes
/// (#273).
List<Widget> _sortingCards(
  BuildContext context,
  Notes notes,
  Places places,
  List<PlacedNote> unsorted,
  VoidCallback changed, {
  required bool compact,
  required Set<String> switched,
  required void Function(Note note) onSwitch,
  Set<String>? selected,
  void Function(Note note, bool on)? onSelect,
}) {
  Future<void> then(Future<Object?> step) async {
    await step;
    changed();
  }

  return diaryCards(
    context,
    unsorted,
    (note) => then(fileNote(context, notes, places, note)),
    (note) => then(mergeNote(context, notes, places, note)),
    (note) => then(openNote(context, notes, places, note)),
    notes: notes,
    onChanged: changed,
    // The screen is Unsorted.
    showPlace: (_) => false,
    onSchedule: (note, spawn) =>
        then(spawnFromNote(context, notes, places, note, spawn: spawn)),
    compact: compact,
    switched: switched,
    onSwitch: onSwitch,
    selected: selected,
    onSelect: onSelect,
  );
}

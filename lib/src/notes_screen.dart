import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'l10n.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';

/// Every Note as a diary of cards, newest first, Unsorted ones included
/// and those of closed locked Areas left out; reached from the top of
/// the notes browser, and filtered to one person by "Their Notes" on
/// their Profile. Swipe right goes back.
class AllNotesScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  /// Whose Notes show as the screen opens (#53).
  final PersonFilter people;

  const AllNotesScreen({
    super.key,
    required this.notes,
    required this.places,
    this.people = PersonFilter.everyone,
  });

  @override
  State<AllNotesScreen> createState() => _AllNotesScreenState();
}

class _AllNotesScreenState extends State<AllNotesScreen> {
  late PersonFilter _people = widget.people;
  late List<PlacedNote> _shown = _load();

  List<PlacedNote> _load() {
    final lock = LockScope.of(context);
    return [
      for (final note in widget.notes.store.notesBy(
        widget.notes.all(),
        _people,
      ))
        if (!(lock?.hides(note) ?? false)) placeNote(widget.places, note),
    ];
  }

  void _filter(PersonFilter people) => setState(() {
    _people = people;
    _shown = _load();
  });

  void _reload() => setState(() => _shown = _load());

  Future<void> _file(Note note) async {
    if (await fileNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _merge(Note note) async {
    if (await mergeNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _open(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  Future<void> _schedule(Note note, bool reminder) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      reminder: reminder,
    );
    if (!mounted) return;
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(
        title: Text(t.allNotesTitle),
        actions: [
          PersonFilterButton(
            store: widget.notes.store,
            filter: _people,
            onChanged: _filter,
          ),
        ],
      ),
      body: BrowserSwipes(
        child: ListView(
          padding: const EdgeInsets.symmetric(vertical: 8),
          children: [
            PersonFilterChips(
              store: widget.notes.store,
              filter: _people,
              onChanged: _filter,
            ),
            if (_shown.isEmpty)
              BrowserEmpty(
                text: _people.isEveryone ? t.noNotesYet : t.nothingFound,
              )
            else
              ...diaryCards(
                context,
                _shown,
                _file,
                _merge,
                _open,
                onSchedule: _schedule,
              ),
          ],
        ),
      ),
    );
  }
}

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'browser.dart';
import 'l10n.dart';
import 'note_card.dart';
import 'system_bars.dart';

/// Opens "merge into…" for [note] and merges it into the Note picked
/// there; true when it was merged.
Future<bool> mergeNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note,
) async {
  final intoId = await Navigator.of(context).push(
    MaterialPageRoute<String>(
      builder: (_) =>
          MergeIntoScreen(notes: notes, places: places, noteId: note.id),
    ),
  );
  if (intoId == null) return false;
  notes.merge(note.id, intoId);
  return true;
}

/// Picks the Note to merge the Note [noteId] into: every other Note as a
/// card, newest first, with a search field over them. Pops with the
/// picked Note's id.
class MergeIntoScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final String noteId;

  const MergeIntoScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.noteId,
  });

  @override
  State<MergeIntoScreen> createState() => _MergeIntoScreenState();
}

class _MergeIntoScreenState extends State<MergeIntoScreen> {
  final _query = TextEditingController();
  late List<PlacedNote> _shown = _load();

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  /// The Notes to merge into: not this one, nor those of closed locked
  /// Areas.
  List<PlacedNote> _load() {
    final lock = LockScope.of(context);
    return [
      for (final note in Search(widget.notes.store).notes(_query.text))
        if (note.id != widget.noteId && !(lock?.hides(note) ?? false))
          placeNote(widget.places, note),
    ];
  }

  void _search(String _) => setState(() => _shown = _load());

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.mergeInto)),
      body: Column(
        children: [
          BrowserSearchField(
            controller: _query,
            hint: t.searchAllHint,
            onChanged: _search,
          ),
          Expanded(
            child: ListView(
              padding: clearOfSystemBars(
                context,
                const EdgeInsets.symmetric(vertical: 8),
              ),
              children: _shown.isEmpty
                  ? [
                      BrowserEmpty(
                        text: _query.text.trim().isEmpty
                            ? t.noOtherNotes
                            : t.nothingFound,
                      ),
                    ]
                  : [
                      for (final (
                            :note,
                            :area,
                            :topic,
                            changes: _,
                            schedules: _,
                            amends: _,
                            :author,
                          )
                          in _shown)
                        NoteCard(
                          note: note,
                          when: when,
                          area: area,
                          topic: topic,
                          author: author,
                          onTap: () => Navigator.of(context).pop(note.id),
                        ),
                    ],
            ),
          ),
        ],
      ),
    );
  }
}

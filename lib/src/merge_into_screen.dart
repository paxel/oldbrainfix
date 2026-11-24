import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'browser.dart';
import 'hidden_from_ai_view.dart';
import 'l10n.dart';
import 'note_card.dart';
import 'page_bar.dart';
import 'system_bars.dart';
import 'text_marks_view.dart';

/// Opens "merge into…" for [note] and merges it into the Note picked
/// there; true when it was merged. Hidden from AI and a Shared Area do
/// not meet in a Merge (#267): asked as when filing. With [adding], by
/// default for an Unsorted Note, whose card names it so (#308), the page
/// is "Add to Note…".
Future<bool> mergeNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note, {
  bool? adding,
}) async {
  final intoId = await Navigator.of(context).push(
    MaterialPageRoute<String>(
      builder: (_) => MergeIntoScreen(
        notes: notes,
        places: places,
        noteId: note.id,
        adding: adding ?? note.isUnsorted,
      ),
    ),
  );
  if (intoId == null || !context.mounted) return false;
  final merged = await unlessHiddenFromAi(context, notes.store, () {
    notes.merge(note.id, intoId);
    return true;
  });
  return merged ?? false;
}

/// Picks the Note to merge the Note [noteId] into: every other Note as a
/// card, newest first, with a search field over them. Pops with the
/// picked Note's id. Without [noteId], for several Notes (#273), every
/// Note is offered.
///
/// With [adding] it is "Add to Note…" (#308), for a capture or an
/// Unsorted Note: "Added to recently" with the Notes this Device added to
/// last at the top while nothing is typed, as File into… has its places
/// (#272), and only the Notes this Device may add to; the pick is kept
/// as added to last. A capture [hidden] from AI is offered no Note of a
/// Shared Area (#267).
class MergeIntoScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final String? noteId;
  final bool adding;
  final bool hidden;

  const MergeIntoScreen({
    super.key,
    required this.notes,
    required this.places,
    this.noteId,
    this.adding = false,
    this.hidden = false,
  });

  @override
  State<MergeIntoScreen> createState() => _MergeIntoScreenState();
}

class _MergeIntoScreenState extends State<MergeIntoScreen>
    with SearchesAfterPause {
  final _query = TextEditingController();

  /// The text [_shown] was searched for; the field's own may be newer.
  String _searched = '';
  late List<PlacedNote> _shown = _load();

  /// With [MergeIntoScreen.adding], the Notes added to last that are
  /// offered, the latest first.
  late final List<PlacedNote> _recent = widget.adding
      ? [
          for (final id in widget.notes.store.recentlyAddedTo)
            ?_shown.where((placed) => placed.note.id == id).firstOrNull,
        ]
      : const [];

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  /// The Notes to merge into: not this one, nor those of closed locked
  /// Areas; adding, only those this Device may add to, and for a capture
  /// hidden from AI none of a Shared Area.
  List<PlacedNote> _load() {
    final lock = LockScope.of(context);
    final store = widget.notes.store;
    bool offered(PlacedNote placed) {
      if (!widget.adding) return true;
      if (!placed.amends) return false;
      if (!widget.hidden) return true;
      final area = store.areaOf(placed.note.id);
      return area == null || !store.isShared(area);
    }

    _searched = _query.text;
    // Placed for all of them at once, not once per Note (obf-1068).
    return [
      for (final placed in placeNotesShown(widget.places, [
        for (final note in Search(store).notes(_searched))
          if (note.id != widget.noteId) note,
      ], lock))
        if (offered(placed)) placed,
    ];
  }

  /// Pops with [id]; adding, it is kept as added to last.
  void _pick(String id) {
    if (widget.adding) widget.notes.store.addedTo(id);
    Navigator.of(context).pop(id);
  }

  void _search() => setState(() => _shown = _load());

  /// A letter typed: the search runs once typing pauses, an emptied
  /// field shows every Note at once.
  void _typed(String query) =>
      query.trim().isEmpty ? _set(query) : searchAfterPause(_search);

  /// The search set as a whole runs now.
  void _set(String _) => searchNow(_search);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: pageBar(
        context,
        title: widget.adding ? t.addToNote : t.mergeInto,
        background: searchHeaderColour(context),
      ),
      body: Column(
        children: [
          // The Notes scroll under its line (obf-1088).
          SearchBand(
            child: BrowserSearchField(
              controller: _query,
              hint: t.searchAllHint,
              onChanged: _typed,
              onSet: _set,
            ),
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
                        text: _searched.trim().isNotEmpty
                            ? t.nothingFound
                            : widget.adding
                            ? t.noNoteToAddTo
                            : t.noOtherNotes,
                      ),
                    ]
                  : [
                      if (_recent.isNotEmpty && _searched.trim().isEmpty) ...[
                        Padding(
                          padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
                          child: Text(
                            t.addToNoteRecent,
                            style: Theme.of(context).textTheme.labelLarge,
                          ),
                        ),
                        for (final placed in _recent)
                          KeyedSubtree(
                            key: ValueKey(
                              'add-to-note-recent-${placed.note.id}',
                            ),
                            child: _card(placed, when),
                          ),
                        const Divider(height: 24),
                      ],
                      for (final placed in _shown) _card(placed, when),
                    ],
            ),
          ),
        ],
      ),
    );
  }

  /// [placed] as a card a tap picks, also on a tag.
  Widget _card(PlacedNote placed, DateFormat when) {
    final (:note, :area, :topic, changes: _, schedules: _, amends: _, :author) =
        placed;
    return TagSearchScope(
      onSearch: null,
      child: NoteCard(
        note: note,
        when: when,
        area: area,
        topic: topic,
        author: author,
        showPlace: true,
        onTap: () => _pick(note.id),
      ),
    );
  }
}

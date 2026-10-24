import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'l10n.dart';
import 'note_card.dart';

/// The deleted Notes, most recently deleted first, each with how long it
/// stays and a button to restore it to where it was filed.
class TrashScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const TrashScreen({super.key, required this.notes, required this.places});

  @override
  State<TrashScreen> createState() => _TrashScreenState();
}

class _TrashScreenState extends State<TrashScreen> {
  late List<PlacedNote> _trashed = _load();

  List<PlacedNote> _load() => [
    for (final note in widget.notes.trashed()) placeNote(widget.places, note),
  ];

  void _restore(Note note) {
    widget.notes.restore(note.id);
    setState(() => _trashed = _load());
  }

  /// Whole days until [note] is purged, by the store's clock.
  int _daysLeft(Note note) {
    final at = widget.notes.purgeAt(note);
    if (at == null) return 0;
    return at.difference(widget.notes.store.clock().toUtc()).inDays;
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.trashTitle)),
      body: ListView(
        padding: const EdgeInsets.symmetric(vertical: 8),
        children: _trashed.isEmpty
            ? [BrowserEmpty(text: t.trashEmpty)]
            : [
                for (final (:note, :area, :topic) in _trashed)
                  NoteCard(
                    note: note,
                    when: when,
                    area: area,
                    topic: topic,
                    footer: Row(
                      children: [
                        Expanded(
                          child: Text(
                            t.goneIn(_daysLeft(note)),
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                        ),
                        TextButton(
                          onPressed: () => _restore(note),
                          child: Text(t.restore),
                        ),
                      ],
                    ),
                  ),
              ],
      ),
    );
  }
}

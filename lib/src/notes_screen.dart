import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'areas_screen.dart';
import 'browser.dart';
import 'capture_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';

/// The list of captured Notes, newest first, with "+" to capture and the
/// magnifier to open the notes browser. Stands in for the Agenda.
class NotesScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  /// The notes browser's last selections, kept between visits.
  final BrowserTrail trail;

  const NotesScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
  });

  @override
  State<NotesScreen> createState() => _NotesScreenState();
}

class _NotesScreenState extends State<NotesScreen> {
  late List<PlacedNote> _shown = _load();

  List<PlacedNote> _load() => [
    for (final note in widget.notes.all()) placeNote(widget.places, note),
  ];

  void _reload() => setState(() => _shown = _load());

  Future<void> _capture() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => CaptureScreen(notes: widget.notes),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _browse() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => AreasScreen(
          notes: widget.notes,
          places: widget.places,
          trail: widget.trail,
        ),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _file(Note note) async {
    if (await fileNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.appTitle)),
      body: _shown.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Embossed(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(t.noNotesYet, textAlign: TextAlign.center),
                  ),
                ),
              ),
            )
          : ListView(
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: [
                for (final (:note, :area, :topic) in _shown)
                  NoteCard(
                    note: note,
                    when: when,
                    area: area,
                    topic: topic,
                    onFile: () => _file(note),
                  ),
              ],
            ),
      floatingActionButton: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          FloatingActionButton.small(
            heroTag: 'browse',
            tooltip: t.browseTooltip,
            onPressed: _browse,
            child: const Icon(Icons.search),
          ),
          const SizedBox(height: 16),
          FloatingActionButton(
            heroTag: 'capture',
            tooltip: t.captureTooltip,
            onPressed: _capture,
            child: const Icon(Icons.add),
          ),
        ],
      ),
    );
  }
}

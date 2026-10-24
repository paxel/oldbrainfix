import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'areas_screen.dart';
import 'capture_screen.dart';
import 'file_into_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';

/// The list of captured Notes, newest first, with "+" to capture and the
/// magnifier to browse Areas and Topics.
class NotesScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const NotesScreen({super.key, required this.notes, required this.places});

  @override
  State<NotesScreen> createState() => _NotesScreenState();
}

/// A Note with the Area and Topic it is filed in, read once per change.
typedef _Shown = ({Note note, Area? area, Topic? topic});

class _NotesScreenState extends State<NotesScreen> {
  late List<_Shown> _shown = _load();

  List<_Shown> _load() => [for (final note in widget.notes.all()) _place(note)];

  _Shown _place(Note note) {
    final place = note.place;
    if (place == null) return (note: note, area: null, topic: null);
    final topic = widget.places.topicById(place);
    final area = widget.places.areaById(topic?.areaId ?? place);
    return (note: note, area: area, topic: topic);
  }

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
        builder: (_) => AreasScreen(places: widget.places),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _file(Note note) async {
    final placeId = await Navigator.of(context).push(
      MaterialPageRoute<String>(
        builder: (_) =>
            FileIntoScreen(places: widget.places, current: note.place),
      ),
    );
    if (!mounted || placeId == null) return;
    widget.notes.file(note.id, placeId);
    _reload();
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

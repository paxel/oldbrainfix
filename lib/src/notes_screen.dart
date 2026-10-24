import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'capture_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';

/// The list of captured Notes, newest first, with "+" to capture.
class NotesScreen extends StatefulWidget {
  final Notes notes;

  const NotesScreen({super.key, required this.notes});

  @override
  State<NotesScreen> createState() => _NotesScreenState();
}

class _NotesScreenState extends State<NotesScreen> {
  late List<Note> _notes = widget.notes.all();

  Future<void> _capture() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => CaptureScreen(notes: widget.notes),
      ),
    );
    if (!mounted) return;
    setState(() => _notes = widget.notes.all());
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.appTitle)),
      body: _notes.isEmpty
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
                for (final note in _notes) NoteCard(note: note, when: when),
              ],
            ),
      floatingActionButton: FloatingActionButton(
        tooltip: t.captureTooltip,
        onPressed: _capture,
        child: const Icon(Icons.add),
      ),
    );
  }
}

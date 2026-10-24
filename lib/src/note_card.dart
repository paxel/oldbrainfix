import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'look.dart';

/// A Note drawn as a softly embossed card: its text and capture time.
class NoteCard extends StatelessWidget {
  final Note note;

  /// Formats the capture time; the list makes it once for all cards.
  final DateFormat when;

  const NoteCard({super.key, required this.note, required this.when});

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Embossed(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(note.text, style: text.bodyLarge),
              const SizedBox(height: 6),
              Text(when.format(note.captured.toLocal()), style: text.bodySmall),
            ],
          ),
        ),
      ),
    );
  }
}

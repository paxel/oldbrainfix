import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'l10n.dart';
import 'look.dart';
import 'place_names.dart';

/// A Note with the Area and Topic it is filed in, read once per change.
typedef PlacedNote = ({Note note, Area? area, Topic? topic});

/// Looks up where [note] is filed.
PlacedNote placeNote(Places places, Note note) {
  final place = note.place;
  if (place == null) return (note: note, area: null, topic: null);
  final topic = places.topicById(place);
  final area = places.areaById(topic?.areaId ?? place);
  return (note: note, area: area, topic: topic);
}

/// A Note drawn as a softly embossed card: its text, capture time and
/// where it is filed. Its Area's colour runs down the left edge and its
/// icon leads the place line.
class NoteCard extends StatelessWidget {
  final Note note;

  /// Formats the capture time; the list makes it once for all cards.
  final DateFormat when;

  /// The Area the Note sits in, directly or through [topic]; null while
  /// it is Unsorted.
  final Area? area;
  final Topic? topic;

  /// Opens "file into…"; no button when null.
  final VoidCallback? onFile;

  const NoteCard({
    super.key,
    required this.note,
    required this.when,
    this.area,
    this.topic,
    this.onFile,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final area = this.area;
    final topic = this.topic;
    final colour = areaColour(area?.colour);
    final place = area == null
        ? t.unsorted
        : [
            areaName(t, area),
            if (topic != null) topicName(t, topic),
          ].join(' / ');
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          child: Container(
            decoration: colour == null
                ? null
                : BoxDecoration(
                    border: Border(left: BorderSide(color: colour, width: 6)),
                  ),
            padding: const EdgeInsets.fromLTRB(16, 16, 4, 16),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(note.text, style: text.bodyLarge),
                      const SizedBox(height: 6),
                      Text(
                        when.format(note.captured.toLocal()),
                        style: text.bodySmall,
                      ),
                      const SizedBox(height: 4),
                      Row(
                        children: [
                          if (areaIconData(area?.icon) case final icon?) ...[
                            Icon(
                              icon,
                              size: 16,
                              color: colour ?? scheme.onSurfaceVariant,
                            ),
                            const SizedBox(width: 4),
                          ],
                          Flexible(child: Text(place, style: text.labelMedium)),
                        ],
                      ),
                    ],
                  ),
                ),
                if (onFile != null)
                  IconButton(
                    tooltip: t.fileInto,
                    onPressed: onFile,
                    icon: const Icon(Icons.drive_file_move_outline),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

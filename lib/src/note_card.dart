import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'l10n.dart';
import 'look.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'profile.dart';
import 'speech_platform.dart';
import 'voice_platform.dart';

/// A Note with the Area and Topic it is filed in, read once per change.
/// A Note with where it is filed, what this Device may do to it in a
/// Shared Area (ADR 0013): change it ([changes]: file, merge, edit,
/// delete) and give it Routines or Reminders ([schedules]), and the
/// Profile of the other person who wrote it ([author], ADR 0017; null
/// for the user's own).
typedef PlacedNote = ({
  Note note,
  Area? area,
  Topic? topic,
  bool changes,
  bool schedules,
  Profile? author,
});

/// Looks up where [note] is filed, what this Device may do to it, and
/// whose it is.
PlacedNote placeNote(Places places, Note note) {
  final store = places.store;
  final changes = store.mayChange(note.id);
  final schedules = store.mayManage(store.areaOf(note.id));
  final author = store.authorOf(note.id);
  final place = note.place;
  if (place == null) {
    return (
      note: note,
      area: null,
      topic: null,
      changes: changes,
      schedules: schedules,
      author: author,
    );
  }
  final topic = places.topicById(place);
  final area = places.areaById(topic?.areaId ?? place);
  return (
    note: note,
    area: area,
    topic: topic,
    changes: changes,
    schedules: schedules,
    author: author,
  );
}

/// [location] as shown: degrees to four places, about ten metres.
String coordinatesText(Coordinates location) =>
    '${location.latitude.toStringAsFixed(4)}, '
    '${location.longitude.toStringAsFixed(4)}';

/// Where [capture] happened as shown on a card: its Named Place, else its
/// coordinates; null while unknown.
String? whereCaptured(CaptureContext capture) =>
    capture.namedPlace?.name ??
    switch (capture.location) {
      final location? => coordinatesText(location),
      null => null,
    };

/// A Note drawn as a softly embossed card: its text, thumbnails of its
/// first [shownPhotos] photos, a play button per voice recording with
/// the start of its Transcript under it, capture
/// time, the Named Place or
/// coordinates it was captured at with an icon when there is more
/// Capture Context, and where it is filed; another person's Note leads
/// its time with their picture (ADR 0017). Its Area's colour runs down
/// the left edge and its icon leads the place line. Its enrich options
/// file it, spawn a Routine or Reminder from it, and merge it. An
/// important Note gets an accent marker and a stronger emboss, an
/// unimportant one is dimmed (UI laws, "Cards").
class NoteCard extends StatelessWidget {
  final Note note;

  /// Formats the capture time; the list makes it once for all cards.
  final DateFormat when;

  /// The Area the Note sits in, directly or through [topic]; null while
  /// it is Unsorted.
  final Area? area;
  final Topic? topic;

  /// The Profile of the other person who wrote the Note; null for the
  /// user's own, which shows no picture.
  final Profile? author;

  /// Opens "file into…"; no button when null.
  final VoidCallback? onFile;

  /// Opens "merge into…"; no button when null.
  final VoidCallback? onMerge;

  /// Spawns a Routine (false) or a Reminder (true) from the Note, picked
  /// from a menu that opens at the button; no button when null.
  final ValueChanged<bool>? onSchedule;

  /// Opens the Note's editor.
  final VoidCallback? onTap;

  /// Shown under the place line, e.g. the trash's restore.
  final Widget? footer;

  /// How much faded an unimportant card is drawn.
  static const unimportantOpacity = 0.55;

  /// How many photo thumbnails a card shows; the rest are counted.
  static const shownPhotos = 3;

  const NoteCard({
    super.key,
    required this.note,
    required this.when,
    this.area,
    this.topic,
    this.author,
    this.onFile,
    this.onMerge,
    this.onSchedule,
    this.onTap,
    this.footer,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final area = this.area;
    final topic = this.topic;
    final colour = areaColour(area?.colour);
    // A Note that spawned a Routine is not Unsorted, though not filed.
    final place = area == null
        ? (note.spawned ? null : t.unsorted)
        : [
            areaName(t, area),
            if (topic != null) topicName(t, topic),
          ].join(' / ');
    final important = note.importance == Importance.important;
    final footer = this.footer;
    final card = Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Embossed(
        strong: important,
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The card's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              onTap: onTap,
              child: Container(
                decoration: colour == null
                    ? null
                    : BoxDecoration(
                        border: Border(
                          left: BorderSide(color: colour, width: 6),
                        ),
                      ),
                padding: const EdgeInsets.fromLTRB(16, 16, 4, 16),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          // A photo Note may have no words.
                          if (note.text.isNotEmpty)
                            Text(note.text, style: text.bodyLarge),
                          if (note.photos.isNotEmpty) ...[
                            if (note.text.isNotEmpty) const SizedBox(height: 8),
                            // Not a sideways list: swipes on a card
                            // belong to the browser.
                            Row(
                              children: [
                                for (final hash in note.photos.take(
                                  shownPhotos,
                                )) ...[
                                  NotePhoto(
                                    key: ValueKey(hash),
                                    hash: hash,
                                    size: 64,
                                  ),
                                  const SizedBox(width: 8),
                                ],
                                if (note.photos.length > shownPhotos)
                                  Text(
                                    t.morePhotos(
                                      note.photos.length - shownPhotos,
                                    ),
                                    style: text.labelLarge,
                                  ),
                              ],
                            ),
                          ],
                          if (note.recordings.isNotEmpty) ...[
                            if (note.text.isNotEmpty || note.photos.isNotEmpty)
                              const SizedBox(height: 8),
                            Wrap(
                              spacing: 8,
                              children: [
                                for (final hash in note.recordings)
                                  RecordingButton(
                                    key: ValueKey(hash),
                                    hash: hash,
                                  ),
                              ],
                            ),
                            for (final hash in note.recordings)
                              TranscriptText(
                                key: ValueKey('transcript:$hash'),
                                noteId: note.id,
                                hash: hash,
                                transcript: note.transcripts[hash],
                              ),
                          ],
                          const SizedBox(height: 6),
                          Wrap(
                            crossAxisAlignment: WrapCrossAlignment.center,
                            spacing: 4,
                            children: [
                              if (author case final author?)
                                Semantics(
                                  label: t.authorPicture(
                                    profileDisplayName(t, author.name),
                                  ),
                                  image: true,
                                  excludeSemantics: true,
                                  child: ProfileAvatar(
                                    key: const ValueKey('author'),
                                    profile: author,
                                    size: 20,
                                  ),
                                ),
                              Text(
                                when.format(note.captured.toLocal()),
                                style: text.bodySmall,
                              ),
                              if (whereCaptured(note.context)
                                  case final where?) ...[
                                const SizedBox(width: 4),
                                Icon(
                                  Icons.place_outlined,
                                  size: 14,
                                  color: scheme.onSurfaceVariant,
                                ),
                                Text(where, style: text.bodySmall),
                              ],
                              // The editor shows the rest.
                              if (note.context.hasMore) ...[
                                const SizedBox(width: 4),
                                Icon(
                                  Icons.more_horiz,
                                  size: 16,
                                  color: scheme.onSurfaceVariant,
                                  semanticLabel: t.moreCaptureContext,
                                ),
                              ],
                            ],
                          ),
                          if (place != null) ...[
                            const SizedBox(height: 4),
                            Row(
                              children: [
                                if (areaIconData(area?.icon)
                                    case final icon?) ...[
                                  Icon(
                                    icon,
                                    size: 16,
                                    color: colour ?? scheme.onSurfaceVariant,
                                  ),
                                  const SizedBox(width: 4),
                                ],
                                Flexible(
                                  child: Text(place, style: text.labelMedium),
                                ),
                              ],
                            ),
                          ],
                          ?footer,
                        ],
                      ),
                    ),
                    if (important)
                      Padding(
                        padding: const EdgeInsets.all(12),
                        child: Icon(
                          Icons.label_important,
                          color: scheme.primary,
                          semanticLabel: t.importanceImportant,
                        ),
                      ),
                    if (onFile != null)
                      IconButton(
                        tooltip: t.fileInto,
                        onPressed: onFile,
                        icon: const Icon(Icons.drive_file_move_outline),
                      ),
                    if (onSchedule case final onSchedule?)
                      PopupMenuButton<bool>(
                        tooltip: t.routineOrReminder,
                        icon: const Icon(Icons.alarm_add_outlined),
                        onSelected: onSchedule,
                        itemBuilder: (_) => [
                          PopupMenuItem(value: false, child: Text(t.routine)),
                          PopupMenuItem(value: true, child: Text(t.reminder)),
                        ],
                      ),
                    if (onMerge != null)
                      IconButton(
                        tooltip: t.mergeInto,
                        onPressed: onMerge,
                        icon: const Icon(Icons.call_merge),
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
    return note.importance == Importance.unimportant
        ? Opacity(opacity: unimportantOpacity, child: card)
        : card;
  }
}

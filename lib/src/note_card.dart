import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'calendar_platform.dart';
import 'l10n.dart';
import 'look.dart';
import 'ocr_platform.dart';
import 'person_screen.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'profile.dart';
import 'speech_platform.dart';
import 'voice_platform.dart';

/// What a Note's card spawns from it: "Routine, Reminder or Frog".
enum Spawn { routine, reminder, frog }

/// A Note with the Area and Topic it is filed in, read once per change.
/// A Note with where it is filed, what this Device may do to it in a
/// Shared Area (ADR 0013): change it ([changes]: file, merge, edit,
/// delete), give it Routines or Reminders ([schedules]) and add to it
/// ([amends]), and the Profile of the other person who wrote it
/// ([author], ADR 0017; null for the user's own).
typedef PlacedNote = ({
  Note note,
  Area? area,
  Topic? topic,
  bool changes,
  bool schedules,
  bool amends,
  Profile? author,
});

/// Looks up where [note] is filed, what this Device may do to it, and
/// whose it is.
PlacedNote placeNote(Places places, Note note) {
  final store = places.store;
  final changes = store.mayChange(note.id);
  final schedules = store.mayManage(store.areaOf(note.id));
  final amends = store.mayWriteIn(note.id);
  final author = store.authorOf(note.id);
  final place = note.place;
  if (place == null) {
    return (
      note: note,
      area: null,
      topic: null,
      changes: changes,
      schedules: schedules,
      amends: amends,
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
    amends: amends,
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

/// What a card's calendar entries offer: adding the Note, or an event
/// of its Routines and Reminders, to the calendar, and all of those
/// events as one calendar file when there are any.
typedef NoteCalendar = ({List<CalendarChoice> add, CalendarChoice? file});

/// A photo ([photo]) or voice recording [hash] a card's ⋮ menu offers
/// to remove while its [text] stays, shown to tell several apart; [run]
/// asks once and removes it (ADR 0020).
typedef MediaRemoval = ({
  bool photo,
  String hash,
  String text,
  VoidCallback run,
});

/// What a card's ⋮ menu offers beyond filing, scheduling and merging:
/// the actions the Note editor has. An entry is left out when its
/// callback is null.
class NoteActions {
  /// Sets the Note's Importance, picked in the menu.
  final ValueChanged<Importance>? onImportance;

  /// Sends the Note to a person as a Sent Item.
  final VoidCallback? onSend;

  /// The calendar choices, read as the menu opens; only shown with a
  /// calendar.
  final NoteCalendar Function()? calendar;

  /// Adds a Section to the Note.
  final VoidCallback? onAmend;

  /// The photos and recordings that may go while their texts stay, read
  /// as the menu opens.
  final List<MediaRemoval> Function()? removals;

  /// Whether the Note is a Tally, read as the menu opens: the menu then
  /// offers [onAddEntry], else [onMakeTally].
  final bool Function()? isTally;

  /// Opens the field page that makes the Note a Tally (#103).
  final VoidCallback? onMakeTally;

  /// Opens the entry page of the Note's Tally.
  final VoidCallback? onAddEntry;

  /// Deletes the Note, after asking once.
  final VoidCallback? onDelete;

  const NoteActions({
    this.onImportance,
    this.onSend,
    this.calendar,
    this.onAmend,
    this.removals,
    this.isTally,
    this.onMakeTally,
    this.onAddEntry,
    this.onDelete,
  });
}

/// A Note drawn as a softly embossed card, top to bottom: the Profile of
/// the other person who wrote it, picture and name (none on the user's
/// own, ADR 0017); its capture time with the Named Place or coordinates
/// it was captured at and an icon when there is more Capture Context;
/// with [showPlace] where it is filed; a separator; its text, thumbnails
/// of its first [shownPhotos] photos, a play button per voice recording
/// with the start of its Transcript under it, and the start of each
/// Transcript and Photo Text whose recording or photo was removed; a
/// separator; and
/// its action row, icons only: file it, spawn a Routine, Reminder or Frog
/// from it, merge it, and ⋮ for the rest with words. Its Area's colour runs
/// down the left edge and its icon leads the place line. An important
/// Note gets an accent marker and a stronger emboss, an unimportant one
/// is dimmed (UI laws, "Cards").
class NoteCard extends StatelessWidget {
  final Note note;

  /// Formats the capture time; the list makes it once for all cards.
  final DateFormat when;

  /// The Area the Note sits in, directly or through [topic]; null while
  /// it is Unsorted.
  final Area? area;
  final Topic? topic;

  /// The Profile of the other person who wrote the Note; null for the
  /// user's own, which shows no Profile line.
  final Profile? author;

  /// Whether the card says its Area and Topic, or Unsorted: only where
  /// the screen around it does not say it already.
  final bool showPlace;

  /// Opens "file into…"; no button when null.
  final VoidCallback? onFile;

  /// Opens "merge into…"; no button when null.
  final VoidCallback? onMerge;

  /// Spawns a Routine, Reminder or Frog from the Note, picked from a menu
  /// that opens at the button; no button when null.
  final ValueChanged<Spawn>? onSchedule;

  /// The ⋮ menu's actions; no ⋮ when null.
  final NoteActions? actions;

  /// Opens the Note's editor.
  final VoidCallback? onTap;

  /// Shown where the action row goes, e.g. the trash's restore.
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
    this.showPlace = false,
    this.onFile,
    this.onMerge,
    this.onSchedule,
    this.actions,
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
    final place = !showPlace
        ? null
        : area == null
        ? (note.spawned ? null : t.unsorted)
        : [
            areaName(t, area),
            if (topic != null) topicName(t, topic),
          ].join(' / ');
    final important = note.importance == Importance.important;
    final footer = this.footer;
    final actions = this.actions;
    final acts =
        onFile != null ||
        onSchedule != null ||
        onMerge != null ||
        actions != null;
    final header = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (author case final author?) ...[
          _AuthorLine(noteId: note.id, author: author),
          const SizedBox(height: 4),
        ],
        Row(
          children: [
            Text(when.format(note.captured.toLocal()), style: text.bodySmall),
            if (whereCaptured(note.context) case final where?) ...[
              const SizedBox(width: 8),
              Icon(
                Icons.place_outlined,
                size: 14,
                color: scheme.onSurfaceVariant,
              ),
              const SizedBox(width: 2),
              Flexible(
                child: Text(
                  where,
                  style: text.bodySmall,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
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
              if (areaIconData(area?.icon) case final icon?) ...[
                Icon(icon, size: 16, color: colour ?? scheme.onSurfaceVariant),
                const SizedBox(width: 4),
              ],
              Flexible(child: Text(place, style: text.labelMedium)),
            ],
          ),
        ],
      ],
    );
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
                padding: EdgeInsets.fromLTRB(16, 16, 16, acts ? 4 : 16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(child: header),
                        if (important)
                          Icon(
                            Icons.label_important,
                            color: scheme.primary,
                            semanticLabel: t.importanceImportant,
                          ),
                      ],
                    ),
                    const Divider(height: 24),
                    // A photo Note may have no words.
                    if (note.text.isNotEmpty)
                      Text(note.text, style: text.bodyLarge),
                    if (note.photos.isNotEmpty) ...[
                      if (note.text.isNotEmpty) const SizedBox(height: 8),
                      // Not a sideways list: swipes on a card belong to
                      // the browser.
                      Row(
                        children: [
                          for (final hash in note.photos.take(shownPhotos)) ...[
                            NotePhoto(
                              key: ValueKey(hash),
                              hash: hash,
                              size: 64,
                            ),
                            const SizedBox(width: 8),
                          ],
                          if (note.photos.length > shownPhotos)
                            Text(
                              t.morePhotos(note.photos.length - shownPhotos),
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
                            RecordingButton(key: ValueKey(hash), hash: hash),
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
                    // Removed, the file shows no more; its text stays.
                    for (final hash in note.removedRecordings)
                      TranscriptText(
                        key: ValueKey('transcript:$hash'),
                        noteId: note.id,
                        hash: hash,
                        transcript: note.transcripts[hash],
                      ),
                    for (final hash in note.removedPhotos)
                      PhotoText(
                        key: ValueKey('ocr:$hash'),
                        noteId: note.id,
                        hash: hash,
                        photoText: note.photoTexts[hash],
                        compact: true,
                      ),
                    if (acts || footer != null) const Divider(height: 24),
                    // Icons only, so the row stays one line in every
                    // language; the words show on a long press.
                    if (acts)
                      Row(
                        key: const ValueKey('note-actions'),
                        children: [
                          if (onFile != null)
                            IconButton(
                              tooltip: t.fileInto,
                              onPressed: onFile,
                              icon: const Icon(Icons.drive_file_move_outline),
                            ),
                          if (onSchedule case final onSchedule?)
                            PopupMenuButton<Spawn>(
                              tooltip: t.routineOrReminder,
                              icon: const Icon(Icons.alarm_add_outlined),
                              onSelected: onSchedule,
                              itemBuilder: (_) => [
                                PopupMenuItem(
                                  value: Spawn.routine,
                                  child: Text(t.routine),
                                ),
                                PopupMenuItem(
                                  value: Spawn.reminder,
                                  child: Text(t.reminder),
                                ),
                                PopupMenuItem(
                                  value: Spawn.frog,
                                  child: Text(t.frog),
                                ),
                              ],
                            ),
                          if (onMerge != null)
                            IconButton(
                              tooltip: t.mergeInto,
                              onPressed: onMerge,
                              icon: const Icon(Icons.call_merge),
                            ),
                          const Spacer(),
                          if (actions != null)
                            _MoreButton(
                              importance: note.importance,
                              actions: actions,
                            ),
                        ],
                      ),
                    ?footer,
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

/// The ⋮ of a card's action row: a menu at the button with words. It
/// leads with the Note's Importance as radio buttons, the one it has
/// marked, then sending, the calendar, adding to the Note, "Add entry" of
/// a Tally or "Make it a Tally", removing a photo or recording while its
/// text stays, one entry each with the start of that text under it, and
/// deleting.
class _MoreButton extends StatelessWidget {
  final Importance importance;
  final NoteActions actions;

  const _MoreButton({required this.importance, required this.actions});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return PopupMenuButton<VoidCallback>(
      key: const ValueKey('note-menu'),
      tooltip: t.more,
      icon: const Icon(Icons.more_vert),
      onSelected: (run) => run(),
      itemBuilder: (button) {
        final calendar = CalendarScope.of(button).available
            ? actions.calendar?.call()
            : null;
        final tally = actions.isTally?.call();
        PopupMenuItem<VoidCallback> item(
          String key,
          IconData icon,
          String label,
          VoidCallback run,
        ) => PopupMenuItem(
          key: ValueKey(key),
          value: run,
          child: Row(
            children: [
              Icon(icon),
              const SizedBox(width: 12),
              Flexible(child: Text(label)),
            ],
          ),
        );
        return [
          if (actions.onImportance case final set?) ...[
            // The heading the choices continue: "Importance" "Important".
            PopupMenuItem(enabled: false, child: Text(t.importance)),
            for (final (value, label) in [
              (Importance.important, t.importanceImportant),
              (Importance.normal, t.importanceNormal),
              (Importance.unimportant, t.importanceUnimportant),
            ])
              PopupMenuItem(
                key: ValueKey('importance-${value.name}'),
                // A tap on the one it has changes nothing.
                value: () {
                  if (value != importance) set(value);
                },
                child: Semantics(
                  checked: value == importance,
                  inMutuallyExclusiveGroup: true,
                  child: Row(
                    children: [
                      Icon(
                        value == importance
                            ? Icons.radio_button_checked
                            : Icons.radio_button_unchecked,
                      ),
                      const SizedBox(width: 12),
                      Flexible(child: Text(label)),
                    ],
                  ),
                ),
              ),
            const PopupMenuDivider(),
          ],
          if (actions.onSend case final send?)
            item('note-send', Icons.send_outlined, t.sendToPerson, send),
          if (calendar != null && calendar.add.isNotEmpty)
            item(
              'note-calendar',
              Icons.event_outlined,
              t.addToCalendar,
              () => runCalendarChoice(button, calendar.add),
            ),
          if (calendar?.file case final file?)
            item(
              'note-calendar-file',
              Icons.share_outlined,
              file.label,
              file.run,
            ),
          if (actions.onAmend case final amend?)
            item('note-amend', Icons.add, t.amend, amend),
          if (tally == true)
            if (actions.onAddEntry case final add?)
              item('note-add-entry', Icons.add_chart, t.addTallyEntry, add),
          if (tally == false)
            if (actions.onMakeTally case final make?)
              item('note-make-tally', Icons.bar_chart, t.makeTally, make),
          for (final removal in actions.removals?.call() ?? <MediaRemoval>[])
            PopupMenuItem(
              key: ValueKey('note-remove-${removal.hash}'),
              value: removal.run,
              child: Row(
                children: [
                  Icon(
                    removal.photo
                        ? Icons.hide_image_outlined
                        : Icons.mic_off_outlined,
                  ),
                  const SizedBox(width: 12),
                  Flexible(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          removal.photo
                              ? t.removePhotoKeepText
                              : t.removeRecordingKeepText,
                        ),
                        // Which one, when there are several.
                        Text(
                          removal.text.trim().split('\n').first,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(button).textTheme.bodySmall?.copyWith(
                            color: Theme.of(button)
                                .colorScheme
                                .onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          if (actions.onDelete case final delete?)
            item('note-delete', Icons.delete_outline, t.delete, delete),
        ];
      },
    );
  }
}

/// Another person's Profile on a card, picture and name, opening their
/// Profile (#52) with room around it for a finger. It follows their
/// Profile and the user's Override as they change, as the writer of the
/// Note [noteId] is shown ([People.authorOf]).
class _AuthorLine extends StatefulWidget {
  final String noteId;

  /// The writer as the card was read.
  final Profile author;

  const _AuthorLine({required this.noteId, required this.author});

  @override
  State<_AuthorLine> createState() => _AuthorLineState();
}

class _AuthorLineState extends State<_AuthorLine> {
  Store? _store;
  late Profile? _author = widget.author;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final store = PersonScope.of(context)?.store;
    if (store == _store) return;
    _store?.removeProfileListener(_changed);
    _store = store?..addProfileListener(_changed);
  }

  @override
  void didUpdateWidget(_AuthorLine old) {
    super.didUpdateWidget(old);
    _author = widget.author;
  }

  @override
  void dispose() {
    _store?.removeProfileListener(_changed);
    super.dispose();
  }

  void _changed() {
    final store = _store;
    if (!mounted || store == null) return;
    setState(() => _author = store.authorOf(widget.noteId));
  }

  @override
  Widget build(BuildContext context) {
    final author = _author;
    if (author == null) return const SizedBox.shrink();
    final t = context.t;
    final id = author.id;
    final name = profileDisplayName(t, author.name);
    final opens = canOpenPerson(context, id);
    final line = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        ProfileAvatar(key: const ValueKey('author'), profile: author, size: 24),
        const SizedBox(width: 8),
        Flexible(
          child: Text(
            name,
            key: const ValueKey('author-name'),
            style: Theme.of(context).textTheme.labelLarge,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
    return Semantics(
      label: t.authorPicture(name),
      image: !opens,
      button: opens,
      excludeSemantics: true,
      onTap: opens ? () => openPerson(context, id) : null,
      child: !opens
          ? line
          : GestureDetector(
              key: const ValueKey('author-open'),
              behavior: HitTestBehavior.opaque,
              onTap: () => openPerson(context, id),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 6),
                child: line,
              ),
            ),
    );
  }
}

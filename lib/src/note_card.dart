import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'calendar_platform.dart';
import 'checklist_view.dart';
import 'l10n.dart';
import 'look.dart';
import 'ocr_platform.dart';
import 'person_screen.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'profile.dart';
import 'shopping_list_view.dart';
import 'speech_platform.dart';
import 'text_marks_view.dart';
import 'voice_platform.dart';

/// What a Note's card spawns from it: "Routine, Reminder, Frog or
/// Session".
enum Spawn { routine, reminder, frog, session }

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
/// opening the Note editor, and the actions it has. An entry is left out
/// when its callback is null.
class NoteActions {
  /// Opens the Note's editor, as a long press on the card does.
  final VoidCallback? onEdit;

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

  /// What the menu offers of a Checklist (#120), read as the menu opens:
  /// whether the Note is one, and making it one, turning it back into a
  /// plain Note and unchecking all of its Lines, each left out when null.
  final ChecklistMenu Function()? checklist;

  /// Picks a Shopping List for the lines of a Note being captured
  /// (#124), read as the menu opens; left out while it is null or gives
  /// null.
  final VoidCallback? Function()? addToShopping;

  /// Deletes the Note, after asking once.
  final VoidCallback? onDelete;

  const NoteActions({
    this.onEdit,
    this.onImportance,
    this.onSend,
    this.calendar,
    this.onAmend,
    this.removals,
    this.isTally,
    this.onMakeTally,
    this.onAddEntry,
    this.checklist,
    this.addToShopping,
    this.onDelete,
  });
}

/// The Checklist and Shopping List entries of a card's ⋮ as the menu
/// opens (#120, #122): [isList] while the Note is either.
typedef ChecklistMenu = ({
  bool isList,
  VoidCallback? make,
  VoidCallback? makeShopping,
  VoidCallback? toPlain,
  VoidCallback? uncheckAll,
});

/// A Note drawn as a softly embossed card, top to bottom: the Profile of
/// the other person who wrote it, picture and name (none on the user's
/// own, ADR 0017); its capture time with the Named Place or coordinates
/// it was captured at and an icon when there is more Capture Context;
/// with [showPlace] where it is filed; a separator; its first line,
/// thumbnails of its first [shownPhotos] photos, a play button per voice
/// recording with the start of its Transcript under it, and the start of
/// each Transcript and Photo Text whose recording or photo was removed; a
/// separator; and
/// its action row, icons only: file it, spawn a Routine, Reminder, Frog or
/// Session from it, merge it, and ⋮ for the rest with words. Its Area's colour runs
/// down the left edge and its icon leads the place line. An important
/// Note gets an accent marker and a stronger emboss, an unimportant one
/// is dimmed (UI laws, "Cards").
///
/// [full] shows the whole Note instead (#114): its Capture Context line
/// by line, its text, every photo at full width with its Photo Text under
/// it and every recording with its Transcript, both labelled and dimmed,
/// and then each of its [sections], oldest first, the same way under its
/// own Capture Context.
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

  /// Spawns a Routine, Reminder, Frog or Session from the Note, picked
  /// from a menu that opens at the button; no button when null.
  final ValueChanged<Spawn>? onSchedule;

  /// The ⋮ menu's actions; no ⋮ when null.
  final NoteActions? actions;

  /// Whether the card shows the whole Note with its Sections, rather
  /// than its first line.
  final bool full;

  /// Reads the Note's Sections, oldest first, for [full]; once per Note
  /// read, never while building.
  final List<Section> Function()? sections;

  /// What a tap on the card does: unfold a compact Note in a list, pick
  /// it in "merge into…"; nothing when null.
  final VoidCallback? onTap;

  /// Opens the Note's editor on a long press; nothing when null.
  final VoidCallback? onEdit;

  /// Shown where the action row goes, e.g. the trash's restore.
  final Widget? footer;

  /// The Note's Checklist, if it is one: its Lines on a [full] card, how
  /// many are checked on a compact one (#120); nothing when null.
  final NoteChecklist? checklist;

  /// The Note's Shopping List, if it is one: its Lines on a [full] card,
  /// how many are open on a compact one (#122); nothing when null.
  final NoteShoppingList? shopping;

  /// Changes the status mark of a line of the Note's text to the state
  /// picked (ADR 0031): each ☐, ✅ or ❌ at the start of a line opens Check,
  /// Fail and Open at the finger; they are plain characters when null, as
  /// in a Section's text, which has no edit.
  final void Function(int line, TextMark to)? onMark;

  /// How much faded an unimportant card is drawn.
  static const unimportantOpacity = 0.55;

  /// How many photo thumbnails a card shows; the rest are counted.
  static const shownPhotos = 3;

  /// How tall a photo on a full card may get, so one of extreme
  /// proportions cannot stretch the card.
  static const photoHeight = 480.0;

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
    this.full = false,
    this.sections,
    this.onTap,
    this.onEdit,
    this.footer,
    this.checklist,
    this.shopping,
    this.onMark,
  });

  /// Where the first line with words of [text] starts and ends, without
  /// the spaces around it, as a compact card shows it.
  static (int, int) firstLine(String text) {
    final start = text.length - text.trimLeft().length;
    var end = start;
    while (end < text.length && text[end] != '\n' && text[end] != '\r') {
      end++;
    }
    return (start, start + text.substring(start, end).trimRight().length);
  }

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
    final checklist = this.checklist;
    final shopping = this.shopping;
    // A Note is at most one of them; the other draws nothing.
    Widget? lists(bool full) => checklist == null && shopping == null
        ? null
        : Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (checklist != null)
                CardChecklist(
                  noteId: note.id,
                  checklist: checklist,
                  full: full,
                ),
              if (shopping != null)
                CardShoppingList(
                  noteId: note.id,
                  shopping: shopping,
                  full: full,
                ),
            ],
          );
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
        if (full)
          CaptureContextLines(capture: note.context, when: when)
        else
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
              onLongPress: onEdit,
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
                    if (full)
                      _FullNote(
                        note: note,
                        when: when,
                        sections: sections,
                        list: lists(true),
                        onMark: onMark,
                      )
                    else ...[
                      // A photo Note may have no words.
                      if (note.text.isNotEmpty)
                        if (firstLine(note.text) case (final start, final end))
                          MarkedText(
                            note.text,
                            key: const ValueKey('note-first-line'),
                            format: note.textFormat,
                            start: start,
                            end: end,
                            style: text.bodyLarge,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            onMark: onMark,
                          ),
                      ?lists(false),
                      if (note.photos.isNotEmpty) ...[
                        if (note.text.isNotEmpty) const SizedBox(height: 8),
                        // Not a sideways list: swipes on a card belong to
                        // the browser.
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
                    ],
                    if (acts || footer != null) const Divider(height: 24),
                    if (acts)
                      NoteActionRow(
                        importance: note.importance,
                        onFile: onFile,
                        onSchedule: onSchedule,
                        onMerge: onMerge,
                        actions: actions,
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

/// A Note's action row, icons only, so it stays one line in every
/// language; the words show on a long press: file it, spawn one of
/// [spawns] from it (a Routine, Reminder, Frog or Session), merge it, and
/// ⋮ for the rest with words. Each is left out when its callback is
/// null. The card draws it, and so does a capture screen (#113).
class NoteActionRow extends StatelessWidget {
  /// The Importance ⋮ marks.
  final Importance importance;
  final VoidCallback? onFile;
  final ValueChanged<Spawn>? onSchedule;
  final VoidCallback? onMerge;
  final NoteActions? actions;

  /// What the spawn menu offers, in this order.
  final List<Spawn> spawns;

  const NoteActionRow({
    super.key = const ValueKey('note-actions'),
    required this.importance,
    this.onFile,
    this.onSchedule,
    this.onMerge,
    this.actions,
    this.spawns = Spawn.values,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final actions = this.actions;
    return Row(
      children: [
        if (onFile != null)
          IconButton(
            tooltip: t.fileInto,
            onPressed: onFile,
            icon: const Icon(Icons.drive_file_move_outline),
          ),
        if (onSchedule case final onSchedule? when spawns.isNotEmpty)
          PopupMenuButton<Spawn>(
            tooltip: t.routineOrReminder,
            icon: const Icon(Icons.alarm_add_outlined),
            onSelected: onSchedule,
            itemBuilder: (_) => [
              for (final spawn in spawns)
                PopupMenuItem(
                  value: spawn,
                  child: Text(switch (spawn) {
                    Spawn.routine => t.routine,
                    Spawn.reminder => t.reminder,
                    Spawn.frog => t.frog,
                    Spawn.session => t.session,
                  }),
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
          _MoreButton(importance: importance, actions: actions),
      ],
    );
  }
}

/// The ⋮ of a card's action row: a menu at the button with words. It
/// leads with Edit, then the Note's Importance as radio buttons, the one it has
/// marked, then sending, the calendar, adding to the Note, "Add entry" of
/// a Tally or "Make it a Tally", "Make it a Checklist" and "Make it a
/// Shopping List" or, for one, "Uncheck all" (a Checklist's) and "Back to
/// a plain Note", removing a photo or recording while its
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
        final list = actions.checklist?.call();
        final addToShopping = actions.addToShopping?.call();
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
          if (actions.onEdit case final edit?) ...[
            item('note-edit', Icons.edit_outlined, t.edit, edit),
            const PopupMenuDivider(),
          ],
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
          // A Checklist or Shopping List is no Tally, and a Tally neither.
          if (tally == false && !(list?.isList ?? false))
            if (actions.onMakeTally case final make?)
              item('note-make-tally', Icons.bar_chart, t.makeTally, make),
          if (addToShopping != null)
            item(
              'note-add-to-shopping',
              Icons.add_shopping_cart,
              t.addToShoppingList,
              addToShopping,
            ),
          if (list?.make case final make?)
            item('note-make-checklist', Icons.checklist, t.makeChecklist, make),
          if (list?.makeShopping case final make?)
            item(
              'note-make-shopping',
              Icons.shopping_cart_outlined,
              t.makeShoppingList,
              make,
            ),
          if (list?.uncheckAll case final uncheck?)
            item(
              'note-uncheck-all',
              Icons.check_box_outline_blank,
              t.checklistUncheckAll,
              uncheck,
            ),
          if (list?.toPlain case final plain?)
            item('note-to-plain', Icons.notes, t.checklistToPlain, plain),
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

/// The Capture Context of a Note or Section, one line per thing known:
/// the time; where it was, the Named Place with its coordinates or the
/// coordinates alone; the app it was shared in from; the calendar events
/// running; the Bluetooth devices connected.
class CaptureContextLines extends StatelessWidget {
  final CaptureContext capture;
  final DateFormat when;

  const CaptureContextLines({
    super.key,
    required this.capture,
    required this.when,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final muted = Theme.of(context).colorScheme.onSurfaceVariant;
    final text = Theme.of(context).textTheme;
    final location = capture.location;
    final named = capture.namedPlace;
    final lines = [
      (Icons.schedule, when.format(capture.captured.toLocal())),
      if (location != null)
        (
          Icons.place_outlined,
          named == null
              ? coordinatesText(location)
              : '${named.name} (${coordinatesText(location)})',
        ),
      if (capture.sourceApp case final app?)
        (Icons.apps, t.contextSourceApp(app.name)),
      if (capture.calendarEvents.isNotEmpty)
        (
          Icons.event_outlined,
          t.contextDuring(capture.calendarEvents.join(', ')),
        ),
      if (capture.bluetoothDevices.isNotEmpty)
        (
          Icons.bluetooth,
          t.contextBluetooth(capture.bluetoothDevices.join(', ')),
        ),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final (i, (icon, line)) in lines.indexed) ...[
          if (i > 0) const SizedBox(height: 4),
          Row(
            children: [
              Icon(icon, size: 20, color: muted),
              const SizedBox(width: 8),
              Flexible(child: Text(line, style: text.bodyMedium)),
            ],
          ),
        ],
      ],
    );
  }
}

/// The whole of a Note on a full card: its text, photos and recordings,
/// then each Section under a separator and its own Capture Context. The
/// Sections are read when the Note is, not while building.
class _FullNote extends StatefulWidget {
  final Note note;
  final DateFormat when;
  final List<Section> Function()? sections;

  /// The Note's Checklist, under its text.
  final Widget? list;

  /// Changes a status mark of the Note's text; never one of a Section's.
  final void Function(int line, TextMark to)? onMark;

  const _FullNote({
    required this.note,
    required this.when,
    this.sections,
    this.list,
    this.onMark,
  });

  @override
  State<_FullNote> createState() => _FullNoteState();
}

class _FullNoteState extends State<_FullNote> {
  List<Section> _sections = const [];

  @override
  void initState() {
    super.initState();
    _read();
  }

  @override
  void didUpdateWidget(_FullNote old) {
    super.didUpdateWidget(old);
    // Read again only when the Note was.
    if (old.note != widget.note ||
        (old.sections == null) != (widget.sections == null)) {
      _read();
    }
  }

  void _read() => _sections = widget.sections?.call() ?? const [];

  @override
  Widget build(BuildContext context) {
    final note = widget.note;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _Whole(
          id: note.id,
          text: note.text,
          format: note.textFormat,
          photos: note.photos,
          photoTexts: note.photoTexts,
          removedPhotos: note.removedPhotos,
          recordings: note.recordings,
          transcripts: note.transcripts,
          removedRecordings: note.removedRecordings,
          list: widget.list,
          onMark: widget.onMark,
        ),
        for (final section in _sections)
          Column(
            key: ValueKey('section:${section.id}'),
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Divider(height: 24),
              CaptureContextLines(capture: section.context, when: widget.when),
              const SizedBox(height: 8),
              _Whole(
                id: section.id,
                text: section.text,
                format: section.textFormat,
                photos: section.photos,
                photoTexts: section.photoTexts,
                removedPhotos: section.removedPhotos,
                recordings: section.recordings,
                transcripts: section.transcripts,
                removedRecordings: section.removedRecordings,
              ),
            ],
          ),
      ],
    );
  }
}

/// The text, photos and recordings of a Note or Section [id] in full:
/// each photo at full width, no taller than [NoteCard.photoHeight],
/// opening full screen on a tap, with its Photo Text under it; each
/// recording's play button with its Transcript; then the texts of the
/// photos and recordings removed while their texts stay.
class _Whole extends StatelessWidget {
  final String id;
  final String text;

  /// The format [text] is in: a formatted one is drawn formatted.
  final String format;
  final List<String> photos;
  final Map<String, String> photoTexts;
  final List<String> removedPhotos;
  final List<String> recordings;
  final Map<String, String> transcripts;
  final List<String> removedRecordings;

  /// A Checklist's Lines, right under the text.
  final Widget? list;

  /// Changes a status mark of the text; plain characters when null.
  final void Function(int line, TextMark to)? onMark;

  const _Whole({
    required this.id,
    required this.text,
    required this.format,
    required this.photos,
    required this.photoTexts,
    required this.removedPhotos,
    required this.recordings,
    required this.transcripts,
    required this.removedRecordings,
    this.list,
    this.onMark,
  });

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(context).textTheme.bodyLarge;
    final list = this.list;
    final parts = <Widget>[
      // A photo Note may have no words.
      if (text.isNotEmpty && list == null)
        MarkedText(text, format: format, style: style, onMark: onMark),
      // It keeps its own space: it is nothing on a Note that is none.
      if (list != null)
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (text.isNotEmpty)
              MarkedText(text, format: format, style: style, onMark: onMark),
            list,
          ],
        ),
      for (final hash in photos)
        Column(
          key: ValueKey(hash),
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            NotePhoto(
              hash: hash,
              maxHeight: NoteCard.photoHeight,
              onTap: () => viewPhoto(context, hash),
            ),
            PhotoText(
              noteId: id,
              hash: hash,
              photoText: photoTexts[hash],
              labelled: true,
            ),
          ],
        ),
      // Removed, the file shows no more; its text stays.
      for (final hash in removedPhotos)
        PhotoText(
          key: ValueKey('ocr:$hash'),
          noteId: id,
          hash: hash,
          photoText: photoTexts[hash],
          labelled: true,
        ),
      for (final hash in recordings)
        Column(
          key: ValueKey(hash),
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            RecordingButton(hash: hash),
            TranscriptText(
              noteId: id,
              hash: hash,
              transcript: transcripts[hash],
              labelled: true,
            ),
          ],
        ),
      for (final hash in removedRecordings)
        TranscriptText(
          key: ValueKey('transcript:$hash'),
          noteId: id,
          hash: hash,
          transcript: transcripts[hash],
          labelled: true,
        ),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final (i, part) in parts.indexed) ...[
          if (i > 0) const SizedBox(height: 8),
          part,
        ],
      ],
    );
  }
}

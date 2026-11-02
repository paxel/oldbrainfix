import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'area_look.dart';
import 'calendar_platform.dart';
import 'card_occurrences.dart';
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
import 'table_screen.dart';
import 'session_note.dart';
import 'tally_view.dart';
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

/// Where [capture] happened as shown on a card and in its Capture Context:
/// its Named Place, else its coordinates; null while unknown.
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

  /// Sets the Importance of a Note being captured, picked in the menu;
  /// a card leaves it out, its editor's badge changes it (#182).
  final ValueChanged<Importance>? onImportance;

  /// Sends the Note to a person as a Sent Item.
  final VoidCallback? onSend;

  /// Hands the Note's text, photos and recordings to another app.
  final VoidCallback? onShare;

  /// Puts the text of the Note and of its Sections on the clipboard
  /// (#183).
  final VoidCallback? onCopy;

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
  /// plain Note, unchecking all of its Lines and its "Show on the Agenda"
  /// (#143), each left out when null.
  final ChecklistMenu Function()? checklist;

  /// Picks a Shopping List for the lines of a Note being captured
  /// (#124), read as the menu opens; left out while it is null or gives
  /// null.
  final VoidCallback? Function()? addToShopping;

  /// Whether the Note shows after each box who ticked it last and when,
  /// and switching that, read as the menu opens (ADR 0035); left out
  /// while it is null or gives null, as for a Note without boxes.
  final ({bool on, VoidCallback toggle})? Function()? showTicks;

  /// Deletes the Note, after asking once.
  final VoidCallback? onDelete;

  const NoteActions({
    this.onEdit,
    this.onImportance,
    this.onSend,
    this.onShare,
    this.onCopy,
    this.calendar,
    this.onAmend,
    this.removals,
    this.isTally,
    this.onMakeTally,
    this.onAddEntry,
    this.checklist,
    this.addToShopping,
    this.showTicks,
    this.onDelete,
  });
}

/// The Checklist and Shopping List entries of a card's ⋮ as the menu
/// opens (#120, #122): [isList] while the Note is either; [onAgenda] how
/// a Shopping List (#186) or a Checklist without a Routine (#219) shows
/// on this person's Agenda, and choosing that.
typedef ChecklistMenu = ({
  bool isList,
  VoidCallback? make,
  VoidCallback? makeShopping,
  VoidCallback? toPlain,
  VoidCallback? uncheckAll,
  ListOnAgendaChoice? onAgenda,
});

/// How a Shopping List or a Checklist shows on this person's Agenda
/// (#186, #219), and choosing another way.
typedef ListOnAgendaChoice = ({
  ListOnAgenda shows,
  ValueChanged<ListOnAgenda> choose,
});

/// The three ways a Shopping List or a Checklist shows on the Agenda with
/// their words, in the order they are offered.
List<(ListOnAgenda, String)> listOnAgendaOptions(AppLocalizations t) => [
  (ListOnAgenda.off, t.settingOff),
  (ListOnAgenda.count, t.shoppingOnAgendaCount),
  (ListOnAgenda.lines, t.shoppingOnAgendaLines),
];

/// A Note drawn as a softly embossed card, top to bottom: the Profile of
/// the other person who wrote it, picture and name (none on the user's
/// own, ADR 0017); with [showPlace] where it is filed; a separator under
/// them; then the Note in its compact or its detailed view ([full], #189).
/// Its Area's colour runs down the left edge and its icon leads the place
/// line. An important Note gets an accent marker and a stronger emboss, an
/// unimportant one is dimmed (UI laws, "Cards").
///
/// The compact view is a glance by the Note's kind ([_CompactNote]), with
/// ⋮ top right and no buttons at the bottom and no Capture Context. The
/// detailed view shows the whole Note (#114): its text, every photo at
/// full width with its Photo Text under it and every recording with its
/// Transcript, both labelled and dimmed, its Capture Context as one line
/// under its text (#170), then each of its [sections], oldest first, the
/// same way; and under a separator its action row, icons only: file it,
/// spawn a Routine, Reminder, Frog or Session from it, merge it, and ⋮ for
/// the rest with words.
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

  /// Whether the card shows its detailed view, the whole Note with its
  /// Sections, rather than its compact view (#189).
  final bool full;

  /// Reads the Note's Sections, oldest first, for [full]; once per Note
  /// read, never while building.
  final List<Section> Function()? sections;

  /// What a tap on the card does: switch its view in a list (#189), pick
  /// it in "merge into…"; nothing when null.
  final VoidCallback? onTap;

  /// Opens the Note's editor on a long press; nothing when null.
  final VoidCallback? onEdit;

  /// Shown where the action row goes, e.g. the trash's restore.
  final Widget? footer;

  /// The Note's Checklist, if it is one: its Lines on a [full] card, how
  /// many are open on a compact one (#120, #189); nothing when null.
  final NoteChecklist? checklist;

  /// The Note's Shopping List, if it is one: its open Lines on a [full]
  /// card, the struck ones folded under them, how many are open on a
  /// compact one (#122, #189); nothing when null.
  final NoteShoppingList? shopping;

  /// The Notes the Note's Tally is read from, if it is one: on a [full]
  /// card its chart, its entries and "Add entry" under its name (#190);
  /// nothing when null.
  final Notes? tally;

  /// The Notes the Note's Session is read from, if it is one: on a [full]
  /// card its pool, today's picks of its Routines and its last weeks
  /// (#191); nothing when null.
  final Notes? session;

  /// Reads the Note's kind (#188), never while building: a Note of a
  /// kind shows its name as the title, on a [full] card its description
  /// folded under it; a plain Note when null.
  final NoteKind Function()? kind;

  /// Reads what the compact view says under the name of the Note's Tally
  /// or of its Session (#189, #191), never while building; nothing when
  /// null.
  final List<String> Function()? glance;

  /// How the boxes of the Note's text, or of a Section's, given by its
  /// entity and its boxes, are ticked (ADR 0035): each ☐, ✅ or ❌ opens
  /// Check, Fail, Open and its History at the finger; they are plain
  /// characters when null.
  final BoxTicking Function(String entity, List<ShownBox> boxes)? boxes;

  /// Changes a table of the Note's text in its full-screen view (ADR
  /// 0033); the table is only read there when null, as a Section's.
  final TableEditing? onTable;

  /// Today's Occurrences of the Note's Routines and Reminders, a box for
  /// each in either view that ticks it as on the Agenda (#215); none when
  /// null.
  final NoteOccurrences? occurrences;

  /// How much faded an unimportant card is drawn.
  static const unimportantOpacity = 0.55;

  /// How many photo thumbnails a card shows; the rest are counted.
  static const shownPhotos = 3;

  /// How tall a photo in the detailed view may get, so one of extreme
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
    this.tally,
    this.session,
    this.kind,
    this.glance,
    this.boxes,
    this.onTable,
    this.occurrences,
  });

  /// Where the first line with words of [text] starts and ends, without
  /// the spaces around it, as a compact card shows it; in a [formatted]
  /// text opening with a code block, the first after its fence.
  static (int, int) firstLine(String text, {bool formatted = false}) {
    final start = text.length - text.trimLeft().length;
    var end = start;
    while (end < text.length && text[end] != '\n' && text[end] != '\r') {
      end++;
    }
    final first = (
      start,
      start + text.substring(start, end).trimRight().length,
    );
    if (!formatted) return first;
    // A code block's opening fence shows nothing: its first line stands
    // in (#212).
    final block = Markdown.parse(text)[Markdown.lineAt(text, start)];
    if (block.code case final code? when code.firstLine == block.line) {
      return nextLines(text, block.end, 1) ?? first;
    }
    return first;
  }

  /// Where the [count] lines after [from] in [text] start and end, from
  /// the first with words, without the spaces around them, as a compact
  /// card shows them under a plain Note's first line (#189); null when no
  /// words follow.
  static (int, int)? nextLines(String text, int from, [int count = 2]) {
    final rest = text.substring(from);
    final start = from + rest.length - rest.trimLeft().length;
    if (start >= text.length) return null;
    var end = start;
    for (var lines = 0; end < text.length; end++) {
      if (text[end] == '\n' && ++lines == count) break;
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
    final tally = full ? this.tally : null;
    final session = full ? this.session : null;
    // A Note is at most one of them; the others draw nothing. A Tally's
    // and a Session's compact view is its glance.
    Widget? lists(bool full) =>
        checklist == null &&
            shopping == null &&
            tally == null &&
            session == null
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
              if (tally != null) CardTally(notes: tally, noteId: note.id),
              if (session != null) CardSession(notes: session, noteId: note.id),
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
    // Its time and place show under the text of the detailed view (#170):
    // with no Profile or place over the text it has no header.
    final headed = author != null || place != null;
    final marker = important
        ? Icon(
            Icons.label_important,
            color: scheme.primary,
            semanticLabel: t.importanceImportant,
          )
        : null;
    // The compact view has no buttons at the bottom; its ⋮ sits top right
    // (#189).
    final menu = !full && actions != null
        ? _MoreButton(
            importance: note.importance,
            actions: actions,
            // What the action row offers, as the compact view has none.
            onFile: onFile,
            onMerge: onMerge,
          )
        : null;
    final bottom = full && acts;
    Widget content = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (headed) ...[
          if (full)
            // The Important marker beside it.
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(child: header),
                ?marker,
              ],
            )
          else
            header,
          const Divider(height: 24),
        ],
        // Under where it is filed, over its text (#215).
        if (occurrences case final occurrences?)
          CardOccurrences(noteId: note.id, occurrences: occurrences),
        if (full)
          // With nothing over its text it starts with it, the Important
          // marker beside it.
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: _FullNote(
                  note: note,
                  when: when,
                  sections: sections,
                  list: lists(true),
                  kind: kind,
                  boxes: boxes,
                  onTable: onTable,
                ),
              ),
              if (!headed) ?marker,
            ],
          )
        else
          _CompactNote(
            note: note,
            list: lists(false),
            kind: kind,
            glance: glance,
            ticking: boxes?.call(note.id, note.boxes),
          ),
      ],
    );
    if (!full && (menu != null || marker != null)) {
      content = Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Padding(
              // In line with ⋮, which sits closer to the card's corner.
              padding: EdgeInsets.only(top: menu == null ? 0 : 12),
              child: content,
            ),
          ),
          Column(children: [?menu, ?marker]),
        ],
      );
    }
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
                padding: EdgeInsets.fromLTRB(
                  16,
                  menu == null ? 16 : 4,
                  menu == null ? 16 : 4,
                  bottom ? 4 : 16,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    content,
                    if (bottom || footer != null) const Divider(height: 24),
                    if (bottom)
                      NoteActionRow(
                        importance: note.importance,
                        onFile: onFile,
                        onSchedule: onSchedule,
                        onMerge: onMerge,
                        actions: actions,
                        // Only a plain Note becomes a Session (ADR 0039);
                        // a Routine of a Session Note runs it already.
                        spawnsNow: switch (kind) {
                          final kind? => () => [
                            for (final spawn in Spawn.values)
                              if (spawn != Spawn.session ||
                                  kind() == NoteKind.plain)
                                spawn,
                          ],
                          null => null,
                        },
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

  /// Reads what the spawn menu offers as it opens, in place of [spawns].
  final List<Spawn> Function()? spawnsNow;

  const NoteActionRow({
    super.key = const ValueKey('note-actions'),
    required this.importance,
    this.onFile,
    this.onSchedule,
    this.onMerge,
    this.actions,
    this.spawns = Spawn.values,
    this.spawnsNow,
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
              for (final spawn in spawnsNow?.call() ?? spawns)
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

/// The ⋮ of a card's action row, or of a compact card's top right: a menu
/// at the button with words. It leads with Edit, then on a compact card
/// File into… and Merge into… (#189), then, for a Note being captured, its Importance as radio
/// buttons, the one it has marked, then Show ticks while it has boxes, then sending, sharing, copying its text, the calendar, adding to the Note, "Add entry" of
/// a Tally or "Make it a Tally", "Make it a Checklist" and "Make it a
/// Shopping List" or, for one, "Show on the Agenda" (a Checklist's a
/// switch, a Shopping List's three choices as radio buttons) and "Uncheck
/// all" (a Checklist's) and "Back to a plain Note", removing a photo or recording while its
/// text stays, one entry each with the start of that text under it, and
/// deleting.
class _MoreButton extends StatelessWidget {
  final Importance importance;
  final NoteActions actions;

  /// "File into…" and "Merge into…" after Edit, for a compact card, which
  /// has no action row (#189); left out when null.
  final VoidCallback? onFile;
  final VoidCallback? onMerge;

  const _MoreButton({
    required this.importance,
    required this.actions,
    this.onFile,
    this.onMerge,
  });

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
        final ticks = actions.showTicks?.call();
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
          if (onFile case final file?)
            item('note-file', Icons.drive_file_move_outline, t.fileInto, file),
          if (onMerge case final merge?)
            item('note-merge', Icons.call_merge, t.mergeInto, merge),
          if (onFile != null || onMerge != null) const PopupMenuDivider(),
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
          if (ticks != null) showTicksItem(button, ticks),
          if (actions.onSend case final send?)
            item('note-send', Icons.send_outlined, t.sendToPerson, send),
          if (actions.onShare case final share?)
            item('note-share', Icons.share_outlined, t.shareToApp, share),
          if (actions.onCopy case final copy?)
            item('note-copy', Icons.copy_outlined, t.copyText, copy),
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
          if (list?.onAgenda case final agenda?) ...[
            const PopupMenuDivider(),
            // The heading the choices continue, as Importance's.
            PopupMenuItem(enabled: false, child: Text(t.checklistShowOnAgenda)),
            for (final (value, label) in listOnAgendaOptions(t))
              PopupMenuItem(
                key: ValueKey('note-show-on-agenda-${value.name}'),
                // A tap on the one it has changes nothing.
                value: () {
                  if (value != agenda.shows) agenda.choose(value);
                },
                child: Semantics(
                  checked: value == agenda.shows,
                  inMutuallyExclusiveGroup: true,
                  child: Row(
                    children: [
                      Icon(
                        value == agenda.shows
                            ? Icons.radio_button_checked
                            : Icons.radio_button_unchecked,
                      ),
                      const SizedBox(width: 12),
                      Flexible(child: Text(label)),
                    ],
                  ),
                ),
              ),
          ],
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

/// The menu entry that switches Show ticks of a Note on and off, [ticks]
/// telling whether it is on (ADR 0035): its words, and the switch below
/// them.
PopupMenuItem<VoidCallback> showTicksItem(
  BuildContext context,
  ({bool on, VoidCallback toggle}) ticks,
) => _switchItem(
  const ValueKey('note-show-ticks'),
  Icons.history_toggle_off,
  context.t.boxShowTicks,
  ticks,
);

/// A menu entry with its icon and words, and the switch below them.
PopupMenuItem<VoidCallback> _switchItem(
  Key key,
  IconData icon,
  String label,
  ({bool on, VoidCallback toggle}) state,
) => PopupMenuItem(
  key: key,
  value: state.toggle,
  child: Semantics(
    toggled: state.on,
    child: Row(
      children: [
        Icon(icon),
        const SizedBox(width: 12),
        Flexible(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(label),
              ExcludeSemantics(
                child: IgnorePointer(
                  child: Switch(value: state.on, onChanged: (_) {}),
                ),
              ),
            ],
          ),
        ),
      ],
    ),
  ),
);

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

/// The Capture Context of a Note or Section in its Captured dialog, one
/// line per thing known: the time; where it was, its Named Place, else
/// its coordinates; the app it was shared in from; the calendar events
/// running; the Bluetooth devices connected. Each detail in [removable]
/// ends in a Remove that calls [onRemove] (ADR 0037); the time has none.
/// Beside its text the same reads as one [CaptureLine] (#170).
class CaptureContextLines extends StatelessWidget {
  final CaptureContext capture;
  final DateFormat when;
  final Set<CapturedDetail> removable;
  final void Function(CapturedDetail detail)? onRemove;

  const CaptureContextLines({
    super.key,
    required this.capture,
    required this.when,
    this.removable = const {},
    this.onRemove,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final muted = Theme.of(context).colorScheme.onSurfaceVariant;
    final text = Theme.of(context).textTheme;
    final lines = <(IconData, String, CapturedDetail?)>[
      (Icons.schedule, when.format(capture.captured.toLocal()), null),
      if (capture.location != null)
        (Icons.place_outlined, whereCaptured(capture)!, CapturedDetail.place),
      if (capture.sourceApp case final app?)
        (Icons.apps, t.contextSourceApp(app.name), CapturedDetail.app),
      if (capture.calendarEvents.isNotEmpty)
        (
          Icons.event_outlined,
          t.contextDuring(capture.calendarEvents.join(', ')),
          CapturedDetail.event,
        ),
      if (capture.bluetoothDevices.isNotEmpty)
        (
          Icons.bluetooth,
          t.contextBluetooth(capture.bluetoothDevices.join(', ')),
          CapturedDetail.device,
        ),
    ];
    final remove = onRemove;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final (i, (icon, line, detail)) in lines.indexed) ...[
          if (i > 0) const SizedBox(height: 4),
          if (remove != null && detail != null && removable.contains(detail))
            Row(
              children: [
                Icon(icon, size: 20, color: muted),
                const SizedBox(width: 8),
                Expanded(child: Text(line, style: text.bodyMedium)),
                Semantics(
                  label: t.removeCapturedLabel(detail.name),
                  excludeSemantics: true,
                  button: true,
                  onTap: () => remove(detail),
                  child: TextButton(
                    key: ValueKey('remove-captured-${detail.name}'),
                    onPressed: () => remove(detail),
                    child: Text(t.remove),
                  ),
                ),
              ],
            )
          else
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

/// [capture] as one line read beside its text (#170): when, then where
/// (its Named Place, else its coordinates), the calendar events running,
/// the Bluetooth devices connected and the app it was shared in from,
/// joined by " · ". A part captured the same local day as the one
/// before it ([after]) shows only its time; another day the date too.
String captureLine(CaptureContext capture, DateFormat when, {DateTime? after}) {
  final at = capture.captured.toLocal();
  final before = after?.toLocal();
  final sameDay =
      before != null &&
      before.year == at.year &&
      before.month == at.month &&
      before.day == at.day;
  return [
    (sameDay ? DateFormat.Hm(when.locale) : when).format(at),
    ?whereCaptured(capture),
    if (capture.calendarEvents.isNotEmpty) capture.calendarEvents.join(', '),
    if (capture.bluetoothDevices.isNotEmpty)
      capture.bluetoothDevices.join(', '),
    ?capture.sourceApp?.name,
  ].join(' · ');
}

/// The Capture Context of a Note or Section under its text, in the
/// background (#170): its [captureLine], small and in the weak colour,
/// wrapping only where it must. A Section whose text was [edited] says
/// under it, the same way, when that was and, when known, who did it
/// ([editedBy], #183). With [onTap] it is a button that opens the
/// details, read as "Captured: …" with its line.
class CaptureLine extends StatelessWidget {
  final CaptureContext capture;
  final DateFormat when;

  /// When the part before it was captured; null for a Note's own.
  final DateTime? after;
  final DateTime? edited;
  final String? editedBy;
  final VoidCallback? onTap;

  const CaptureLine({
    super.key,
    required this.capture,
    required this.when,
    this.after,
    this.onTap,
  }) : edited = null,
       editedBy = null;

  /// The line of [section]: its Capture Context and when its text was
  /// edited, and by whom.
  CaptureLine.ofSection(
    Section section, {
    super.key,
    required this.when,
    this.after,
    this.onTap,
  }) : capture = section.context,
       edited = section.modified,
       editedBy = switch (section.editedBy?.name.trim()) {
         // A Profile without a name says nothing of who it is.
         final name? when name.isNotEmpty => name,
         _ => null,
       };

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final style = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    final line = captureLine(capture, when, after: after);
    final at = edited;
    final by = editedBy;
    final tap = onTap;
    final shown = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Read once, as a badge is.
        if (tap == null)
          Text(line, style: style)
        else
          Semantics(
            button: true,
            label: t.settingBadgeLabel(t.capturedHeading, line),
            excludeSemantics: true,
            onTap: tap,
            child: Text(line, style: style),
          ),
        if (at != null)
          Padding(
            key: const ValueKey('section-edited'),
            padding: const EdgeInsets.only(top: 2),
            child: Text(
              by == null
                  ? t.sectionEdited(when.format(at.toLocal()))
                  : t.sectionEditedBy(when.format(at.toLocal()), by),
              style: style,
            ),
          ),
      ],
    );
    if (tap == null) return shown;
    return InkWell(
      onTap: tap,
      borderRadius: BorderRadius.circular(4),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: shown,
      ),
    );
  }
}

/// The compact view of a Note's card (#189), by its kind: a Checklist or
/// a Shopping List shows its name and how many of its Lines are open
/// ([list]), a Tally its name and each field's total today against its
/// target ([glance]), a Session its name, the next run of its Routines
/// and this week's runs Done and Missed ([glance], #191); a plain Note
/// its first line and the two lines after it; a Note without words its
/// photos and recordings. The kind and the glance are read when the Note
/// is, not while building.
class _CompactNote extends StatefulWidget {
  final Note note;
  final Widget? list;
  final NoteKind Function()? kind;
  final List<String> Function()? glance;

  /// How the boxes of the text are ticked; plain characters when null.
  final BoxTicking? ticking;

  const _CompactNote({
    required this.note,
    this.list,
    this.kind,
    this.glance,
    this.ticking,
  });

  @override
  State<_CompactNote> createState() => _CompactNoteState();
}

class _CompactNoteState extends State<_CompactNote> {
  NoteKind _kind = NoteKind.plain;
  List<String> _glance = const [];

  @override
  void initState() {
    super.initState();
    _read();
  }

  @override
  void didUpdateWidget(_CompactNote old) {
    super.didUpdateWidget(old);
    // Its kind and its Tally's entries change without its text, so this
    // reads every time the card is built anew.
    _read();
  }

  void _read() {
    _kind = widget.kind?.call() ?? NoteKind.plain;
    _glance = widget.glance?.call() ?? const [];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final text = theme.textTheme;
    final note = widget.note;
    final shown = note.shownText;
    Widget marked(
      int start,
      int end, {
      Key? key,
      TextStyle? style,
      required int lines,
    }) => MarkedText(
      shown,
      key: key,
      format: note.textFormat,
      start: start,
      end: end,
      style: style,
      maxLines: lines,
      overflow: TextOverflow.ellipsis,
      boxes: note.boxes,
      source: note.text,
      ticking: widget.ticking,
    );
    final glance = [
      for (final (i, line) in _glance.indexed)
        Padding(
          key: ValueKey('card-glance-$i'),
          padding: const EdgeInsets.only(top: 4),
          child: Text(line, style: text.bodyMedium),
        ),
    ];
    if (_kind != NoteKind.plain) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (NoteKinds.name(shown).length case final end when end > 0)
            marked(
              0,
              end,
              key: const ValueKey('card-name'),
              style: text.titleMedium,
              lines: 1,
            ),
          ?widget.list,
          ...glance,
        ],
      );
    }
    final (start, end) = NoteCard.firstLine(
      shown,
      formatted: note.textFormat == TextFormats.markdown,
    );
    final next = glance.isEmpty ? NoteCard.nextLines(shown, end) : null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // A photo Note may have no words.
        if (shown.trim().isNotEmpty)
          marked(
            start,
            end,
            key: const ValueKey('note-first-line'),
            style: text.bodyLarge,
            lines: 1,
          ),
        if (next case (final from, final to))
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: marked(
              from,
              to,
              key: const ValueKey('note-next-lines'),
              style: text.bodyMedium,
              lines: 2,
            ),
          ),
        ...glance,
        // Without words it shows what it holds instead.
        if (shown.trim().isEmpty) ...[
          if (note.photos.isNotEmpty)
            // Not a sideways list: swipes on a card belong to the browser.
            Row(
              children: [
                for (final hash in note.photos.take(NoteCard.shownPhotos)) ...[
                  NotePhoto(key: ValueKey(hash), hash: hash, size: 64),
                  const SizedBox(width: 8),
                ],
                if (note.photos.length > NoteCard.shownPhotos)
                  Text(
                    t.morePhotos(note.photos.length - NoteCard.shownPhotos),
                    style: text.labelLarge,
                  ),
              ],
            ),
          if (note.recordings.isNotEmpty) ...[
            if (note.photos.isNotEmpty) const SizedBox(height: 8),
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
      ],
    );
  }
}

/// The setting of this Device that keeps the description of the Note
/// [noteId] unfolded on its card (#189); nothing kept means folded.
String descriptionUnfoldedSetting(String noteId) =>
    'descriptionUnfolded:$noteId';

/// The whole of a Note in the detailed view: its text, photos and recordings,
/// then each Section under a separator and its own Capture Context. The
/// Sections are read when the Note is, not while building.
class _FullNote extends StatefulWidget {
  final Note note;
  final DateFormat when;
  final List<Section> Function()? sections;

  /// The Note's Checklist, under its text.
  final Widget? list;

  /// Reads the Note's kind (#188); a plain Note when null.
  final NoteKind Function()? kind;

  /// How the boxes of the Note's text and of its Sections' are ticked.
  final BoxTicking Function(String entity, List<ShownBox> boxes)? boxes;

  /// Changes a table of the Note's text; never one of a Section's.
  final TableEditing? onTable;

  const _FullNote({
    required this.note,
    required this.when,
    this.sections,
    this.list,
    this.kind,
    this.boxes,
    this.onTable,
  });

  @override
  State<_FullNote> createState() => _FullNoteState();
}

class _FullNoteState extends State<_FullNote> {
  List<Section> _sections = const [];
  NoteKind _kind = NoteKind.plain;

  @override
  void initState() {
    super.initState();
    _read();
  }

  @override
  void didUpdateWidget(_FullNote old) {
    super.didUpdateWidget(old);
    // Read again only when the Note was; its kind may change without its
    // text, as a Tally made of it.
    if (old.note != widget.note ||
        (old.sections == null) != (widget.sections == null)) {
      _read();
    } else {
      _readKind();
    }
  }

  void _read() {
    _sections = widget.sections?.call() ?? const [];
    _readKind();
  }

  void _readKind() => _kind = widget.kind?.call() ?? NoteKind.plain;

  @override
  Widget build(BuildContext context) {
    final note = widget.note;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _Whole(
          id: note.id,
          text: note.shownText,
          source: note.text,
          boxes: note.boxes,
          ticking: widget.boxes?.call(note.id, note.boxes),
          format: note.textFormat,
          photos: note.photos,
          photoTexts: note.photoTexts,
          removedPhotos: note.removedPhotos,
          recordings: note.recordings,
          transcripts: note.transcripts,
          removedRecordings: note.removedRecordings,
          list: widget.list,
          onTable: widget.onTable,
          named: _kind != NoteKind.plain,
        ),
        const SizedBox(height: 8),
        CaptureLine(
          key: ValueKey('captured-line:${note.id}'),
          capture: note.context,
          when: widget.when,
        ),
        for (final (i, section) in _sections.indexed)
          Column(
            key: ValueKey('section:${section.id}'),
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Divider(height: 24),
              _Whole(
                id: section.id,
                text: section.shownText,
                source: section.text,
                boxes: section.boxes,
                ticking: widget.boxes?.call(section.id, section.boxes),
                format: section.textFormat,
                photos: section.photos,
                photoTexts: section.photoTexts,
                removedPhotos: section.removedPhotos,
                recordings: section.recordings,
                transcripts: section.transcripts,
                removedRecordings: section.removedRecordings,
              ),
              const SizedBox(height: 8),
              CaptureLine.ofSection(
                section,
                key: ValueKey('captured-line:${section.id}'),
                when: widget.when,
                after: i == 0
                    ? note.captured
                    : _sections[i - 1].context.captured,
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

  /// The text as it shows, its [boxes] in it, and as it is stored.
  final String text;
  final String source;
  final List<ShownBox> boxes;

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

  /// How the boxes of the text are ticked; plain characters when null.
  final BoxTicking? ticking;

  /// Changes a table of the text; it is only read when null.
  final TableEditing? onTable;

  /// Whether the text is a name and a description, of a Note of a kind
  /// (#188).
  final bool named;

  const _Whole({
    required this.id,
    required this.text,
    required this.source,
    required this.boxes,
    required this.format,
    required this.photos,
    required this.photoTexts,
    required this.removedPhotos,
    required this.recordings,
    required this.transcripts,
    required this.removedRecordings,
    this.list,
    this.ticking,
    this.onTable,
    this.named = false,
  });

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(context).textTheme.bodyLarge;
    final list = this.list;
    // A photo Note may have no words.
    final Widget? words = text.isEmpty
        ? null
        : named
        ? _NameAndDescription(
            noteId: id,
            text: text,
            source: source,
            boxes: boxes,
            format: format,
            ticking: ticking,
            onTable: onTable,
          )
        : MarkedText(
            text,
            format: format,
            style: style,
            boxes: boxes,
            source: source,
            ticking: ticking,
            tableEdit: onTable,
          );
    final parts = <Widget>[
      if (list == null)
        ?words
      // It keeps its own space: it is nothing on a Note that is none.
      else
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [?words, list],
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

/// The text of a Note of a kind in its detailed view (#188): its name as the
/// title, and its description under it, small and weak, folded to one
/// line until a tap unfolds it; a tap on the description folds or unfolds
/// it and nothing else, the card's own tap not included, and this Device
/// keeps it per Note (#189). [text] is the text as it shows, its [boxes] in
/// it, as [MarkedText] takes it.
class _NameAndDescription extends StatefulWidget {
  /// The Note whose fold this Device keeps.
  final String noteId;

  final String text;
  final String source;
  final List<ShownBox> boxes;
  final String format;
  final BoxTicking? ticking;
  final TableEditing? onTable;

  const _NameAndDescription({
    required this.noteId,
    required this.text,
    required this.source,
    required this.boxes,
    required this.format,
    this.ticking,
    this.onTable,
  });

  @override
  State<_NameAndDescription> createState() => _NameAndDescriptionState();
}

class _NameAndDescriptionState extends State<_NameAndDescription> {
  var _unfolded = false;

  /// Where the fold is kept: per Note in a setting of this Device (#189);
  /// it is only held while the card shows without one.
  Store? _store;

  String get _setting => descriptionUnfoldedSetting(widget.noteId);

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final store = PersonScope.of(context)?.store;
    if (store == _store) return;
    _store = store;
    _unfolded = store?.localSetting(_setting) == 'true';
  }

  void _toggle() {
    setState(() => _unfolded = !_unfolded);
    final store = _store;
    if (store == null) return;
    _unfolded
        ? store.setLocalSetting(_setting, 'true')
        : store.removeLocalSetting(_setting);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final text = widget.text;
    final nameEnd = NoteKinds.name(text).length;
    final from = text.length - NoteKinds.description(text).length;
    final description = text.substring(from);
    final (start, end) = NoteCard.firstLine(description);
    Widget part({required int start, int? end, TextStyle? style, int? lines}) =>
        MarkedText(
          text,
          format: widget.format,
          start: start,
          end: end,
          style: style,
          maxLines: lines,
          overflow: lines == null ? null : TextOverflow.ellipsis,
          boxes: widget.boxes,
          source: widget.source,
          ticking: widget.ticking,
          tableEdit: widget.onTable,
        );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        KeyedSubtree(
          key: const ValueKey('card-name'),
          child: part(
            start: 0,
            end: nameEnd,
            style: theme.textTheme.titleMedium,
          ),
        ),
        if (description.trim().isNotEmpty)
          Semantics(
            key: const ValueKey('card-description'),
            container: true,
            button: true,
            hint: _unfolded ? t.noteDescriptionFold : t.noteDescriptionUnfold,
            // Its own tap, so the card's does not run too.
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: _toggle,
              child: Padding(
                padding: const EdgeInsets.only(top: 4),
                child: part(
                  start: _unfolded ? from : from + start,
                  end: _unfolded ? null : from + end,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  lines: _unfolded ? null : 1,
                ),
              ),
            ),
          ),
      ],
    );
  }
}

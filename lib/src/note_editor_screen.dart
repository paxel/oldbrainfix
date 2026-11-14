import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'area_look.dart';
import 'areas_screen.dart';
import 'attachment_views.dart';
import 'box_ids.dart';
import 'calendar_platform.dart';
import 'checklist_view.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'failure_dialog.dart';
import 'file_platform.dart';
import 'frogs_screen.dart';
import 'hidden_from_ai_view.dart';
import 'home_shortcuts.dart';
import 'l10n.dart';
import 'line_editing.dart';
import 'location.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_history_screen.dart';
import 'occurrence_answers.dart';
import 'ocr_platform.dart';
import 'pack_screen.dart';
import 'page_bar.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'profile.dart' show profileDisplayName;
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'sent_item_screen.dart';
import 'session_editor.dart';
import 'session_note.dart';
import 'setting_badges.dart';
import 'share_note.dart';
import 'shopping_list_view.dart';
import 'speech_platform.dart';
import 'system_bars.dart';
import 'tally_entry_screen.dart';
import 'tally_field_screen.dart';
import 'table_screen.dart';
import 'tally_view.dart';
import 'text_marks_view.dart';
import 'two_row_text_field.dart';
import 'voice_platform.dart';

/// Opens the editor of [note]; returns once the user left it. With
/// [typeLines] the cursor starts in a new Line of a list after its open
/// ones (#161, #276).
Future<void> openNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note, {
  bool typeLines = false,
}) => Navigator.of(context).push(
  MaterialPageRoute<void>(
    builder: (_) => NoteEditorScreen(
      notes: notes,
      places: places,
      drafts: DraftsScope.of(context),
      noteId: note.id,
      typeLines: typeLines,
    ),
  ),
);

/// What the calendar offers for [note], with [text] as typed in place
/// of its text: the Note on its own at a time the calendar app lets the
/// user pick, each event of its Routines and Reminders, and all of those
/// events as one calendar file when there are any.
NoteCalendar noteCalendar(
  BuildContext context,
  Routines routines,
  Note note, {
  String text = '',
}) {
  final t = context.t;
  final typed = text.trim();
  final owned = routines.ofNote(note.id);
  final events = [
    for (final routine in owned) ...eventsOf(context, routines, routine),
  ];
  return (
    add: [
      (
        label: t.addNoteToCalendar,
        run: () => addToCalendar(
          context,
          CalendarEvent.ofText(
            note.id,
            typed.isEmpty ? noteText(t, note) : typed,
          ),
        ),
      ),
      for (final routine in owned) ...addChoices(context, routines, routine),
    ],
    file: events.isEmpty
        ? null
        : (
            label: t.shareCalendarFile,
            run: () => shareCalendarFile(context, routines.store, events),
          ),
  );
}

/// Asks for the text of a new Section of the Note [noteId] and adds it;
/// whether it did. The Section gets its own Capture Context when it
/// arrives, without asking; [attached] runs each time some did.
Future<bool> amendNote(
  BuildContext context,
  Notes notes,
  String noteId, {
  VoidCallback? attached,
}) async {
  final typed = await showDialog<({String text, bool markdown})>(
    context: context,
    builder: (_) => _AmendDialog(
      twoRows: notes.store.twoRowEditorOn,
      bar: notes.store.advanced,
    ),
  );
  final trimmed = typed?.text.trim();
  if (!context.mounted || typed == null || trimmed == null) return false;
  if (trimmed.isEmpty) return false;
  final section = notes.amend(
    noteId,
    trimmed,
    format: typed.markdown ? TextFormats.markdown : TextFormats.plain,
  );
  final here = currentIfGranted(LocationScope.of(context));
  final extras = currentExtras(notes.store, ContextScope.of(context));
  for (final attaching in [
    attachLocation(notes, section.id, here),
    attachExtras(notes, section.id, extras),
  ]) {
    unawaited(
      attaching.then((done) {
        if (done) attached?.call();
      }),
    );
  }
  return true;
}

/// Moves the Note [noteId] to the trash without asking and offers to
/// undo it at the top (#268); whether it went. Its Routines and Reminders
/// always go with it (#262), and the undo brings them back with it.
bool deleteNote(BuildContext context, Notes notes, String noteId) {
  final note = notes.byId(noteId);
  if (note == null || note.isTrashed || !notes.store.mayChange(noteId)) {
    return false;
  }
  final book = Routines(notes.store);
  book.trashNote(noteId);
  AnnouncementsScope.of(context)?.offerUndo(context.t.undoNoteTrashed, () {
    // The Trash, a purge or a sync may have got there first.
    if (notes.byId(noteId)?.isTrashed != true) return;
    if (!notes.store.mayChange(noteId)) return;
    book.restoreNote(noteId);
  });
  return true;
}

/// Removes the "From …" badge of the Note [noteId] (#164) without asking
/// and offers to undo it at the top (#268), which brings it back.
void removePackFrom(BuildContext context, Store store, String noteId) {
  if (store.packFrom(noteId) == null || !store.mayChange(noteId)) return;
  store.removePackFrom(noteId);
  AnnouncementsScope.of(context)?.offerUndo(context.t.undoPackFromRemoved, () {
    if (store.packFrom(noteId) != null || !store.mayChange(noteId)) return;
    store.restorePackFrom(noteId);
  });
}

/// Asks once whether the photo ([photo]) or voice recording [hash] of
/// the Note or Section [noteId] goes while its text stays, with the verb
/// on the button in the error colour, and removes it; whether it did.
Future<bool> removeMedia(
  BuildContext context,
  Notes notes,
  String noteId,
  String hash, {
  required bool photo,
}) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => _RemoveDialog(
      title: photo
          ? context.t.removePhotoQuestion
          : context.t.removeRecordingQuestion,
      detail: context.t.removeMediaDetail,
    ),
  );
  // A sync while the dialog was open may have taken it already.
  final allowed = photo
      ? notes.mayRemovePhoto(noteId, hash)
      : notes.mayRemoveRecording(noteId, hash);
  if (confirmed != true || !allowed) return false;
  if (photo) {
    notes.removePhoto(noteId, hash);
  } else {
    notes.removeRecording(noteId, hash);
  }
  return true;
}

/// Asks once whether the Attachment [hash] of the Note or Section
/// [noteId] goes while its Attachment Text stays (ADR 0020, #267), and
/// removes it; whether it did.
Future<bool> removeAttachment(
  BuildContext context,
  Notes notes,
  String noteId,
  String hash,
) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => _RemoveDialog(
      title: context.t.removeFileQuestion,
      detail: context.t.removeMediaDetail,
    ),
  );
  // A sync while the dialog was open may have taken it already.
  if (confirmed != true || !notes.mayRemoveAttachment(noteId, hash)) {
    return false;
  }
  notes.removeAttachment(noteId, hash);
  return true;
}

/// Removes the Attachment Text of the file [hash] of the Note or Section
/// [noteId] while the file stays, without asking, and offers to undo it
/// at the top (#268); whether it did (ADR 0025). The undo writes the text
/// back.
bool removeAttachmentText(
  BuildContext context,
  Notes notes,
  String noteId,
  String hash,
) {
  final text = notes.byId(noteId)?.attachmentTexts[hash];
  if (!notes.mayRemoveAttachmentText(noteId, hash) || text == null) {
    return false;
  }
  notes.removeAttachmentText(noteId, hash);
  AnnouncementsScope.of(context)
      ?.offerUndo(context.t.undoAttachmentTextRemoved, () {
        // Read again meanwhile, or the file gone: nothing to bring back.
        final now = notes.byId(noteId);
        final removed =
            now != null &&
            now.attachments.any((a) => a.hash == hash) &&
            now.removedAttachmentTexts.contains(hash);
        if (!removed || !notes.store.mayChange(noteId)) return;
        notes.setAttachmentText(noteId, hash, text);
      });
  return true;
}

/// Removes the Photo Text ([photo]) or Transcript of the photo or voice
/// recording [hash] of the Note or Section [noteId] while the file stays,
/// without asking, and offers to undo it at the top (#268); whether it
/// did (ADR 0025). The undo writes the text back.
bool removeGeneratedText(
  BuildContext context,
  Notes notes,
  String noteId,
  String hash, {
  required bool photo,
}) {
  final allowed = photo
      ? notes.mayRemovePhotoText(noteId, hash)
      : notes.mayRemoveTranscript(noteId, hash);
  final note = notes.byId(noteId);
  final text = photo ? note?.photoTexts[hash] : note?.transcripts[hash];
  if (!allowed || text == null) return false;
  if (photo) {
    notes.removePhotoText(noteId, hash);
  } else {
    notes.removeTranscript(noteId, hash);
  }
  final t = context.t;
  AnnouncementsScope.of(context)
      ?.offerUndo(photo ? t.undoPhotoTextRemoved : t.undoTranscriptRemoved, () {
        // Made again meanwhile, or the file gone: nothing to bring back.
        final now = notes.byId(noteId);
        final removed = photo
            ? now != null &&
                  now.photos.contains(hash) &&
                  now.removedPhotoTexts.contains(hash)
            : now != null &&
                  now.recordings.contains(hash) &&
                  now.removedTranscripts.contains(hash);
        if (!removed || !notes.store.mayChange(noteId)) return;
        if (photo) {
          notes.setPhotoText(noteId, hash, text);
        } else {
          notes.setTranscript(noteId, hash, text);
        }
      });
  return true;
}

/// The photos and recordings of the Note [noteId] that this Device may
/// remove while their texts stay, for its card's ⋮ menu; a merged
/// Note's Sections are not on the card and are left to the editor.
List<MediaRemoval> mediaRemovals(
  BuildContext context,
  Notes notes,
  String noteId,
  void Function(Future<Object?> step) then,
) {
  final note = notes.byId(noteId);
  if (note == null) return const [];
  MediaRemoval removal(String hash, String? text, {required bool photo}) => (
    kind: photo ? RemovedMedia.photo : RemovedMedia.recording,
    hash: hash,
    text: text ?? '',
    run: () => then(removeMedia(context, notes, noteId, hash, photo: photo)),
  );
  return [
    for (final hash in note.photos)
      if (notes.mayRemovePhoto(noteId, hash))
        removal(hash, note.photoTexts[hash], photo: true),
    for (final hash in note.recordings)
      if (notes.mayRemoveRecording(noteId, hash))
        removal(hash, note.transcripts[hash], photo: false),
    // Its files too (#267), named by the file, not its text.
    for (final file in note.attachments)
      if (notes.mayRemoveAttachment(noteId, file.hash))
        (
          kind: RemovedMedia.file,
          hash: file.hash,
          text: file.name,
          run: () => then(removeAttachment(context, notes, noteId, file.hash)),
        ),
  ];
}

/// The text of the Note [noteId] and of each of its Sections, oldest
/// first, as they show, the ids of their boxes left out, a blank line
/// between them, for "Copy text" (#183); a text that is empty is left out.
String noteCopyText(Notes notes, String noteId) {
  final note = notes.byId(noteId);
  if (note == null) return '';
  return [
    note.shownText,
    for (final section in notes.sections(noteId)) section.shownText,
  ].map((text) => text.trim()).where((text) => text.isNotEmpty).join('\n\n');
}

/// The ⋮ menu of [placed]'s card: [onEdit] opening the editor, and what
/// the Note editor offers beyond filing, scheduling and merging, as far
/// as this Device may (ADR 0013).
/// [changed] runs after each step that may have changed the Note.
NoteActions noteActions(
  BuildContext context,
  Notes notes,
  PlacedNote placed,
  VoidCallback changed, {
  VoidCallback? onEdit,
}) {
  final id = placed.note.id;
  final from = notes.store.packFrom(id);
  void then(Future<Object?> step) => unawaited(
    step.then((_) {
      if (context.mounted) changed();
    }),
  );
  return NoteActions(
    advanced: notes.store.advanced,
    onEdit: onEdit,
    onSend: () => unawaited(sendNoteFile(context, notes.store, id)),
    onShare: () => unawaited(shareNoteToApp(context, notes, id)),
    onExportPack: () => unawaited(exportPack(context, notes.store, id)),
    packFrom: from == null ? null : profileDisplayName(context.t, from.name),
    onRemovePackFrom: from == null || !placed.changes
        ? null
        : () {
            removePackFrom(context, notes.store, id);
            changed();
          },
    // Copied says nothing: the text is on the screen (UI laws).
    onCopy: () => unawaited(
      Clipboard.setData(ClipboardData(text: noteCopyText(notes, id))),
    ),
    calendar: () => noteCalendar(context, Routines(notes.store), placed.note),
    onAmend: placed.amends
        ? () => then(
            amendNote(
              context,
              notes,
              id,
              attached: () {
                if (context.mounted) changed();
              },
            ),
          )
        : null,
    removals: placed.changes
        ? () => mediaRemovals(context, notes, id, then)
        : null,
    isTally: () => Tallies(notes.store).isTally(id),
    // Only a Note in the lists becomes a Tally (ADR 0028), and no Code
    // text (#213); only in Advanced (#197).
    onMakeTally:
        notes.store.advanced &&
            placed.changes &&
            !placed.note.isTrashed &&
            !placed.note.isMerged &&
            placed.note.textFormat != TextFormats.code
        ? () => then(makeTally(context, notes, id))
        : null,
    onAddEntry: placed.amends
        ? () => then(addTallyEntry(context, notes, id))
        : null,
    // Where the phone can have one, for a Tally entries may be added to
    // (#284).
    onAddToHome:
        placed.amends &&
            !placed.note.isTrashed &&
            HomeShortcutsScope.of(context).available
        ? () => unawaited(addTallyToHome(context, notes, id))
        : null,
    checklist: () {
      final lists = Checklists(notes.store);
      final shopping = ShoppingLists(notes.store);
      // As the card shows it: for the Occurrence it is on (#121).
      final list = lists.shownNow(id, deviceWallClock);
      final shops = shopping.isShoppingList(id);
      void run(bool Function(Notes notes, String id) step) {
        if (step(notes, id)) changed();
      }

      return (
        isList: list != null || shops,
        make: lists.mayMake(id) ? () => run(makeChecklist) : null,
        makeShopping: shopping.mayMake(id) ? () => run(makeShoppingList) : null,
        toPlain:
            (list != null || shops) &&
                placed.changes &&
                placed.note.textEditable
            ? () => run(shops ? shoppingListToPlain : checklistToPlain)
            : null,
        uncheckAll:
            list != null && list.checkedCount > 0 && mayCheckShown(lists, list)
            ? () {
                lists.uncheckAll(id, on: list.on);
                changed();
              }
            : null,
        onAgenda:
            checklistOnAgendaEntry(lists, id, changed) ??
            shoppingOnAgendaEntry(shopping, id, changed),
      );
    },
    showTicks: () => showTicksEntry(notes, id, changed),
    hideFromAi: () => hiddenFromAiEntry(notes.store, id, changed),
    onDelete: placed.changes
        ? () {
            if (deleteNote(context, notes, id)) changed();
          }
        : null,
  );
}

/// How the boxes of [placed]'s card and of its Sections are ticked (ADR
/// 0035): wherever this Device holds a role, also on another person's
/// Note in a Shared Area and in a Section, as long as the Note is not in
/// the trash; each box says who ticked it last while the Note shows its
/// ticks on this Device. A tick is no edit of the text. [changed] runs
/// after a tick.
BoxTicking Function(String entity, List<ShownBox> boxes) boxTicks(
  Notes notes,
  PlacedNote placed,
  VoidCallback changed,
) {
  final store = notes.store;
  final shows = showsTicks(store, placed.note.id);
  final trashed = placed.note.isTrashed;
  return (entity, boxes) => BoxTicking(
    notes: notes,
    entity: entity,
    ticks: {for (final box in boxes) box.id: ?box.last},
    mayTick: !trashed && store.mayWriteIn(entity),
    showTicks: shows,
    changed: changed,
  );
}

/// The "Show on the Agenda" choice of the Checklist [noteId] (#143,
/// #219): how it shows on this person's Agenda, and choosing another way;
/// null unless it is a Checklist without a Routine. It is offered while
/// no Line is open too, for when Lines open again. [changed] runs after
/// a choice.
ListOnAgendaChoice? checklistOnAgendaEntry(
  Checklists lists,
  String noteId,
  VoidCallback changed,
) {
  if (!lists.mayShowOnAgenda(noteId)) return null;
  return (
    shows: lists.onAgenda(noteId),
    choose: (shows) {
      lists.setOnAgenda(noteId, shows);
      changed();
    },
  );
}

/// The "Show on the Agenda" choice of the Shopping List [noteId] (#186):
/// how it shows on this person's Agenda, and choosing another way; null
/// unless it is a Shopping List not in the trash nor merged. It is
/// offered while no Line is open too, for when Lines come back.
/// [changed] runs after a choice.
ListOnAgendaChoice? shoppingOnAgendaEntry(
  ShoppingLists lists,
  String noteId,
  VoidCallback changed,
) {
  if (!lists.mayShowOnAgenda(noteId)) return null;
  return (
    shows: lists.onAgenda(noteId),
    choose: (shows) {
      lists.setOnAgenda(noteId, shows);
      changed();
    },
  );
}

/// The Show ticks entry of the menu of the Note [noteId] (ADR 0035): on
/// or off, and switching it on this Device; null while neither the Note
/// nor a Section of it has a box. [changed] runs after a switch.
({bool on, VoidCallback toggle})? showTicksEntry(
  Notes notes,
  String noteId,
  VoidCallback changed,
) {
  final note = notes.byId(noteId);
  if (note == null) return null;
  if (note.boxes.isEmpty &&
      notes.sections(noteId).every((section) => section.boxes.isEmpty)) {
    return null;
  }
  final on = showsTicks(notes.store, noteId);
  return (
    on: on,
    toggle: () {
      setShowsTicks(notes.store, noteId, !on);
      changed();
    },
  );
}

/// What the full-screen view of a table of [placed]'s card writes (ADR
/// 0033): the Note's text after each change, as an edit of the text;
/// null where this Device may not change it, so the table is only read
/// there. A change that leaves no text keeps the old one, as the editor
/// does. [changed] runs after the edit.
TableEditing? tableEdit(Notes notes, PlacedNote placed, VoidCallback changed) {
  if (!placed.changes || !placed.note.textEditable) return null;
  final id = placed.note.id;
  return TableEditing(
    twoRows: notes.store.twoRowEditorOn,
    write: (change) {
      // A sync while it was open may have changed what may be written.
      final note = notes.byId(id);
      if (note == null ||
          !note.textEditable ||
          !notes.store.mayChange(id) ||
          note.isTrashed ||
          change.text.trim().isEmpty) {
        return null;
      }
      notes.edit(id, change.text);
      changed();
      // As kept: its new boxes have their ids (ADR 0035).
      return notes.byId(id)?.text;
    },
  );
}

/// Everything about one Note: its text, fixed in place and saved on
/// leaving, its photos at full width, each opening full screen when
/// tapped and with its Photo Text under it, a play button per voice
/// recording with its Transcript under it, the texts of the photos and
/// recordings removed while their texts stay, a row of badges under the
/// text for its settings (#160), its Importance, where it is filed, its
/// Capture Context, its Routines and Reminders, its kind, Show on the
/// Agenda and Show ticks, each opening its small editor, and its
/// Sections, each with its own Capture Context. A Section this Device may
/// edit, while the Note is not in the trash, is edited in a field of its
/// own as the Note's text is and saved with it; the others are only read,
/// their text selected and copied with a long press (#183). "Add to
/// Note" amends it; the "+" badge spawns a Routine, Reminder, Frog or
/// Session from it, as the card's menu does; the calendar button adds it,
/// or its Routines and Reminders, to the calendar; the send button sends
/// it as a Sent Item; "merge into…" merges it into another Note; the
/// trash button deletes it. A photo or recording with a text, of the
/// Note or a Section, has a button under it that removes it and keeps the
/// text (ADR 0020).
///
/// A Note of a kind (#188), a Checklist, Shopping List or Tally, shows its
/// text as its name, a heading, and its description under it, folded to
/// one line until a tap; then the kind's content, its Lines or its Tally,
/// and the rest under it. Its kind badge changes the kind.
///
/// Under each Transcript and Photo Text, "Replace Note text" and "Append
/// to Note text" take it over into the text of its Note or Section, and
/// "Remove generated text" removes it while the file stays; "Transcribe
/// again" and "Read the text again" make it anew while the file is on
/// this Device, a removed one included (ADR 0025).
///
/// Typed text survives the app being killed: it goes to the log when the
/// app goes to the background, and a [Draft] of it is kept after every
/// [draftPause] of typing until it does. A draft newer than the Note's
/// text comes back into the field when the Note is opened again.
///
/// A text in a format this version does not know shows as it is stored,
/// read-only, with a note that a newer version of the app is needed to
/// change it (ADR 0018); nothing here writes it.
class NoteEditorScreen extends StatefulWidget {
  /// How long typing rests before the draft is written.
  static const draftPause = Duration(seconds: 1);

  final Notes notes;
  final Places places;
  final Drafts drafts;
  final String noteId;

  /// Whether the cursor starts in a new empty Line of a Checklist or
  /// Shopping List after its open Lines: one just made from the
  /// browser's "+" (#161), or one opened from its group on the Agenda
  /// (#276).
  final bool typeLines;

  const NoteEditorScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.drafts,
    required this.noteId,
    this.typeLines = false,
  });

  @override
  State<NoteEditorScreen> createState() => _NoteEditorScreenState();
}

class _NoteEditorScreenState extends State<NoteEditorScreen> {
  /// Its tags are highlighted as they are typed (ADR 0032), a formatted
  /// text formatted, its marks shown in the bottom row (ADR 0033).
  late final _text = TagTextController(
    hideMarks: widget.notes.store.twoRowEditorOn,
  );
  late final _twoRows = widget.notes.store.twoRowEditorOn;

  /// The name of a Note of a kind (#188): the first line of its text,
  /// [_text] then holding its description, the rest. A line break typed
  /// or pasted into it moves what follows into the description.
  late final _name = TagTextController(hideMarks: _twoRows);

  /// Whether the fields hold the Note's text as a name and a description:
  /// for a Note of a kind, as it was when its text was put into them.
  var _named = false;

  /// Whether the description shows whole: while it is edited, or, where
  /// it is only read, once a tap unfolded it.
  var _descriptionOpen = false;
  final _descriptionFocus = FocusNode();
  late final AppLifecycleListener _lifecycle;
  Timer? _draftTimer;

  /// What is typed into the Lines of a Checklist or Shopping List (#141).
  final _lines = TypedLines();
  PlacedNote? _placed;
  List<Section> _sections = const [];
  List<Routine> _routines = const [];

  /// What the Note asks to check off now (#215): of each of its Routines
  /// and Reminders this Device may check off, the Occurrence due, else
  /// the latest Missed one.
  List<OpenOccurrence> _open = const [];

  /// Those whose schedule a newer version of the app saved (ADR 0034).
  List<NewerRoutine> _newerRoutines = const [];

  /// The Frog its badge shows (#259): the open one, or one eaten today or
  /// yesterday; none in the trash.
  Frog? _frog;

  /// The photos and recordings, by the Note or Section they belong to,
  /// that this Device may remove while their texts stay.
  Set<(String, String)> _removable = const {};

  /// The files of the Note and of its Sections, as for [_removable], this
  /// Device may remove while their texts stay, and whose texts it may
  /// remove or read again (#267).
  Set<(String, String)> _fileRemovable = const {};
  Set<(String, String)> _fileTextRemovable = const {};
  Set<(String, String)> _fileReadable = const {};

  /// The photos and recordings, as for [_removable], whose texts this
  /// Device may remove while they stay.
  Set<(String, String)> _textRemovable = const {};

  /// The photos and recordings, as for [_removable], whose texts this
  /// Device may make again: their files are here.
  Set<(String, String)> _remakeable = const {};

  /// Those being made again now.
  final _remaking = <(String, String)>{};

  /// The captured details, by the Note or Section they belong to, that
  /// this Device may remove while the capture time stays (ADR 0037).
  Set<(String, CapturedDetail)> _capturedRemovable = const {};

  /// The Note and Sections whose text this Device may change, by id.
  Set<String> _textChangeable = const {};

  /// The fields of the Sections whose text this Device may edit (#183),
  /// by the Section's id, each holding its text as stored, the ids of its
  /// boxes in it, as [_text] holds the Note's.
  final _sectionTexts = <String, TagTextController>{};

  /// How the boxes of the Note's text and of each Section's, by its id,
  /// are ticked (ADR 0035), and whether any of them has one.
  BoxTicking? _ticking;
  Map<String, BoxTicking> _sectionTicking = const {};
  bool _hasBoxes = false;

  /// Whether the Note shows its ticks on this Device (ADR 0035).
  bool _showsTicks = false;

  /// The Note's own mark Hidden from AI, and switching it, while it is
  /// marked or may be (#267); and whether something else hides it, such
  /// as its Area.
  ({bool on, VoidCallback toggle})? _hiddenFromAi;
  bool _hiddenByOther = false;
  late final _routineBook = Routines(widget.notes.store);
  late final _frogs = Frogs(widget.notes.store);

  /// Whether this Device may Amend the Note: anywhere it holds a role.
  bool _amends = true;

  /// The kind of the Note (#188), and the kinds this Device may change
  /// it to.
  NoteKind _kind = NoteKind.plain;
  Set<NoteKind> _kinds = const {};

  /// Whether a paste that looks like code waits for "Keep as code"
  /// under the text (#213), until it is taken or closed.
  bool _offersCode = false;

  /// A Session Note's setup (#191), and its pool and count as its page
  /// edits them; null for any other Note.
  SessionSetup? _setup;
  SessionDraft? _sessionDraft;

  /// Whether this Device may change the Session's setup: as it may change
  /// the Note, in the lists (ADR 0039).
  bool _setsUp = false;

  /// How a Shopping List (#186) or a Checklist without a Routine (#219)
  /// shows on this person's Agenda; null for any other Note.
  ListOnAgendaChoice? _onAgenda;

  @override
  void initState() {
    super.initState();
    _load();
    _show();
    _text.addListener(_typed);
    _name
      ..addListener(_typed)
      ..addListener(_nameTyped);
    _descriptionFocus.addListener(_descriptionFocused);
    _lifecycle = AppLifecycleListener(onStateChange: _lifecycleChanged);
    // Fire and forget: a draft cut off mid-write reads as none, and
    // anything else reaches the crash guard.
    unawaited(_restoreDraft());
  }

  /// The app's announcements, whose undos this editor follows (#268).
  Announcements? _announcements;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final announcements = AnnouncementsScope.of(context);
    if (announcements == _announcements) return;
    _announcements?.undone.removeListener(_undone);
    _announcements = announcements?..undone.addListener(_undone);
  }

  /// An undo at the top may have brought back what this editor shows: a
  /// Routine's or the Frog's badge, a Transcript, the "From …" badge.
  void _undone() {
    if (!mounted) return;
    _saveText();
    _reload();
  }

  @override
  void dispose() {
    _announcements?.undone.removeListener(_undone);
    _draftTimer?.cancel();
    _lifecycle.dispose();
    _text.dispose();
    _name.dispose();
    _descriptionFocus.dispose();
    _sessionDraft?.dispose();
    for (final field in _sectionTexts.values) {
      field.dispose();
    }
    super.dispose();
  }

  /// Going to the background may be the last chance before the app is
  /// killed: the text goes to the log.
  void _lifecycleChanged(AppLifecycleState state) {
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.hidden) {
      setState(_saveText);
    }
  }

  void _typed() {
    _draftTimer?.cancel();
    _draftTimer = Timer(NoteEditorScreen.draftPause, _writeDraft);
  }

  /// The Note's text as typed: of a Note of a kind its name and its
  /// description joined (#188).
  String get _typedText =>
      _named ? NoteKinds.join(_name.text, _text.text) : _text.text;

  /// The text [stored] as the fields hold it once put into them; of a Note
  /// of a kind its line break after the name is a "\n".
  String _asTyped(String stored) => _named
      ? NoteKinds.join(NoteKinds.name(stored), NoteKinds.description(stored))
      : stored;

  /// Puts [text] into the fields, a Note of a kind's split into its name
  /// and description.
  void _setTyped(String text) {
    if (!_named) {
      _text.text = text;
      return;
    }
    _name.text = NoteKinds.name(text);
    _text.text = NoteKinds.description(text);
  }

  /// A name holds one line: at a line break, typed with Return or pasted,
  /// what follows goes to the start of the description, a line of its own
  /// before what it held, and the cursor follows it there.
  void _nameTyped() {
    final typed = _name.text;
    final name = NoteKinds.name(typed);
    if (name.length == typed.length) return;
    final rest = NoteKinds.description(typed);
    final description = _text.text;
    _name.value = TextEditingValue(
      text: name,
      selection: TextSelection.collapsed(offset: name.length),
    );
    _text.value = TextEditingValue(
      text: rest.isEmpty
          ? description
          : description.isEmpty
          ? rest
          : '$rest\n$description',
      selection: TextSelection.collapsed(offset: rest.length),
    );
    _openDescription();
  }

  /// Unfolds the description; where it is edited, the cursor goes into
  /// it.
  void _openDescription({bool atEnd = false}) {
    setState(() => _descriptionOpen = true);
    final placed = _placed;
    if (placed == null || !placed.changes || !placed.note.textEditable) return;
    if (atEnd) {
      _text.selection = TextSelection.collapsed(offset: _text.text.length);
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _descriptionFocus.requestFocus();
    });
  }

  /// An edited description shows whole while it is edited, and folds
  /// when the cursor leaves it.
  void _descriptionFocused() {
    final open = _descriptionFocus.hasFocus;
    if (open == _descriptionOpen || !mounted) return;
    setState(() => _descriptionOpen = open);
  }

  /// Folds the description again.
  void _foldDescription() {
    _descriptionFocus.unfocus();
    setState(() => _descriptionOpen = false);
  }

  /// Keeps the text as typed in a draft while it differs from the Note's.
  void _writeDraft() {
    final note = _placed?.note;
    if (note == null || !note.textEditable) return;
    final text = _typedText;
    if (text.trim().isEmpty || text.trim() == _asTyped(note.text)) {
      unawaited(widget.drafts.delete(note.id));
    } else {
      unawaited(
        widget.drafts.write(note.id, (
          text: text,
          saved: widget.notes.store.clock().toUtc(),
        )),
      );
    }
  }

  /// Puts a draft newer than the Note's text back into the field, unless
  /// the user already started typing; an older draft is dropped.
  Future<void> _restoreDraft() async {
    final draft = await widget.drafts.read(widget.noteId);
    if (!mounted || draft == null) return;
    final note = _placed?.note;
    final setAt = widget.notes.textSetAt(widget.noteId);
    if (note == null || setAt == null || !note.textEditable) return;
    if (draft.saved.isAfter(setAt) && draft.text.trim() != note.text) {
      if (_typedText == _asTyped(note.text)) _setTyped(draft.text);
    } else {
      unawaited(widget.drafts.delete(widget.noteId));
    }
  }

  void _load() {
    final note = widget.notes.byId(widget.noteId);
    _placed = note == null ? null : placeNote(widget.places, note);
    _amends = widget.notes.store.mayWriteIn(widget.noteId);
    final lists = Checklists(widget.notes.store);
    final shopping = ShoppingLists(widget.notes.store);
    _onAgenda =
        checklistOnAgendaEntry(lists, widget.noteId, _reload) ??
        shoppingOnAgendaEntry(shopping, widget.noteId, _reload);
    _kind = NoteKinds(widget.notes.store).of(widget.noteId);
    final changes = _placed?.changes ?? false;
    // Back to a plain Note where its text may change, as the card's ⋮.
    final plainable = changes && (note?.textEditable ?? false);
    // As the card's "Make it a Tally": only a Note in the lists.
    final listed = changes && note != null && !note.isTrashed && !note.isMerged;
    final advanced = widget.notes.store.advanced;
    final setup = _setup = _kind == NoteKind.session
        ? Sessions(widget.notes.store).setupOf(widget.noteId)
        : null;
    _setsUp = setup != null && listed;
    // Its controller is in no widget: the weekdays are a badge here. The
    // pool's tag filter stays while the page is open (#220).
    final picked = _sessionDraft?.picked ?? const <String>{};
    _sessionDraft?.dispose();
    _sessionDraft = setup == null
        ? null
        : (SessionDraft.of(widget.notes.store, widget.noteId, setup)
            ..picked.addAll(picked));
    _kinds = switch (_kind) {
      // A Code text becomes no other kind (#213).
      NoteKind.plain when note?.textFormat == TextFormats.code => const {},
      NoteKind.plain => {
        if (lists.mayMake(widget.noteId)) NoteKind.checklist,
        if (shopping.mayMake(widget.noteId)) NoteKind.shoppingList,
        // Made only in Advanced (#197).
        if (listed && advanced) NoteKind.tally,
        if (listed && advanced) NoteKind.session,
      },
      NoteKind.checklist => {
        if (plainable) NoteKind.plain,
        if (shopping.maySwitch(widget.noteId)) NoteKind.shoppingList,
      },
      NoteKind.shoppingList => {
        if (plainable) NoteKind.plain,
        if (shopping.maySwitch(widget.noteId)) NoteKind.checklist,
      },
      // No way leads out of a Tally.
      NoteKind.tally => const {},
      NoteKind.session => {if (listed) NoteKind.plain},
    };
    _sections = widget.notes.sections(widget.noteId);
    _routines = _routineBook.ofNote(widget.noteId);
    _open = note == null || note.isTrashed
        ? const []
        : [
            for (final open in Alarms(
              widget.notes.store,
            ).openOf(widget.noteId, deviceWallClock))
              if (widget.notes.store.mayCheckOff(open.routine.id)) open,
          ];
    _newerRoutines = _routineBook.newerOfNote(widget.noteId);
    _frog = note == null || note.isTrashed
        ? null
        : badgeFrog(_frogs, widget.noteId, deviceWallClock);
    final notes = widget.notes;
    final store = notes.store;
    final files = [
      if (note != null) (note.id, note.photos, note.recordings),
      for (final section in _sections)
        (section.id, section.photos, section.recordings),
    ];
    _removable = {
      for (final (id, photos, recordings) in files) ...[
        for (final hash in photos)
          if (notes.mayRemovePhoto(id, hash)) (id, hash),
        for (final hash in recordings)
          if (notes.mayRemoveRecording(id, hash)) (id, hash),
      ],
    };
    _textRemovable = {
      for (final (id, photos, recordings) in files) ...[
        for (final hash in photos)
          if (notes.mayRemovePhotoText(id, hash)) (id, hash),
        for (final hash in recordings)
          if (notes.mayRemoveTranscript(id, hash)) (id, hash),
      ],
    };
    _remakeable = {
      for (final (id, photos, recordings) in files)
        if (store.mayChange(id)) ...[
          for (final hash in photos)
            if (notes.photoBytes(hash) != null) (id, hash),
          for (final hash in recordings)
            if (notes.recordingBytes(hash) != null) (id, hash),
        ],
    };
    final attached = [
      if (note != null) (note.id, note.attachments),
      for (final section in _sections) (section.id, section.attachments),
    ];
    _fileRemovable = {
      for (final (id, files) in attached)
        for (final file in files)
          if (notes.mayRemoveAttachment(id, file.hash)) (id, file.hash),
    };
    _fileTextRemovable = {
      for (final (id, files) in attached)
        for (final file in files)
          if (notes.mayRemoveAttachmentText(id, file.hash)) (id, file.hash),
    };
    _fileReadable = {
      for (final (id, files) in attached)
        if (store.mayChange(id))
          for (final file in files)
            // Not one left to a computer (Q645).
            if (file.kind.hasText && !file.computer) (id, file.hash),
    };
    _capturedRemovable = {
      for (final id in [?note?.id, for (final section in _sections) section.id])
        for (final detail in CapturedDetail.values)
          if (notes.mayRemoveCaptured(id, detail)) (id, detail),
    };
    _textChangeable = {
      if (_placed case final placed? when placed.changes)
        if (placed.note.textEditable) placed.note.id,
      for (final section in _sections)
        if (section.textEditable && store.mayChange(section.id)) section.id,
    };
    // Ticks wherever this Device holds a role, unless in the trash.
    final ticks = note != null && !note.isTrashed;
    final shows = _showsTicks = showsTicks(store, widget.noteId);
    _hiddenFromAi = hiddenFromAiEntry(store, widget.noteId, _reload);
    _hiddenByOther = store
        .hiddenFromAiBy(widget.noteId)
        .any((mark) => mark != widget.noteId);
    BoxTicking ticking(String id, List<ShownBox> boxes) => BoxTicking(
      notes: notes,
      entity: id,
      ticks: {for (final box in boxes) box.id: ?box.last},
      mayTick: ticks && store.mayWriteIn(id),
      showTicks: shows,
      changed: _reload,
    );
    _ticking = note == null ? null : ticking(note.id, note.boxes);
    _text.boxes = _ticking;
    _name.boxes = _ticking;
    _sectionTicking = {
      for (final section in _sections)
        section.id: ticking(section.id, section.boxes),
    };
    _hasBoxes =
        (note?.boxes.isNotEmpty ?? false) ||
        _sections.any((section) => section.boxes.isNotEmpty);
    _loadSectionTexts(editable: note != null && !note.isTrashed);
  }

  /// Gives each Section this Device may edit its field, and only those
  /// ([Notes.mayEditSection]), none while the Note is in the trash
  /// ([editable] false). What is typed in a field stays when the Sections
  /// are read again; only saving puts their stored text back into it.
  void _loadSectionTexts({required bool editable}) {
    final notes = widget.notes;
    final ids = {
      if (editable)
        for (final section in _sections)
          if (notes.mayEditSection(section.id)) section.id,
    };
    for (final id in [..._sectionTexts.keys]) {
      if (ids.contains(id)) continue;
      final gone = _sectionTexts.remove(id);
      // Its field lets go of it in the next frame.
      if (gone != null) {
        WidgetsBinding.instance.addPostFrameCallback((_) => gone.dispose());
      }
    }
    for (final section in _sections) {
      if (!ids.contains(section.id)) continue;
      final field = _sectionTexts.putIfAbsent(
        section.id,
        () => TagTextController(hideMarks: _twoRows)
          ..markdown = section.textFormat == TextFormats.markdown
          ..text = section.text,
      );
      field.boxes = _sectionTicking[section.id];
    }
  }

  void _reload() => setState(_load);

  /// The Note's History (#193); a text brought back there shows here, in
  /// the Note's field and its Sections'.
  Future<void> _openHistory() async {
    _saveText();
    final used = await openNoteHistory(
      context,
      widget.notes,
      widget.places,
      widget.noteId,
    );
    if (!mounted || !used) return;
    setState(() {
      _load();
      _show();
      for (final section in _sections) {
        if (_sectionTexts[section.id] case final field?) {
          _takeStored(field, section.text);
        }
      }
    });
  }

  /// Puts the Note's text as it is stored into the field, in its format;
  /// a Note of a kind's into its name and description fields (#188), the
  /// description folded.
  void _show() {
    final note = _placed?.note;
    _named = _kind != NoteKind.plain;
    _descriptionOpen = false;
    final markdown = note?.textFormat == TextFormats.markdown;
    _text.markdown = markdown;
    _name.markdown = markdown;
    _text.code = !_named && note?.textFormat == TextFormats.code;
    _setTyped(note?.text ?? '');
  }

  /// Writes the text when it changed; a blank text keeps the old one.
  /// The draft goes either way: the log now holds what counts. What was
  /// typed into the Lines is stored first, by anyone who may add them;
  /// the Sections' texts after it ([_saveSections]).
  void _saveText() {
    _draftTimer?.cancel();
    _lines.keep();
    _saveNoteText();
    _saveSections();
  }

  void _saveNoteText() {
    final note = _placed?.note;
    if (note == null || !(_placed?.changes ?? false)) return;
    unawaited(widget.drafts.delete(note.id));
    // A text in a format not known here is never overwritten.
    if (!note.textEditable) return;
    // An emptied name gets the stored one back: nothing of the
    // description becomes the name (#188).
    if (_named && _name.text.trim().isEmpty) {
      _takeStored(_name, NoteKinds.name(note.text));
    }
    final typed = _typedText;
    // The first format makes it formatted for good (ADR 0033).
    final formats = _text.markdown && note.textFormat != TextFormats.markdown;
    // A name and description left as they were write nothing (#188).
    if (typed == _asTyped(note.text) && !formats) return;
    final text = typed.trim();
    if (text.isEmpty || (text == note.text && !formats)) return;
    widget.notes.edit(
      note.id,
      text,
      format: _text.markdown ? TextFormats.markdown : null,
    );
    _load();
    final stored = _placed?.note.text;
    if (!_named || stored == null) {
      _takeStored(_text, stored);
      return;
    }
    _name.markdown = _text.markdown;
    _takeStored(_name, NoteKinds.name(stored));
    _takeStored(_text, NoteKinds.description(stored));
  }

  /// Writes the text of each Section whose field changed, as the Note's
  /// (#183): fixed in place, a blank text keeping the old one; each field
  /// then holds the text as it was kept.
  void _saveSections() {
    final notes = widget.notes;
    var saved = false;
    for (final MapEntry(key: id, value: field) in _sectionTexts.entries) {
      final section = notes.section(id);
      if (section == null || !notes.mayEditSection(id)) continue;
      final text = field.text.trim();
      final formats =
          field.markdown && section.textFormat != TextFormats.markdown;
      if (text.isEmpty || (text == section.text && !formats)) continue;
      notes.editSection(
        id,
        text,
        format: field.markdown ? TextFormats.markdown : null,
      );
      saved = true;
    }
    if (!saved) return;
    _load();
    for (final section in _sections) {
      if (_sectionTexts[section.id] case final field?) {
        _takeStored(field, section.text);
      }
    }
  }

  /// Takes the text as it was kept ([stored]) back into [field], its new
  /// boxes with their ids, so the next save keeps them and their ticks
  /// (ADR 0035); the cursor stays where it was in the text around it, the
  /// spaces around the text as typed.
  void _takeStored(TagTextController field, String? stored) {
    if (stored == null) return;
    final value = field.value;
    final typed = value.text;
    final body = typed.trim();
    if (body == stored) return;
    final lead = typed.length - typed.trimLeft().length;
    final next =
        typed.substring(0, lead) + stored + typed.substring(lead + body.length);
    if (withoutIds(next) != withoutIds(typed)) {
      field.value = TextEditingValue(
        text: stored,
        selection: TextSelection.collapsed(offset: stored.length),
      );
      return;
    }
    int moved(int offset) =>
        offset < 0 ? offset : idFreeOffset(next, idFreeCount(typed, offset));
    final selection = value.selection;
    final composing = value.composing;
    field.value = TextEditingValue(
      text: next,
      selection: selection.copyWith(
        baseOffset: moved(selection.baseOffset),
        extentOffset: moved(selection.extentOffset),
      ),
      composing: composing.isValid
          ? TextRange(start: moved(composing.start), end: moved(composing.end))
          : composing,
    );
  }

  /// A tap on the box at [start] of the field, the Note's (its name's
  /// with [name]) or that of the Section [section], the finger at [at]:
  /// the text is kept first, so the box is the one stored, then its menu
  /// opens there (ADR 0035).
  Future<void> _tapBox(
    int start,
    Offset at, {
    String? section,
    bool name = false,
  }) async {
    final field = name
        ? _name
        : section == null
        ? _text
        : _sectionTexts[section];
    if (field == null) return;
    final id = TextBoxes.find(field.text)
        .where((box) => box.start == start)
        .firstOrNull
        ?.id;
    if (id == null) return;
    _saveText();
    setState(() {});
    final box = TextBoxes.find(field.text)
        .where((box) => box.id == id)
        .firstOrNull;
    final ticking = section == null ? _ticking : _sectionTicking[section];
    // Only a box of the text as stored can be ticked.
    final stored = section == null
        ? _placed?.note.boxes
        : _sections.where((s) => s.id == section).firstOrNull?.boxes;
    final kept = stored?.any((b) => b.id == id) ?? false;
    if (box == null || ticking == null || !kept) return;
    await openBoxMenu(
      context,
      at,
      id: id,
      state: ticking.stateOf(id, box.state),
      ticking: ticking,
    );
  }

  /// Shows after each box who ticked it last and when, or no more, for
  /// this Note on this Device.
  void _toggleTicks() {
    final store = widget.notes.store;
    setShowsTicks(store, widget.noteId, !showsTicks(store, widget.noteId));
    _reload();
  }

  /// The "Show on the Agenda" badge's editor of a Shopping List (#186)
  /// or a Checklist (#219): the three ways as radio buttons, the one it
  /// has marked; a tap on another writes it and closes.
  Future<void> _editOnAgenda(ListOnAgendaChoice agenda) async {
    final t = context.t;
    final picked = await showDialog<ListOnAgenda>(
      context: context,
      builder: (context) => BadgeDialog(
        setting: t.checklistShowOnAgenda,
        icon: Icons.today_outlined,
        content: RadioColumn<ListOnAgenda>(
          value: agenda.shows,
          options: [
            for (final (value, label) in listOnAgendaOptions(t))
              RadioOption(
                key: ValueKey('shopping-agenda-${value.name}'),
                value: value,
                title: Text(label),
              ),
          ],
          onChanged: (picked) => Navigator.of(context).pop(picked),
        ),
      ),
    );
    if (!mounted || picked == null) return;
    agenda.choose(picked);
  }

  void _setImportance(Importance picked) {
    _saveText();
    widget.notes.setImportance(widget.noteId, picked);
    _reload();
  }

  /// The Importance badge's editor: the three as choice chips, the one
  /// it has marked; a tap on another writes it and closes.
  Future<void> _editImportance() async {
    final current = _placed?.note.importance;
    if (current == null) return;
    final t = context.t;
    final picked = await showDialog<Importance>(
      context: context,
      builder: (context) => BadgeDialog(
        setting: t.importance,
        icon: _importanceIcon,
        content: ChoiceChips<Importance>(
          value: current,
          options: [
            for (final (value, label) in [
              (Importance.unimportant, t.importanceUnimportant),
              (Importance.normal, t.importanceNormal),
              (Importance.important, t.importanceImportant),
            ])
              RadioOption(
                key: ValueKey('importance-${value.name}'),
                value: value,
                title: Text(label),
              ),
          ],
          onChanged: (picked) => Navigator.of(context).pop(picked),
        ),
      ),
    );
    if (!mounted || picked == null) return;
    _setImportance(picked);
  }

  /// The Captured badge's editor: the time, place, calendar event,
  /// Bluetooth device and app of the Capture Context, each on a line of
  /// its own, every detail but the time with a Remove where this Device
  /// may remove it (ADR 0037). It shows what is left after each. With
  /// [section] it is that Section's, opened from its line (#170).
  Future<void> _showCaptured(DateFormat when, {String? section}) =>
      showDialog<void>(
        context: context,
        builder: (context) => StatefulBuilder(
          builder: (context, setDialog) {
            final (id, capture) = switch (section) {
              null => (_placed?.note.id, _placed?.note.context),
              final section => (
                section,
                _sections.where((s) => s.id == section).firstOrNull?.context,
              ),
            };
            return BadgeDialog(
              setting: context.t.capturedHeading,
              icon: Icons.schedule,
              content: id == null || capture == null
                  ? const SizedBox.shrink()
                  : CaptureContextLines(
                      key: ValueKey(
                        section == null
                            ? 'captured-details'
                            : 'captured-details:$section',
                      ),
                      capture: capture,
                      when: when,
                      removable: _capturedOf(id),
                      onRemove: (detail) async {
                        await _removeCaptured(id, detail);
                        if (context.mounted) setDialog(() {});
                      },
                    ),
            );
          },
        ),
      );

  /// The details of the Note or Section [id] this Device may remove.
  Set<CapturedDetail> _capturedOf(String id) => {
    for (final (of, detail) in _capturedRemovable)
      if (of == id) detail,
  };

  /// Removes the captured [detail] of the Note or Section [id] after
  /// asking once; the capture time stays. A refusal is announced at the
  /// top with its cause; a detail a sync took meanwhile is gone already.
  Future<void> _removeCaptured(String id, CapturedDetail detail) async {
    _saveText();
    final t = context.t;
    final notes = widget.notes;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => _RemoveDialog(
        title: id == widget.noteId
            ? t.removeCapturedQuestion(detail.name)
            : t.removeCapturedSectionQuestion(detail.name),
        detail: t.removeCapturedDetail,
      ),
    );
    if (!mounted) return;
    if (confirmed != true) return;
    final shown = id == widget.noteId
        ? notes.byId(id)?.context
        : notes
              .sections(widget.noteId)
              .where((section) => section.id == id)
              .firstOrNull
              ?.context;
    Object? refused;
    if (shown != null && shown.holds(detail)) {
      try {
        notes.removeCaptured(id, detail);
      } on Object catch (error) {
        refused = error;
      }
    }
    _reload();
    if (refused == null) return;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context);
    announcements?.announce(
      t.removeCapturedFailed,
      onTap: () => unawaited(
        showFailure(
          navigator.context,
          title: t.removeCapturedFailedTitle,
          cause: t.removeCapturedFailedCause,
          fix: t.removeCapturedFailedFix,
          details: 'remove captured ${detail.name}: $refused',
        ),
      ),
    );
  }

  /// The kind badge's editor (#188): the kinds as choice chips, the
  /// Note's marked, and those this Device may change it to; a tap on
  /// another changes it and closes.
  Future<void> _editKind() async {
    final t = context.t;
    final current = _kind;
    final picked = await showDialog<NoteKind>(
      context: context,
      builder: (context) => BadgeDialog(
        setting: t.badgeKind,
        icon: kindBadgeIcon,
        content: ChoiceChips<NoteKind>(
          value: current,
          options: [
            for (final kind in NoteKind.values)
              if (kind == current || _kinds.contains(kind))
                RadioOption(
                  key: ValueKey('kind-${kind.name}'),
                  value: kind,
                  title: Text(kindName(t, kind)),
                ),
          ],
          onChanged: (picked) => Navigator.of(context).pop(picked),
        ),
      ),
    );
    if (!mounted || picked == null || picked == current) return;
    await _changeKind(picked);
  }

  /// The text format badge's editor (#213): the formats the text may
  /// take, a formatted text never plain again (ADR 0033).
  Future<void> _editFormat() async {
    final current = _placed?.note.textFormat;
    if (current == null) return;
    final t = context.t;
    final picked = await showDialog<String>(
      context: context,
      builder: (context) => BadgeDialog(
        setting: t.badgeTextFormat,
        icon: textFormatBadgeIcon,
        content: ChoiceChips<String>(
          value: current,
          options: [
            for (final format in [
              TextFormats.plain,
              TextFormats.markdown,
              TextFormats.code,
            ])
              if (format != TextFormats.plain ||
                  current != TextFormats.markdown)
                RadioOption(
                  key: ValueKey('format-$format'),
                  value: format,
                  title: Text(textFormatName(t, format)),
                ),
          ],
          onChanged: (picked) => Navigator.of(context).pop(picked),
        ),
      ),
    );
    if (!mounted || picked == null || picked == current) return;
    _setFormat(picked);
  }

  /// Puts the Note's text, as typed, into [format] (#213) and shows it
  /// so.
  void _setFormat(String format) {
    _saveText();
    widget.notes.setFormat(widget.noteId, format);
    _offersCode = false;
    setState(_load);
    _show();
  }

  /// Makes the Note the kind [kind], its text and Lines as typed kept
  /// first, then shows it as it is now: a list made of a plain Note's text
  /// has its first line as name and the rest as Lines, a plain Note made
  /// of a list every Line as a line of its text, and a list switched to
  /// the other kind of list the same Lines (ADR 0030). A Tally starts with
  /// the page of its first field.
  Future<void> _changeKind(NoteKind kind) async {
    _saveText();
    final notes = widget.notes;
    final id = widget.noteId;
    final changed = switch ((_kind, kind)) {
      (NoteKind.session, NoteKind.plain) => _unmakeSession(),
      (NoteKind.shoppingList, NoteKind.plain) => shoppingListToPlain(notes, id),
      (_, NoteKind.plain) => checklistToPlain(notes, id),
      (NoteKind.plain, NoteKind.checklist) => makeChecklist(notes, id),
      (NoteKind.plain, NoteKind.shoppingList) => makeShoppingList(notes, id),
      (NoteKind.plain, NoteKind.tally) => await makeTally(context, notes, id),
      (NoteKind.plain, NoteKind.session) => await _makeSession(),
      (_, NoteKind.checklist || NoteKind.shoppingList) => _switchList(),
      _ => false,
    };
    if (!mounted || !changed) return;
    setState(_load);
    _show();
  }

  /// The setup's page of a Session made of the plain Note; whether it
  /// was made.
  Future<bool> _makeSession() =>
      makeSession(context, widget.notes, widget.places, widget.noteId);

  /// Makes the Session a plain Note again; its Routines then run plain,
  /// their picks and entries kept (ADR 0039). Whether it did.
  bool _unmakeSession() {
    final sessions = Sessions(widget.notes.store);
    // A sync while the dialog was open may have changed it.
    if (!sessions.isSessionNote(widget.noteId)) return false;
    sessions.unmake(widget.noteId);
    return true;
  }

  /// Writes the pool and the count as the Session's page changed them in
  /// [draft], the one it shows, read again since or not.
  void _sessionChanged(SessionDraft draft) {
    final setup = _setup;
    if (setup == null) return;
    draft.write(setup);
    _reload();
  }

  /// "New Tally" in the Session's pool.
  Future<void> _newPoolTally() async {
    _saveText();
    await addPoolTally(
      context,
      widget.notes,
      widget.noteId,
      text: _sessionDraft?.newTallyText ?? '',
    );
    if (!mounted) return;
    _reload();
  }

  /// Switches a Checklist to a Shopping List or back, its Lines carried
  /// over; whether it did.
  bool _switchList() {
    final shopping = ShoppingLists(widget.notes.store);
    // A sync while the dialog was open may have changed it.
    if (!shopping.maySwitch(widget.noteId)) return false;
    shopping.switchList(widget.noteId, deviceWallClock);
    return true;
  }

  /// The editor of an on-or-off badge: its switch, writing at once.
  Future<void> _editSwitch(
    String setting,
    IconData icon,
    bool on,
    VoidCallback toggle,
  ) => showDialog<void>(
    context: context,
    builder: (context) => BadgeDialog(
      setting: setting,
      icon: icon,
      content: BadgeSwitch(setting: setting, on: on, toggle: toggle),
    ),
  );

  /// The "+" badge's menu at the finger [at]: Routine, Reminder, Frog and
  /// Session, each spawned as the card's menu does; on a Session Note no
  /// Session, as a Routine of it runs it already (#191); no Frog while
  /// one is open (#259); in Simple only Routine and Reminder (#197).
  Future<void> _addMenu(Offset at) async {
    final t = context.t;
    final advanced = widget.notes.store.advanced;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final picked = await showMenu<Spawn>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        for (final (spawn, icon, label) in [
          (Spawn.routine, const Icon(Icons.repeat), t.routine),
          (
            Spawn.reminder,
            const Icon(Icons.notifications_outlined),
            t.reminder,
          ),
          // Frogs and Sessions are made only in Advanced (#197); a Note
          // has one open Frog at a time (#259).
          if (advanced && (_frog?.isDone ?? true))
            (Spawn.frog, const FrogIcon(), t.frog),
          // A Code text becomes no Session (#213).
          if (advanced &&
              _kind != NoteKind.session &&
              _placed?.note.textFormat != TextFormats.code)
            (Spawn.session, const Icon(Icons.event_repeat), t.session),
        ])
          PopupMenuItem(
            key: ValueKey('spawn-${spawn.name}'),
            value: spawn,
            child: Row(
              children: [
                icon,
                const SizedBox(width: 12),
                Flexible(child: Text(label)),
              ],
            ),
          ),
      ],
    );
    if (!mounted || picked == null) return;
    await _spawn(picked);
  }

  /// The icon of the Importance, in its badge and its editor.
  static const _importanceIcon = Icons.label_important_outline;

  /// The icon of Area and Topic, in its badge, the Area's colour.
  static const _placeIcon = Icons.folder_outlined;

  /// Under the text after a paste that looks like code (#213): says so
  /// and offers to keep the whole text as code; Close lets it be.
  Widget _codeOffer(AppLocalizations t) => Padding(
    key: const ValueKey('code-offer'),
    padding: const EdgeInsets.only(top: 8),
    child: Row(
      children: [
        Icon(textFormatIcon(TextFormats.code), size: 20),
        const SizedBox(width: 8),
        Expanded(child: Text(t.codePasted)),
        TextButton(
          key: const ValueKey('keep-as-code'),
          onPressed: () => _setFormat(TextFormats.code),
          child: Text(t.keepAsCode),
        ),
        IconButton(
          key: const ValueKey('code-offer-close'),
          tooltip: t.close,
          onPressed: () => setState(() => _offersCode = false),
          icon: const Icon(Icons.close),
        ),
      ],
    ),
  );

  /// The "From …" badge's menu at the finger [at]: Remove “From [name]”,
  /// which removes it (#164) without asking; the announcement at the top
  /// undoes it (#268).
  Future<void> _removePackFrom(String name, Offset at) async {
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final remove = await showMenu<bool>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        PopupMenuItem(
          key: const ValueKey('remove-pack-from'),
          value: true,
          child: Row(
            children: [
              const Icon(Icons.delete_outline),
              const SizedBox(width: 12),
              Flexible(child: Text(context.t.removePackFrom(name))),
            ],
          ),
        ),
      ],
    );
    if (remove != true || !mounted) return;
    removePackFrom(context, widget.notes.store, widget.noteId);
    setState(() {});
  }

  /// The row of badges under the text (#160): each setting of the Note
  /// as its icon and its value, an icon for a fixed set of values, short
  /// text otherwise (#192), a tap opening its small editor; a setting
  /// that does not apply has none. "+" ends it.
  Widget _badges(
    PlacedNote placed,
    DateFormat when,
    String place,
    Color? colour,
  ) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final (:note, :area, :changes, :schedules, topic: _, amends: _, author: _) =
        placed;
    final ticks = _showsTicks;
    return Wrap(
      key: const ValueKey('note-badges'),
      spacing: 8,
      runSpacing: 8,
      children: [
        SettingBadge(
          key: const ValueKey('badge-importance'),
          setting: t.importance,
          icon: _importanceIcon,
          value: switch (note.importance) {
            Importance.unimportant => t.importanceUnimportant,
            Importance.normal => t.importanceNormal,
            Importance.important => t.importanceImportant,
          },
          valueIcon: importanceIcon(note.importance),
          onTap: changes ? (_) => _editImportance() : null,
        ),
        // Area and Topic in one: File into… picks both at once.
        SettingBadge(
          key: const ValueKey('badge-place'),
          setting: t.badgePlace,
          icon: _placeIcon,
          iconColour: colour,
          value: place,
          onTap: changes ? (_) => _file() : null,
        ),
        SettingBadge(
          key: const ValueKey('badge-captured'),
          setting: t.capturedHeading,
          icon: Icons.schedule,
          value: when.format(note.context.captured.toLocal()),
          onTap: (_) => _showCaptured(when),
        ),
        // Who the Pack it came from is from, until removed (#164).
        if (widget.notes.store.packFrom(note.id) case final from?)
          SettingBadge(
            key: const ValueKey('badge-pack-from'),
            setting: t.packTitle,
            icon: Icons.inventory_2_outlined,
            value: t.packFrom(profileDisplayName(t, from.name)),
            onTap: changes
                ? (at) => _removePackFrom(profileDisplayName(t, from.name), at)
                : null,
          ),
        for (final routine in _routines)
          SettingBadge(
            key: ValueKey('badge-routine-${routine.id}'),
            setting: routine.isReminder ? t.reminder : t.routine,
            icon: routine.isReminder
                ? Icons.notifications_outlined
                : Icons.repeat,
            value: scheduleText(t, locale, routine.schedule),
            onTap: (_) => _openRoutine(routine),
          ),
        for (final routine in _newerRoutines)
          SettingBadge(
            key: ValueKey('newer-${routine.id}'),
            setting: t.routine,
            icon: Icons.repeat,
            value: t.newerVersionNeeded,
          ),
        // Its open Frog, or one eaten today or yesterday (#259).
        if (_frog case final frog?) _frogBadge(frog),
        // Every Note has a kind (#188); with nothing to change it to,
        // the badge only shows it.
        SettingBadge(
          key: const ValueKey('badge-kind'),
          setting: t.badgeKind,
          icon: kindBadgeIcon,
          value: kindName(t, _kind),
          valueIcon: kindIcon(_kind),
          onTap: _kinds.isEmpty ? null : (_) => _editKind(),
        ),
        // A Plain Note's text is plain, formatted or code (#213).
        if (_kind == NoteKind.plain && note.textEditable)
          SettingBadge(
            key: const ValueKey('badge-format'),
            setting: t.badgeTextFormat,
            icon: textFormatBadgeIcon,
            value: textFormatName(t, note.textFormat),
            valueIcon: textFormatIcon(note.textFormat),
            onTap: changes ? (_) => _editFormat() : null,
          ),
        // A Session's rule and weekdays' counts (#191).
        if (_setup case final setup?)
          ...sessionBadges(
            context,
            widget.notes.store,
            setup,
            changes: _setsUp,
            onChanged: _reload,
          ),
        if (_onAgenda case final agenda?)
          SettingBadge(
            key: const ValueKey('badge-agenda'),
            setting: t.checklistShowOnAgenda,
            icon: Icons.today_outlined,
            value: switch (agenda.shows) {
              ListOnAgenda.off => t.settingOff,
              ListOnAgenda.count => t.shoppingOnAgendaCount,
              ListOnAgenda.lines => t.shoppingOnAgendaLines,
            },
            valueIcon: listOnAgendaIcon(agenda.shows),
            onTap: (_) => _editOnAgenda(agenda),
          ),
        if (_hasBoxes)
          SettingBadge(
            key: const ValueKey('badge-ticks'),
            setting: t.boxShowTicks,
            icon: Icons.history_toggle_off,
            value: ticks ? t.settingOn : t.settingOff,
            valueIcon: switchIcon(ticks),
            onTap: (_) => _editSwitch(
              t.boxShowTicks,
              Icons.history_toggle_off,
              ticks,
              _toggleTicks,
            ),
          ),
        // Off in a Shared Area, where it is not offered (#267).
        if (_hiddenFromAi case (:final on, :final toggle))
          SettingBadge(
            key: const ValueKey('badge-hidden-from-ai'),
            setting: t.hiddenFromAi,
            icon: hiddenFromAiIcon,
            value: on
                ? t.settingOn
                : _hiddenByOther
                ? t.hiddenFromAiByArea
                : t.settingOff,
            valueIcon: switchIcon(on),
            onTap: (_) =>
                _editSwitch(t.hiddenFromAi, hiddenFromAiIcon, on, toggle),
          ),
        // In a Shared Area only a Manager sets them up (ADR 0013).
        if (schedules)
          SettingBadge(
            key: const ValueKey('badge-add'),
            setting: t.routineOrReminder,
            icon: Icons.add,
            value: null,
            onTap: _addMenu,
          ),
      ],
    );
  }

  /// The Frog's badge (#259): the frog, then Open, or Eaten today or
  /// yesterday; a tap offers what this Device may do to it.
  Widget _frogBadge(Frog frog) {
    final t = context.t;
    final store = widget.notes.store;
    final eats = !frog.isDone && store.mayCheckOff(frog.id);
    final removes = store.mayManage(store.areaOf(frog.id));
    final done = frog.done;
    return SettingBadge(
      key: const ValueKey('badge-frog'),
      setting: t.frog,
      iconWidget: const FrogIcon(size: 18),
      value: done == null
          ? t.frogOpen
          : Day.today(done, deviceWallClock) ==
                Day.today(store.clock(), deviceWallClock)
          ? t.frogEatenToday
          : t.frogEatenYesterday,
      onTap: eats || removes
          ? (at) => _frogMenu(frog, at, eats: eats, removes: removes)
          : null,
    );
  }

  /// The Frog badge's menu at the finger [at]: Eat now while it is open
  /// and this Device may check it off, Remove the Frog for who may
  /// manage it (ADR 0013), which the announcement at the top undoes, as
  /// in the Frog view (#268).
  Future<void> _frogMenu(
    Frog frog,
    Offset at, {
    required bool eats,
    required bool removes,
  }) async {
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final picked = await showMenu<bool>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        if (eats)
          PopupMenuItem(
            key: const ValueKey('frog-eat-now'),
            value: true,
            child: Row(
              children: [
                const Icon(Icons.check_circle_outline),
                const SizedBox(width: 12),
                Flexible(child: Text(t.frogEatNow)),
              ],
            ),
          ),
        if (removes)
          PopupMenuItem(
            key: const ValueKey('frog-remove'),
            value: false,
            child: Row(
              children: [
                const Icon(Icons.delete_outline),
                const SizedBox(width: 12),
                Flexible(child: Text(t.frogRemove)),
              ],
            ),
          ),
      ],
    );
    if (!mounted || picked == null) return;
    _saveText();
    if (picked) {
      _frogs.markDone(frog.id);
    } else if (!trashFrog(context, _frogs, frog)) {
      return;
    }
    _reload();
  }

  Future<void> _file() async {
    final note = _placed?.note;
    if (note == null) return;
    _saveText();
    if (await fileNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _amend() async {
    _saveText();
    final added = await amendNote(
      context,
      widget.notes,
      widget.noteId,
      attached: () {
        if (mounted) _reload();
      },
    );
    if (!mounted || !added) return;
    _reload();
  }

  /// Spawns [spawn] from the Note, through the same flow as the card's
  /// menu.
  Future<void> _spawn(Spawn spawn) async {
    final note = _placed?.note;
    if (note == null) return;
    _saveText();
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      spawn: spawn,
    );
    if (!mounted) return;
    final named = _named;
    _reload();
    // Spawning a Session makes a plain Note one (#191).
    if (named != (_kind != NoteKind.plain)) _show();
  }

  Future<void> _openRoutine(Routine routine) async {
    _saveText();
    await openRoutine(context, widget.notes, widget.places, routine);
    if (!mounted) return;
    _reload();
  }

  /// What the calendar button offers: the Note on its own at a time the
  /// calendar app lets the user pick, each event of its Routines and
  /// Reminders, and all of those as one calendar file.
  List<CalendarChoice> _calendarChoices() {
    final note = _placed?.note;
    if (note == null) return const [];
    final (:add, :file) = noteCalendar(
      context,
      _routineBook,
      note,
      text: TextBoxes.strip(_typedText),
    );
    return [...add, ?file];
  }

  /// Removes the photo ([photo]) or recording [hash] of the Note or
  /// Section [id] after asking once; its text stays.
  Future<void> _removeMedia(String id, String hash, bool photo) async {
    _saveText();
    final removed = await removeMedia(
      context,
      widget.notes,
      id,
      hash,
      photo: photo,
    );
    if (!mounted || !removed) return;
    _reload();
  }

  /// The button under the photo ([photo]) or recording [hash] of the
  /// Note or Section [id] that removes it and keeps its text; none when
  /// it may not go.
  List<Widget> _removeButton(String id, String hash, {required bool photo}) {
    if (!_removable.contains((id, hash))) return const [];
    final t = context.t;
    return [
      Align(
        alignment: Alignment.centerLeft,
        child: TextButton.icon(
          key: ValueKey('remove:$hash'),
          onPressed: () => _removeMedia(id, hash, photo),
          icon: Icon(
            photo ? Icons.hide_image_outlined : Icons.mic_off_outlined,
          ),
          label: Text(
            photo ? t.removePhotoKeepText : t.removeRecordingKeepText,
          ),
        ),
      ),
    ];
  }

  /// Makes the Transcript or Photo Text [generated] the text of the Note
  /// or Section [id], or adds it on a line of its own after the text
  /// there ([append]); the Note's field then shows the result.
  void _takeOver(String id, String generated, {required bool append}) {
    _saveText();
    final target = widget.notes.byId(id);
    if (target == null || !target.textEditable) return;
    // Into a formatted text it goes escaped, reading as itself.
    widget.notes.takeOver(id, generated, append: append);
    setState(_load);
    if (id == widget.noteId) _show();
    // A Section's field shows the text it has now, or the next save
    // would put the old one back.
    if ((_sectionTexts[id], widget.notes.section(id)) case (
      final field?,
      final section?,
    )) {
      field
        ..markdown = section.textFormat == TextFormats.markdown
        ..text = section.text;
    }
  }

  /// Removes the Photo Text ([photo]) or Transcript of [hash] of the Note
  /// or Section [id] without asking; the file stays, and the announcement
  /// at the top undoes it (#268).
  void _removeGenerated(String id, String hash, bool photo) {
    _saveText();
    if (removeGeneratedText(context, widget.notes, id, hash, photo: photo)) {
      _reload();
    }
  }

  /// The files of the Note or Section [id] (#267): each with its
  /// Attachment Text, Replace and Append for a text that says something,
  /// Remove for the text and for the file where this Device may, and
  /// Read the text again; then the texts of the files removed while their
  /// texts stay, under their names.
  List<Widget> _fileParts(
    String id,
    List<Attachment> files,
    Map<String, String> texts,
    List<Attachment> removed,
  ) {
    final t = context.t;
    Widget button(
      String key,
      String hash,
      IconData icon,
      String label,
      VoidCallback? run,
    ) => Align(
      alignment: Alignment.centerLeft,
      child: TextButton.icon(
        key: ValueKey('$key:$hash'),
        onPressed: run,
        icon: Icon(icon),
        label: Text(label),
      ),
    );
    List<Widget> takeOver(String hash, String? text) => [
      if (text != null &&
          text.trim().isNotEmpty &&
          _textChangeable.contains(id)) ...[
        button(
          'replace-text',
          hash,
          Icons.swap_horiz,
          t.replaceNoteText,
          () => _takeOver(id, text, append: false),
        ),
        button(
          'append-text',
          hash,
          Icons.playlist_add,
          t.appendToNoteText,
          () => _takeOver(id, text, append: true),
        ),
      ],
    ];
    return [
      for (final file in files) ...[
        const SizedBox(height: 16),
        AttachmentRow(
          key: ValueKey('file:${file.hash}'),
          file: file,
          noteId: id,
          text: texts[file.hash],
        ),
        AttachmentText(file: file, text: texts[file.hash]),
        ...takeOver(file.hash, texts[file.hash]),
        if (_fileTextRemovable.contains((id, file.hash)))
          button(
            'remove-text',
            file.hash,
            Icons.backspace_outlined,
            t.removeGeneratedText,
            () {
              _saveText();
              if (removeAttachmentText(context, widget.notes, id, file.hash)) {
                _reload();
              }
            },
          ),
        if (_fileReadable.contains((id, file.hash)))
          button(
            'again',
            file.hash,
            Icons.refresh,
            t.readTextAgain,
            _remaking.contains((id, file.hash))
                ? null
                : () => _readFileAgain(id, file.hash),
          ),
        if (_fileRemovable.contains((id, file.hash)))
          button(
            'remove',
            file.hash,
            Icons.link_off,
            t.removeFileKeepText,
            () async {
              _saveText();
              final gone = await removeAttachment(
                context,
                widget.notes,
                id,
                file.hash,
              );
              if (mounted && gone) _reload();
            },
          ),
      ],
      // Removed, the file shows no more; its text stays.
      for (final file in removed) ...[
        const SizedBox(height: 16),
        GeneratedTextLabel(
          key: ValueKey('removed-file:${file.hash}'),
          label: file.name.isEmpty ? t.attachmentTextLabel : file.name,
          child: AttachmentText(file: file, text: texts[file.hash]),
        ),
        ...takeOver(file.hash, texts[file.hash]),
      ],
    ];
  }

  /// Reads the file [hash] of the Note or Section [id] again (#267).
  Future<void> _readFileAgain(String id, String hash) async {
    _saveText();
    setState(() => _remaking.add((id, hash)));
    await readAttachmentAgain(context, id, hash);
    if (!mounted) return;
    setState(() {
      _remaking.remove((id, hash));
      _load();
    });
  }

  /// Picks files with the system's file picker and attaches them to the
  /// Note (#267); one that cannot be is said at the top.
  Future<void> _attachFile() async {
    _saveText();
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final files = FileScope.of(context);
    final picked = await files.pickFiles();
    if (picked.isEmpty) return;
    final read = await readPickedFiles(files, picked, widget.notes.store);
    announceNotAttached(t, announcements, navigator, read);
    // Written also when the editor was left meanwhile: nothing is lost.
    attachFiles(widget.notes, widget.noteId, read.files);
    if (mounted) _reload();
  }

  /// Reads the photo ([photo]) or transcribes the recording [hash] of the
  /// Note or Section [id] again; a failure is announced at the top, also
  /// when the editor was left meanwhile.
  Future<void> _makeAgain(String id, String hash, bool photo) async {
    _saveText();
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context);
    final reader = PhotoReaderScope.of(context);
    final transcriber = SpeechScope.of(context)?.transcriber;
    setState(() => _remaking.add((id, hash)));
    final made = photo
        ? await reader?.readAgain(id, hash) ?? false
        : await transcriber?.transcribeAgain(id, hash) ?? false;
    if (mounted) {
      setState(() {
        _remaking.remove((id, hash));
        _load();
      });
    }
    if (made || announcements == null) return;
    announcements.announce(
      photo ? t.readTextAgainFailed : t.transcribeAgainFailed,
      onTap: () => unawaited(
        showFailure(
          navigator.context,
          title: photo
              ? t.readTextAgainFailedTitle
              : t.transcribeAgainFailedTitle,
          cause: photo
              ? t.readTextAgainFailedCause
              : t.transcribeAgainFailedCause,
          fix: t.generatedTextAgainFix,
          details: photo
              ? 'read again: no Photo Text'
              : 'transcribe again: no Transcript',
        ),
      ),
    );
  }

  /// The buttons under the Photo Text ([photo]) or Transcript [text] of
  /// [hash] of the Note or Section [id], one per line: Replace and Append
  /// for a text that says something, Remove for one this Device may
  /// remove, and again while the file is here and an engine runs. Only
  /// "again" shows for a removed text.
  List<Widget> _generatedButtons(
    String id,
    String hash,
    String? text, {
    required bool photo,
  }) {
    final t = context.t;
    // RustO is tried until it is known not to run; Whisper needs a model.
    final engine = photo
        ? switch (PhotoReaderScope.of(context)) {
            final reader? => reader.supported != false,
            null => false,
          }
        : SpeechScope.of(context)?.model.current != null;
    final remaking = _remaking.contains((id, hash));
    Widget button(String key, IconData icon, String label, VoidCallback? run) =>
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            key: ValueKey('$key:$hash'),
            onPressed: run,
            icon: Icon(icon),
            label: Text(label),
          ),
        );
    return [
      if (text != null &&
          text.trim().isNotEmpty &&
          _textChangeable.contains(id)) ...[
        button(
          'replace-text',
          Icons.swap_horiz,
          t.replaceNoteText,
          () => _takeOver(id, text, append: false),
        ),
        button(
          'append-text',
          Icons.playlist_add,
          t.appendToNoteText,
          () => _takeOver(id, text, append: true),
        ),
      ],
      if (_textRemovable.contains((id, hash)))
        button(
          'remove-text',
          Icons.backspace_outlined,
          t.removeGeneratedText,
          () => _removeGenerated(id, hash, photo),
        ),
      if (engine && _remakeable.contains((id, hash)))
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            key: ValueKey('again:$hash'),
            onPressed: remaking ? null : () => _makeAgain(id, hash, photo),
            icon: remaking
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.refresh),
            label: Text(photo ? t.readTextAgain : t.transcribeAgain),
          ),
        ),
    ];
  }

  /// Merges the Note, with its text as typed, into the one picked; it
  /// then is a Section there and the editor closes.
  Future<void> _merge() async {
    final note = _placed?.note;
    if (note == null) return;
    _saveText();
    if (await mergeNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      Navigator.of(context).pop();
    }
  }

  /// The text of a Note of a kind (#188): its name as a heading, one line,
  /// and under it its description, small and weak, folded to one line
  /// while it is not edited; a tap unfolds it, and puts the cursor in it
  /// where this Device may change it ([editable]), else only shows it
  /// whole. An empty one that may change is a field with its hint.
  Widget _nameAndDescription(Note note, {required bool editable}) {
    final t = context.t;
    final theme = Theme.of(context);
    final weak = theme.colorScheme.onSurfaceVariant;
    final small = theme.textTheme.bodyMedium?.copyWith(color: weak);
    final description = _text.text;
    final open = _descriptionOpen || (editable && description.isEmpty);
    final Widget shown;
    if (open) {
      shown = Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: TwoRowTextField(
              key: const ValueKey('note-description'),
              controller: _text,
              focusNode: _descriptionFocus,
              twoRows: _twoRows,
              readOnly: !editable,
              boxes: true,
              bar: widget.notes.store.advanced,
              onBox: _tapBox,
              formats: true,
              onFormatted: () => setState(_saveText),
              style: small,
              decoration: InputDecoration(
                filled: false,
                border: InputBorder.none,
                isDense: true,
                contentPadding: const EdgeInsets.only(bottom: 12),
                hintText: t.noteDescription,
              ),
            ),
          ),
          if (description.isNotEmpty)
            IconButton(
              key: const ValueKey('note-description-fold'),
              tooltip: t.noteDescriptionFold,
              visualDensity: VisualDensity.compact,
              onPressed: _foldDescription,
              icon: Icon(Icons.expand_less, color: weak),
            ),
        ],
      );
    } else if (description.isEmpty) {
      // Only read and empty: nothing to show.
      shown = const SizedBox(height: 12);
    } else {
      final visible = TextBoxes.strip(description);
      final (start, end) = NoteCard.firstLine(visible);
      shown = Semantics(
        key: const ValueKey('note-description-folded'),
        container: true,
        button: true,
        hint: t.noteDescriptionUnfold,
        child: InkWell(
          onTap: () => _openDescription(atEnd: true),
          child: Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Row(
              children: [
                Expanded(
                  child: MarkedText(
                    visible,
                    format: note.textFormat,
                    start: start,
                    end: end,
                    style: small,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                Icon(Icons.expand_more, color: weak, size: 20),
              ],
            ),
          ),
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TwoRowTextField(
          key: const ValueKey('note-name'),
          controller: _name,
          twoRows: _twoRows,
          readOnly: !editable,
          boxes: true,
          bar: widget.notes.store.advanced,
          onBox: (start, at) => _tapBox(start, at, name: true),
          style: theme.textTheme.titleLarge,
          decoration: InputDecoration(
            filled: false,
            border: InputBorder.none,
            hintText: t.noteName,
          ),
        ),
        shown,
      ],
    );
  }

  /// Moves the Note to the trash as the card's Delete does, without
  /// asking, and closes the editor; the announcement at the top undoes it
  /// (#268).
  void _trash() {
    if (!deleteNote(context, widget.notes, widget.noteId)) return;
    Navigator.of(context).pop();
  }

  /// Checks off [open] from the line at the top (#215); with [trash] the
  /// Note goes to the trash with its one Reminder, and the editor closes.
  /// It closes too when the Done moved the Note there by the person's
  /// setting (#211).
  void _checkOff(OpenOccurrence open, {bool trash = false}) {
    final trashed = _routineBook.checkOff(open.routine.id, open.occurrence);
    if (!trash && !trashed) {
      _reload();
      return;
    }
    if (!trashed) _routineBook.trashNote(widget.noteId);
    Navigator.of(context).pop();
  }

  /// Answers [open] from the line at the top with a snooze, Skip or
  /// Tomorrow (#269); the latter two take it off the line.
  void _answer(OpenOccurrence open, OccurrenceAnswer answer) {
    answerOccurrence(
      context,
      _routineBook,
      AlarmSyncScope.of(context),
      open.routine.id,
      open.occurrence,
      answer,
    );
    _reload();
  }

  /// The lines at the top of the editor, above the name (#215): of each
  /// Occurrence the Note asks to check off "Due 15:00" or "Missed" with
  /// its day, and Done, Snooze while it is not Missed and Not today
  /// (#269); for a one-time Reminder, the Note's only one, also
  /// "Done and move to trash" where this Device may move the Note.
  List<Widget> _openLines(AppLocalizations t, bool changes) {
    final locale = Localizations.localeOf(context).toString();
    final today = Day.today(widget.notes.store.clock(), deviceWallClock);
    return [
      for (final open in _open)
        Padding(
          key: ValueKey('open-${open.routine.id}@${open.occurrence.key}'),
          padding: const EdgeInsets.only(bottom: 8),
          child: Wrap(
            spacing: 8,
            runSpacing: 4,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text(switch ((
                open.missed,
                occurrenceTime(t, locale, open.occurrence),
              )) {
                (true, final time) => t.occurrenceMissedAt(
                  '${dayText(locale, open.occurrence.day)} $time',
                ),
                (false, final time) when open.occurrence.day == today =>
                  t.occurrenceDue(time),
                (false, final time) => t.occurrenceDue(
                  '${dayText(locale, open.occurrence.day)} $time',
                ),
              }, style: Theme.of(context).textTheme.titleMedium),
              FilledButton(
                key: const ValueKey('open-done'),
                onPressed: () => _checkOff(open),
                child: Text(t.markDone),
              ),
              // Snooze rings from now on: not for one Missed (#269).
              if (!open.missed)
                AnswerButton(
                  key: const ValueKey('open-snooze'),
                  label: t.snooze,
                  items: () => occurrenceAnswerItems(
                    t,
                    (answer) => answer,
                    snooze: true,
                    notToday: false,
                    tomorrow: false,
                  ),
                  onAnswer: (answer) => _answer(open, answer),
                ),
              AnswerButton(
                key: const ValueKey('open-not-today'),
                label: t.notToday,
                items: () => occurrenceAnswerItems(
                  t,
                  (answer) => answer,
                  snooze: false,
                  notToday: true,
                  // Tomorrow only for one of today or later.
                  tomorrow: !(open.occurrence.day < today),
                ),
                onAnswer: (answer) => _answer(open, answer),
              ),
              if (changes && open.routine.isReminder && _routines.length == 1)
                OutlinedButton(
                  key: const ValueKey('open-done-trash'),
                  // It moves to the trash: the error colour, as Delete.
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Theme.of(context).colorScheme.error,
                  ),
                  onPressed: () => _checkOff(open, trash: true),
                  child: Text(t.doneAndTrash),
                ),
            ],
          ),
        ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final placed = _placed;
    if (placed == null) return Scaffold(appBar: AppBar());
    final (:note, :area, :topic, :changes, schedules: _, amends: _, author: _) =
        placed;
    // In a Shared Area a Member changes only their own Notes, and only a
    // Manager their Routines (ADR 0013); Amend stays.
    final amends = _amends;
    final editable = changes && note.textEditable;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    final colour = areaColour(area?.colour);
    final place = area == null
        ? (note.spawned ? t.notFiled : t.unsorted)
        : [
            areaName(t, area),
            if (topic != null) topicName(t, topic),
          ].join(' / ');
    return PopScope(
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) return;
        // Leaving drops the Lines left empty (#141).
        _lines.leave();
        _saveText();
      },
      child: Scaffold(
        appBar: pageBar(
          context,
          title: t.noteTitle,
          actions: [
            BarAction(
              key: const ValueKey('send-note-file'),
              label: t.sendToPerson,
              onPressed: () {
                _saveText();
                unawaited(
                  sendNoteFile(context, widget.notes.store, widget.noteId),
                );
              },
              icon: Icons.send_outlined,
            ),
            BarAction(
              key: const ValueKey('share-note'),
              label: t.shareToApp,
              onPressed: () {
                _saveText();
                unawaited(shareNoteToApp(context, widget.notes, widget.noteId));
              },
              icon: Icons.share_outlined,
            ),
            BarAction(
              key: const ValueKey('pack-note'),
              label: t.exportAsPack,
              onPressed: () {
                _saveText();
                unawaited(
                  exportPack(context, widget.notes.store, widget.noteId),
                );
              },
              icon: Icons.inventory_2_outlined,
            ),
            CalendarButton(choices: _calendarChoices),
            BarAction(
              key: const ValueKey('note-history'),
              label: t.historyTitle,
              onPressed: _openHistory,
              icon: Icons.history,
            ),
            if (changes) ...[
              if (FileScope.of(context).pickable)
                BarAction(
                  key: const ValueKey('attach-file'),
                  label: t.attachFile,
                  onPressed: () => unawaited(_attachFile()),
                  icon: Icons.attach_file,
                ),
              BarAction(
                label: t.mergeInto,
                onPressed: _merge,
                icon: Icons.call_merge,
              ),
              BarAction(
                label: t.moveToTrash,
                onPressed: _trash,
                icon: Icons.delete_outline,
              ),
            ],
          ],
        ),
        body: ListView(
          padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
          children: [
            if (!note.textEditable) ...[
              const NewerFormatNote(key: ValueKey('note-newer-format')),
              const SizedBox(height: 8),
            ],
            ..._openLines(t, changes),
            Embossed(
              strong: note.importance == Importance.important,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: _named
                    ? _nameAndDescription(note, editable: editable)
                    : TwoRowTextField(
                        key: const ValueKey('note-text'),
                        controller: _text,
                        twoRows: _twoRows,
                        readOnly: !editable,
                        // ☐ goes in at the cursor; a tap on a box ticks it,
                        // also where the text is only read (ADR 0035).
                        boxes: true,
                        bar: widget.notes.store.advanced,
                        onBox: _tapBox,
                        // The first format is written at once (ADR 0033).
                        formats: true,
                        onFormatted: () => setState(_saveText),
                        // A paste that looks like code offers it (#213).
                        onCodePasted:
                            editable &&
                                _kind == NoteKind.plain &&
                                note.textFormat != TextFormats.code
                            ? () => setState(() => _offersCode = true)
                            : null,
                        style: Theme.of(context).textTheme.bodyLarge,
                        decoration: const InputDecoration(
                          filled: false,
                          border: InputBorder.none,
                        ),
                      ),
              ),
            ),
            if (_offersCode && !_named) _codeOffer(t),
            // The kind's content right under its name (#188): nothing
            // while the Note is no Checklist, Shopping List or Tally.
            ChecklistSection(
              notes: widget.notes,
              noteId: widget.noteId,
              typed: _lines,
              startLine: widget.typeLines,
            ),
            ShoppingSection(
              notes: widget.notes,
              noteId: widget.noteId,
              typed: _lines,
              startLine: widget.typeLines,
            ),
            if (_kind == NoteKind.tally) const SizedBox(height: 16),
            TallySetup(notes: widget.notes, noteId: widget.noteId),
            if (_sessionDraft case final draft?) ...[
              const SizedBox(height: 16),
              SessionNoteSetup(
                draft: draft,
                changes: _setsUp,
                onChanged: _sessionChanged,
                onNewTally: _newPoolTally,
              ),
            ],
            // A list's or Tally's panel keeps its own space under it.
            if (!_named) const SizedBox(height: 16),
            _badges(placed, when, place, colour),
            // Its past runs (#264), under its settings.
            if (_sessionDraft != null) ...[
              const SizedBox(height: 16),
              EditorPanel(
                heading: context.t.sessionRuns,
                child: SessionRuns(notes: widget.notes, noteId: widget.noteId),
              ),
            ],
            for (final hash in note.photos) ...[
              const SizedBox(height: 16),
              NotePhoto(
                key: ValueKey(hash),
                hash: hash,
                onTap: () => viewPhoto(context, hash),
              ),
              PhotoText(
                key: ValueKey('ocr:$hash'),
                noteId: note.id,
                hash: hash,
                photoText: note.photoTexts[hash],
              ),
              ..._generatedButtons(
                note.id,
                hash,
                note.photoTexts[hash],
                photo: true,
              ),
              ..._removeButton(note.id, hash, photo: true),
            ],
            // Removed, the file shows no more; its text stays.
            for (final hash in note.removedPhotos) ...[
              PhotoText(
                key: ValueKey('ocr:$hash'),
                noteId: note.id,
                hash: hash,
                photoText: note.photoTexts[hash],
              ),
              ..._generatedButtons(
                note.id,
                hash,
                note.photoTexts[hash],
                photo: true,
              ),
            ],
            for (final hash in note.recordings) ...[
              const SizedBox(height: 16),
              Align(
                alignment: Alignment.centerLeft,
                child: RecordingButton(
                  key: ValueKey(hash),
                  hash: hash,
                  compact: false,
                ),
              ),
              TranscriptText(
                key: ValueKey('transcript:$hash'),
                noteId: note.id,
                hash: hash,
                transcript: note.transcripts[hash],
                compact: false,
              ),
              ..._generatedButtons(
                note.id,
                hash,
                note.transcripts[hash],
                photo: false,
              ),
              ..._removeButton(note.id, hash, photo: false),
            ],
            for (final hash in note.removedRecordings) ...[
              TranscriptText(
                key: ValueKey('transcript:$hash'),
                noteId: note.id,
                hash: hash,
                transcript: note.transcripts[hash],
                compact: false,
              ),
              ..._generatedButtons(
                note.id,
                hash,
                note.transcripts[hash],
                photo: false,
              ),
            ],
            ..._fileParts(
              note.id,
              note.attachments,
              note.attachmentTexts,
              note.removedAttachments,
            ),
            const SizedBox(height: 16),
            for (final (i, section) in _sections.indexed)
              EditorPanel(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (!section.textEditable) ...[
                      NewerFormatNote(key: ValueKey('newer:${section.id}')),
                      const SizedBox(height: 8),
                    ],
                    // Edited in place as the Note's text, by the same
                    // people (#183); otherwise selected and copied with a
                    // long press, a tap on a box or tag still going to it.
                    if (_sectionTexts[section.id] case final field?)
                      TwoRowTextField(
                        key: ValueKey('section-text:${section.id}'),
                        controller: field,
                        twoRows: _twoRows,
                        boxes: true,
                        bar: widget.notes.store.advanced,
                        onBox: (start, at) =>
                            _tapBox(start, at, section: section.id),
                        formats: true,
                        onFormatted: () => setState(_saveText),
                        style: Theme.of(context).textTheme.bodyLarge,
                        // It sits where its text was read before.
                        decoration: const InputDecoration(
                          filled: false,
                          border: InputBorder.none,
                          isDense: true,
                          contentPadding: EdgeInsets.zero,
                        ),
                      )
                    else if (section.text.isNotEmpty)
                      SelectionArea(
                        child: MarkedText(
                          section.shownText,
                          format: section.textFormat,
                          style: Theme.of(context).textTheme.bodyLarge,
                          boxes: section.boxes,
                          source: section.text,
                          ticking: _sectionTicking[section.id],
                        ),
                      ),
                    for (final hash in section.photos) ...[
                      const SizedBox(height: 8),
                      NotePhoto(
                        key: ValueKey(hash),
                        hash: hash,
                        onTap: () => viewPhoto(context, hash),
                      ),
                      PhotoText(
                        key: ValueKey('ocr:$hash'),
                        noteId: section.id,
                        hash: hash,
                        photoText: section.photoTexts[hash],
                      ),
                      ..._generatedButtons(
                        section.id,
                        hash,
                        section.photoTexts[hash],
                        photo: true,
                      ),
                      ..._removeButton(section.id, hash, photo: true),
                    ],
                    for (final hash in section.removedPhotos) ...[
                      PhotoText(
                        key: ValueKey('ocr:$hash'),
                        noteId: section.id,
                        hash: hash,
                        photoText: section.photoTexts[hash],
                      ),
                      ..._generatedButtons(
                        section.id,
                        hash,
                        section.photoTexts[hash],
                        photo: true,
                      ),
                    ],
                    for (final hash in section.recordings) ...[
                      const SizedBox(height: 8),
                      RecordingButton(
                        key: ValueKey(hash),
                        hash: hash,
                        compact: false,
                      ),
                      TranscriptText(
                        key: ValueKey('transcript:$hash'),
                        noteId: section.id,
                        hash: hash,
                        transcript: section.transcripts[hash],
                        compact: false,
                      ),
                      ..._generatedButtons(
                        section.id,
                        hash,
                        section.transcripts[hash],
                        photo: false,
                      ),
                      ..._removeButton(section.id, hash, photo: false),
                    ],
                    for (final hash in section.removedRecordings) ...[
                      TranscriptText(
                        key: ValueKey('transcript:$hash'),
                        noteId: section.id,
                        hash: hash,
                        transcript: section.transcripts[hash],
                        compact: false,
                      ),
                      ..._generatedButtons(
                        section.id,
                        hash,
                        section.transcripts[hash],
                        photo: false,
                      ),
                    ],
                    // A merged Note's files travel with it (ADR 0047).
                    ..._fileParts(
                      section.id,
                      section.attachments,
                      section.attachmentTexts,
                      section.removedAttachments,
                    ),
                    const SizedBox(height: 4),
                    // Its details, each removable on its own, in the
                    // same dialog as the Note's Captured badge (#170).
                    CaptureLine.ofSection(
                      section,
                      key: ValueKey('captured-line:${section.id}'),
                      when: when,
                      after: i == 0
                          ? note.captured
                          : _sections[i - 1].context.captured,
                      onTap: () => _showCaptured(when, section: section.id),
                    ),
                  ],
                ),
              ),
            const SizedBox(height: 8),
            if (amends)
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  onPressed: _amend,
                  icon: const Icon(Icons.add),
                  label: Text(t.amend),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// The icon of the text format badge (#213), the same for every format;
/// the badge shows the format itself as [textFormatIcon].
const textFormatBadgeIcon = Icons.text_format;

/// The value icon of the text format [format] in its badge (#213).
IconData textFormatIcon(String format) => switch (format) {
  TextFormats.markdown => Icons.format_bold,
  TextFormats.code => Icons.code,
  _ => Icons.short_text,
};

/// The text format [format] in words (#213).
String textFormatName(AppLocalizations t, String format) => switch (format) {
  TextFormats.markdown => t.textFormatFormatted,
  TextFormats.code => t.textFormatCode,
  _ => t.textFormatPlain,
};

/// The icon of the kind badge (#192), the same for every kind; the
/// badge shows the kind itself as [kindIcon].
const kindBadgeIcon = Icons.category_outlined;

/// The value icon of the Importance [importance] in its badge (#192).
IconData importanceIcon(Importance importance) => switch (importance) {
  Importance.important => Icons.priority_high,
  Importance.normal => Icons.remove,
  Importance.unimportant => Icons.low_priority,
};

/// The value icon of a Shopping List's or a Checklist's Show on the
/// Agenda [shows] in its badge (#192, #219).
IconData listOnAgendaIcon(ListOnAgenda shows) => switch (shows) {
  ListOnAgenda.off => switchIcon(false),
  ListOnAgenda.count => Icons.numbers,
  ListOnAgenda.lines => Icons.format_list_bulleted,
};

/// The icon of the kind [kind] (#188), the value of its badge.
IconData kindIcon(NoteKind kind) => switch (kind) {
  NoteKind.plain => Icons.notes,
  NoteKind.checklist => Icons.checklist,
  NoteKind.shoppingList => Icons.shopping_cart_outlined,
  NoteKind.tally => Icons.bar_chart,
  NoteKind.session => Icons.event_repeat,
};

/// The kind [kind] in words.
String kindName(AppLocalizations t, NoteKind kind) => switch (kind) {
  NoteKind.plain => t.kindPlain,
  NoteKind.checklist => t.checklistHeading,
  NoteKind.shoppingList => t.shoppingHeading,
  NoteKind.tally => t.tallyHeading,
  NoteKind.session => t.session,
};

/// Says that a text is in a format this version does not know and shows
/// as it is stored: a newer version of the app is needed to change it.
class NewerFormatNote extends StatelessWidget {
  const NewerFormatNote({super.key});

  @override
  Widget build(BuildContext context) {
    final muted = Theme.of(context).colorScheme.onSurfaceVariant;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(Icons.info_outline, size: 20, color: muted),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            context.t.textNewerFormat,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ),
      ],
    );
  }
}

/// One embossed block of an editor, with an optional heading.
class EditorPanel extends StatelessWidget {
  final String? heading;
  final Widget child;

  const EditorPanel({super.key, required this.child, this.heading});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 16),
    child: Embossed(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (heading case final heading?) ...[
              Text(heading, style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 8),
            ],
            child,
          ],
        ),
      ),
    ),
  );
}

/// Asks once whether a photo or voice recording goes while its text
/// stays, or a text while its file stays: true to remove it, null to
/// cancel.
class _RemoveDialog extends StatelessWidget {
  final String title;
  final String detail;

  const _RemoveDialog({required this.title, required this.detail});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(title),
      content: Text(detail),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(
          key: const ValueKey('remove-media-confirm'),
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.remove),
        ),
      ],
    );
  }
}

/// Asks for the text of a new Section.
class _AmendDialog extends StatefulWidget {
  /// Whether the text is typed in two rows (UI laws, "Text editor").
  final bool twoRows;

  /// Whether the format bar shows: only in Advanced (#197).
  final bool bar;

  const _AmendDialog({required this.twoRows, required this.bar});

  @override
  State<_AmendDialog> createState() => _AmendDialogState();
}

class _AmendDialogState extends State<_AmendDialog> {
  /// Formatted once a format is applied (ADR 0033).
  late final _text = TagTextController(hideMarks: widget.twoRows);

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final media = MediaQuery.of(context);
    final keyboard = media.viewInsets.bottom;
    // The dialog stays clear of what rides on the keyboard.
    final below = keyboard > 0
        ? keyboard +
              TwoRowTextField.above(twoRows: widget.twoRows, bar: widget.bar)
        : keyboard;
    return MediaQuery(
      data: media.copyWith(
        viewInsets: media.viewInsets.copyWith(bottom: below),
      ),
      child: AlertDialog(
        title: Text(t.amend),
        content: TwoRowTextField(
          controller: _text,
          twoRows: widget.twoRows,
          autofocus: true,
          formats: true,
          bar: widget.bar,
          decoration: InputDecoration(hintText: t.amendHint),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(t.cancel),
          ),
          TextButton(
            onPressed: () =>
                Navigator.of(context)
                    .pop((text: _text.text, markdown: _text.markdown)),
            child: Text(t.add),
          ),
        ],
      ),
    );
  }
}

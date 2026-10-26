import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'areas_screen.dart';
import 'calendar_platform.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'l10n.dart';
import 'location.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'ocr_platform.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'sent_item_screen.dart';
import 'speech_platform.dart';
import 'system_bars.dart';
import 'voice_platform.dart';

/// Opens the editor of [note]; returns once the user left it.
Future<void> openNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note,
) => Navigator.of(context).push(
  MaterialPageRoute<void>(
    builder: (_) => NoteEditorScreen(
      notes: notes,
      places: places,
      drafts: DraftsScope.of(context),
      noteId: note.id,
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
  final text = await showDialog<String>(
    context: context,
    builder: (_) => const _AmendDialog(),
  );
  final trimmed = text?.trim();
  if (!context.mounted || trimmed == null || trimmed.isEmpty) return false;
  final section = notes.amend(noteId, trimmed);
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

/// Asks once whether the Note [noteId] goes to the trash, with the verb
/// on the button in the error colour, and moves it there; whether it
/// did. When it spawned Routines or Reminders, the dialog offers to
/// delete the Note only or with them; they never go as a side effect.
Future<bool> deleteNote(
  BuildContext context,
  Notes notes,
  String noteId,
) async {
  final book = Routines(notes.store);
  final linked = book.ofNote(noteId).length;
  final withRoutines = await showDialog<bool>(
    context: context,
    builder: (context) => _DeleteNoteDialog(linked: linked),
  );
  if (withRoutines == null) return false;
  book.trashNote(noteId, withRoutines: withRoutines);
  return true;
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
    builder: (_) => _RemoveMediaDialog(photo: photo),
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
    photo: photo,
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
  ];
}

/// The ⋮ menu of [placed]'s card: what the Note editor offers beyond
/// filing, scheduling and merging, as far as this Device may (ADR 0013).
/// [changed] runs after each step that may have changed the Note.
NoteActions noteActions(
  BuildContext context,
  Notes notes,
  PlacedNote placed,
  VoidCallback changed,
) {
  final id = placed.note.id;
  void then(Future<Object?> step) => unawaited(
    step.then((_) {
      if (context.mounted) changed();
    }),
  );
  return NoteActions(
    onImportance: placed.changes
        ? (picked) {
            notes.setImportance(id, picked);
            changed();
          }
        : null,
    onSend: () => unawaited(sendNoteFile(context, notes.store, id)),
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
    onDelete: placed.changes
        ? () => then(deleteNote(context, notes, id))
        : null,
  );
}

/// Everything about one Note: its text, fixed in place and saved on
/// leaving, its photos at full width, each opening full screen when
/// tapped and with its Photo Text under it, a play button per voice
/// recording with its Transcript under it, the texts of the photos and
/// recordings removed while their texts stay, its Importance, its Capture
/// Context and where it is filed,
/// its Routines and Reminders, and its Sections, each with its own
/// Capture Context. "Add to Note" amends it; "Routine" and "Reminder"
/// spawn one from it; the calendar button adds it, or its Routines and
/// Reminders, to the calendar; the send button sends it as a Sent Item;
/// "merge into…" merges it into another Note; the trash button deletes
/// it. A photo or recording with a text, of the Note or a Section, has a
/// button under it that removes it and keeps the text (ADR 0020).
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

  const NoteEditorScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.drafts,
    required this.noteId,
  });

  @override
  State<NoteEditorScreen> createState() => _NoteEditorScreenState();
}

class _NoteEditorScreenState extends State<NoteEditorScreen> {
  final _text = TextEditingController();
  late final AppLifecycleListener _lifecycle;
  Timer? _draftTimer;
  PlacedNote? _placed;
  List<Section> _sections = const [];
  List<Routine> _routines = const [];

  /// The photos and recordings, by the Note or Section they belong to,
  /// that this Device may remove while their texts stay.
  Set<(String, String)> _removable = const {};
  late final _routineBook = Routines(widget.notes.store);

  /// Whether this Device may Amend the Note: anywhere it holds a role.
  bool _amends = true;

  @override
  void initState() {
    super.initState();
    _load();
    _text.text = _placed?.note.text ?? '';
    _text.addListener(_typed);
    _lifecycle = AppLifecycleListener(onStateChange: _lifecycleChanged);
    _restoreDraft();
  }

  @override
  void dispose() {
    _draftTimer?.cancel();
    _lifecycle.dispose();
    _text.dispose();
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

  /// Keeps the text as typed in a draft while it differs from the Note's.
  void _writeDraft() {
    final note = _placed?.note;
    if (note == null || !note.textEditable) return;
    final text = _text.text;
    if (text.trim().isEmpty || text.trim() == note.text) {
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
      if (_text.text == note.text) _text.text = draft.text;
    } else {
      unawaited(widget.drafts.delete(widget.noteId));
    }
  }

  void _load() {
    final note = widget.notes.byId(widget.noteId);
    _placed = note == null ? null : placeNote(widget.places, note);
    _amends = widget.notes.store.mayWriteIn(widget.noteId);
    _sections = widget.notes.sections(widget.noteId);
    _routines = _routineBook.ofNote(widget.noteId);
    final notes = widget.notes;
    _removable = {
      for (final (id, photos, recordings) in [
        if (note != null) (note.id, note.photos, note.recordings),
        for (final section in _sections)
          (section.id, section.photos, section.recordings),
      ]) ...[
        for (final hash in photos)
          if (notes.mayRemovePhoto(id, hash)) (id, hash),
        for (final hash in recordings)
          if (notes.mayRemoveRecording(id, hash)) (id, hash),
      ],
    };
  }

  void _reload() => setState(_load);

  /// Writes the text when it changed; a blank text keeps the old one.
  /// The draft goes either way: the log now holds what counts.
  void _saveText() {
    _draftTimer?.cancel();
    final note = _placed?.note;
    if (note == null || !(_placed?.changes ?? false)) return;
    unawaited(widget.drafts.delete(note.id));
    // A text in a format not known here is never overwritten.
    if (!note.textEditable) return;
    final text = _text.text.trim();
    if (text.isEmpty || text == note.text) return;
    widget.notes.edit(note.id, text);
    _load();
  }

  void _setImportance(Importance picked) {
    _saveText();
    widget.notes.setImportance(widget.noteId, picked);
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

  Future<void> _spawn({required bool reminder}) async {
    final note = _placed?.note;
    if (note == null) return;
    _saveText();
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      reminder: reminder,
    );
    if (!mounted) return;
    _reload();
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
      text: _text.text,
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

  /// Moves the Note to the trash at once, as it can be restored. When it
  /// spawned Routines or Reminders, asks first whether they go too; they
  /// never vanish as a side effect.
  Future<void> _trash() async {
    final routines = _routineBook.ofNote(widget.noteId);
    var withRoutines = false;
    if (routines.isNotEmpty) {
      final answer = await showDialog<bool>(
        context: context,
        builder: (_) => _TrashRoutinesDialog(routines: routines),
      );
      if (!mounted || answer == null) return;
      withRoutines = answer;
    }
    _routineBook.trashNote(widget.noteId, withRoutines: withRoutines);
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final placed = _placed;
    if (placed == null) return Scaffold(appBar: AppBar());
    final (:note, :area, :topic, :changes, :schedules, amends: _, author: _) =
        placed;
    // In a Shared Area a Member changes only their own Notes, and only a
    // Manager their Routines (ADR 0013); Amend stays.
    final amends = _amends;
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
        if (didPop) _saveText();
      },
      child: Scaffold(
        appBar: AppBar(
          title: Text(t.noteTitle),
          actions: [
            IconButton(
              key: const ValueKey('send-note-file'),
              tooltip: t.sendNote,
              onPressed: () {
                _saveText();
                unawaited(
                  sendNoteFile(context, widget.notes.store, widget.noteId),
                );
              },
              icon: const Icon(Icons.send_outlined),
            ),
            CalendarButton(choices: _calendarChoices),
            if (changes) ...[
              IconButton(
                tooltip: t.mergeInto,
                onPressed: _merge,
                icon: const Icon(Icons.call_merge),
              ),
              IconButton(
                tooltip: t.moveToTrash,
                onPressed: _trash,
                icon: const Icon(Icons.delete_outline),
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
            Embossed(
              strong: note.importance == Importance.important,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: TextField(
                  controller: _text,
                  readOnly: !changes || !note.textEditable,
                  maxLines: null,
                  textCapitalization: TextCapitalization.sentences,
                  style: Theme.of(context).textTheme.bodyLarge,
                  decoration: const InputDecoration(
                    filled: false,
                    border: InputBorder.none,
                  ),
                ),
              ),
            ),
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
              ..._removeButton(note.id, hash, photo: true),
            ],
            // Removed, the file shows no more; its text stays.
            for (final hash in note.removedPhotos)
              PhotoText(
                key: ValueKey('ocr:$hash'),
                noteId: note.id,
                hash: hash,
                photoText: note.photoTexts[hash],
              ),
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
              ..._removeButton(note.id, hash, photo: false),
            ],
            for (final hash in note.removedRecordings)
              TranscriptText(
                key: ValueKey('transcript:$hash'),
                noteId: note.id,
                hash: hash,
                transcript: note.transcripts[hash],
                compact: false,
              ),
            const SizedBox(height: 16),
            EditorPanel(
              heading: t.importance,
              child: RadioColumn<Importance>(
                value: note.importance,
                options: [
                  RadioOption(
                    value: Importance.unimportant,
                    title: Text(t.importanceUnimportant),
                  ),
                  RadioOption(
                    value: Importance.normal,
                    title: Text(t.importanceNormal),
                  ),
                  RadioOption(
                    value: Importance.important,
                    title: Text(t.importanceImportant),
                  ),
                ],
                onChanged: changes ? _setImportance : null,
              ),
            ),
            EditorPanel(
              heading: t.capturedHeading,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  CaptureContextLines(capture: note.context, when: when),
                  Row(
                    children: [
                      Icon(
                        areaIconData(area?.icon) ?? Icons.folder_outlined,
                        size: 20,
                        color:
                            colour ??
                            Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 8),
                      Expanded(child: Text(place)),
                      if (changes)
                        IconButton(
                          tooltip: t.fileInto,
                          onPressed: _file,
                          icon: const Icon(Icons.drive_file_move_outline),
                        ),
                    ],
                  ),
                ],
              ),
            ),
            EditorPanel(
              heading: t.routinesHeading,
              // The rows' ink shows on the panel, not under it.
              child: Material(
                type: MaterialType.transparency,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (final routine in _routines)
                      ListTile(
                        contentPadding: EdgeInsets.zero,
                        leading: Icon(
                          routine.isReminder
                              ? Icons.notifications_outlined
                              : Icons.repeat,
                        ),
                        title: Text(
                          scheduleText(
                            t,
                            Localizations.localeOf(context).toString(),
                            routine.schedule,
                          ),
                        ),
                        onTap: () => _openRoutine(routine),
                      ),
                    if (!schedules)
                      Text(t.onlyManagers)
                    else
                      Wrap(
                        spacing: 8,
                        children: [
                          FilledButton.tonalIcon(
                            onPressed: () => _spawn(reminder: false),
                            icon: const Icon(Icons.repeat),
                            label: Text(t.routine),
                          ),
                          FilledButton.tonalIcon(
                            onPressed: () => _spawn(reminder: true),
                            icon: const Icon(Icons.notifications_outlined),
                            label: Text(t.reminder),
                          ),
                        ],
                      ),
                  ],
                ),
              ),
            ),
            for (final section in _sections)
              EditorPanel(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (!section.textEditable) ...[
                      NewerFormatNote(key: ValueKey('newer:${section.id}')),
                      const SizedBox(height: 8),
                    ],
                    if (section.text.isNotEmpty)
                      Text(
                        section.text,
                        style: Theme.of(context).textTheme.bodyLarge,
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
                      ..._removeButton(section.id, hash, photo: true),
                    ],
                    for (final hash in section.removedPhotos)
                      PhotoText(
                        key: ValueKey('ocr:$hash'),
                        noteId: section.id,
                        hash: hash,
                        photoText: section.photoTexts[hash],
                      ),
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
                      ..._removeButton(section.id, hash, photo: false),
                    ],
                    for (final hash in section.removedRecordings)
                      TranscriptText(
                        key: ValueKey('transcript:$hash'),
                        noteId: section.id,
                        hash: hash,
                        transcript: section.transcripts[hash],
                        compact: false,
                      ),
                    const SizedBox(height: 8),
                    CaptureContextLines(capture: section.context, when: when),
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

/// Asks whether the Routines and Reminders of a Note being deleted go
/// to the trash too: true for yes, false for no, null to cancel.
class _TrashRoutinesDialog extends StatelessWidget {
  final List<Routine> routines;

  const _TrashRoutinesDialog({required this.routines});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final count = routines.length;
    final reminders = routines.where((r) => r.isReminder).length;
    final question = reminders == count
        ? t.deleteNoteReminders(count)
        : reminders == 0
        ? t.deleteNoteRoutines(count)
        : t.deleteNoteRoutinesAndReminders(count);
    return AlertDialog(
      title: Text(question),
      content: Text(t.deleteNoteLinkedDetail),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.no),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.yes),
        ),
      ],
    );
  }
}

/// Asks once whether a Note goes to the trash: true to delete its
/// [linked] Routines and Reminders with it, false to delete the Note
/// only, null to cancel.
class _DeleteNoteDialog extends StatelessWidget {
  final int linked;

  const _DeleteNoteDialog({required this.linked});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    Widget destructive(String label, bool value) => TextButton(
      style: TextButton.styleFrom(
        foregroundColor: Theme.of(context).colorScheme.error,
      ),
      onPressed: () => Navigator.of(context).pop(value),
      child: Text(label),
    );
    return AlertDialog(
      title: Text(t.deleteNoteQuestion),
      content: Text(
        linked == 0 ? t.deleteNoteDetail : t.deleteNoteLinkedDetail,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        if (linked == 0)
          destructive(t.delete, false)
        else ...[
          destructive(t.deleteNoteOnly, false),
          destructive(t.deleteNoteWithLinked(linked), true),
        ],
      ],
    );
  }
}

/// Asks once whether a photo ([photo]) or voice recording goes while its
/// text stays: true to remove it, null to cancel.
class _RemoveMediaDialog extends StatelessWidget {
  final bool photo;

  const _RemoveMediaDialog({required this.photo});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(photo ? t.removePhotoQuestion : t.removeRecordingQuestion),
      content: Text(t.removeMediaDetail),
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
  const _AmendDialog();

  @override
  State<_AmendDialog> createState() => _AmendDialogState();
}

class _AmendDialogState extends State<_AmendDialog> {
  final _text = TextEditingController();

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(t.amend),
      content: TextField(
        controller: _text,
        autofocus: true,
        maxLines: null,
        textCapitalization: TextCapitalization.sentences,
        decoration: InputDecoration(hintText: t.amendHint),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(_text.text),
          child: Text(t.add),
        ),
      ],
    );
  }
}

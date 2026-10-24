import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'areas_screen.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'l10n.dart';
import 'location.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';

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

/// Everything about one Note: its text, fixed in place and saved on
/// leaving, its photos at full width, each opening full screen when
/// tapped, its Importance, its Capture Context and where it is filed,
/// its Routines and Reminders, and its Sections, each with its own
/// Capture Context. "Add to Note" amends it; "Routine" and "Reminder"
/// spawn one from it; "merge into…" merges it into another Note; the
/// trash button deletes it.
///
/// Typed text survives the app being killed: it goes to the log when the
/// app goes to the background, and a [Draft] of it is kept after every
/// [draftPause] of typing until it does. A draft newer than the Note's
/// text comes back into the field when the Note is opened again.
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
  late final _routineBook = Routines(widget.notes.store);

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
    if (note == null) return;
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
    if (note == null || setAt == null) return;
    if (draft.saved.isAfter(setAt) && draft.text.trim() != note.text) {
      if (_text.text == note.text) _text.text = draft.text;
    } else {
      unawaited(widget.drafts.delete(widget.noteId));
    }
  }

  void _load() {
    final note = widget.notes.byId(widget.noteId);
    _placed = note == null ? null : placeNote(widget.places, note);
    _sections = widget.notes.sections(widget.noteId);
    _routines = _routineBook.ofNote(widget.noteId);
  }

  void _reload() => setState(_load);

  /// Writes the text when it changed; a blank text keeps the old one.
  /// The draft goes either way: the log now holds what counts.
  void _saveText() {
    _draftTimer?.cancel();
    final note = _placed?.note;
    if (note == null) return;
    unawaited(widget.drafts.delete(note.id));
    final text = _text.text.trim();
    if (text.isEmpty || text == note.text) return;
    widget.notes.edit(note.id, text);
    _load();
  }

  void _setImportance(Set<Importance> picked) {
    _saveText();
    widget.notes.setImportance(widget.noteId, picked.single);
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
    final text = await showDialog<String>(
      context: context,
      builder: (_) => const _AmendDialog(),
    );
    final trimmed = text?.trim();
    if (!mounted || trimmed == null || trimmed.isEmpty) return;
    final section = widget.notes.amend(widget.noteId, trimmed);
    _reload();
    // The Section gets its own Capture Context when it arrives, without
    // asking.
    final here = currentIfGranted(LocationScope.of(context));
    final extras = currentExtras(widget.notes.store, ContextScope.of(context));
    for (final attached in [
      attachLocation(widget.notes, section.id, here),
      attachExtras(widget.notes, section.id, extras),
    ]) {
      unawaited(
        attached.then((attached) {
          if (attached && mounted) _reload();
        }),
      );
    }
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
    final (:note, :area, :topic) = placed;
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
        ),
        body: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Embossed(
              strong: note.importance == Importance.important,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: TextField(
                  controller: _text,
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
            ],
            const SizedBox(height: 16),
            EditorPanel(
              heading: t.importance,
              child: SegmentedButton<Importance>(
                segments: [
                  ButtonSegment(
                    value: Importance.unimportant,
                    label: Text(t.importanceUnimportant),
                  ),
                  ButtonSegment(
                    value: Importance.normal,
                    label: Text(t.importanceNormal),
                  ),
                  ButtonSegment(
                    value: Importance.important,
                    label: Text(t.importanceImportant),
                  ),
                ],
                selected: {note.importance},
                showSelectedIcon: false,
                onSelectionChanged: _setImportance,
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
                    ],
                    const SizedBox(height: 8),
                    CaptureContextLines(capture: section.context, when: when),
                  ],
                ),
              ),
            const SizedBox(height: 8),
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

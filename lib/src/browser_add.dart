import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'areas_screen.dart';
import 'capture_fan.dart';
import 'capture_screen.dart';
import 'context_platform.dart';
import 'frogs_screen.dart';
import 'l10n.dart';
import 'location.dart';
import 'note_editor_screen.dart';
import 'session_form_screen.dart';
import 'tally_form_screen.dart';

/// What the browser's "+" makes (#161): a Note, a Checklist, a Shopping
/// List, a Tally, a Session or a Frog, and in an Area a Topic.
enum NewKind { note, checklist, shopping, tally, session, frog, topic }

/// The kinds the "+" of the page showing [place] offers, null for All
/// Notes, in the order they fan out from the button: Tally, Session,
/// Frog and Topic only in Advanced (#197); Session and Frog only where
/// this Device may set them up, a Topic only with [topic].
List<NewKind> newKinds(Store store, String? place, {bool topic = false}) {
  final manages = store.mayManage(place == null ? null : store.areaOf(place));
  final advanced = store.advanced;
  return [
    NewKind.note,
    NewKind.checklist,
    NewKind.shopping,
    if (advanced) NewKind.tally,
    if (advanced && manages) ...[NewKind.session, NewKind.frog],
    if (advanced && topic && manages) NewKind.topic,
  ];
}

/// Fans [kinds] out of the "+" laid out under [button]; the kind picked,
/// or null when the fan was closed.
Future<NewKind?> pickNewKind(
  BuildContext context,
  BuildContext button,
  List<NewKind> kinds,
) async {
  final anchor = fanAnchor(context, button);
  if (anchor == null) return null;
  return Navigator.of(context).push(
    FanRoute<NewKind>(
      anchor: anchor,
      values: kinds,
      describe: _describe,
      closeKey: const ValueKey('addFanClose'),
      barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
    ),
  );
}

FanEntry _describe(BuildContext context, NewKind kind) {
  final t = context.t;
  final (icon, label) = switch (kind) {
    NewKind.note => (const Icon(Icons.edit_note), t.addFanNote),
    NewKind.checklist => (const Icon(Icons.checklist), t.checklistHeading),
    NewKind.shopping => (
      const Icon(Icons.shopping_cart_outlined),
      t.shoppingHeading,
    ),
    NewKind.tally => (const Icon(Icons.bar_chart), t.tallyHeading),
    NewKind.session => (const Icon(Icons.event_repeat), t.session),
    NewKind.frog => (const FrogIcon(), t.frog),
    NewKind.topic => (const Icon(Icons.folder_outlined), t.newTopic),
  };
  return (key: ValueKey('addFan-${kind.name}'), icon: icon, label: label);
}

/// Makes a new [kind] filed into [place], Unsorted where it is null, each
/// on its own form: the Note through Capture, in an Area or a Topic in
/// the full editor ([editNew]); a Checklist or a Shopping
/// List from its name, then its editor with the Lines typed in place; a
/// Tally or a Session on its one page; a Frog from its name. Leaving a
/// form without its Create or Save makes nothing. [onChanged] runs once
/// something was made. Not for [NewKind.topic], which the Area adds.
/// Returns the id of the Note made, null when nothing was.
Future<String?> makeNew(
  BuildContext context,
  Notes notes,
  Places places,
  NewKind kind, {
  String? place,
  required VoidCallback onChanged,
}) async {
  final t = context.t;
  final store = notes.store;
  switch (kind) {
    case NewKind.note when place != null:
      return editNew(context, notes, places, place, onChanged: onChanged);
    case NewKind.note:
      String? made;
      await captureNote(
        context,
        notes,
        place: place,
        onChanged: onChanged,
        onCaptured: (id) => made = id,
      );
      return made;
    case NewKind.checklist || NewKind.shopping:
      final checklist = kind == NewKind.checklist;
      final name = await askName(
        context,
        title: checklist ? t.newChecklist : t.newShoppingList,
      );
      if (!context.mounted || name == null) return null;
      final note = store.batch(() {
        final note = notes.capture(name, place: place);
        if (checklist) {
          Checklists(store).make(note.id);
        } else {
          ShoppingLists(store).make(note.id);
        }
        return note;
      });
      onChanged();
      await openNote(context, notes, places, note, typeLines: true, made: true);
      if (context.mounted) onChanged();
      return note.id;
    case NewKind.tally:
      final made = await Navigator.of(context).push<String>(
        MaterialPageRoute(
          builder: (_) => TallyFormScreen(notes: notes, place: place),
        ),
      );
      if (made != null) onChanged();
      return made;
    case NewKind.session:
      final made = await Navigator.of(context).push<String>(
        MaterialPageRoute(
          builder: (_) =>
              SessionFormScreen(notes: notes, places: places, place: place),
        ),
      );
      if (made == null) return null;
      onChanged();
      // The page pops the Session's Routine.
      return Routines(store).byId(made)?.noteId;
    case NewKind.frog:
      final name = await askName(context, title: t.newFrog);
      if (!context.mounted || name == null) return null;
      final note = store.batch(() {
        final note = notes.capture(name, place: place);
        Frogs(store).spawn(note.id);
        return note;
      });
      onChanged();
      return note.id;
    case NewKind.topic:
      return null;
  }
}

/// Opens the Note editor on a new Note filed into the Area or Topic
/// [place] (obf-1140): who is already there has taken the time to sort
/// and gets the full editor, not the quick note. Sorted, the Note has no
/// Nudge; its location and the rest of its Capture Context are attached
/// as they arrive, without asking. Left with nothing in it, the Note goes
/// again, as the quick note saves nothing for an empty text. [onChanged]
/// runs once the editor was left, and again when more of its Capture
/// Context arrived. Returns the id of the Note made, null when it went.
Future<String?> editNew(
  BuildContext context,
  Notes notes,
  Places places,
  String place, {
  required VoidCallback onChanged,
}) async {
  final here = currentIfGranted(LocationScope.of(context));
  final extras = currentExtras(notes.store, ContextScope.of(context));
  final note = notes.capture('', place: place);
  for (final attached in [
    attachLocation(notes, note.id, here),
    attachExtras(notes, note.id, extras),
  ]) {
    unawaited(attached.then((attached) => attached ? onChanged() : null));
  }
  await openNote(context, notes, places, note, made: true, typeText: true);
  final empty = _leftEmpty(notes, note.id);
  if (empty) notes.discard(note.id);
  onChanged();
  // Gone also where the editor poured it into a Shopping List.
  return empty || notes.byId(note.id) == null ? null : note.id;
}

/// Whether the Note [noteId] was left with nothing in it: no text, still
/// a plain Note, without photo, recording, file or Section, nothing
/// spawned from it, not in the trash and not merged.
bool _leftEmpty(Notes notes, String noteId) {
  final note = notes.byId(noteId);
  return note != null &&
      note.text.trim().isEmpty &&
      note.photos.isEmpty &&
      note.recordings.isEmpty &&
      note.attachments.isEmpty &&
      !note.spawned &&
      !note.isTrashed &&
      !note.isMerged &&
      NoteKinds(notes.store).of(noteId) == NoteKind.plain &&
      notes.sections(noteId).isEmpty &&
      notes.store.mayChange(noteId);
}

/// When the Note [made] by "+" is one [filter], the person filter of the
/// page it was made on, hides, says so at the top, "Made, hidden by the
/// filter", with "Show", whose tap runs [show] to clear the filter
/// (obf-1070). Nothing for no Note made or a page showing everyone's.
void sayIfFilterHides(
  BuildContext context,
  Store store,
  String? made,
  PersonFilter filter, {
  required VoidCallback show,
}) {
  if (made == null || store.wroteBy(made, filter)) return;
  final t = context.t;
  AnnouncementsScope.of(context)
      ?.offer(t.madeHiddenByFilter, t.showHidden, show);
}

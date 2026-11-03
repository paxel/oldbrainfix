import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'areas_screen.dart';
import 'capture_fan.dart';
import 'capture_screen.dart';
import 'frogs_screen.dart';
import 'l10n.dart';
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
    NewKind.topic => (const Icon(Icons.folder_outlined), t.topicChoice),
  };
  return (key: ValueKey('addFan-${kind.name}'), icon: icon, label: label);
}

/// Makes a new [kind] filed into [place], Unsorted where it is null, each
/// on its own form: the Note through Capture; a Checklist or a Shopping
/// List from its name, then its editor with the Lines typed in place; a
/// Tally or a Session on its one page; a Frog from its name. Leaving a
/// form without its Create or Save makes nothing. [onChanged] runs once
/// something was made. Not for [NewKind.topic], which the Area adds.
Future<void> makeNew(
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
    case NewKind.note:
      await captureNote(context, notes, place: place, onChanged: onChanged);
    case NewKind.checklist || NewKind.shopping:
      final checklist = kind == NewKind.checklist;
      final name = await askName(
        context,
        title: checklist ? t.newChecklist : t.newShoppingList,
      );
      if (!context.mounted || name == null) return;
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
      await openNote(context, notes, places, note, typeLines: true);
      if (context.mounted) onChanged();
    case NewKind.tally:
      final made = await Navigator.of(context).push<String>(
        MaterialPageRoute(
          builder: (_) => TallyFormScreen(notes: notes, place: place),
        ),
      );
      if (made != null) onChanged();
    case NewKind.session:
      final made = await Navigator.of(context).push<String>(
        MaterialPageRoute(
          builder: (_) =>
              SessionFormScreen(notes: notes, places: places, place: place),
        ),
      );
      if (made != null) onChanged();
    case NewKind.frog:
      final name = await askName(context, title: t.newFrog);
      if (!context.mounted || name == null) return;
      store.batch(() {
        final note = notes.capture(name, place: place);
        Frogs(store).spawn(note.id);
      });
      onChanged();
    case NewKind.topic:
      return;
  }
}

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'place_names.dart';

// The mark Hidden from AI on the phone (#267, ADR 0049): what a card,
// the Note editor and an Area's menu show of it, and the question the
// ways of giving something to other people ask when it is in the way.

/// The icon of the mark, on a card, an Area's row and the editor's badge.
const hiddenFromAiIcon = Icons.shield_outlined;

/// The switch of the mark of the Note or Area [id] as a menu or badge
/// offers it, read as it opens: on while it is marked itself, whose tap
/// takes the mark off, which is never refused; off while it may be marked,
/// as in no Shared Area; null when it is neither, so it is not offered.
/// [changed] runs after the tap.
({bool on, VoidCallback toggle})? hiddenFromAiEntry(
  Store store,
  String id,
  VoidCallback changed,
) {
  final on = store.isMarkedHiddenFromAi(id);
  if (!on && !store.mayHideFromAi(id)) return null;
  return (
    on: on,
    // Read again on each tap, as a switch may be tapped twice.
    toggle: () {
      final now = store.isMarkedHiddenFromAi(id);
      // A sync since it was read may have shared it.
      if (!now && !store.mayHideFromAi(id)) return;
      store.setHiddenFromAi(id, !now);
      changed();
    },
  );
}

/// What the marks [marks] stand for, as the question names them: a Note
/// by its first line, an Area by its name; one that is gone by nothing.
String hiddenFromAiNames(AppLocalizations t, Store store, List<String> marks) {
  final notes = Notes(store);
  final places = Places(store);
  final names = [
    for (final id in marks)
      if (places.areaById(id) case final area?)
        areaName(t, area)
      else if (notes.byId(id) case final note?)
        noteText(t, note).trim().split('\n').first.trim(),
  ];
  return namesText(t, names);
}

/// Asks once, as something Hidden from AI is about to be given to other
/// people (filed into a Shared Area, shared, sent or put into a Pack),
/// whether to show it to the AI first, naming the [marks] in the way;
/// "Show to the AI" takes them off. Whether they were taken off: the
/// caller then gives it, as the user asked; Cancel gives nothing.
///
/// Only the marks of what is given come off here: the Notes', and the
/// Area's when it is the Area [area] being shared or packed. An Area's
/// mark in the way of a Note of it is only named, with where it comes
/// off, so one tap never shows every other Note of that Area to the AI.
Future<bool> showToAiFirst(
  BuildContext context,
  Store store,
  List<String> marks, {
  String? area,
}) async {
  if (marks.isEmpty) return true;
  final t = context.t;
  final kept = [
    for (final mark in marks)
      if (!mark.startsWith('note:') && mark != area) mark,
  ];
  if (kept.isNotEmpty) {
    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        key: const ValueKey('hidden-from-ai-area-first'),
        icon: const Icon(hiddenFromAiIcon),
        title: Text(t.hiddenFromAiRefusedTitle),
        content: Text(
          t.hiddenFromAiAreaFirst(hiddenFromAiNames(t, store, kept)),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(t.close),
          ),
        ],
      ),
    );
    return false;
  }
  final names = hiddenFromAiNames(t, store, marks);
  final show = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      key: const ValueKey('hidden-from-ai-refused'),
      icon: const Icon(hiddenFromAiIcon),
      title: Text(t.hiddenFromAiRefusedTitle),
      content: Text(t.hiddenFromAiRefusedBody(names)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        TextButton(
          key: const ValueKey('hidden-from-ai-show'),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.hiddenFromAiShow),
        ),
      ],
    ),
  );
  if (show != true) return false;
  for (final mark in marks) {
    store.setHiddenFromAi(mark, false);
  }
  return true;
}

/// Runs [give], a way of giving something to other people, and when the
/// core refuses it as Hidden from AI ([HiddenFromAiRefused]) asks
/// [showToAiFirst] and, once the marks are off, runs it again: at most as
/// often as the user says yes, each time with the marks then in the way.
/// Its result, or null when nothing was given.
/// [area] is the Area being given, as for [showToAiFirst].
Future<T?> unlessHiddenFromAi<T>(
  BuildContext context,
  Store store,
  T Function() give, {
  String? area,
}) async {
  while (true) {
    try {
      return give();
    } on HiddenFromAiRefused catch (refused) {
      if (!context.mounted) return null;
      if (!await showToAiFirst(context, store, refused.marks, area: area)) {
        return null;
      }
      if (!context.mounted) return null;
    }
  }
}

/// The menu entry of the mark, "Hidden from AI" with its switch below
/// it, [on] while marked; picking it gives [value].
PopupMenuItem<T> hiddenFromAiMenuItem<T>(
  BuildContext context, {
  required Key key,
  required T value,
  required bool on,
}) => PopupMenuItem(
  key: key,
  value: value,
  child: Semantics(
    toggled: on,
    child: Row(
      children: [
        const Icon(hiddenFromAiIcon),
        const SizedBox(width: 12),
        Flexible(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(context.t.hiddenFromAi),
              ExcludeSemantics(
                child: IgnorePointer(
                  child: Switch(value: on, onChanged: (_) {}),
                ),
              ),
            ],
          ),
        ),
      ],
    ),
  ),
);

/// The mark on an Area's row or a card: its icon, read by a screen
/// reader as "Hidden from AI".
class HiddenFromAiMark extends StatelessWidget {
  final double? size;

  const HiddenFromAiMark({super.key, this.size});

  @override
  Widget build(BuildContext context) => Icon(
    hiddenFromAiIcon,
    size: size,
    color: Theme.of(context).colorScheme.onSurfaceVariant,
    semanticLabel: context.t.hiddenFromAi,
  );
}

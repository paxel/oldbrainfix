import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_links.dart';

/// The Unsorted Notes a list shows, in its order, read anew at each call:
/// what the Note editor opened from that list moves on to once its Note
/// is sorted (#313).
typedef UnsortedShown = List<Note> Function();

/// The Note to open once [sorted] left Unsorted (#313): the next one
/// after it in [before], the list's order as the step began, that [now]
/// still shows, else the first [now] shows; none when it shows none.
Note? nextUnsorted(List<String> before, List<Note> now, String sorted) {
  final still = {for (final note in now) note.id: note};
  for (final id in before.skip(before.indexOf(sorted) + 1)) {
    if (still[id] case final note?) return note;
  }
  return now.firstOrNull;
}

/// Whether a step on the cards [shown] sorted the last Unsorted Note
/// (#313): one of them is now filed, merged or set up as something, not
/// in the trash, and nothing is Unsorted any more.
bool sortedTheLast(Notes notes, Iterable<String> shown) {
  if (notes.unsortedCount() > 0) return false;
  return shown.any((id) {
    final note = notes.byId(id);
    return note != null && !note.isTrashed && !note.isUnsorted;
  });
}

/// Shows the Agenda once Unsorted is empty (#313), as an app link to it
/// does: every page over it goes.
void showAgendaSorted(BuildContext context) {
  final open = AppLinkScope.of(context);
  if (open == null) {
    Navigator.of(context).popUntil((route) => route.isFirst);
    return;
  }
  unawaited(open(context, const AppLink(AppLinkKind.agenda)));
}

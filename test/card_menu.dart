import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// The group of a card's ⋮ that holds the entry keyed [entry]: This Note,
/// Add, Share or Tidy, as the Note editor's groups (obf-1014).
String cardGroupOf(String entry) => switch (entry) {
  'note-file' ||
  'note-importance' ||
  'note-show-ticks' ||
  'note-hidden-from-ai' ||
  'note-remove-pack-from' ||
  'note-add-to-home' ||
  'note-make-tally' ||
  'note-make-checklist' ||
  'note-make-shopping' ||
  'note-show-on-agenda' ||
  'note-to-plain' => 'note-group-this-note',
  _ when entry.startsWith('importance-') => 'note-group-this-note',
  _ when entry.startsWith('note-show-on-agenda-') => 'note-group-this-note',
  'note-amend' || 'note-add-entry' => 'note-group-add',
  _ when entry.startsWith('note-spawn-') => 'note-group-add',
  'note-send' ||
  'note-share' ||
  'note-copy' ||
  'note-pack' ||
  'note-calendar' ||
  'note-calendar-file' => 'note-group-share',
  // Merge into…, adding to a Shopping List, Uncheck all, Move to trash
  // and each removal of a photo, recording or file.
  _ => 'note-group-tidy',
};

/// With a card's ⋮ open, opens the group that holds [entry], and for a
/// choice of the Importance or Show on the Agenda its menu too.
Future<void> openCardGroup(WidgetTester tester, String entry) async {
  await tester.tap(find.byKey(ValueKey(cardGroupOf(entry))));
  await tester.pumpAndSettle();
  final choice = entry.startsWith('importance-')
      ? 'note-importance'
      : entry.startsWith('note-show-on-agenda-')
      ? 'note-show-on-agenda'
      : null;
  if (choice == null) return;
  await tester.tap(find.byKey(ValueKey(choice)));
  await tester.pumpAndSettle();
}

/// Opens a card's ⋮ [menu], the only one on screen by default, and the
/// group that holds [entry].
Future<void> openCardMenu(
  WidgetTester tester,
  String entry, {
  Finder? menu,
}) async {
  await tester.tap(menu ?? find.byKey(const ValueKey('note-menu')));
  await tester.pumpAndSettle();
  await openCardGroup(tester, entry);
}

/// Taps [entry] in its group of a card's ⋮ [menu].
Future<void> tapCardAction(
  WidgetTester tester,
  String entry, {
  Finder? menu,
}) async {
  await openCardMenu(tester, entry, menu: menu);
  await tester.tap(find.byKey(ValueKey(entry)));
  await tester.pumpAndSettle();
}

/// Closes the menu open, as a tap beside it does.
Future<void> closeMenu(WidgetTester tester) async {
  await tester.tapAt(Offset.zero);
  await tester.pumpAndSettle();
}

/// How many entries the menu open shows, headings not counted.
int menuEntries(WidgetTester tester) => find
    .byWidgetPredicate((widget) => widget is PopupMenuItem && widget.enabled)
    .evaluate()
    .length;

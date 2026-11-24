import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// The groups of a table column's menu that may hold [op] (obf-1014):
/// Insert, Move, Align, and Sort rows on the editor's grid or Show sorted
/// full screen.
List<String> tableGroupsOf(String op) => switch (op) {
  'table-before' || 'table-after' => const ['table-group-insert'],
  'table-back' || 'table-forward' => const ['table-group-move'],
  'table-alignLeft' ||
  'table-alignCenter' ||
  'table-alignRight' => const ['table-group-align'],
  'table-sortUp' ||
  'table-sortDown' => const ['table-group-sort', 'table-group-sorted'],
  'table-asWritten' => const ['table-group-sorted'],
  _ => const [],
};

/// With a row's or column's menu open, opens the group that holds [op]
/// unless the menu shows it itself, as a row's does.
Future<void> openTableGroup(WidgetTester tester, String op) async {
  if (find.byKey(ValueKey(op)).evaluate().isNotEmpty) return;
  for (final group in tableGroupsOf(op)) {
    final entry = find.byKey(ValueKey(group));
    if (entry.evaluate().isEmpty) continue;
    await tester.tap(entry);
    await tester.pumpAndSettle();
    return;
  }
}

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// The Note editor's "Add a part" (obf-1072), on its bar or in its ⋮.
final addAPartAction = find.byKey(const ValueKey('editor-add-part'));

/// Opens the Amend box as the user does: "Add a part" on the Note
/// editor's bar, or in its ⋮ where the bar is full.
Future<void> addAPart(WidgetTester tester) async {
  if (addAPartAction.evaluate().isEmpty) {
    await tester.tap(find.byKey(const ValueKey('bar-more')));
    await tester.pumpAndSettle();
  }
  await tester.tap(addAPartAction);
  await tester.pumpAndSettle();
}

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'editor_actions.dart';

/// The Note editor's "Add a part" (obf-1072), in its Add group.
final addAPartAction = find.byKey(const ValueKey('editor-add-part'));

/// Add in the Amend box, not the heading of the editor's Add group.
final amendAdd = find.descendant(
  of: find.byType(AlertDialog),
  matching: find.text('Add'),
);

/// Opens the Amend box as the user does: "Add a part" in the Note
/// editor's Add group at the page's end (obf-1073).
Future<void> addAPart(WidgetTester tester) =>
    tapEditorAction(tester, addAPartAction);

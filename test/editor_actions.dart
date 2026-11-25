import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';

/// Scrolls the Note editor's page until [finder] is built and on screen,
/// as its groups Add, Share and Tidy at the page's end (obf-1073); the
/// page's list, not the find bar's field above it.
Future<void> revealInEditor(WidgetTester tester, Finder finder) async {
  await tester.scrollUntilVisible(
    finder,
    100,
    scrollable: find
        .descendant(
          of: find.descendant(
            of: find.byType(NoteEditorScreen),
            matching: find.byType(ListView),
          ),
          matching: find.byType(Scrollable),
        )
        .first,
  );
  // Whole, not only its top edge.
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
}

/// Taps the Note editor's action [finder] in its groups at the page's
/// end (obf-1073), scrolled into view first.
Future<void> tapEditorAction(WidgetTester tester, Finder finder) async {
  await revealInEditor(tester, finder);
  await tester.tap(finder);
  await tester.pumpAndSettle();
}

/// Taps the bar action [finder], from ⋮ where the bar is full.
Future<void> tapBarAction(WidgetTester tester, Finder finder) async {
  if (finder.evaluate().isEmpty) {
    await tester.tap(find.byKey(const ValueKey('bar-more')));
    await tester.pumpAndSettle();
  }
  await tester.tap(finder);
  await tester.pumpAndSettle();
}

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// Expects the row [row] that opens a page to read as tappable (UI laws,
/// "Design system", obf-1108): its [words] and icon in the accent, a
/// chevron at its end.
void expectAccentRow(WidgetTester tester, Finder row, String words) {
  final accent = Theme.of(tester.element(row)).colorScheme.primary;
  expect(
    tester
        .widget<Text>(find.descendant(of: row, matching: find.text(words)))
        .style
        ?.color,
    accent,
  );
  expect(tester.widget<ListTile>(row).iconColor, accent);
  expect(
    find.descendant(of: row, matching: find.byIcon(Icons.chevron_right)),
    findsOneWidget,
  );
}

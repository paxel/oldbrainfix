import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// The choice chip labelled [text] (#226).
Finder chip(String text) => find.widgetWithText(ChoiceChip, text);

/// Whether the choice chip labelled [text] is the selected one.
bool chipPicked(WidgetTester tester, String text) =>
    tester.widget<ChoiceChip>(chip(text)).selected;

/// The labels of the choice chips under [of], in their order.
List<String> chipLabels(WidgetTester tester, Finder of) => [
  for (final c in tester.widgetList<ChoiceChip>(
    find.descendant(of: of, matching: find.byType(ChoiceChip)),
  ))
    (c.label as Text).data!,
];

/// The label of the selected choice chip under [of].
String pickedChip(WidgetTester tester, Finder of) =>
    (tester
                .widgetList<ChoiceChip>(
                  find.descendant(of: of, matching: find.byType(ChoiceChip)),
                )
                .singleWhere((c) => c.selected)
                .label
            as Text)
        .data!;

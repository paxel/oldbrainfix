import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// The fan's entry for [way]: text, voice, photo or gallery.
Finder fanEntry(String way) => find.byKey(ValueKey('captureFan-$way'));

/// Opens text Capture as the user does: "+" on the Agenda, or on the lock
/// screen, then Text in the fan it opens.
Future<void> openTextCapture(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.add));
  await tester.pumpAndSettle();
  await tester.tap(fanEntry('text'));
  await tester.pumpAndSettle();
}

/// Opens Capture by [way] in the Agenda's fan: voice, photo, gallery,
/// paste or file, the ways in that bring something (#352).
Future<void> openCaptureBy(WidgetTester tester, String way) async {
  await tester.tap(find.byIcon(Icons.add));
  await tester.pumpAndSettle();
  await tester.tap(fanEntry(way));
  await tester.pumpAndSettle();
}

/// Leaves the quick note by its 1 h Nudge button, which saves as Back
/// does and as its Save on top did before (#352).
Future<void> saveCapture(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey('nudge60')));
  await tester.pumpAndSettle();
}

/// Leaves the quick note by Android's Back, which saves (#352); it has
/// no back arrow to tap.
Future<void> backFromCapture(WidgetTester tester) async {
  await tester.binding.handlePopRoute();
  await tester.pumpAndSettle();
}

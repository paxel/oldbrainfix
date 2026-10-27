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

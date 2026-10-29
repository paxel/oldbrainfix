import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';

/// Expects the visible announcement to be [announced] alone, taps it as
/// its strip does and expects the failure dialog (UI laws,
/// "Announcements") to say [cause] and [fix], under [title] when given,
/// with the full text, Close and Report; closes it.
Future<void> expectFailureAnnounced(
  WidgetTester tester,
  Announcements announcements,
  String announced, {
  String? title,
  required String cause,
  required String fix,
}) async {
  expect(announcements.visible, announced);
  await tapVisible(tester, announcements);
  if (title != null) expect(find.text(title), findsOneWidget);
  expect(find.text('$cause $fix'), findsOneWidget);
  expect(find.byKey(const ValueKey('failureDetails')), findsOneWidget);
  expect(find.text('Report'), findsOneWidget);
  await tester.tap(find.text('Close'));
  await tester.pumpAndSettle();
  expect(find.byType(AlertDialog), findsNothing);
}

/// Expects the visible announcement to be [announced] alone, taps it as
/// its strip does and expects its dialog to hold [details]; closes it.
Future<void> expectDoneAnnounced(
  WidgetTester tester,
  Announcements announcements,
  String announced,
  String details,
) async {
  expect(announcements.visible, announced);
  await tapVisible(tester, announcements);
  expect(find.textContaining(details), findsOneWidget);
  await tester.tap(find.text('Close'));
  await tester.pumpAndSettle();
  expect(find.byType(AlertDialog), findsNothing);
}

/// Taps the visible announcement as its strip does: it goes, and what
/// it is about opens.
Future<void> tapVisible(
  WidgetTester tester,
  Announcements announcements,
) async {
  final action = announcements.visibleAction;
  announcements.dismiss();
  action?.call();
  await tester.pumpAndSettle();
}

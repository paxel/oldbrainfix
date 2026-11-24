import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// The Profile picture in the Agenda's top corner, where the settings
/// gear was.
final profileButton = find.byKey(const ValueKey('profile'));

/// Opens the Profile page as the user does, from the Agenda.
Future<void> openProfilePage(WidgetTester tester) async {
  await tester.tap(profileButton);
  await tester.pumpAndSettle();
}

/// Opens this Device's settings as the user does: the Profile picture in
/// the Agenda's corner, then Settings on the Profile page.
Future<void> openDeviceSettings(WidgetTester tester) async {
  await openProfilePage(tester);
  final entry = find.byKey(const ValueKey('deviceSettings'));
  await tester.ensureVisible(entry);
  await tester.pumpAndSettle();
  await tester.tap(entry);
  await tester.pumpAndSettle();
}

/// Opens the page of the Imports as the user does (obf-1096): the Areas
/// list from the Agenda, then ⋮ ▸ Import… in its top bar.
Future<void> openImports(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey('browse')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('areas-more')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('areas-import')));
  await tester.pumpAndSettle();
}

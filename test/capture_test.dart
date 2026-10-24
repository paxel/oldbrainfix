import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  testWidgets('+ captures a text Note that shows in the list', (tester) async {
    final store = Store.inMemory()..starterSetOffered = true;
    addTearDown(store.close);
    await tester.pumpWidget(OldBrainFixApp(store: store));

    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'nees mor sauce');
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(find.text('nees mor sauce'), findsOneWidget);
    expect(Notes(store).all().single.text, 'nees mor sauce');
  });

  testWidgets('saving empty text captures nothing', (tester) async {
    final store = Store.inMemory()..starterSetOffered = true;
    addTearDown(store.close);
    await tester.pumpWidget(OldBrainFixApp(store: store));

    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(Notes(store).all(), isEmpty);
  });

  testWidgets('the app speaks German', (tester) async {
    final store = Store.inMemory()..starterSetOffered = true;
    addTearDown(store.close);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();

    expect(find.textContaining('Heute steht nichts an'), findsOneWidget);
  });
}

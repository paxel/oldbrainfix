import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Height of the fake status bar, in logical pixels.
const statusBar = 24.0;

void main() {
  late Announcements announcements;

  Future<void> pumpApp(WidgetTester tester) async {
    tester.view.padding = FakeViewPadding(
      top: statusBar * tester.view.devicePixelRatio,
    );
    addTearDown(tester.view.reset);
    final store = Store.inMemory();
    addTearDown(store.close);
    announcements = Announcements();
    addTearDown(announcements.dispose);
    await tester.pumpWidget(
      OldBrainFixApp(store: store, announcements: announcements),
    );
  }

  Rect strip(WidgetTester tester, String text) => tester.getRect(
    find.ancestor(of: find.text(text), matching: find.byType(Material)).first,
  );

  testWidgets('shows at the top under the status bar, pushing the page down', (
    tester,
  ) async {
    await pumpApp(tester);
    final barBefore = tester.getRect(find.byType(AppBar));
    expect(barBefore.top, 0);

    announcements.announce('Saved');
    await tester.pumpAndSettle();

    final line = strip(tester, 'Saved');
    expect(line.top, 0);
    expect(
      tester.getRect(find.text('Saved')).top,
      greaterThanOrEqualTo(statusBar),
    );
    final bar = tester.getRect(find.byType(AppBar));
    expect(bar.top, line.bottom, reason: 'the page sits right below it');
    expect(
      bar.height,
      barBefore.height - statusBar,
      reason: 'the page does not pad for the status bar a second time',
    );
  });

  testWidgets('one at a time; a tap shows the next', (tester) async {
    await pumpApp(tester);
    announcements
      ..announce('first')
      ..announce('second');
    await tester.pumpAndSettle();
    expect(find.text('first'), findsOneWidget);
    expect(find.text('second'), findsNothing);

    await tester.tap(find.text('first'));
    await tester.pumpAndSettle();
    expect(find.text('first'), findsNothing);
    expect(find.text('second'), findsOneWidget);

    await tester.tap(find.text('second'));
    await tester.pumpAndSettle();
    expect(find.text('second'), findsNothing);
    expect(tester.getRect(find.byType(AppBar)).top, 0);
  });

  testWidgets('never as a snackbar or from the bottom', (tester) async {
    await pumpApp(tester);
    announcements.announce('Saved');
    await tester.pumpAndSettle();
    expect(find.byType(SnackBar), findsNothing);
    expect(find.byType(BottomSheet), findsNothing);
    expect(
      tester.getRect(find.text('Saved')).bottom,
      lessThan(
        tester.view.physicalSize.height / tester.view.devicePixelRatio / 4,
      ),
    );
  });
}

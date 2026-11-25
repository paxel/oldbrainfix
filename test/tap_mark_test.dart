import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/tap_mark.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_settings.dart';

void main() {
  final mark = find.byKey(const ValueKey('tapMark'));

  /// A page with a text, an outlined and an icon button, a card's tap
  /// and a button that opens a dialog with a button, under the look's
  /// theme marking the taps into [marks].
  Widget page(TapMarks marks) => MaterialApp(
    theme: lookTheme(
      Brightness.light,
      tapMarks: TapMarkSplashFactory(marks.show),
    ),
    builder: (context, child) =>
        TapMarkLayer(marks: marks, child: child ?? const SizedBox()),
    home: Builder(
      builder: (context) => Scaffold(
        appBar: AppBar(
          actions: [IconButton(onPressed: () {}, icon: const Icon(Icons.redo))],
        ),
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextButton(onPressed: () {}, child: const Text('text')),
              OutlinedButton(onPressed: () {}, child: const Text('outlined')),
              IconButton(onPressed: () {}, icon: const Icon(Icons.undo)),
              InkWell(onTap: () {}, child: const Text('card')),
              FilledButton(
                onPressed: () => showDialog<void>(
                  context: context,
                  builder: (context) => AlertDialog(
                    actions: [
                      TextButton(onPressed: () {}, child: const Text('in')),
                    ],
                  ),
                ),
                child: const Text('dialog'),
              ),
            ],
          ),
        ),
        floatingActionButton: TapMarked(
          child: FloatingActionButton(
            onPressed: () {},
            child: const Icon(Icons.add),
          ),
        ),
      ),
    ),
  );

  testWidgets('a tap on each kind of button shows a green "!" above the '
      'finger that grows, fades and goes after 0.9 s', (tester) async {
    final marks = TapMarks();
    addTearDown(marks.dispose);
    await tester.pumpWidget(page(marks));
    for (final button in [
      find.text('text'),
      find.text('outlined'),
      find.byIcon(Icons.undo),
      find.byIcon(Icons.add),
    ]) {
      final at = tester.getCenter(button);
      await tester.tap(button);
      await tester.pump();
      expect(mark, findsOneWidget, reason: '$button');
      final icon = tester.widget<Icon>(mark);
      expect(icon.icon, Icons.priority_high);
      expect(icon.size, 56);
      expect(icon.color, Colors.green.shade600);
      expect(tester.getCenter(mark).dy, lessThan(at.dy - 40));
      expect((tester.getCenter(mark).dx - at.dx).abs(), lessThan(1));
      final fade = tester.widget<FadeTransition>(
        find.ancestor(of: mark, matching: find.byType(FadeTransition)),
      );
      await tester.pump(const Duration(milliseconds: 450));
      expect(fade.opacity.value, inExclusiveRange(0, 1));
      await tester.pump(const Duration(milliseconds: 500));
      expect(mark, findsNothing);
    }
  }, variant: TargetPlatformVariant.only(TargetPlatform.android));

  testWidgets('on the iPhone too, with its own ink', (tester) async {
    final marks = TapMarks();
    addTearDown(marks.dispose);
    await tester.pumpWidget(page(marks));
    await tester.tap(find.text('outlined'));
    await tester.pump();
    expect(mark, findsOneWidget);
    await tester.pumpAndSettle();
    expect(mark, findsNothing);
  }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));

  testWidgets('above a button of the top bar it stays on the screen', (
    tester,
  ) async {
    final marks = TapMarks();
    addTearDown(marks.dispose);
    await tester.pumpWidget(page(marks));
    await tester.tap(find.byIcon(Icons.redo));
    await tester.pump();
    final at = tester.getRect(mark);
    expect(at.top, greaterThanOrEqualTo(0));
    expect(
      at.right,
      lessThanOrEqualTo(tester.getSize(find.byType(Scaffold)).width),
    );
    await tester.pumpAndSettle();
  });

  testWidgets('a tap on a card, no button, shows none', (tester) async {
    final marks = TapMarks();
    addTearDown(marks.dispose);
    await tester.pumpWidget(page(marks));
    await tester.tap(find.text('card'));
    await tester.pump();
    expect(mark, findsNothing);
  });

  testWidgets('shows over a dialog', (tester) async {
    final marks = TapMarks();
    addTearDown(marks.dispose);
    await tester.pumpWidget(page(marks));
    await tester.tap(find.text('dialog'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('in'));
    await tester.pump();
    expect(mark, findsOneWidget);
    // The layer holds the app's navigator, the dialog's route too.
    expect(
      find.descendant(
        of: find.byType(TapMarkLayer),
        matching: find.byType(AlertDialog),
      ),
      findsOneWidget,
    );
    await tester.pumpAndSettle();
    expect(mark, findsNothing);
  });

  testWidgets('with animations removed it stands still and goes after '
      '0.9 s', (tester) async {
    final marks = TapMarks();
    addTearDown(marks.dispose);
    // The system's "remove animations".
    tester.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures(disableAnimations: true);
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
    await tester.pumpWidget(page(marks));
    await tester.tap(find.text('text'));
    await tester.pump();
    expect(mark, findsOneWidget);
    expect(
      find.ancestor(of: mark, matching: find.byType(FadeTransition)),
      findsNothing,
    );
    expect(
      find.ancestor(of: mark, matching: find.byType(ScaleTransition)),
      findsNothing,
    );
    // Not cut short as other animations are then.
    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump(const Duration(milliseconds: 300));
    expect(mark, findsOneWidget);
    await tester.pump(const Duration(milliseconds: 200));
    await tester.pump();
    expect(mark, findsNothing);
  });

  group('the app', () {
    late Store store;

    setUp(() => store = Store.inMemory()..starterSetOffered = true);
    tearDown(() => store.close());

    testWidgets('marks the Agenda\'s "+" from the start; Settings ▸ Look '
        'switches it off for this Device', (tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store, tapMarks: true));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.add));
      await tester.pump();
      expect(mark, findsOneWidget);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('captureFanClose')));
      await tester.pumpAndSettle();

      await openDeviceSettings(tester);
      final toggle = find.byKey(const ValueKey('tapMarks'));
      await tester.ensureVisible(toggle);
      await tester.pumpAndSettle();
      expect(find.text('Show button taps'), findsOneWidget);
      expect(tester.widget<Switch>(toggle).value, isTrue);
      await tester.tap(toggle);
      await tester.pumpAndSettle();
      expect(tester.widget<Switch>(toggle).value, isFalse);
      expect(store.localSetting(tapMarksOffKey), 'true');
      expect(mark, findsNothing);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.add));
      await tester.pump();
      expect(mark, findsNothing);
      await tester.pumpAndSettle();
    });

    testWidgets('shows none where it was not started with them, as in tests '
        'and screenshots', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.add));
      await tester.pump();
      expect(mark, findsNothing);
      await tester.pumpAndSettle();
    });
  });
}

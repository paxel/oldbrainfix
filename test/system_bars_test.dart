import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/trash_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';

void main() {
  // The three-button navigation bar of a Pixel: 48 logical pixels.
  const bar = 48.0;
  const height = 800.0;

  late Store store;

  setUp(() => store = Store.inMemory()..starterSetOffered = true);
  tearDown(() => store.close());

  /// A phone of [height] whose bottom [bar] belongs to the system.
  void phoneWithNavigationBar(WidgetTester tester) {
    final ratio = tester.view.devicePixelRatio;
    tester.view.physicalSize = Size(400 * ratio, height * ratio);
    final inset = FakeViewPadding(bottom: bar * ratio);
    tester.view.padding = inset;
    tester.view.viewPadding = inset;
    addTearDown(tester.view.reset);
  }

  Future<void> show(WidgetTester tester, Widget screen) async {
    final lock = AppLock(store: store);
    addTearDown(lock.dispose);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: LockScope(lock: lock, child: screen),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Scrolls the page's list to its very end.
  Future<void> scrollToEnd(WidgetTester tester) async {
    await tester.fling(
      find.byType(Scrollable).first,
      const Offset(0, -20000),
      5000,
    );
    await tester.pumpAndSettle();
  }

  void expectAboveBar(WidgetTester tester, Finder last) {
    final rect = tester.getRect(last);
    expect(rect.bottom, lessThanOrEqualTo(height - bar));
  }

  testWidgets('the Starter Set scrolls its last entry above the bar', (
    tester,
  ) async {
    phoneWithNavigationBar(tester);
    await show(tester, StarterSetScreen(onDone: (_) {}));
    await scrollToEnd(tester);

    expectAboveBar(tester, find.byType(CheckboxListTile).last);
    expect(find.text('Medication'), findsOneWidget);
    expectAboveBar(tester, find.text('Medication'));
  });

  testWidgets('the trash scrolls its last Note above the bar', (tester) async {
    phoneWithNavigationBar(tester);
    final notes = Notes(store);
    for (var i = 0; i < 30; i++) {
      notes.trash(notes.capture('Note $i').id);
    }
    await show(tester, TrashScreen(notes: notes, places: Places(store)));
    await scrollToEnd(tester);

    final last = find.byType(NoteCard).last;
    expect(last, findsOneWidget);
    expectAboveBar(tester, last);
  });

  testWidgets('the capture screen keeps its remind me line above the bar', (
    tester,
  ) async {
    phoneWithNavigationBar(tester);
    await show(
      tester,
      CaptureScreen(
        notes: Notes(store),
        photos: FakePhotos(),
        voice: FakeVoice(),
      ),
    );

    for (final key in [
      const ValueKey('capturePhoto'),
      const ValueKey('captureVoice'),
      for (final (_, key) in NudgeButtons.choices) key,
    ]) {
      expectAboveBar(tester, find.byKey(key));
    }
  });
}

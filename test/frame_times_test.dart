import 'dart:ui' show FrameTiming;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/about_screen.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/frame_times.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:package_info_plus/package_info_plus.dart';

void main() {
  late Store store;

  setUp(() => store = Store.inMemory());
  tearDown(() => store.close());

  /// A frame that started at [start] ms and took [build] and [raster] ms.
  FrameTiming frame(int start, double build, double raster) {
    final at = start * 1000;
    final built = at + (build * 1000).round();
    return FrameTiming(
      vsyncStart: at,
      buildStart: at,
      buildFinish: built,
      rasterStart: built,
      rasterFinish: built + (raster * 1000).round(),
      rasterFinishWallTime: built + (raster * 1000).round(),
    );
  }

  final overlay = find.byKey(const ValueKey('frameTimes'));
  final version = find.byKey(const ValueKey('version'));

  Future<({FrameTimesSwitch frameTimes, Announcements announcements})> about(
    WidgetTester tester, {
    Locale? locale,
  }) async {
    final frameTimes = FrameTimesSwitch(store);
    final announcements = Announcements();
    addTearDown(frameTimes.dispose);
    addTearDown(announcements.dispose);
    await tester.pumpWidget(
      MaterialApp(
        locale: locale,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => FrameTimesScope(
          frameTimes: frameTimes,
          child: FrameTimesOverlay(
            frameTimes: frameTimes,
            child: AnnouncementsScope(
              announcements: announcements,
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        ),
        home: AboutScreen(
          version: Future.value(
            PackageInfo(
              appName: 'Old Brain Fix',
              packageName: 'oldbrainfix',
              version: '0.5.0',
              buildNumber: '1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return (frameTimes: frameTimes, announcements: announcements);
  }

  Future<void> tapVersion(WidgetTester tester, int times) async {
    for (var i = 0; i < times; i++) {
      await tester.tap(version);
      await tester.pump();
    }
  }

  testWidgets('seven taps on the version switch the frame times on, seven '
      'more off, kept for this Device', (tester) async {
    final shown = await about(tester);
    expect(overlay, findsNothing, reason: 'off by default');

    await tapVersion(tester, 6);
    expect(overlay, findsNothing);
    expect(store.localSetting('frameTimes'), isNull);

    await tapVersion(tester, 1);
    expect(overlay, findsOneWidget);
    expect(shown.announcements.visible, 'Frame times on');
    expect(store.localSetting('frameTimes'), 'true');
    // Kept: a new start shows them again.
    expect(FrameTimesSwitch(store).value, isTrue);

    await tapVersion(tester, 7);
    expect(overlay, findsNothing);
    expect(store.localSetting('frameTimes'), isNull);
    expect(FrameTimesSwitch(store).value, isFalse);
  });

  testWidgets('says so in German', (tester) async {
    final shown = await about(tester, locale: const Locale('de'));
    await tapVersion(tester, 7);
    expect(shown.announcements.visible, 'Bildzeiten an');
    await tapVersion(tester, 7);
    expect(shown.announcements.visible, 'Bildzeiten an');
    shown.announcements.dismiss();
    expect(shown.announcements.visible, 'Bildzeiten aus');
  });

  testWidgets('shows the times the engine reports', (tester) async {
    store.setLocalSetting('frameTimes', 'true');
    await about(tester);
    expect(overlay, findsOneWidget);

    tester.binding.platformDispatcher.onReportTimings?.call([
      frame(0, 2, 3),
      frame(16, 9.5, 12.25),
      frame(32, 1, 1),
    ]);
    await tester.pump();
    expect(
      find.text('Last frames: build 9.5 ms, raster 12.3 ms'),
      findsOneWidget,
    );
    expect(find.text('Slowest in the last second: 21.8 ms'), findsOneWidget);
  });

  group('the window', () {
    test('shows the slowest of the last frames and of the last second, at '
        'most every half second', () {
      final window = FrameTimesWindow();
      expect(window.add([frame(0, 4, 6), frame(16, 8, 2)]), isTrue);
      expect(window.lastBuild, const Duration(milliseconds: 8));
      expect(window.lastRaster, const Duration(milliseconds: 6));
      expect(window.worst, const Duration(milliseconds: 10));

      // Within half a second nothing changes on screen.
      expect(window.add([frame(100, 30, 30)]), isFalse);
      expect(window.worst, const Duration(milliseconds: 10));

      // Then the slowest since, and of the last second.
      expect(window.add([frame(600, 1, 1)]), isTrue);
      expect(window.lastBuild, const Duration(milliseconds: 30));
      expect(window.worst, const Duration(milliseconds: 60));

      // A second on, the slow frame is out of the window.
      expect(window.add([frame(1200, 1, 2)]), isTrue);
      expect(window.lastBuild, const Duration(milliseconds: 1));
      expect(window.lastRaster, const Duration(milliseconds: 2));
      expect(window.worst, const Duration(milliseconds: 3));
    });
  });
}

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/crash_guard.dart';

/// A crash report nobody can trace to a build is worthless: every
/// report names the version, the system and when it happened.
void main() {
  test('the report header carries build, system, locale and time', () {
    final header = crashReportHeader(at: DateTime.utc(2026, 8, 20, 12));
    final lines = header.split('\n');
    expect(lines, hasLength(4));
    expect(lines[0], startsWith('Old Brain Fix '));
    // Without a package context the version is unknown, and the header
    // says so instead of leaving a blank where the version should be.
    expect(lines[0].trim(), isNot('Old Brain Fix'));
    expect(lines[1], contains(Platform.operatingSystem));
    expect(lines[1].length, greaterThan(Platform.operatingSystem.length + 1));
    expect(lines[2], startsWith('locale '));
    expect(lines[3], '2026-08-20T12:00:00.000Z');
  });

  group('the running marker', () {
    late Directory dir;

    setUp(() async {
      dir = Directory.systemTemp.createTempSync('oldbrainfix-crash');
      await initCrashGuard(dir, restart: () {});
    });

    tearDown(() => dir.deleteSync(recursive: true));

    test('a clean exit leaves nothing behind; a kill is noticed', () async {
      markRunning();
      markCleanExit();
      expect(await previousRunDied(), isFalse);

      markRunning();
      expect(
        await previousRunDied(),
        isTrue,
        reason: 'a run that never went out of sight cleanly was killed',
      );
      expect(await previousRunDied(), isFalse, reason: 'asked exactly once');
    });

    test('the system\'s exit reason decides where it is known', () {
      // A crash, a native crash or a freeze always; a memory kill only
      // when the app was on screen; a saver's or the user's kill never.
      for (final reason in ['crash', 'native']) {
        expect(exitDeservesReport(reason, foreground: false), isTrue);
      }
      // A hang or a memory kill only counts on screen: in the background
      // it is the system's energy management ending the app (#206).
      for (final reason in ['anr', 'low_memory']) {
        expect(exitDeservesReport(reason, foreground: true), isTrue);
        expect(exitDeservesReport(reason, foreground: false), isFalse);
      }
      expect(exitDeservesReport('other', foreground: true), isFalse);
    });

    test('the last crash is readable once and can be cleared', () {
      expect(lastCrashText(), isNull);
      FlutterError.onError!(
        FlutterErrorDetails(
          exception: StateError('boom'),
          stack: StackTrace.current,
        ),
      );
      final text = lastCrashText();
      expect(text, contains('boom'));
      expect(text, contains(Platform.operatingSystem));
      clearLastCrash();
      expect(lastCrashText(), isNull);
    });

    test('a crash the Kotlin side wrote is read like a Dart one', () {
      // android/…/CrashLog.kt writes into the same file.
      File('${dir.path}/last_crash.txt').writeAsStringSync(
        'Old Brain Fix 0.1.0+1\n\njava.lang.IllegalStateException: ring',
      );
      expect(lastCrashText(), contains('IllegalStateException'));
    });

    // The app's own lifecycle, faked: off Android there is no exit
    // record, so these runs are decided by the marker alone, as on
    // Android before 11.
    group('watched through the lifecycle', () {
      late AppLifecycleListener watch;

      // As Android's embedding says it: one message per state, the
      // framework filling in the steps between.
      Future<void> lifecycle(AppLifecycleState state) =>
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
              .handlePlatformMessage(
                SystemChannels.lifecycle.name,
                SystemChannels.lifecycle.codec.encodeMessage('$state'),
                (_) {},
              );

      setUp(() async {
        TestWidgetsFlutterBinding.ensureInitialized();
        await lifecycle(AppLifecycleState.resumed);
        markRunning();
        watch = watchRunning();
      });

      tearDown(() => watch.dispose());

      test('a kill while in the background shows no report', () async {
        await lifecycle(AppLifecycleState.paused);
        // The system ends the process here; nothing more runs.
        expect(await previousRunDied(), isFalse);
      });

      test('a death while on screen shows the report', () async {
        expect(await previousRunDied(), isTrue);
      });

      test('back on screen, a death counts again', () async {
        await lifecycle(AppLifecycleState.paused);
        await lifecycle(AppLifecycleState.resumed);
        expect(await previousRunDied(), isTrue);
      });
    });

    test('the next start offers the report at the top, once', () async {
      FlutterError.onError!(
        FlutterErrorDetails(
          exception: StateError('boom'),
          stack: StackTrace.current,
        ),
      );
      final t = await AppLocalizations.delegate.load(const Locale('en'));
      final announcements = Announcements();
      addTearDown(announcements.dispose);
      final mailed = <String>[];
      offerCrashReport(
        announcements,
        t,
        navigator: GlobalKey<NavigatorState>(),
        mail: (body) async => mailed.add(body),
      );
      expect(announcements.visible, t.crashLastRunBody);
      // Taken now: the same crash is not offered again.
      expect(lastCrashText(), isNull);
      expect(mailed, isEmpty, reason: 'nothing goes without a tap');
      // Kept for Send feedback (#214), which can still send it along.
      expect(keptCrashReport()?.text, contains('boom'));
      expect(keptCrashReport()?.text, startsWith('Old Brain Fix '));
    });

    test('Send feedback reads a crash not offered yet, else the one '
        'offered last (#214)', () async {
      expect(keptCrashReport(), isNull);
      File('${dir.path}/last_crash_seen.txt').writeAsStringSync('older');
      expect(keptCrashReport()?.text, 'older');
      File('${dir.path}/last_crash.txt').writeAsStringSync('newer');
      expect(keptCrashReport()?.text, 'newer');
    });
  });

  group('the report dialog', () {
    // No crash guard here: it would take over FlutterError.onError and
    // swallow the test's own failures. Without it there is no crash
    // file, so the report is the one of a kill without a log.
    late Announcements announcements;
    late GlobalKey<NavigatorState> navigator;
    late List<String> mailed;

    Future<AppLocalizations> offer(WidgetTester tester, Locale locale) async {
      announcements = Announcements();
      addTearDown(announcements.dispose);
      navigator = GlobalKey<NavigatorState>();
      mailed = [];
      final t = await AppLocalizations.delegate.load(locale);
      offerCrashReport(
        announcements,
        t,
        navigator: navigator,
        mail: (body) async => mailed.add(body),
      );
      await tester.pumpWidget(
        MaterialApp(
          navigatorKey: navigator,
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) =>
              AnnouncementStrip(announcements: announcements, child: child!),
          home: const Scaffold(),
        ),
      );
      return t;
    }

    testWidgets('the tap opens what happened, with Close and Report', (
      tester,
    ) async {
      final t = await offer(tester, const Locale('en'));
      await tester.tap(find.text(t.crashLastRunBody));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text(t.crashLastRunTitle), findsOneWidget);
      expect(
        find.text('${t.crashLastRunKilledCause} ${t.crashLastRunFix}'),
        findsOneWidget,
      );
      expect(find.textContaining('without a crash log'), findsOneWidget);
      expect(find.text(t.close), findsOneWidget);
      expect(find.text(t.report), findsOneWidget);
      expect(mailed, isEmpty, reason: 'the tap alone mails nothing');
    });

    testWidgets('Close dismisses it for this run', (tester) async {
      final t = await offer(tester, const Locale('de'));
      await tester.tap(find.text(t.crashLastRunBody));
      await tester.pumpAndSettle();
      await tester.tap(find.text(t.close));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.text(t.crashLastRunBody), findsNothing);
      expect(announcements.visible, isNull);
      expect(lastCrashText(), isNull);
      expect(mailed, isEmpty);
    });

    testWidgets('Report mails the report', (tester) async {
      final t = await offer(tester, const Locale('en'));
      await tester.tap(find.text(t.crashLastRunBody));
      await tester.pumpAndSettle();
      await tester.tap(find.text(t.report));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(mailed.single, startsWith('Old Brain Fix '));
      expect(mailed.single, contains('without a crash log'));
      // The crash report carries its own header: not a second one.
      expect('\nlocale '.allMatches(mailed.single), hasLength(1));
      expect(announcements.visible, isNull);
    });
  });

  test('the mail body drops framework frames and puts ours first', () {
    final body = [
      'Old Brain Fix 0.1.0+1',
      '',
      'Bad state: No element',
      '',
      '#0 Stream.first. (dart:async/stream.dart:1653)',
      '#1 _rootRun (dart:async/zone_root.dart:27)',
      '#2 _CustomZone.run (dart:async/zone.dart:810)',
      '#3 something (package:flutter/src/widgets/framework.dart:1)',
      '#4 _HomeState.initState (package:oldbrainfix/src/app.dart:79)',
      '#5 Store.open (package:oldbrainfix_core/src/store.dart:400)',
    ].join('\n');
    final mail = mailBody(body);
    expect(mail, isNot(contains('dart:async')));
    expect(mail, isNot(contains('zone')));
    final ours = mail.indexOf('src/app.dart');
    expect(ours, greaterThan(0));
    expect(mail.indexOf('oldbrainfix_core'), greaterThan(ours));
    expect(
      mail.indexOf('flutter/src'),
      greaterThan(mail.indexOf('oldbrainfix_core')),
    );
    expect(mail, startsWith('Old Brain Fix 0.1.0+1'));
    expect(mailBody('x' * 3000).length, mailBodyLimit);
  });
}

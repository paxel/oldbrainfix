import 'dart:io';

import 'package:flutter/material.dart';
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
      for (final reason in ['crash', 'native', 'anr']) {
        expect(exitDeservesReport(reason, foreground: false), isTrue);
      }
      expect(exitDeservesReport('low_memory', foreground: true), isTrue);
      expect(exitDeservesReport('low_memory', foreground: false), isFalse);
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
        mail: (body) async => mailed.add(body),
      );
      expect(announcements.visible, t.crashLastRunBody);
      // Taken now: the same crash is not offered again.
      expect(lastCrashText(), isNull);
      expect(mailed, isEmpty, reason: 'nothing goes without a tap');
      announcements.visibleAction!();
      expect(mailed.single, contains('boom'));
    });

    test('a kill without a crash log still offers a report', () async {
      final t = await AppLocalizations.delegate.load(const Locale('de'));
      final announcements = Announcements();
      addTearDown(announcements.dispose);
      final mailed = <String>[];
      offerCrashReport(
        announcements,
        t,
        mail: (body) async => mailed.add(body),
      );
      expect(announcements.visible, t.crashLastRunBody);
      announcements.visibleAction!();
      expect(mailed.single, startsWith('Old Brain Fix '));
      expect(mailed.single, contains('without a crash log'));
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

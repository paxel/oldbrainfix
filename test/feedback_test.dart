import 'dart:io';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/crash_guard.dart';
import 'package:oldbrainfix/src/feedback_screen.dart';
import 'package:oldbrainfix/src/pack_screen.dart' show packWork, packWorkRuns;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_alarms.dart';
import 'open_settings.dart';

/// Send feedback (#214): the page builds the text from the ticks, shows
/// it in the preview, and sends exactly that by mail or to the share
/// sheet, the Notes picked in the Pack's tree beside it as a Pack.
void main() {
  const facts = FeedbackFacts(
    version: '0.4.0+57',
    system: 'Android 14 (SDK 34), Google Pixel 7',
    install: 'Play',
  );

  group('where the app came from', () {
    test('Play and F-Droid by their stores, an APK by the package installer '
        'or none; unknown when the system would not say', () {
      expect(installMethodOf('com.android.vending'), 'Play');
      expect(installMethodOf('org.fdroid.fdroid'), 'F-Droid');
      expect(installMethodOf('org.fdroid.basic'), 'F-Droid');
      expect(installMethodOf(null), 'APK');
      expect(installMethodOf('com.google.android.packageinstaller'), 'APK');
      expect(installMethodOf('com.android.packageinstaller'), 'APK');
      expect(installMethodOf('com.aurora.store'), 'unknown (com.aurora.store)');
      expect(installMethodOf('com.android.vending', known: false), 'unknown');
      expect(installMethodOf(null, known: false), 'unknown');
    });
  });

  group('the text sent', () {
    test('the words first, then each fact ticked, nothing of one not', () {
      expect(
        feedbackBody(
          ' It counts wrong. ',
          version: facts.version,
          system: facts.system,
          install: facts.install,
        ),
        'It counts wrong.\n\n'
        'Version: 0.4.0+57\n'
        'System: Android 14 (SDK 34), Google Pixel 7\n'
        'Installed from: Play',
      );
      expect(
        feedbackBody('Hi', system: facts.system),
        'Hi\n\nSystem: Android 14 (SDK 34), Google Pixel 7',
      );
      expect(feedbackBody('Hi'), 'Hi');
    });

    test('the crash report last, thinned and cut to fit a mail link', () {
      final frames = [
        for (var i = 0; i < 200; i++)
          '#$i      main (package:oldbrainfix/src/app.dart:$i:1)',
      ];
      final crash =
          'Old Brain Fix 0.4.0+57\n\nStateError: boom\n'
          '#900     _rootRun (dart:async/zone.dart:1:1)\n${frames.join('\n')}';
      final body = feedbackBody('Words', version: '0.4.0+57', crash: crash);
      expect(
        body,
        startsWith('Words\n\nVersion: 0.4.0+57\n\nLast crash report:\n'),
      );
      expect(body, contains('StateError: boom'));
      expect(body, isNot(contains('dart:async')));
      expect(body.length, mailBodyLimit);
      // The words are the user's: a line looking like a frame stays.
      expect(feedbackBody('#1 bug', crash: 'x'), startsWith('#1 bug\n'));
    });
  });

  group('the page', () {
    late Store store;
    late Directory dir;
    late FakeBundles bundles;
    late List<String> mailed;

    setUp(() {
      // The Pack's work runs inline but where a test puts the isolate back.
      packWork = <R>(work) async => work();
      packWorkRuns = 0;
      store = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      dir = Directory.systemTemp.createTempSync('feedback_');
      bundles = FakeBundles(dir);
      mailed = [];
    });
    tearDown(() {
      packWork = Isolate.run;
      store.close();
      dir.deleteSync(recursive: true);
    });

    Future<AppLocalizations> open(
      WidgetTester tester, {
      ({String text, DateTime at})? crash,
      Locale locale = const Locale('en'),
      bool tall = true,
    }) async {
      // Tall, so the whole page is built: the list builds only what
      // shows.
      tester.view.physicalSize = Size(1080, tall ? 8000 : 2400);
      tester.view.devicePixelRatio = 2.5;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        MaterialApp(
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: BundleScope(
            bundles: bundles,
            child: FeedbackScreen(
              store: store,
              facts: () async => facts,
              crash: () => crash,
              mail: (body) async => mailed.add(body),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      return AppLocalizations.delegate.load(locale);
    }

    Future<void> tap(WidgetTester tester, Finder finder) async {
      await tester.ensureVisible(finder);
      await tester.pumpAndSettle();
      await tester.tap(finder);
      await tester.pumpAndSettle();
    }

    String preview(WidgetTester tester) => tester
        .widget<SelectableText>(find.byKey(const ValueKey('feedback-preview')))
        .data!;

    bool enabled(WidgetTester tester, String key) =>
        tester.widget<ButtonStyleButton>(find.byKey(ValueKey(key))).enabled;

    bool? ticked(WidgetTester tester, String key) =>
        tester.widget<Checkbox>(find.byKey(ValueKey('feedback-$key'))).value;

    testWidgets('the facts ticked and the crash off at first; the preview '
        'follows the ticks; Mail sends exactly the preview', (tester) async {
      final t = await open(
        tester,
        crash: (text: 'StateError: boom', at: DateTime(2026, 11, 2, 14, 2)),
      );
      expect(ticked(tester, 'version'), isTrue);
      expect(ticked(tester, 'system'), isTrue);
      expect(ticked(tester, 'install'), isTrue);
      expect(ticked(tester, 'crash'), isFalse);
      expect(ticked(tester, 'notes'), isFalse);
      expect(find.text(t.feedbackTextFirst), findsOneWidget);
      expect(enabled(tester, 'feedback-mail'), isFalse);
      expect(enabled(tester, 'feedback-share'), isFalse);

      await tester.enterText(
        find.byKey(const ValueKey('feedback-text')),
        'It counts wrong.',
      );
      await tester.pump();
      expect(
        preview(tester),
        feedbackBody(
          'It counts wrong.',
          version: facts.version,
          system: facts.system,
          install: facts.install,
        ),
      );
      await tap(tester, find.byKey(const ValueKey('feedback-system')));
      await tap(tester, find.byKey(const ValueKey('feedback-crash')));
      final shown = preview(tester);
      expect(shown, isNot(contains('Pixel')));
      expect(shown, contains('StateError: boom'));
      expect(shown, contains('Installed from: Play'));
      expect(mailed, isEmpty, reason: 'nothing leaves without a tap');

      await tap(tester, find.byKey(const ValueKey('feedback-mail')));
      expect(mailed, [shown]);
    });

    testWidgets('without a crash report kept its tick is greyed', (
      tester,
    ) async {
      final t = await open(tester);
      expect(find.text(t.feedbackCrashNone), findsOneWidget);
      final box = tester.widget<Checkbox>(
        find.byKey(const ValueKey('feedback-crash')),
      );
      expect(box.onChanged, isNull);
    });

    testWidgets('text too long for a mail link goes only through Share', (
      tester,
    ) async {
      final t = await open(tester);
      await tester.enterText(
        find.byKey(const ValueKey('feedback-text')),
        'x' * (mailBodyLimit + 1),
      );
      await tester.pump();
      expect(enabled(tester, 'feedback-mail'), isFalse);
      expect(enabled(tester, 'feedback-share'), isTrue);
      expect(find.text(t.feedbackTooLongForMail), findsOneWidget);
    });

    testWidgets('Notes go along as a Pack picked in its tree, beside the '
        'text to the share sheet, addressed for a mail app', (tester) async {
      final places = Places(store);
      final health = places.addArea('Health');
      Notes(store).capture('Drink water', place: health.id);
      Notes(store).capture('Loose thought');
      final t = await open(tester);
      await tester.enterText(
        find.byKey(const ValueKey('feedback-text')),
        'Look at these.',
      );
      await tap(tester, find.byKey(const ValueKey('feedback-notes')));
      expect(find.text(t.feedbackNotesTitle), findsOneWidget);
      expect(find.text('Health'), findsOneWidget);
      final pack = store.packOfAll();
      final water = pack.notes.indexWhere((n) => n.text == 'Drink water');
      final box = find.byKey(ValueKey('pack-item-${PackItemKeys.note(water)}'));
      expect(tester.widget<CheckboxListTile>(box).value, isFalse);
      await tap(tester, box);
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(ticked(tester, 'notes'), isTrue);
      expect(find.byKey(const ValueKey('feedback-along')), findsOneWidget);
      expect(find.text(t.feedbackPackNotInMail), findsOneWidget);
      final shown = preview(tester);
      await tap(tester, find.byKey(const ValueKey('feedback-share')));
      final sent = bundles.texts.single;
      expect(sent.text, shown);
      expect(sent.files.single.type, packMimeType);
      expect(bundles.mails.single, (to: crashMail, subject: feedbackSubject));
      final opened = Store.inMemory().openPack(
        bundles.fileBytes[sent.files.single.path]!,
      );
      expect(opened.pack.notes.map((n) => n.text), ['Drink water']);
    });

    testWidgets('the Pack is written on another isolate, Share… waiting '
        'meanwhile', (tester) async {
      Notes(store).capture('Drink water');
      final t = await open(tester);
      await tester.enterText(
        find.byKey(const ValueKey('feedback-text')),
        'Look at this.',
      );
      await tap(tester, find.byKey(const ValueKey('feedback-notes')));
      await tap(tester, find.byKey(const ValueKey('pack-tick-all')));
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.text(t.feedbackPackNotInMail), findsOneWidget);
      packWork = Isolate.run;
      final share = find.byKey(const ValueKey('feedback-share'));
      await tester.ensureVisible(share);
      await tester.pumpAndSettle();
      await tester.tap(share);
      await tester.pump();
      await tester.pump();
      expect(enabled(tester, 'feedback-share'), isFalse);
      expect(
        find.descendant(
          of: share,
          matching: find.byType(CircularProgressIndicator),
        ),
        findsOneWidget,
      );
      expect(bundles.texts, isEmpty);
      // The result arrives on the real clock, what awaits it on the
      // test's.
      for (var i = 0; i < 600 && bundles.texts.isEmpty; i++) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 100)),
        );
        await tester.pump();
      }
      await tester.pumpAndSettle();
      expect(packWorkRuns, 1);
      expect(enabled(tester, 'feedback-share'), isTrue);
      final sent = bundles.texts.single;
      final opened = Store.inMemory().openPack(
        bundles.fileBytes[sent.files.single.path]!,
      );
      expect(opened.pack.notes.map((n) => n.text), ['Drink water']);
    });

    testWidgets('a tree left with nothing picked sends no Notes', (
      tester,
    ) async {
      Notes(store).capture('Loose thought');
      await open(tester);
      await tester.enterText(
        find.byKey(const ValueKey('feedback-text')),
        'Hello',
      );
      await tap(tester, find.byKey(const ValueKey('feedback-notes')));
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(ticked(tester, 'notes'), isFalse);
      await tap(tester, find.byKey(const ValueKey('feedback-share')));
      expect(bundles.texts.single.files, isEmpty);
    });

    testWidgets('fits a phone in German', (tester) async {
      final t = await open(
        tester,
        locale: const Locale('de'),
        tall: false,
        crash: (text: 'boom', at: DateTime(2026, 11, 2, 14, 2)),
      );
      expect(find.text(t.feedbackTitle), findsOneWidget);
      expect(t.feedbackTitle, 'Feedback senden');
      expect(find.text('Vom 2.11.2026 14:02'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });

  testWidgets('Settings ▸ Send feedback opens the page', (tester) async {
    final store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    addTearDown(store.close);
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    await openDeviceSettings(tester);
    final entry = find.byKey(const ValueKey('sendFeedback'));
    await tester.scrollUntilVisible(
      entry,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.tap(entry);
    await tester.pumpAndSettle();
    expect(find.byType(FeedbackScreen), findsOneWidget);
  });
}

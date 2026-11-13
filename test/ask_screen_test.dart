import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_links.dart';
import 'package:oldbrainfix/src/ask_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../packages/oldbrainfix_core/test/test_host.dart';
import 'announced.dart';
import 'fake_ask.dart';

/// Asking from the phone (#267, ADR 0051): the phone asks the user's own
/// desktop, only once the user allowed the provider it names; answers
/// link their Notes, which open here, and can be kept as a Note; a
/// question not answered says why at the top.
void main() {
  late Store store;
  late Announcements announcements;
  late List<AppLink> opened;

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 13, 9))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    announcements = Announcements();
    opened = [];
  });
  tearDown(() {
    store.close();
    announcements.dispose();
  });

  Finder key(String value) => find.byKey(ValueKey(value));

  /// The Ask page over a page that opens it, on a tall screen.
  Future<void> pumpAsk(
    WidgetTester tester,
    DesktopAsking asking, {
    bool settle = true,
  }) async {
    tester.view.physicalSize = const Size(1200, 3600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => AnnouncementsScope(
          announcements: announcements,
          child: AppLinkScope(
            onOpen: (_, link) async => opened.add(link),
            child: child ?? const SizedBox.shrink(),
          ),
        ),
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () => openAsk(context, store, asking: asking),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    // While a real connection looks for the computer, its spinner never
    // settles.
    settle
        ? await tester.pumpAndSettle()
        : await tester.pumpFrames(
            tester.widget(find.byType(MaterialApp)),
            const Duration(seconds: 1),
          );
  }

  Future<void> ask(WidgetTester tester, String question) async {
    await tester.enterText(key('ask-question'), question);
    await tester.tap(key('ask-button'));
    await tester.pumpAndSettle();
  }

  group('the permission', () {
    testWidgets('names the provider and what is sent, and nothing is asked '
        'before it is given', (tester) async {
      final desk = FakeAsking();
      await pumpAsk(tester, desk);
      expect(key('ask-consent'), findsOneWidget);
      expect(
        find.text(
          'Your questions go from this phone to your computer Desk, which '
          'sends them to Anthropic, model claude-opus-5-5, at '
          'api.anthropic.com.',
        ),
        findsOneWidget,
      );
      expect(find.text('What Anthropic receives:'), findsOneWidget);
      expect(
        find.text('The names of your Areas, Topics and tags.'),
        findsOneWidget,
      );
      expect(find.textContaining('Attachment Texts'), findsOneWidget);
      expect(find.textContaining('Nothing Hidden from AI'), findsOneWidget);
      expect(key('ask-question'), findsNothing, reason: 'no question yet');

      // Not now leaves without allowing anything.
      await tester.tap(key('ask-not-now'));
      await tester.pumpAndSettle();
      expect(find.text('open'), findsOneWidget);
      expect(store.askAllowed(fakeClaude), isFalse);

      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expect(key('ask-consent'), findsOneWidget, reason: 'asked again');
      await tester.tap(key('ask-allow'));
      await tester.pumpAndSettle();
      expect(store.askAllowed(fakeClaude), isTrue);
      expect(desk.asked, isEmpty);
      await ask(tester, 'Where is my passport?');
      expect(desk.asked.single.$1, 'Where is my passport?');
      expect(find.text('An answer.'), findsOneWidget);

      // Once allowed, the page opens at the question.
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expect(key('ask-consent'), findsNothing);
      expect(key('ask-question'), findsOneWidget);
      expect(
        find.text(
          'Questions go through Desk to Anthropic, model claude-opus-5-5. '
          'Only what the model finds and reads in your Notes is sent there, '
          'never what is Hidden from AI.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('is asked again when the computer asks another provider, '
        'the question kept in the field and not sent', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking();
      await pumpAsk(tester, desk);
      await ask(tester, 'First?');
      expect(desk.asked, hasLength(1));

      const deepseek = DesktopProvider(
        protocol: AskProtocol.openai,
        model: 'deepseek-chat',
        host: 'api.deepseek.com',
      );
      desk.shown = deepseek;
      await ask(tester, 'Second?');
      expect(desk.asked, hasLength(1), reason: 'not sent to another provider');
      expect(key('ask-consent'), findsOneWidget);
      expect(find.text('What api.deepseek.com receives:'), findsOneWidget);
      await tester.tap(key('ask-allow'));
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(key('ask-question')).controller?.text,
        'Second?',
        reason: 'the question waits in the field',
      );
      await tester.tap(key('ask-button'));
      await tester.pumpAndSettle();
      expect(desk.asked.last.$1, 'Second?');

      // Another model of the same provider is another permission too.
      desk.shown = const DesktopProvider(
        protocol: AskProtocol.openai,
        model: 'deepseek-reasoner',
        host: 'api.deepseek.com',
      );
      await ask(tester, 'Third?');
      expect(desk.asked, hasLength(2));
      expect(key('ask-consent'), findsOneWidget);
    });

    testWidgets('a provider the computer cannot ask yet offers no '
        'permission', (tester) async {
      final desk = FakeAsking(
        shown: const DesktopProvider(
          protocol: AskProtocol.anthropic,
          model: 'claude-opus-5-5',
          host: 'api.anthropic.com',
          unready: AskUnready.noKey,
        ),
      );
      await pumpAsk(tester, desk);
      expect(key('ask-consent'), findsNothing);
      expect(key('ask-question'), findsNothing);
      expect(
        find.text('Anthropic is not set up on the computer yet.'),
        findsOneWidget,
      );
      expect(
        find.text(
          "Enter the API key under Ask panel in the computer's Settings.",
        ),
        findsOneWidget,
      );
      desk.shown = fakeClaude;
      await tester.tap(key('ask-try-again'));
      await tester.pumpAndSettle();
      expect(key('ask-consent'), findsOneWidget);
    });
  });

  group('answers', () {
    testWidgets('link their Notes, which open, and a follow-up carries the '
        'earlier exchanges', (tester) async {
      store.allowAsking(fakeClaude);
      final passport = Notes(store).capture('Passport\nIn the blue folder');
      final gone = Notes(store).capture('Old passport');
      Notes(store).trash(gone.id);
      final desk = FakeAsking(
        answer: (question) => AskReply.answered(
          DesktopAnswer(
            text: 'In the [blue folder](oldbrainfix://note/${passport.id}).',
            notes: [passport.id, gone.id, 'note:not-here'],
          ),
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'Where is my passport?');
      expect(find.text('From these Notes'), findsOneWidget);
      expect(find.text('Old passport'), findsNothing, reason: 'in the Trash');
      await tester.tap(key('ask-source-0-${passport.id}'));
      await tester.pumpAndSettle();
      expect(opened, [AppLink(AppLinkKind.note, passport.id)]);
      expect(find.text('Passport'), findsOneWidget);

      await ask(tester, 'Since when?');
      expect(desk.asked.last.$2.single.question, 'Where is my passport?');
      expect(
        desk.asked.last.$2.single.answer,
        contains('blue folder'),
        reason: 'the earlier answer goes along',
      );
      // Newest first.
      expect(
        tester.getTopLeft(find.text('Since when?')).dy,
        lessThan(tester.getTopLeft(find.text('Where is my passport?')).dy),
      );
      await tester.tap(key('ask-start-over'));
      await tester.pumpAndSettle();
      expect(find.text('Since when?'), findsNothing);
      await ask(tester, 'Anew?');
      expect(desk.asked.last.$2, isEmpty, reason: 'started over');
    });

    testWidgets('Keep as Note saves the question and the answer as a '
        'Markdown Note in Unsorted with no Nudge', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking(
        answer: (_) => const AskReply.answered(
          DesktopAnswer(text: '**Tuesday** at 9.', notes: []),
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'When is the dentist?');
      final before = Notes(store).all().length;
      await tester.tap(key('ask-keep-0'));
      await tester.pumpAndSettle();
      final all = Notes(store).all();
      expect(all, hasLength(before + 1));
      final kept = all.firstWhere((n) => n.text.startsWith('When is'));
      expect(kept.text, 'When is the dentist?\n\n**Tuesday** at 9.');
      expect(kept.textFormat, TextFormats.markdown);
      expect(kept.place, isNull, reason: 'Unsorted');
      expect(kept.nudge, isNull, reason: 'no Nudge');
      expect(key('ask-keep-0'), findsNothing);
      await tester.tap(key('ask-kept-0'));
      await tester.pumpAndSettle();
      expect(opened, [AppLink(AppLinkKind.note, kept.id)]);
    });
  });

  group('failures', () {
    testWidgets('a question not answered goes back into the field and says '
        'why at the top', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking(
        answer: (_) => const AskReply.failed(
          AskFailure.key,
          message: 'invalid x-api-key',
          review: true,
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'Where?');
      expect(
        tester.widget<TextField>(key('ask-question')).controller?.text,
        'Where?',
      );
      expect(key('ask-review'), findsOneWidget, reason: 'a draft waits');
      await expectFailureAnnounced(
        tester,
        announcements,
        'The question was not answered.',
        title: 'Not answered',
        cause: "Anthropic refused the computer's API key.",
        fix: "Check the API key under Ask panel in the computer's Settings.",
      );

      desk.answer = (_) => const AskReply.failed(AskFailure.busy);
      await ask(tester, 'Where?');
      await expectFailureAnnounced(
        tester,
        announcements,
        'The question was not answered.',
        cause: 'The computer is answering another question.',
        fix: 'Ask again in a moment.',
      );

      desk.answer = (_) => const AskReply.failed(AskFailure.unknown);
      await ask(tester, 'Where?');
      await expectFailureAnnounced(
        tester,
        announcements,
        'The question was not answered.',
        cause: 'The computer answered in a way this version cannot read.',
        fix: 'Install the latest Old Brain Fix on this phone.',
      );

      desk.askTrouble = AskTrouble.timedOut;
      await ask(tester, 'Where?');
      await expectFailureAnnounced(
        tester,
        announcements,
        'The question was not answered.',
        cause: 'Your computer did not answer in time.',
        fix: 'Check that the computer is on and awake, then ask again.',
      );
    });

    testWidgets('no computer of the user\'s that answers says why in the '
        'page', (tester) async {
      final desk = FakeAsking(trouble: AskTrouble.unreachable);
      await pumpAsk(tester, desk);
      expect(find.text('No computer of yours answered.'), findsOneWidget);
      expect(find.textContaining('address by hand'), findsOneWidget);
      expect(key('ask-question'), findsNothing);

      desk.trouble = AskTrouble.declined;
      await tester.tap(key('ask-try-again'));
      await tester.pumpAndSettle();
      expect(
        find.text('Your computer does not let this phone ask.'),
        findsOneWidget,
      );
    });

    testWidgets('without a computer of the user\'s paired, none is asked', (
      tester,
    ) async {
      await pumpAsk(tester, PairedDesktops(store, found: (_) => null));
      expect(
        find.text('No computer of yours is paired with this phone.'),
        findsOneWidget,
      );
    });
  });

  testWidgets('through the paired connection, the question reaches the '
      'computer only once allowed', (tester) async {
    HttpOverrides.global = null;
    final desk = Store.inMemory(clock: () => DateTime.utc(2026, 11, 13, 9));
    final host = TestHost(desk);
    addTearDown(desk.close);
    final passport = Notes(store).capture('Passport');
    await tester.runAsync(() async {
      await host.start();
      host
        ..provider = fakeClaude
        ..answering = (body) => AskReply.answered(
          DesktopAnswer(
            text: 'In the [blue folder](oldbrainfix://note/${passport.id}).',
            notes: [passport.id],
          ),
        );
      await lanSync(store, decodePairCode(host.code())!, name: 'Phone');
    });
    addTearDown(() => tester.runAsync(host.stop));
    desk.rememberDevice(
      PairedDevice(
        id: store.deviceId,
        name: 'Phone',
        fingerprint: '',
        host: null,
        port: null,
        lastSync: desk.clock(),
      ),
    );
    Future<void> settle(bool Function() done) async {
      for (var i = 0; i < 100 && !done(); i++) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 20)),
        );
        await tester.pump();
      }
      await tester.pump();
    }

    await pumpAsk(
      tester,
      _OnRealClock(PairedDesktops(store, found: (_) => null)),
      settle: false,
    );
    await settle(() => key('ask-consent').evaluate().isNotEmpty);
    expect(key('ask-consent'), findsOneWidget);
    expect(host.asked, isEmpty, reason: 'no question before the permission');
    await tester.tap(key('ask-allow'));
    await tester.pump();
    await tester.enterText(key('ask-question'), 'Where is my passport?');
    await tester.tap(key('ask-button'));
    await settle(() => find.text('From these Notes').evaluate().isNotEmpty);
    expect(host.asked.single, {
      'earlier': <Object?>[],
      'question': 'Where is my passport?',
    });
    expect(host.askers.toSet(), {store.deviceId});
    expect(find.text('From these Notes'), findsOneWidget);
    expect(find.text('Passport'), findsOneWidget);
  });

  testWidgets('the Agenda offers Ask once a computer of the user\'s is '
      'paired', (tester) async {
    tester.view.physicalSize = const Size(2400, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    expect(key('ask'), findsNothing);

    store.rememberDevice(
      PairedDevice(
        id: 'device:desk',
        name: 'Desk',
        fingerprint: 'ab' * 32,
        host: '192.168.1.5',
        port: 4000,
        lastSync: store.clock(),
        shared: true,
      ),
    );
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    expect(key('ask'), findsNothing, reason: "another person's Device");

    store.rememberDevice(
      PairedDevice(
        id: 'device:laptop',
        name: 'Laptop',
        fingerprint: 'cd' * 32,
        host: '192.168.1.6',
        port: 4000,
        lastSync: store.clock(),
      ),
    );
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    expect(key('ask'), findsOneWidget);
    expect(find.byTooltip('Ask'), findsOneWidget);
  });
}

/// [inner] with its connections on the real clock, outside the test's
/// fake one, whose timeouts would otherwise pass at the first pump.
class _OnRealClock implements DesktopAsking {
  final DesktopAsking inner;

  _OnRealClock(this.inner);

  @override
  String? get computer => inner.computer;

  @override
  Future<DesktopProvider> provider() => Zone.root.run(inner.provider);

  @override
  Future<AskReply> ask(String question, List<AskExchange> earlier) =>
      Zone.root.run(() => inner.ask(question, earlier));
}

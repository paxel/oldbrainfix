import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_links.dart';
import 'package:oldbrainfix/src/ask_screen.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../packages/oldbrainfix_core/test/test_host.dart';
import 'bundle_test.dart' show FakeBundles;
import 'fake_alarms.dart';
import 'fake_ask.dart';
import 'open_settings.dart';

/// Asking from the phone (#267, ADR 0051): the phone asks the user's own
/// desktop, only once the user allowed the provider it names; answers
/// link their Notes, which open here, and can be kept as a Note, copied,
/// added to a Note and shared (#316); a question not answered says why at
/// the top. The questions stand as a thread, the oldest first, each with
/// Edit (#315).
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

  /// The Ask page over a page that opens it, on a tall screen unless
  /// [size] says otherwise.
  Future<void> pumpAsk(
    WidgetTester tester,
    DesktopAsking asking, {
    bool settle = true,
    Size size = const Size(1200, 3600),
    BundlePlatform bundles = const NoBundles(),
  }) async {
    tester.view.physicalSize = size;
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
            child: BundleScope(
              bundles: bundles,
              child: child ?? const SizedBox.shrink(),
            ),
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
        'permission: the question field says why and what to do, and Ask '
        'cannot be pressed (#317)', (tester) async {
      const openai = DesktopProvider(
        protocol: AskProtocol.openai,
        model: 'qwen2.5:3b',
        host: 'llm.example.org',
      );
      for (final (why, said) in [
        (
          AskUnready.aiOff,
          "The AI is switched off on Desk. Switch on AI (experimental) in "
              "the computer's AI menu.",
        ),
        (
          AskUnready.noModel,
          "No model is named on Desk. Name a model under Ask panel in the "
              "computer's AI settings.",
        ),
        (
          AskUnready.noKey,
          "llm.example.org has no API key on Desk. Enter the API key under "
              "Ask panel in the computer's AI settings.",
        ),
        (
          AskUnready.badUrl,
          'Desk may not ask llm.example.org at its address. Give the '
              'provider an https:// address under Ask panel in the '
              "computer's AI settings; http:// only on the computer or the "
              'home network.',
        ),
      ]) {
        final desk = FakeAsking(
          shown: DesktopProvider(
            protocol: openai.protocol,
            model: openai.model,
            host: openai.host,
            unready: why,
          ),
        );
        await pumpAsk(tester, desk);
        expect(key('ask-consent'), findsNothing, reason: '$why');
        expect(key('ask-question'), findsOneWidget, reason: '$why');
        expect(find.text(said), findsOneWidget, reason: '$why');
        expect(key('ask-sends'), findsNothing, reason: '$why');
        expect(
          tester.widget<FilledButton>(key('ask-button')).onPressed,
          isNull,
          reason: 'Ask cannot be pressed: $why',
        );
        await tester.enterText(key('ask-question'), 'Where?');
        await tester.testTextInput.receiveAction(TextInputAction.send);
        await tester.pumpAndSettle();
        expect(desk.asked, isEmpty, reason: '$why');
        expect(find.text(said), findsOneWidget, reason: 'it stays: $why');
        await tester.pumpWidget(const SizedBox.shrink());
      }

      final desk = FakeAsking(
        shown: const DesktopProvider(
          protocol: AskProtocol.anthropic,
          model: 'claude-opus-5-5',
          host: 'api.anthropic.com',
          unready: AskUnready.aiOff,
        ),
      );
      await pumpAsk(tester, desk);
      desk.shown = fakeClaude;
      await tester.tap(key('ask-try-again'));
      await tester.pumpAndSettle();
      expect(key('ask-consent'), findsOneWidget);
    });

    testWidgets('a provider that cannot be asked since keeps the question '
        'in the field, which says why (#317)', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking();
      await pumpAsk(tester, desk);
      desk.shown = const DesktopProvider(
        protocol: AskProtocol.anthropic,
        model: 'claude-opus-5-5',
        host: 'api.anthropic.com',
        unready: AskUnready.aiOff,
      );
      await ask(tester, 'Where?');
      expect(desk.asked, isEmpty);
      expect(
        tester.widget<TextField>(key('ask-question')).controller?.text,
        'Where?',
      );
      expect(
        find.textContaining('The AI is switched off on Desk.'),
        findsOneWidget,
      );
      expect(tester.widget<FilledButton>(key('ask-button')).onPressed, isNull);
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
      // A thread, the oldest first (#315).
      expect(
        tester.getTopLeft(find.text('Where is my passport?')).dy,
        lessThan(tester.getTopLeft(find.text('Since when?')).dy),
      );
      await tester.tap(key('ask-start-over'));
      await tester.pumpAndSettle();
      expect(find.text('Since when?'), findsNothing);
      await ask(tester, 'Anew?');
      expect(desk.asked.last.$2, isEmpty, reason: 'started over');
    });

    testWidgets('show the other Notes read as "Also read" under the linked '
        'ones, which open; one in the Trash or not here stays out', (
      tester,
    ) async {
      store.allowAsking(fakeClaude);
      final passport = Notes(store).capture('Passport\nIn the blue folder');
      final booth = Notes(store).capture('Photo booth at the station');
      final gone = Notes(store).capture('Old passport');
      Notes(store).trash(gone.id);
      var linked = true;
      final desk = FakeAsking(
        answer: (_) => AskReply.answered(
          DesktopAnswer(
            text: linked
                ? 'In the [blue folder](oldbrainfix://note/${passport.id}).'
                : 'In the blue folder.',
            notes: [if (linked) passport.id],
            read: [booth.id, gone.id, passport.id, 'note:not-here'],
          ),
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'Where is my passport?');
      expect(find.text('Also read'), findsOneWidget);
      expect(key('ask-source-0-${passport.id}'), findsOneWidget);
      expect(key('ask-also-read-0-${booth.id}'), findsOneWidget);
      expect(
        key('ask-also-read-0-${passport.id}'),
        findsNothing,
        reason: 'linked, so above',
      );
      expect(find.text('Old passport'), findsNothing, reason: 'in the Trash');
      // Linked first, then "Also read" under them.
      expect(
        tester.getTopLeft(key('ask-source-0-${passport.id}')).dy,
        lessThan(tester.getTopLeft(key('ask-also-read-0')).dy),
      );
      expect(
        tester.getTopLeft(key('ask-also-read-0')).dy,
        lessThan(tester.getTopLeft(key('ask-also-read-0-${booth.id}')).dy),
      );
      await tester.tap(key('ask-also-read-0-${booth.id}'));
      await tester.pumpAndSettle();
      expect(opened, [AppLink(AppLinkKind.note, booth.id)]);

      // An answer that links nothing still shows what was read, in the
      // order read.
      linked = false;
      await ask(tester, 'And now?');
      expect(key('ask-source-1'), findsNothing);
      expect(
        tester.getTopLeft(key('ask-also-read-1-${booth.id}')).dy,
        lessThan(tester.getTopLeft(key('ask-also-read-1-${passport.id}')).dy),
      );
    });

    testWidgets('say when a source could not be checked, and only '
        'then', (tester) async {
      store.allowAsking(fakeClaude);
      var unchecked = true;
      final desk = FakeAsking(
        answer: (_) => AskReply.answered(
          DesktopAnswer(
            text: 'In the blue folder.',
            notes: const [],
            unchecked: unchecked,
          ),
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'Where is my passport?');
      expect(key('ask-unchecked-0'), findsOneWidget);
      expect(find.text('A source could not be checked.'), findsOneWidget);
      unchecked = false;
      await ask(tester, 'And the ID card?');
      // The oldest first: the new answer is marked nothing, the first
      // still.
      expect(key('ask-unchecked-0'), findsOneWidget);
      expect(key('ask-unchecked-1'), findsNothing);
      expect(find.text('A source could not be checked.'), findsOneWidget);
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

  group('an answer (#316)', () {
    const answer = '**Tuesday** at 9, see [Dentist](oldbrainfix://note/n1).';
    FakeAsking desk() => FakeAsking(
      answer: (_) =>
          const AskReply.answered(DesktopAnswer(text: answer, notes: [])),
    );

    testWidgets('offers Copy text, Keep as Note, Add to Note… and Share to '
        'an app in a row, in that order', (tester) async {
      store.allowAsking(fakeClaude);
      await pumpAsk(tester, desk());
      await ask(tester, 'When is the dentist?');
      final xs = [
        'ask-copy-0',
        'ask-keep-0',
        'ask-add-to-note-0',
        'ask-share-0',
      ].map((k) => tester.getTopLeft(key(k)).dx).toList();
      expect(xs, orderedEquals([...xs]..sort()));
      expect(find.text('Copy text'), findsOneWidget);
      expect(find.text('Add to Note…'), findsOneWidget);
      expect(find.text('Share to an app'), findsOneWidget);
    });

    testWidgets('Copy text puts the answer on the clipboard as Markdown, its '
        'links kept, and says nothing', (tester) async {
      store.allowAsking(fakeClaude);
      final copied = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied.add((call.arguments as Map)['text'] as String);
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await pumpAsk(tester, desk());
      await ask(tester, 'When is the dentist?');
      await tester.tap(key('ask-copy-0'));
      await tester.pumpAndSettle();
      expect(copied, [answer]);
      expect(announcements.visible, isNull);
    });

    testWidgets('Share to an app hands the answer with its HTML to the share '
        'sheet; no app taking it is announced', (tester) async {
      store.allowAsking(fakeClaude);
      final dir = Directory.systemTemp.createTempSync('ask-share');
      addTearDown(() => dir.deleteSync(recursive: true));
      final bundles = FakeBundles(dir);
      await pumpAsk(tester, desk(), bundles: bundles);
      await ask(tester, 'When is the dentist?');
      await tester.tap(key('ask-share-0'));
      await tester.pumpAndSettle();
      final shared = bundles.texts.single;
      expect(shared.text, answer);
      expect(shared.files, isEmpty);
      expect(shared.html, contains('<strong>Tuesday</strong>'));
      expect(shared.html, contains('Dentist'));
      expect(shared.html, isNot(contains('oldbrainfix://')));
      expect(announcements.visible, isNull);

      bundles.shares = false;
      await tester.tap(key('ask-share-0'));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'The answer was not shared.');
    });

    testWidgets('Add to Note… adds the question and the answer to the Note '
        'picked as a Section, where that Note lies, and names it', (
      tester,
    ) async {
      store.allowAsking(fakeClaude);
      final places = Places(store);
      final health = places.addArea('Health');
      final notes = Notes(store);
      final dentist = notes.capture('Dentist visits', place: health.id);
      await pumpAsk(tester, desk());
      await ask(tester, 'When is the dentist?');
      final before = notes.all().length;
      await tester.tap(key('ask-add-to-note-0'));
      await tester.pumpAndSettle();
      expect(find.text('Add to Note…'), findsWidgets);
      await tester.tap(find.text('Dentist visits').last);
      await tester.pumpAndSettle();
      final sections = notes.sections(dentist.id);
      expect(sections, hasLength(1));
      expect(
        sections.single.text,
        'When is the dentist?\n\n$answer',
        reason: 'what Keep as Note keeps',
      );
      expect(sections.single.textFormat, TextFormats.markdown);
      expect(notes.byId(dentist.id)!.place, health.id);
      expect(notes.all(), hasLength(before), reason: 'nothing in Unsorted');
      expect(store.recentlyAddedTo.first, dentist.id);
      // The page stays, names the Note and opens it; Keep as Note stays.
      expect(key('ask-keep-0'), findsOneWidget);
      expect(find.text('Added to “Dentist visits”'), findsOneWidget);
      await tester.tap(key('ask-added-0'));
      await tester.pumpAndSettle();
      expect(opened, [AppLink(AppLinkKind.note, dentist.id)]);
    });

    testWidgets('Add to Note… left without a pick changes nothing', (
      tester,
    ) async {
      store.allowAsking(fakeClaude);
      Notes(store).capture('Dentist visits');
      await pumpAsk(tester, desk());
      await ask(tester, 'When is the dentist?');
      final before = Notes(store).all().map((n) => n.text).toList();
      await tester.tap(key('ask-add-to-note-0'));
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(Notes(store).all().map((n) => n.text).toList(), before);
      expect(key('ask-added-0'), findsNothing);
    });
  });

  group('the thread (#315)', () {
    String field(WidgetTester tester) =>
        tester.widget<TextField>(key('ask-question')).controller!.text;

    testWidgets('holds the questions, the oldest first, above the field', (
      tester,
    ) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking(
        answer: (question) => AskReply.answered(
          DesktopAnswer(text: 'Answer to $question', notes: const []),
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'Where is my passport?');
      await ask(tester, 'And which drawer?');
      double top(String value) => tester.getTopLeft(key(value)).dy;
      expect(top('ask-question-0'), lessThan(top('ask-answer-0')));
      expect(top('ask-answer-0'), lessThan(top('ask-question-1')));
      expect(top('ask-question-1'), lessThan(top('ask-answer-1')));
      expect(top('ask-answer-1'), lessThan(top('ask-question')));
      expect(
        tester.widget<Text>(key('ask-question-1')).data,
        'And which drawer?',
      );
    });

    testWidgets('Edit puts a question into the field, the cursor at its end, '
        'to ask it again after the others; the thread keeps it', (
      tester,
    ) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking(
        answer: (question) => AskReply.answered(
          DesktopAnswer(text: 'Answer to $question', notes: const []),
        ),
      );
      await pumpAsk(tester, desk);
      await ask(tester, 'Where is my passport?');
      await ask(tester, 'And which drawer?');
      // A text button with its icon (obf-1104).
      expect(
        find.descendant(
          of: key('ask-edit-0'),
          matching: find.byIcon(Icons.edit_outlined),
        ),
        findsOneWidget,
      );
      await tester.tap(key('ask-edit-0'));
      await tester.pumpAndSettle();
      final typed = tester.widget<TextField>(key('ask-question')).controller!;
      expect(typed.text, 'Where is my passport?');
      expect(
        typed.selection,
        const TextSelection.collapsed(offset: 'Where is my passport?'.length),
      );
      expect(
        tester.widget<TextField>(key('ask-question')).focusNode?.hasFocus,
        isTrue,
      );
      await tester.enterText(key('ask-question'), 'Where is my old passport?');
      await tester.tap(key('ask-button'));
      await tester.pumpAndSettle();
      expect(desk.asked.last.$1, 'Where is my old passport?');
      expect(
        [for (final e in desk.asked.last.$2) e.question],
        ['Where is my passport?', 'And which drawer?'],
        reason: 'asked after the whole thread',
      );
      expect(
        tester.widget<Text>(key('ask-question-0')).data,
        'Where is my passport?',
      );
      expect(
        tester.widget<Text>(key('ask-question-2')).data,
        'Where is my old passport?',
      );
      expect(field(tester), isEmpty);
    });

    testWidgets('while a question runs, Edit waits', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking();
      await pumpAsk(tester, desk);
      await ask(tester, 'Where is my passport?');
      desk.hold = Completer<void>();
      await tester.enterText(key('ask-question'), 'And which drawer?');
      await tester.tap(key('ask-button'));
      await tester.pump();
      expect(tester.widget<TextButton>(key('ask-edit-0')).onPressed, isNull);
      desk.hold!.complete();
      await tester.pumpAndSettle();
      expect(tester.widget<TextButton>(key('ask-edit-0')).onPressed, isNotNull);
    });

    testWidgets('a question asked scrolls the thread to it, the field '
        'staying under it', (tester) async {
      store.allowAsking(fakeClaude);
      final long = List.filled(30, 'The passport is in the blue folder.');
      final desk = FakeAsking(
        answer: (_) => AskReply.answered(
          DesktopAnswer(text: long.join(' '), notes: const []),
        ),
      );
      await pumpAsk(tester, desk, size: const Size(400, 800));
      for (final question in ['First?', 'Second?', 'Third?', 'Fourth?']) {
        await ask(tester, question);
      }
      final fieldTop = tester.getTopLeft(key('ask-question')).dy;
      final newest = tester.getTopLeft(key('ask-question-3')).dy;
      expect(newest, greaterThan(0));
      expect(newest, lessThan(fieldTop));
      // The first scrolled away, out of the list or above it.
      final first = key('ask-question-0');
      if (first.evaluate().isNotEmpty) {
        expect(tester.getBottomLeft(first).dy, lessThan(80));
      }
    });
  });

  group('failures', () {
    testWidgets('a question not answered goes back into the field, and '
        'its cause and fix stay under it until it is asked again (#317)', (
      tester,
    ) async {
      final t = lookupAppLocalizations(const Locale('en'));
      store.allowAsking(fakeClaude);
      final desk = FakeAsking();
      await pumpAsk(tester, desk);
      void fails(AskReply reply) {
        desk
          ..askTrouble = null
          ..answer = (_) => reply;
      }

      final kinds = <(void Function(), String, String, String)>[
        (
          () => fails(
            const AskReply.failed(
              AskFailure.unready,
              unready: AskUnready.aiOff,
            ),
          ),
          'The AI is switched off on Desk.',
          "Switch on AI (experimental) in the computer's AI menu.",
          'ask: unready',
        ),
        (
          () => fails(const AskReply.failed(AskFailure.unready)),
          t.phoneAskUnreadyCause('Anthropic'),
          t.phoneAskUnreadyFix,
          'ask: unready',
        ),
        (
          () => fails(
            const AskReply.failed(
              AskFailure.key,
              message: 'invalid x-api-key',
              review: true,
            ),
          ),
          "Anthropic refused the computer's API key.",
          "Check the API key under Ask panel in the computer's AI settings.",
          'invalid x-api-key',
        ),
        (
          () => fails(
            const AskReply.failed(
              AskFailure.provider,
              status: 404,
              message: 'model not found',
            ),
          ),
          t.phoneAskProviderFailed('Anthropic'),
          t.phoneAskProviderFix,
          'HTTP 404: model not found',
        ),
        (
          () => fails(
            const AskReply.failed(
              AskFailure.network,
              message: 'Connection refused',
            ),
          ),
          'No answer came from Anthropic to the computer.',
          t.phoneAskNoReplyFix,
          'Connection refused',
        ),
        (
          () => fails(
            const AskReply.failed(AskFailure.unreadable, message: '<html>'),
          ),
          t.phoneAskUnreadable('Anthropic'),
          t.phoneAskUnreadableFix,
          '<html>',
        ),
        (
          () => fails(const AskReply.failed(AskFailure.rounds)),
          t.phoneAskTooManyRounds,
          t.phoneAskNarrowerFix,
          'ask: rounds',
        ),
        (
          () => fails(const AskReply.failed(AskFailure.refused)),
          t.phoneAskRefused,
          t.phoneAskOtherWordsFix,
          'ask: refused',
        ),
        (
          () => fails(const AskReply.failed(AskFailure.stopped)),
          t.phoneAskStopped,
          t.phoneAskAgainFix,
          'ask: stopped',
        ),
        (
          () => fails(const AskReply.failed(AskFailure.busy)),
          'The computer is answering another question.',
          'Ask again in a moment.',
          'ask: busy',
        ),
        (
          () => fails(const AskReply.failed(AskFailure.unknown)),
          'The computer answered in a way this version cannot read.',
          'Install the latest Old Brain Fix on this phone.',
          'ask: unknown',
        ),
        for (final trouble in AskTrouble.values)
          (
            () => desk.askTrouble = trouble,
            askTroubleWhy(t, trouble).$1,
            askTroubleWhy(t, trouble).$2,
            'ask: ${trouble.name}',
          ),
      ];
      for (final (fail, cause, fix, details) in kinds) {
        fail();
        await ask(tester, 'Where?');
        expect(
          tester.widget<TextField>(key('ask-question')).controller?.text,
          'Where?',
          reason: cause,
        );
        expect(
          tester.widget<Text>(key('ask-failure')).data,
          cause,
          reason: cause,
        );
        expect(
          tester.widget<Text>(key('ask-failure-fix')).data,
          fix,
          reason: cause,
        );
        expect(
          tester.widget<Text>(key('ask-failure-question')).data,
          'Where?',
          reason: 'under its question: $cause',
        );
        expect(announcements.visible, isNull, reason: 'not at the top: $cause');
        // Nothing to copy, keep, add or share under a failure (#316).
        for (final action in ['Copy text', 'Keep as Note', 'Add to Note…']) {
          expect(find.text(action), findsNothing, reason: '$cause: $action');
        }
        expect(find.text('Share to an app'), findsNothing, reason: cause);
        // It stays while the user does something else.
        await tester.enterText(key('ask-question'), 'Where now?');
        await tester.pump(const Duration(minutes: 1));
        expect(find.text(cause), findsOneWidget, reason: 'stays: $cause');
        await tester.tap(key('ask-failure-details'));
        await tester.pumpAndSettle();
        expect(find.text(details), findsOneWidget, reason: cause);
        expect(find.text('Report'), findsOneWidget, reason: cause);
        await tester.tap(find.text('Close'));
        await tester.pumpAndSettle();
        await tester.enterText(key('ask-question'), 'Where?');
      }
      // A draft the model made before it failed waits for its review.
      fails(
        const AskReply.failed(AskFailure.network, message: 'x', review: true),
      );
      await ask(tester, 'Where?');
      expect(key('ask-review'), findsOneWidget, reason: 'a draft waits');

      // Asked again, the failure goes and the answer shows.
      desk
        ..askTrouble = null
        ..answer = (_) => const AskReply.answered(
          DesktopAnswer(text: 'In the blue folder.', notes: []),
        );
      await ask(tester, 'Where?');
      expect(key('ask-failure'), findsNothing);
      expect(find.text('In the blue folder.'), findsOneWidget);
    });

    testWidgets('while a question runs, the status line says whom it asks '
        'and how many Notes were read, and Stop puts it back and drops '
        'what comes after (#317)', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking(
        answer: (_) => const AskReply.answered(
          DesktopAnswer(text: 'Too late.', notes: []),
        ),
      );
      var read = 0;
      desk
        ..hold = Completer<void>()
        ..progressOf = (id) => AskProgress(asking: true, read: read);
      await pumpAsk(tester, desk);
      await tester.enterText(key('ask-question'), 'Where is my passport?');
      await tester.tap(key('ask-button'));
      await tester.pump();
      expect(tester.widget<Text>(key('ask-status')).data, 'Asking Desk…');
      expect(find.text('Where is my passport?'), findsOneWidget);
      read = 3;
      await tester.pump(const Duration(seconds: 3));
      expect(
        tester.widget<Text>(key('ask-status')).data,
        'Asking Desk… reading 3 Notes',
      );
      expect(key('ask-stop'), findsOneWidget);

      await tester.tap(key('ask-stop'));
      await tester.pump();
      expect(key('ask-status'), findsNothing);
      expect(desk.stopped, [desk.ids.single]);
      expect(
        tester.widget<TextField>(key('ask-question')).controller?.text,
        'Where is my passport?',
      );
      expect(key('ask-failure'), findsNothing, reason: 'the user stopped it');
      desk.hold!.complete();
      await tester.pump(const Duration(seconds: 3));
      expect(find.text('Too late.'), findsNothing, reason: 'dropped');
      expect(key('ask-answer-0'), findsNothing);
    });

    testWidgets('a computer that tells no progress, as 0.6.0, shows that it '
        'asks, and is not asked again', (tester) async {
      store.allowAsking(fakeClaude);
      final desk = FakeAsking()..hold = Completer<void>();
      var polls = 0;
      desk.progressOf = (_) {
        polls++;
        return null;
      };
      await pumpAsk(tester, desk);
      await tester.enterText(key('ask-question'), 'Where?');
      await tester.tap(key('ask-button'));
      await tester.pump(const Duration(seconds: 10));
      expect(tester.widget<Text>(key('ask-status')).data, 'Asking Desk…');
      expect(polls, 1);
      desk.hold!.complete();
      await tester.pumpAndSettle();
      expect(find.text('An answer.'), findsOneWidget);
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
      'id': isA<String>(),
      'question': 'Where is my passport?',
    });
    expect(host.askers.toSet(), {store.deviceId});
    expect(find.text('From these Notes'), findsOneWidget);
    expect(find.text('Passport'), findsOneWidget);
  });

  testWidgets('an announcement that shows at the top while a question '
      'waits loses no question: it reaches the computer, and the status line '
      'names it (#317)', (tester) async {
    HttpOverrides.global = null;
    store.askOn = true;
    final desk = Store.inMemory(clock: () => DateTime.utc(2026, 11, 13, 9));
    final host = TestHost(desk);
    addTearDown(desk.close);
    await tester.runAsync(() async {
      await host.start();
      host
        ..provider = fakeClaude
        ..answering = ((body) => const AskReply.answered(
          DesktopAnswer(text: 'In the blue folder.', notes: []),
        ))
        ..progress = ((id) => const AskProgress(asking: true));
      await lanSync(store, decodePairCode(host.code())!, name: 'Phone');
    });
    addTearDown(() => tester.runAsync(host.stop));
    store.allowAsking(fakeClaude);
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
        await tester.pump(const Duration(milliseconds: 100));
      }
      await tester.pump();
    }

    tester.view.physicalSize = const Size(1080, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        alarms: FakeAlarms(),
        announcements: announcements,
      ),
    );
    await settle(() => key('ask').evaluate().isNotEmpty);
    await tester.tap(key('ask'));
    await settle(() => key('ask-question').evaluate().isNotEmpty);
    await tester.enterText(key('ask-question'), 'Where is my passport?');
    await tester.tap(key('ask-button'));
    await tester.pump();
    // Something else finishes meanwhile: the strip at the top shows, and
    // the page is built anew under it.
    announcements.announce('Something else finished');
    await tester.pump();
    expect(tester.widget<Text>(key('ask-status')).data, 'Asking Desk…');
    await settle(() => find.text('In the blue folder.').evaluate().isNotEmpty);
    expect(host.asked.single['question'], 'Where is my passport?');
    expect(find.text('In the blue folder.'), findsOneWidget);
    expect(key('ask-failure'), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  /// The user's own computer that hosts, paired.
  void pairLaptop() => store.rememberDevice(
    PairedDevice(
      id: 'device:laptop',
      name: 'Laptop',
      fingerprint: 'cd' * 32,
      host: '192.168.1.6',
      port: 4000,
      lastSync: store.clock(),
    ),
  );

  Future<void> startApp(WidgetTester tester) async {
    tester.view.physicalSize = const Size(2400, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
  }

  /// Scrolls Settings to [finder] and taps it.
  Future<void> tapInSettings(WidgetTester tester, Finder finder) async {
    if (finder.evaluate().isEmpty) {
      await tester.scrollUntilVisible(
        finder,
        200,
        scrollable: find.byType(Scrollable).last,
      );
    }
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  group('Ask (experimental)', () {
    testWidgets('is off at first: no Ask in the Agenda\'s bar, even with a '
        'computer of the user\'s paired', (tester) async {
      pairLaptop();
      await startApp(tester);
      expect(store.askOn, isFalse);
      expect(key('ask'), findsNothing);
      expect(find.byTooltip('Ask'), findsNothing);
    });

    testWidgets('is switched on in Settings in Advanced only, found by the '
        'search, and shows Ask in the Agenda\'s bar at once', (tester) async {
      // Never chosen is Advanced.
      pairLaptop();
      await startApp(tester);
      await openDeviceSettings(tester);
      final askSwitch = find.byKey(
        const ValueKey('askOn'),
        skipOffstage: false,
      );
      expect(askSwitch, findsOneWidget);
      expect(find.text('Ask (experimental)', skipOffstage: false), findsOne);

      await tester.enterText(
        find.descendant(
          of: key('settingsSearch'),
          matching: find.byType(TextField),
        ),
        'experimental',
      );
      await tester.pumpAndSettle();
      expect(askSwitch, findsOneWidget, reason: 'the search finds it');
      expect(key('settingsSearchNothing'), findsNothing);
      await tapInSettings(tester, askSwitch);
      expect(store.askOn, isTrue);
      expect(store.allEntries(), isEmpty, reason: 'a setting of this Device');

      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(key('ask'), findsOneWidget);

      // Simple hides the switch; what was switched on stays on.
      store.advanced = false;
      await openDeviceSettings(tester);
      expect(askSwitch, findsNothing);
      await tester.enterText(
        find.descendant(
          of: key('settingsSearch'),
          matching: find.byType(TextField),
        ),
        'experimental',
      );
      await tester.pumpAndSettle();
      expect(key('settingsSearchNothing'), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(key('ask'), findsOneWidget);
    });
  });

  testWidgets('the Agenda offers Ask once it is switched on and a computer '
      'of the user\'s is paired', (tester) async {
    store.askOn = true;
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
  Future<AskReply> ask(
    String question,
    List<AskExchange> earlier, {
    required String id,
  }) => Zone.root.run(() => inner.ask(question, earlier, id: id));

  @override
  Future<AskProgress?> progress(String id) =>
      Zone.root.run(() => inner.progress(id));

  @override
  Future<void> stop(String id) => Zone.root.run(() => inner.stop(id));
}

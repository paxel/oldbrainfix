import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_links.dart';
import 'app_lock.dart';
import 'auto_sync.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'page_bar.dart';
import 'system_bars.dart';
import 'text_marks_view.dart';

/// How the provider query reaches a paired Device; the app passes the
/// core's [desktopProvider].
typedef ProviderCall = Future<DesktopProvider> Function(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
});

/// How a question reaches a paired Device; the app passes the core's
/// [askDesktop].
typedef QuestionCall = Future<AskReply> Function(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  required String question,
  List<AskExchange> earlier,
});

Future<DesktopProvider> _desktopProvider(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
}) => desktopProvider(store, device, at: at);

Future<AskReply> _askDesktop(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  required String question,
  List<AskExchange> earlier = const [],
}) => askDesktop(store, device, at: at, question: question, earlier: earlier);

/// The user's own desktop as the phone asks it (#267, ADR 0051): which
/// provider it asks, and the question. Throws an [AskException] when no
/// desktop of the user's answers.
abstract interface class DesktopAsking {
  /// The name of the Device that answered last, if one did.
  String? get computer;

  Future<DesktopProvider> provider();

  /// Asks the Device that named the provider last; a question once sent
  /// is not sent again elsewhere.
  Future<AskReply> ask(String question, List<AskExchange> earlier);
}

/// The paired Devices of the user's own that host, the one synced with
/// last first: those a question may go to. Another person's Device is
/// never asked, as the question would go to their provider.
List<PairedDevice> askableDevices(Store store) => [
  for (final device in store.pairedDevices())
    if (!device.shared &&
        device.fingerprint.isNotEmpty &&
        store.isOwnDevice(device.id))
      device,
]..sort((a, b) => b.lastSync.compareTo(a.lastSync));

/// [DesktopAsking] over the paired connection (ADR 0003): each own Device
/// that hosts is tried in turn, where discovery [found] it, at its address
/// by hand, and where it was last, until one names its provider; a phone
/// or an older desktop answers no questions and the next is tried.
class PairedDesktops implements DesktopAsking {
  final Store store;
  final PairedAddress? Function(String id) found;
  final ProviderCall providerCall;
  final QuestionCall questionCall;

  PairedDevice? _device;

  PairedDesktops(
    this.store, {
    required this.found,
    this.providerCall = _desktopProvider,
    this.questionCall = _askDesktop,
  });

  @override
  String? get computer => _device?.name;

  List<PairedAddress> _at(PairedDevice device) =>
      store.pairedAddresses(device, found: found(device.id));

  @override
  Future<DesktopProvider> provider() async {
    AskTrouble? worst;
    for (final device in askableDevices(store)) {
      try {
        final provider = await providerCall(store, device, at: _at(device));
        _device = device;
        return provider;
      } on AskException catch (e) {
        if (worst == null || _rank(e.trouble) > _rank(worst)) {
          worst = e.trouble;
        }
      }
    }
    throw AskException(worst ?? AskTrouble.notOwn);
  }

  /// How much a trouble says about the desktop: a phone that answers no
  /// questions says least, as the desktop may be another Device.
  static int _rank(AskTrouble trouble) => switch (trouble) {
    AskTrouble.notOffered => 0,
    AskTrouble.notOwn => 1,
    AskTrouble.unreachable => 2,
    AskTrouble.failed => 3,
    AskTrouble.timedOut => 4,
    AskTrouble.wrongHost => 5,
    AskTrouble.declined => 6,
  };

  @override
  Future<AskReply> ask(String question, List<AskExchange> earlier) async {
    final device = _device;
    if (device == null) throw const AskException(AskTrouble.notOwn);
    return questionCall(
      store,
      device,
      at: _at(device),
      question: question,
      earlier: earlier,
    );
  }
}

/// Opens the Ask page, asking the user's own desktop through [asking],
/// or over the paired connection when none is given.
Future<void> openAsk(
  BuildContext context,
  Store store, {
  DesktopAsking? asking,
}) {
  final auto = AutoSyncScope.of(context);
  return Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => AskScreen(
        store: store,
        asking:
            asking ?? PairedDesktops(store, found: (id) => auto?.foundAt(id)),
      ),
    ),
  );
}

/// A question asked and its answer.
class _Asked {
  final String question;
  final DesktopAnswer answer;

  /// The Notes its answer links that this phone shows, by id with their
  /// first lines.
  final List<(String, String)> sources;

  /// The Note it was kept as.
  String? kept;

  _Asked(this.question, this.answer, this.sources);
}

/// What the page shows above its answers.
enum _Phase { looking, unreachable, unready, consent, ready }

/// Ask, from the Agenda's bar (#267, ADR 0051): the phone runs no model
/// and talks to no provider; it asks the user's own desktop, which runs
/// the question as its Ask panel does and answers with links to the
/// Notes it came from, which open here. Before the first question to a
/// provider the page names it and what is sent, and asks the user's
/// permission (App Store guideline 5.1.2(i)); a provider or model the
/// user did not allow is asked about again, and nothing is sent before.
/// A follow-up is asked after the earlier answers; Start over drops them,
/// as leaving the page does. Keep as Note saves a question and its answer
/// as a Markdown Note in Unsorted, with no Nudge. A question not answered
/// goes back into the field and is announced at the top, its tap the
/// cause and the fix.
class AskScreen extends StatefulWidget {
  final Store store;
  final DesktopAsking asking;

  const AskScreen({super.key, required this.store, required this.asking});

  @override
  State<AskScreen> createState() => _AskScreenState();
}

class _AskScreenState extends State<AskScreen> {
  final _question = TextEditingController();
  final _focus = FocusNode();
  var _phase = _Phase.looking;
  DesktopProvider? _provider;

  /// Why no desktop named its provider, while [_Phase.unreachable].
  AskTrouble _trouble = AskTrouble.unreachable;

  /// The question waiting for its answer.
  String? _asking;

  /// The questions answered, the newest first.
  final _asked = <_Asked>[];

  /// Whether changes of an AI Task wait for the review on the desktop,
  /// as the last reply said.
  var _review = false;

  @override
  void initState() {
    super.initState();
    unawaited(_look());
  }

  @override
  void dispose() {
    _question.dispose();
    _focus.dispose();
    super.dispose();
  }

  /// Learns which provider the desktop asks.
  Future<void> _look() async {
    setState(() => _phase = _Phase.looking);
    try {
      final provider = await widget.asking.provider();
      if (!mounted) return;
      setState(() => _show(provider));
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _trouble = e is AskException ? e.trouble : AskTrouble.failed;
        _phase = _Phase.unreachable;
      });
    }
  }

  /// Shows [provider]: asked about first, unless the user allowed it.
  void _show(DesktopProvider provider) {
    _provider = provider;
    _phase = !provider.ready
        ? _Phase.unready
        : widget.store.askAllowed(provider)
        ? _Phase.ready
        : _Phase.consent;
  }

  void _allow() {
    final provider = _provider;
    if (provider == null) return;
    widget.store.allowAsking(provider);
    setState(() => _phase = _Phase.ready);
    _focus.requestFocus();
  }

  /// The answers so far, which a follow-up is asked after, the oldest
  /// first; an answer without words is left out, as the desktop does.
  List<AskExchange> get _earlier => [
    for (final asked in _asked.reversed)
      if (asked.answer.text.trim().isNotEmpty)
        AskExchange(asked.question, asked.answer.text),
  ];

  /// Asks the question typed, once the provider the desktop asks now is
  /// one the user allowed; else the question stays in the field.
  Future<void> _ask() async {
    final question = _question.text.trim();
    if (question.isEmpty || _asking != null) return;
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    setState(() => _asking = question);
    _question.clear();
    void back() {
      _asking = null;
      if (_question.text.isEmpty) _question.text = question;
    }

    try {
      // Read again before each question: the desktop may ask another
      // provider since.
      final provider = await widget.asking.provider();
      if (!mounted) return;
      if (!provider.ready || !widget.store.askAllowed(provider)) {
        setState(() {
          back();
          _show(provider);
        });
        return;
      }
      if (provider != _provider) setState(() => _provider = provider);
      final reply = await widget.asking.ask(question, _earlier);
      if (!mounted) return;
      final answer = reply.answer;
      setState(() {
        _review = reply.review;
        if (answer == null) {
          back();
        } else {
          _asking = null;
          _asked.insert(0, _Asked(question, answer, _sources(answer)));
        }
      });
      if (answer == null) {
        _announce(t, announcements, navigator, _failureWhy(t, reply));
      }
    } on Object catch (e) {
      if (!mounted) return;
      setState(back);
      final trouble = e is AskException ? e.trouble : AskTrouble.failed;
      final (cause, fix) = askTroubleWhy(t, trouble);
      _announce(t, announcements, navigator, (
        cause,
        fix,
        e is AskException ? 'ask: ${trouble.name}' : '$e',
      ));
    }
  }

  void _announce(
    AppLocalizations t,
    Announcements? announcements,
    NavigatorState navigator,
    (String, String, String) why,
  ) {
    final (cause, fix, details) = why;
    announceFailureWith(
      announcements,
      navigator,
      t.phoneAskFailed,
      title: t.phoneAskFailedTitle,
      cause: cause,
      fix: fix,
      details: details,
    );
  }

  /// Why the desktop gave no answer, how to fix it, and what the
  /// provider said.
  (String, String, String) _failureWhy(AppLocalizations t, AskReply reply) {
    final provider = providerName(_provider);
    final failure = reply.failure ?? AskFailure.unknown;
    final (cause, fix) = switch (failure) {
      AskFailure.unready => (
        t.phoneAskUnreadyCause(provider),
        switch (reply.unready) {
          final why? => unreadyLine(t, why),
          null => t.phoneAskUnreadyFix,
        },
      ),
      AskFailure.key => (t.phoneAskKeyRefused(provider), t.phoneAskKeyFix),
      AskFailure.provider => (
        t.phoneAskProviderFailed(provider),
        t.phoneAskProviderFix,
      ),
      AskFailure.network => (t.phoneAskNoReply(provider), t.phoneAskNoReplyFix),
      AskFailure.unreadable => (
        t.phoneAskUnreadable(provider),
        t.phoneAskUnreadableFix,
      ),
      AskFailure.rounds => (t.phoneAskTooManyRounds, t.phoneAskNarrowerFix),
      AskFailure.refused => (t.phoneAskRefused, t.phoneAskOtherWordsFix),
      AskFailure.stopped => (t.phoneAskStopped, t.phoneAskAgainFix),
      AskFailure.busy => (t.phoneAskBusy, t.phoneAskBusyFix),
      AskFailure.unknown => (t.phoneAskUnknown, t.phoneAskUpdatePhoneFix),
    };
    final said = failure == AskFailure.provider
        ? 'HTTP ${reply.status}: ${reply.message}'
        : reply.message;
    return (cause, fix, said.isEmpty ? 'ask: ${failure.name}' : said);
  }

  /// The Notes [answer] links that this phone shows, by their first
  /// lines: not one it does not hold, nor one in the Trash, merged, or in
  /// a locked Area while it is closed, whose link in the text still asks
  /// for the PIN.
  List<(String, String)> _sources(DesktopAnswer answer) {
    final notes = Notes(widget.store);
    final places = Places(widget.store);
    final lock = LockScope.of(context);
    return [
      for (final id in answer.notes)
        if (notes.byId(id) case final note?
            when !note.isTrashed && !note.isMerged)
          if (!_closed(lock, places, note.place)) (id, _title(note)),
    ];
  }

  static bool _closed(AppLock? lock, Places places, String? place) {
    if (lock == null || place == null) return false;
    return lock.isAreaClosed(places.topicById(place)?.areaId ?? place);
  }

  static String _title(Note note) {
    final line = note.visibleText.split('\n').first.trim();
    return line.isEmpty ? note.id : line;
  }

  /// Keeps the question and its answer as a Markdown Note in Unsorted,
  /// the question its first line, with no Nudge, as the desktop does.
  void _keep(_Asked asked) {
    if (asked.kept != null) return;
    final note = Notes(widget.store).capture(
      '${asked.question.trim()}\n\n${asked.answer.text.trim()}',
      format: TextFormats.markdown,
    );
    setState(() => asked.kept = note.id);
  }

  void _open(String id) =>
      unawaited(followAppLink(context, AppLink(AppLinkKind.note, id)));

  void _startOver() {
    setState(() {
      _asked.clear();
      _review = false;
    });
    _focus.requestFocus();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.phoneAskTitle,
        actions: [
          if (_asked.isNotEmpty && _asking == null)
            BarAction(
              key: const ValueKey('ask-start-over'),
              icon: Icons.restart_alt,
              label: t.phoneAskStartOver,
              onPressed: _startOver,
            ),
        ],
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: switch (_phase) {
          _Phase.looking => [_looking(t)],
          _Phase.unreachable => _unreachable(t),
          _Phase.unready => _unready(t),
          _Phase.consent => _consent(t),
          _Phase.ready => _ready(t),
        },
      ),
    );
  }

  Widget _looking(AppLocalizations t) => Row(
    key: const ValueKey('ask-looking'),
    children: [
      const SizedBox.square(
        dimension: 20,
        child: CircularProgressIndicator(strokeWidth: 2),
      ),
      const SizedBox(width: 12),
      Expanded(child: Text(t.phoneAskLooking)),
    ],
  );

  Widget _tryAgain(AppLocalizations t) => Align(
    alignment: Alignment.centerLeft,
    child: FilledButton.tonal(
      key: const ValueKey('ask-try-again'),
      onPressed: _look,
      child: Text(t.phoneAskTryAgain),
    ),
  );

  List<Widget> _unreachable(AppLocalizations t) {
    final (cause, fix) = askTroubleWhy(t, _trouble);
    return [
      Text(
        cause,
        key: const ValueKey('ask-unreachable'),
        style: TextStyle(color: Theme.of(context).colorScheme.error),
      ),
      const SizedBox(height: 8),
      Text(fix),
      const SizedBox(height: 16),
      _tryAgain(t),
    ];
  }

  List<Widget> _unready(AppLocalizations t) {
    final provider = _provider;
    return [
      Text(
        t.phoneAskUnreadyCause(providerName(provider)),
        key: const ValueKey('ask-unready'),
        style: TextStyle(color: Theme.of(context).colorScheme.error),
      ),
      const SizedBox(height: 8),
      Text(switch (provider?.unready) {
        final why? => unreadyLine(t, why),
        null => t.phoneAskUnreadyFix,
      }),
      const SizedBox(height: 16),
      _tryAgain(t),
    ];
  }

  /// What is sent where, and the permission, before the first question
  /// to this provider (App Store guideline 5.1.2(i)).
  List<Widget> _consent(AppLocalizations t) {
    final theme = Theme.of(context);
    final provider = _provider;
    if (provider == null) return const [];
    final name = providerName(provider);
    Widget item(String text) => Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('•  '),
          Expanded(child: Text(text)),
        ],
      ),
    );
    return [
      Text(
        t.phoneAskConsentTitle,
        key: const ValueKey('ask-consent'),
        style: theme.textTheme.titleMedium,
      ),
      const SizedBox(height: 8),
      Text(
        t.phoneAskConsentIntro(
          widget.asking.computer ?? '',
          name,
          provider.model,
          provider.host,
        ),
      ),
      const SizedBox(height: 16),
      Text(t.phoneAskConsentWhat(name), style: theme.textTheme.titleSmall),
      item(t.phoneAskConsentQuestion),
      item(t.phoneAskConsentNames),
      item(t.phoneAskConsentRoutines),
      item(t.phoneAskConsentFound),
      item(t.phoneAskConsentRead),
      const SizedBox(height: 16),
      Text(t.phoneAskConsentNever(name)),
      const SizedBox(height: 8),
      Text(t.phoneAskConsentAgain),
      const SizedBox(height: 16),
      FilledButton(
        key: const ValueKey('ask-allow'),
        onPressed: _allow,
        child: Text(t.phoneAskAllow),
      ),
      const SizedBox(height: 8),
      OutlinedButton(
        key: const ValueKey('ask-not-now'),
        onPressed: () => Navigator.of(context).maybePop(),
        child: Text(t.notNow),
      ),
    ];
  }

  List<Widget> _ready(AppLocalizations t) {
    final theme = Theme.of(context);
    final provider = _provider;
    final name = providerName(provider);
    final asking = _asking;
    return [
      Text(
        t.phoneAskSends(
          widget.asking.computer ?? '',
          name,
          provider?.model ?? '',
        ),
        key: const ValueKey('ask-sends'),
        style: theme.textTheme.bodySmall,
      ),
      if (_review) ...[
        const SizedBox(height: 8),
        Text(
          t.phoneAskTaskWaits,
          key: const ValueKey('ask-review'),
          style: TextStyle(color: theme.colorScheme.primary),
        ),
      ],
      const SizedBox(height: 16),
      TextField(
        key: const ValueKey('ask-question'),
        controller: _question,
        focusNode: _focus,
        enabled: asking == null,
        minLines: 1,
        maxLines: 4,
        textCapitalization: TextCapitalization.sentences,
        textInputAction: TextInputAction.send,
        onSubmitted: (_) => unawaited(_ask()),
        decoration: InputDecoration(
          hintText: t.phoneAskHint,
          border: const OutlineInputBorder(),
        ),
      ),
      const SizedBox(height: 8),
      if (asking == null)
        Align(
          alignment: Alignment.centerRight,
          child: FilledButton(
            key: const ValueKey('ask-button'),
            onPressed: () => unawaited(_ask()),
            child: Text(t.phoneAskButton),
          ),
        )
      else ...[
        Text(asking, style: theme.textTheme.titleSmall),
        const SizedBox(height: 8),
        Row(
          key: const ValueKey('ask-waiting'),
          children: [
            const SizedBox.square(
              dimension: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
            const SizedBox(width: 12),
            Expanded(child: Text(t.phoneAskAsking(name))),
          ],
        ),
      ],
      for (final (i, asked) in _asked.indexed) ..._answer(t, asked, i),
    ];
  }

  /// One question and its answer, whose links open their Notes.
  List<Widget> _answer(AppLocalizations t, _Asked asked, int i) {
    final theme = Theme.of(context);
    final weak = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    final kept = asked.kept;
    return [
      const SizedBox(height: 16),
      const Divider(),
      const SizedBox(height: 8),
      Text(asked.question, style: theme.textTheme.titleSmall),
      const SizedBox(height: 8),
      MarkedText(
        asked.answer.text,
        key: ValueKey('ask-answer-$i'),
        format: TextFormats.markdown,
      ),
      if (asked.answer.cut) ...[
        const SizedBox(height: 4),
        Text(t.phoneAskCut, style: weak),
      ],
      if (asked.answer.unchecked) ...[
        const SizedBox(height: 4),
        Text(
          t.phoneAskUnchecked,
          key: ValueKey('ask-unchecked-$i'),
          style: weak,
        ),
      ],
      if (asked.sources.isNotEmpty) ...[
        const SizedBox(height: 8),
        Text(t.phoneAskFromNotes, style: weak),
        for (final (id, title) in asked.sources)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              key: ValueKey('ask-source-$i-$id'),
              onPressed: () => _open(id),
              icon: const Icon(Icons.notes),
              label: Text(title),
            ),
          ),
      ],
      const SizedBox(height: 8),
      Align(
        alignment: Alignment.centerLeft,
        child: kept == null
            ? OutlinedButton.icon(
                key: ValueKey('ask-keep-$i'),
                onPressed: () => _keep(asked),
                icon: const Icon(Icons.note_add_outlined),
                label: Text(t.phoneAskKeep),
              )
            : TextButton.icon(
                key: ValueKey('ask-kept-$i'),
                onPressed: () => _open(kept),
                icon: const Icon(Icons.check),
                label: Text(t.phoneAskKept),
              ),
      ),
    ];
  }
}

/// The provider as the phone names it: Anthropic by its name, an
/// OpenAI-compatible one by its host, as the desktop does.
String providerName(DesktopProvider? provider) => switch (provider) {
  null => '',
  DesktopProvider(protocol: AskProtocol.anthropic) => 'Anthropic',
  DesktopProvider(:final host) => host,
};

/// What the desktop lacks to ask its provider, and where to set it.
String unreadyLine(AppLocalizations t, AskUnready why) => switch (why) {
  AskUnready.noModel => t.phoneAskUnreadyModel,
  AskUnready.noKey => t.phoneAskUnreadyKey,
  AskUnready.badUrl => t.phoneAskUnreadyUrl,
};

/// Why no desktop took the question or named its provider, and what to
/// do, one sentence each.
(String, String) askTroubleWhy(AppLocalizations t, AskTrouble trouble) =>
    switch (trouble) {
      AskTrouble.notOwn => (t.phoneAskNoComputer, t.phoneAskNoComputerFix),
      AskTrouble.unreachable => (
        t.phoneAskUnreachable,
        t.phoneAskUnreachableFix,
      ),
      AskTrouble.wrongHost => (t.phoneAskWrongHost, t.phoneAskWrongHostFix),
      AskTrouble.declined => (t.phoneAskDeclined, t.phoneAskDeclinedFix),
      AskTrouble.notOffered => (t.phoneAskNotOffered, t.phoneAskNotOfferedFix),
      AskTrouble.timedOut => (t.phoneAskTimedOut, t.phoneAskTimedOutFix),
      AskTrouble.failed => (t.phoneAskBrokeOff, t.nothingLostFix),
    };

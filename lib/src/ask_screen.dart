import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
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
  String? id,
});

/// How the phone learns how far a paired Device is with the question
/// [id], or stops it; the app passes the core's [askProgress] and
/// [stopAsking].
typedef ProgressCall = Future<AskProgress?> Function(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  required String id,
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
  String? id,
}) => askDesktop(
  store,
  device,
  at: at,
  question: question,
  earlier: earlier,
  id: id,
);

Future<AskProgress?> _askProgress(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  required String id,
}) => askProgress(store, device, at: at, id: id);

Future<AskProgress?> _stopAsking(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  required String id,
}) => stopAsking(store, device, at: at, id: id);

/// The user's own desktop as the phone asks it (#267, ADR 0051): which
/// provider it asks, and the question. Throws an [AskException] when no
/// desktop of the user's answers.
abstract interface class DesktopAsking {
  /// The name of the Device that answered last, if one did.
  String? get computer;

  Future<DesktopProvider> provider();

  /// Asks the Device that named the provider last the question [id]; a
  /// question once sent is not sent again elsewhere.
  Future<AskReply> ask(
    String question,
    List<AskExchange> earlier, {
    required String id,
  });

  /// How far that Device is with the question [id] (#317); null when it
  /// does not tell, as a desktop of 0.6.0.
  Future<AskProgress?> progress(String id);

  /// Stops the question [id] on that Device at its next round (#317).
  Future<void> stop(String id);
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
  final ProgressCall progressCall;
  final ProgressCall stopCall;

  PairedDevice? _device;

  PairedDesktops(
    this.store, {
    required this.found,
    this.providerCall = _desktopProvider,
    this.questionCall = _askDesktop,
    this.progressCall = _askProgress,
    this.stopCall = _stopAsking,
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
  Future<AskReply> ask(
    String question,
    List<AskExchange> earlier, {
    required String id,
  }) async {
    final device = _device;
    if (device == null) throw const AskException(AskTrouble.notOwn);
    return questionCall(
      store,
      device,
      at: _at(device),
      question: question,
      earlier: earlier,
      id: id,
    );
  }

  @override
  Future<AskProgress?> progress(String id) async {
    final device = _device;
    if (device == null) return null;
    return progressCall(store, device, at: _at(device), id: id);
  }

  @override
  Future<void> stop(String id) async {
    final device = _device;
    if (device == null) return;
    await stopCall(store, device, at: _at(device), id: id);
  }
}

/// Opens the Ask page, asking the user's own desktop through [asking],
/// or over the paired connection when none is given. The connection is
/// made once, here: the route builds its page again whenever what is
/// above it changes, such as an announcement showing at the top, and a
/// connection made anew there would have forgotten the computer that
/// named its provider, so that the question waiting went nowhere (#317).
Future<void> openAsk(
  BuildContext context,
  Store store, {
  DesktopAsking? asking,
}) {
  final auto = AutoSyncScope.of(context);
  final desktops =
      asking ?? PairedDesktops(store, found: (id) => auto?.foundAt(id));
  return Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => AskScreen(store: store, asking: desktops),
    ),
  );
}

/// A question not answered (#317): why, how to fix it, and what the
/// desktop or the provider said, which Details shows with Report.
class _Failed {
  final String question;
  final String cause;
  final String fix;
  final String details;

  const _Failed(this.question, this.cause, this.fix, this.details);
}

/// A question asked and its answer.
class _Asked {
  final String question;
  final DesktopAnswer answer;

  /// The Notes its answer links that this phone shows, by id with their
  /// first lines.
  final List<(String, String)> sources;

  /// The other Notes the desktop's model read for it that this phone
  /// shows, in the order read: "Also read" (#314).
  final List<(String, String)> alsoRead;

  /// The Note it was kept as.
  String? kept;

  _Asked(this.question, this.answer, this.sources, this.alsoRead);
}

/// What the page shows above its answers: in [ready] the question field,
/// which says what the computer lacks while its provider cannot be asked.
enum _Phase { looking, unreachable, consent, ready }

/// Ask, from the Agenda's bar (#267, ADR 0051): the phone runs no model
/// and talks to no provider; it asks the user's own desktop, which runs
/// the question as its Ask panel does and answers with links to the
/// Notes it came from, which open here. Before the first question to a
/// provider the page names it and what is sent, and asks the user's
/// permission (App Store guideline 5.1.2(i)); a provider or model the
/// user did not allow is asked about again, and nothing is sent before.
/// A follow-up is asked after the earlier answers; Start over drops them,
/// as leaving the page does. Keep as Note saves a question and its answer
/// as a Markdown Note in Unsorted, with no Nudge. While the computer's
/// provider cannot be asked, the field says why and what to do, and Ask
/// waits. While a question runs, a status line under it says whom it asks
/// and how many Notes were read, with Stop (#317). A question not
/// answered goes back into the field, and its cause and fix stay under
/// it until the next is asked.
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

  /// The question waiting for its answer, and its id.
  String? _asking;
  String? _askingId;

  /// How many Notes the computer's model read for it so far (#317).
  var _read = 0;
  Timer? _polling;

  /// The last question not answered, until the next is asked (#317).
  _Failed? _failed;

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
    _polling?.cancel();
    _question.dispose();
    _focus.dispose();
    super.dispose();
  }

  /// How often the page asks how far the computer is.
  static const _pollEvery = Duration(seconds: 2);

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

  /// Shows [provider]: asked about first, unless the user allowed it or
  /// it cannot be asked yet, which the field says.
  void _show(DesktopProvider provider) {
    _provider = provider;
    _phase = !provider.ready || widget.store.askAllowed(provider)
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

  /// Whether the question typed may be asked now.
  bool get _mayAsk => _asking == null && (_provider?.ready ?? false);

  /// Asks the question typed, once the provider the desktop asks now is
  /// one the user allowed; else the question stays in the field. Its
  /// failure stays under it until the next is asked (#317).
  Future<void> _ask() async {
    final question = _question.text.trim();
    if (question.isEmpty || !_mayAsk) return;
    final t = context.t;
    // One connection for the whole question, whatever the page is
    // rebuilt with meanwhile.
    final asking = widget.asking;
    final id = newAskId();
    setState(() {
      _asking = question;
      _askingId = id;
      _read = 0;
      _failed = null;
    });
    _question.clear();
    bool current() => mounted && _askingId == id;
    void back() {
      _stopPolling();
      _asking = null;
      _askingId = null;
      if (_question.text.isEmpty) _question.text = question;
    }

    try {
      // Read again before each question: the desktop may ask another
      // provider since.
      final provider = await asking.provider();
      if (!current()) return;
      if (!provider.ready || !widget.store.askAllowed(provider)) {
        setState(() {
          back();
          _show(provider);
        });
        return;
      }
      if (provider != _provider) setState(() => _provider = provider);
      _poll(asking, id);
      final reply = await asking.ask(question, _earlier, id: id);
      // Stopped meanwhile, or another question asked: dropped.
      if (!current()) return;
      final answer = reply.answer;
      setState(() {
        _review = reply.review;
        if (answer == null) {
          back();
          final (cause, fix, details) = _failureWhy(t, reply);
          _failed = _Failed(question, cause, fix, details);
        } else {
          _stopPolling();
          _asking = null;
          _askingId = null;
          _asked.insert(
            0,
            _Asked(
              question,
              answer,
              _sources(answer.notes),
              _sources(answer.alsoRead),
            ),
          );
        }
      });
    } on Object catch (e) {
      if (!current()) return;
      final trouble = e is AskException ? e.trouble : AskTrouble.failed;
      final (cause, fix) = askTroubleWhy(t, trouble);
      setState(() {
        back();
        _failed = _Failed(
          question,
          cause,
          fix,
          e is AskException ? 'ask: ${trouble.name}' : '$e',
        );
      });
    }
  }

  /// Asks how far the computer is with the question [id] now and then
  /// while it runs; a computer that does not tell is not asked again.
  void _poll(DesktopAsking asking, String id) {
    _stopPolling();
    var waiting = false;
    _polling = Timer.periodic(_pollEvery, (_) async {
      if (waiting) return;
      waiting = true;
      try {
        final progress = await asking.progress(id);
        if (!mounted || _askingId != id) return;
        if (progress == null) {
          _stopPolling();
        } else if (progress.asking && progress.read != _read) {
          setState(() => _read = progress.read);
        }
      } on Object {
        // The answer says what went wrong; the line stays as it was.
      } finally {
        waiting = false;
      }
    });
  }

  void _stopPolling() {
    _polling?.cancel();
    _polling = null;
  }

  /// Stops waiting for the answer: the question goes back into the field,
  /// what comes for it later is dropped, and the computer is told to stop
  /// it at its next round (#317).
  void _stop() {
    final id = _askingId;
    final question = _asking;
    if (id == null || question == null) return;
    setState(() {
      _stopPolling();
      _asking = null;
      _askingId = null;
      if (_question.text.isEmpty) _question.text = question;
    });
    unawaited(widget.asking.stop(id).catchError((Object _) {}));
    _focus.requestFocus();
  }

  void _details(_Failed failed) => unawaited(
    showFailure(
      context,
      title: context.t.phoneAskFailedTitle,
      cause: failed.cause,
      fix: failed.fix,
      details: failed.details,
    ),
  );

  /// Why the desktop gave no answer, how to fix it, and what the
  /// provider said.
  (String, String, String) _failureWhy(AppLocalizations t, AskReply reply) {
    final provider = providerName(_provider);
    final failure = reply.failure ?? AskFailure.unknown;
    final (cause, fix) = switch (failure) {
      AskFailure.unready => switch (reply.unready) {
        final why? => (
          unreadyCause(t, why, widget.asking.computer ?? '', provider),
          unreadyLine(t, why),
        ),
        null => (t.phoneAskUnreadyCause(provider), t.phoneAskUnreadyFix),
      },
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

  /// The Notes [ids] of an answer that this phone shows, by their first
  /// lines: not one it does not hold, nor one in the Trash, merged, or in
  /// a locked Area while it is closed, whose link in the text still asks
  /// for the PIN.
  List<(String, String)> _sources(List<String> ids) {
    final notes = Notes(widget.store);
    final places = Places(widget.store);
    final lock = LockScope.of(context);
    return [
      for (final id in ids)
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
      _failed = null;
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

  /// What the computer lacks to ask its provider, and where to set it,
  /// for the question field while it cannot be asked (#317).
  String? _unreadyText(AppLocalizations t) {
    final provider = _provider;
    final why = provider?.unready;
    if (provider == null || why == null) return null;
    return '${unreadyCause(t, why, widget.asking.computer ?? '', providerName(provider))} '
        '${unreadyLine(t, why)}';
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
    final computer = widget.asking.computer ?? '';
    final asking = _asking;
    final unready = _unreadyText(t);
    final failed = _failed;
    return [
      if (unready == null)
        Text(
          t.phoneAskSends(computer, name, provider?.model ?? ''),
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
          errorText: unready,
          errorMaxLines: 6,
        ),
      ),
      const SizedBox(height: 8),
      if (asking == null)
        Wrap(
          alignment: WrapAlignment.end,
          spacing: 8,
          runSpacing: 8,
          children: [
            if (unready != null) _tryAgain(t),
            FilledButton(
              key: const ValueKey('ask-button'),
              onPressed: _mayAsk ? () => unawaited(_ask()) : null,
              child: Text(t.phoneAskButton),
            ),
          ],
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
            Expanded(
              child: Text(
                _read == 0
                    ? t.phoneAskAsking(computer)
                    : t.phoneAskAskingReading(computer, _read),
                key: const ValueKey('ask-status'),
              ),
            ),
            TextButton(
              key: const ValueKey('ask-stop'),
              onPressed: _stop,
              child: Text(t.phoneAskStop),
            ),
          ],
        ),
      ],
      if (asking == null && failed != null) ..._failure(t, failed),
      for (final (i, asked) in _asked.indexed) ..._answer(t, asked, i),
    ];
  }

  /// The last question not answered, with its cause and fix (#317).
  List<Widget> _failure(AppLocalizations t, _Failed failed) {
    final theme = Theme.of(context);
    return [
      const SizedBox(height: 16),
      const Divider(),
      const SizedBox(height: 8),
      Text(
        failed.question,
        key: const ValueKey('ask-failure-question'),
        style: theme.textTheme.titleSmall,
      ),
      const SizedBox(height: 8),
      Text(
        failed.cause,
        key: const ValueKey('ask-failure'),
        style: TextStyle(color: theme.colorScheme.error),
      ),
      const SizedBox(height: 4),
      Text(failed.fix, key: const ValueKey('ask-failure-fix')),
      Align(
        alignment: Alignment.centerLeft,
        child: TextButton(
          key: const ValueKey('ask-failure-details'),
          onPressed: () => _details(failed),
          child: Text(t.phoneAskFailureDetails),
        ),
      ),
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
      for (final (heading, key, notes) in [
        (t.phoneAskFromNotes, 'ask-source', asked.sources),
        (t.phoneAskAlsoRead, 'ask-also-read', asked.alsoRead),
      ])
        if (notes.isNotEmpty) ...[
          const SizedBox(height: 8),
          Text(heading, key: ValueKey('$key-$i'), style: weak),
          for (final (id, title) in notes)
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                key: ValueKey('$key-$i-$id'),
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

/// Why the desktop [computer] cannot ask [provider] yet (#317).
String unreadyCause(
  AppLocalizations t,
  AskUnready why,
  String computer,
  String provider,
) => switch (why) {
  AskUnready.noModel => t.phoneAskUnreadyModelCause(computer),
  AskUnready.noKey => t.phoneAskUnreadyKeyCause(provider, computer),
  AskUnready.badUrl => t.phoneAskUnreadyUrlCause(provider, computer),
  AskUnready.aiOff => t.phoneAskUnreadyAiOffCause(computer),
};

/// What the desktop lacks to ask its provider, and where to set it.
String unreadyLine(AppLocalizations t, AskUnready why) => switch (why) {
  AskUnready.noModel => t.phoneAskUnreadyModel,
  AskUnready.noKey => t.phoneAskUnreadyKey,
  AskUnready.badUrl => t.phoneAskUnreadyUrl,
  AskUnready.aiOff => t.phoneAskUnreadyAiOff,
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

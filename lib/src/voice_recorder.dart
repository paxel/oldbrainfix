import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'recording_pause.dart';
import 'sync_line.dart';
import 'voice_platform.dart';

/// One voice recording into a Note, for the quick note and the Note
/// editor alike: it starts at once, the microphone asked for with an
/// explanation the first time, and ends by one big stop button or by its
/// screen going, and what was recorded is kept with [keep] (UI laws,
/// "Capture"). With "End a recording after a pause" on (obf-1053), a
/// pause of its length after something was heard ends and keeps it as
/// Stop does. On the iPhone the recording goes on with the app in the
/// background and the phone locked (#350), and ends after such a pause
/// there too; on Android, which records under no foreground service,
/// leaving the app or locking the phone ends and keeps it (#368).
class VoiceRecorder {
  /// How often the microphone's level is asked while a pause is to end
  /// the recording.
  static const levelEvery = Duration(milliseconds: 200);

  /// The level below which the microphone counts as quiet: 0.02 of the
  /// loudest, about −34 dBFS at its peak, under speech and above a quiet
  /// room.
  static const quietBelow = 0.02;

  final VoicePlatform voice;

  /// Writes what was recorded into its Note.
  final void Function(Uint8List bytes) keep;

  /// Runs once a recording ended and [keep] had it.
  final VoidCallback? ended;

  /// The pause after which a recording ends on its own, read as each
  /// starts; null, as by default, ignores pauses (obf-1053).
  final Duration? Function()? pause;

  /// The running time, counted while it records.
  final elapsed = ValueNotifier(Duration.zero);

  /// Ends the recording on Android once the app is left or the phone
  /// locked (#368).
  late final AppLifecycleListener _lifecycle;

  /// The recorder whose recording runs, if one does. A recording goes on
  /// in the background on the iPhone (#350), and another screen can ask
  /// for one meanwhile, as a voice Capture over the editor's: it ends and
  /// keeps that one first, as the microphone records one at a time.
  static VoiceRecorder? _running;

  /// The recording being started and then running, whether it did start;
  /// null while none runs.
  Future<bool>? _recording;

  /// Ending the recording and keeping it, once asked for.
  Future<void>? _stopping;

  /// Counts the recording's seconds while it runs.
  Timer? _ticker;

  /// Asks the microphone's level every [levelEvery] while a pause is to
  /// end the recording.
  Timer? _listening;

  VoiceRecorder(this.voice, {required this.keep, this.ended, this.pause}) {
    _lifecycle = AppLifecycleListener(
      onStateChange: (state) {
        if (defaultTargetPlatform == TargetPlatform.iOS) return;
        if (state == AppLifecycleState.paused ||
            state == AppLifecycleState.hidden) {
          unawaited(stop());
        }
      },
    );
  }

  /// Whether a recording is being started or runs.
  bool get recording => _recording != null;

  /// Starts recording, asking for the microphone first, with an
  /// explanation, while it is not allowed; any playing ends, and another
  /// recording running is ended and kept first. [wanted] is asked after
  /// each wait: once false, as when its screen is being left, nothing
  /// starts. True once it records; false when the microphone is not
  /// allowed or does not work; null when it was not wanted any more or
  /// was stopped meanwhile.
  Future<bool?> start(
    BuildContext context, {
    required bool Function() wanted,
  }) async {
    if (_recording != null) return null;
    elapsed.value = Duration.zero;
    var allowed = await voice.granted();
    if (!context.mounted || !wanted()) return null;
    if (!allowed) allowed = await explainAndAskMicrophone(context, voice);
    if (!context.mounted || !wanted()) return null;
    if (!allowed) return false;
    // Recording ends any playing, and the play buttons show it.
    await VoiceScope.playbackOf(context)?.stop();
    if (!context.mounted || !wanted()) return null;
    final running = _running;
    if (running != null && running != this) await running.stop();
    if (!context.mounted || !wanted() || _recording != null) return null;
    final recording = _recording = voice.start();
    _running = this;
    final started = await recording;
    // Stopped meanwhile, as by leaving: nothing to count.
    if (_recording != recording) return null;
    if (!started) {
      _recording = null;
      if (_running == this) _running = null;
      return false;
    }
    _ticker = Timer.periodic(const Duration(seconds: 1), (timer) {
      elapsed.value = Duration(seconds: timer.tick);
    });
    if (pause?.call() case final pause?) _listen(recording, pause);
    return true;
  }

  /// Ends [recording] once the microphone was quiet for [pause] after it
  /// heard something: a recording not spoken into yet goes on. The quiet
  /// is counted by the timer's ticks, a level the phone cannot tell
  /// counting as none, so a late or failing ask never ends it early.
  void _listen(Future<bool> recording, Duration pause) {
    final quietTicks = (pause.inMilliseconds / levelEvery.inMilliseconds)
        .ceil();
    var heard = false;
    var asking = false;
    int? quietFrom;
    _listening = Timer.periodic(levelEvery, (timer) async {
      if (asking) return;
      asking = true;
      final level = await voice.level();
      asking = false;
      if (_recording != recording) return;
      if (level == null) {
        quietFrom = null;
      } else if (level >= quietBelow) {
        heard = true;
        quietFrom = null;
      } else if (heard) {
        final from = quietFrom ??= timer.tick;
        if (timer.tick - from + 1 >= quietTicks) unawaited(stop());
      }
    });
  }

  /// Ends the recording and keeps it; the same ending when asked again,
  /// nothing when none runs.
  Future<void> stop() {
    final recording = _recording;
    if (recording != null) {
      _recording = null;
      if (_running == this) _running = null;
      _ticker?.cancel();
      _listening?.cancel();
      _stopping = _keep(recording);
    }
    return _stopping ?? Future.value();
  }

  /// Waits for [recording] to have started, ends it and keeps what was
  /// recorded.
  Future<void> _keep(Future<bool> recording) async {
    if (await recording) {
      final bytes = await voice.stop();
      if (bytes != null) keep(bytes);
    }
    _stopping = null;
    ended?.call();
  }

  /// Ends a recording still running and keeps it, as when its screen was
  /// closed from elsewhere, so the screen times out again.
  void dispose() {
    _lifecycle.dispose();
    if (_recording != null) unawaited(stop());
    _ticker?.cancel();
    _listening?.cancel();
    elapsed.dispose();
  }
}

/// Voice on its screen: [typed], the text typed so far, then the running
/// time and the big stop button; or, while [micMissing], why there is
/// none, with "Allow access" ([onRetry]) and "Type instead"
/// ([onTypeInstead]).
class VoicePanel extends StatelessWidget {
  final String typed;
  final bool micMissing;
  final ValueListenable<Duration> elapsed;
  final VoidCallback onStop;
  final VoidCallback onRetry;
  final VoidCallback onTypeInstead;

  const VoicePanel({
    super.key,
    this.typed = '',
    required this.micMissing,
    required this.elapsed,
    required this.onStop,
    required this.onRetry,
    required this.onTypeInstead,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    return Column(
      children: [
        if (typed.isNotEmpty)
          Align(
            alignment: Alignment.centerLeft,
            child: Text(
              typed,
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodyLarge,
            ),
          ),
        Expanded(
          child: Center(
            child: SingleChildScrollView(
              child: micMissing
                  ? Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(t.microphoneMissing, textAlign: TextAlign.center),
                        const SizedBox(height: 16),
                        FilledButton(
                          onPressed: onRetry,
                          child: Text(t.allowAccess),
                        ),
                        TextButton(
                          onPressed: onTypeInstead,
                          child: Text(t.typeInstead),
                        ),
                      ],
                    )
                  : Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(t.recording, style: theme.textTheme.titleMedium),
                        const SizedBox(height: 8),
                        ValueListenableBuilder(
                          valueListenable: elapsed,
                          builder: (context, elapsed, _) => Text(
                            elapsedText(elapsed),
                            key: const ValueKey('elapsed'),
                            style: theme.textTheme.displayMedium,
                          ),
                        ),
                        const SizedBox(height: 24),
                        SizedBox.square(
                          dimension: 160,
                          child: FilledButton(
                            key: const ValueKey('stopRecording'),
                            onPressed: onStop,
                            style: FilledButton.styleFrom(
                              shape: const CircleBorder(),
                              padding: EdgeInsets.zero,
                            ),
                            child: Icon(
                              Icons.stop,
                              size: 96,
                              semanticLabel: t.stopRecording,
                            ),
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(t.stopRecording),
                      ],
                    ),
            ),
          ),
        ),
      ],
    );
  }
}

/// Records a voice recording into the Note [noteId] from its editor: the
/// recording starts at once on a page of its own with the big stop
/// button; Stop or Back end it, on Android also leaving the app or locking
/// the phone (#368), write it into the Note, also when the editor was left
/// meanwhile, and close the page.
Future<void> recordInto(BuildContext context, Notes notes, String noteId) =>
    Navigator.of(context).push<void>(
      MaterialPageRoute(
        builder: (_) => _RecordScreen(notes: notes, noteId: noteId),
      ),
    );

class _RecordScreen extends StatefulWidget {
  final Notes notes;
  final String noteId;

  const _RecordScreen({required this.notes, required this.noteId});

  @override
  State<_RecordScreen> createState() => _RecordScreenState();
}

class _RecordScreenState extends State<_RecordScreen> {
  late final _recorder = VoiceRecorder(
    VoiceScope.of(context),
    keep: (bytes) => widget.notes.addRecording(widget.noteId, bytes),
    ended: _close,
    pause: () => recordingPause(widget.notes.store),
  );

  /// Whether the microphone was found not allowed or not working.
  var _micMissing = false;

  /// Set once leaving began, so nothing starts and the page closes once.
  var _leaving = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _start());
  }

  @override
  void dispose() {
    _leaving = true;
    // Closed from elsewhere while recording: it ends and is kept.
    _recorder.dispose();
    super.dispose();
  }

  Future<void> _start() async {
    if (!mounted) return;
    setState(() => _micMissing = false);
    final started = await _recorder.start(
      context,
      wanted: () => mounted && !_leaving,
    );
    if (!mounted || started != false) return;
    setState(() => _micMissing = true);
  }

  /// Ends the recording, which is kept, and the page closes after.
  Future<void> _stop() async {
    if (_recorder.recording) return _recorder.stop();
    _close();
  }

  /// Closes the page once, back to the editor; also from under another
  /// page, as a voice Capture opened meanwhile that ended this recording.
  void _close() {
    if (!mounted || _leaving) return;
    _leaving = true;
    final route = ModalRoute.of(context);
    final navigator = Navigator.of(context);
    if (route == null || route.isCurrent) {
      navigator.pop();
    } else {
      navigator.removeRoute(route);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: false,
    // Back ends and keeps the recording, as Stop does.
    onPopInvokedWithResult: (didPop, _) {
      if (!didPop) unawaited(_stop());
    },
    child: Scaffold(
      appBar: AppBar(flexibleSpace: const SyncLine()),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: VoicePanel(
            micMissing: _micMissing,
            elapsed: _recorder.elapsed,
            onStop: () => unawaited(_stop()),
            onRetry: () => unawaited(_start()),
            onTypeInstead: _close,
          ),
        ),
      ),
    ),
  );
}

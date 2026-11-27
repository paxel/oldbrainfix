import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// The platform seam for voice Notes: the microphone and playing a
/// recording back (android/…/Voice.kt). Widget tests hand in a fake.
abstract interface class VoicePlatform {
  /// Whether this Device can record at all; the mic button only shows
  /// where it can.
  bool get available;

  /// Whether the user allowed the microphone. Never asks.
  Future<bool> granted();

  /// Asks the user for the microphone; whether it is allowed now.
  Future<bool> request();

  /// Starts recording; whether it did. The screen stays on while it
  /// runs (#350).
  Future<bool> start();

  /// Ends the recording; its bytes, or null when nothing usable was
  /// recorded.
  Future<Uint8List?> stop();

  /// How loud the microphone was at its loudest since the last ask while
  /// a recording runs, 0 silent to 1 the loudest it records; null while
  /// none runs or when the phone cannot tell (obf-1053).
  Future<double?> level();

  /// Plays the recording [bytes], stopping whatever played before.
  /// Whether it plays.
  Future<bool> play(Uint8List bytes);

  /// Stops playing.
  Future<void> stopPlaying();

  /// Plays recordings at [speed], 1 being normal speed: the one playing
  /// at once, and every one after it.
  Future<void> setSpeed(double speed);

  /// A recording played to its end.
  Stream<void> get playbackEnded;
}

/// No microphone: the default where none is handed in.
class NoVoice implements VoicePlatform {
  const NoVoice();

  @override
  bool get available => false;

  @override
  Future<bool> granted() async => false;

  @override
  Future<bool> request() async => false;

  @override
  Future<bool> start() async => false;

  @override
  Future<Uint8List?> stop() async => null;

  @override
  Future<double?> level() async => null;

  @override
  Future<bool> play(Uint8List bytes) async => false;

  @override
  Future<void> stopPlaying() async {}

  @override
  Future<void> setSpeed(double speed) async {}

  @override
  Stream<void> get playbackEnded => const Stream.empty();
}

/// Android's own MediaRecorder and MediaPlayer behind the app's method
/// channel; no Google library.
class AndroidVoice implements VoicePlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/voice');
  final _ended = StreamController<void>.broadcast();

  AndroidVoice() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'played') _ended.add(null);
    });
  }

  @override
  bool get available => true;

  @override
  Future<bool> granted() => _ask('granted');

  @override
  Future<bool> request() => _ask('request');

  @override
  Future<bool> start() => _ask('start');

  @override
  Future<Uint8List?> stop() async {
    try {
      return await _channel.invokeMethod<Uint8List>('stop');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  @override
  Future<double?> level() async {
    try {
      return await _channel.invokeMethod<double>('level');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  @override
  Future<bool> play(Uint8List bytes) => _ask('play', bytes);

  @override
  Future<void> stopPlaying() => _ask('stopPlaying');

  @override
  Future<void> setSpeed(double speed) => _ask('speed', speed);

  @override
  Stream<void> get playbackEnded => _ended.stream;

  static Future<bool> _ask(String method, [Object? arguments]) async {
    try {
      return await _channel.invokeMethod<bool>(method, arguments) ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }
}

/// The one recording playing at a time, by its hash, for every play
/// button to show; null while none plays.
class Playback extends ValueNotifier<String?> {
  final VoicePlatform voice;

  /// The bytes of a stored recording by its hash, or null when they are
  /// not on this Device.
  final Uint8List? Function(String hash) bytes;

  /// How fast recordings play, for every speed label to show.
  final ValueNotifier<double> speed;

  /// Keeps a chosen speed for the next start.
  final ValueChanged<double>? onSpeed;

  /// The Note pages open now, for the playing bar to stay off the page
  /// of the Note playing (obf-1155).
  final pages = NotePages();

  /// The Note whose recording plays, for the playing bar to name and
  /// open (obf-1155); null while none plays or one plays of no Note yet.
  String? get noteId => value == null ? null : _noteId;
  String? _noteId;

  late final StreamSubscription<void> _ended;

  Playback({
    required this.voice,
    required this.bytes,
    double speed = 1,
    this.onSpeed,
  }) : speed = ValueNotifier(speed),
       super(null) {
    _ended = voice.playbackEnded.listen((_) => value = null);
  }

  /// Plays the recording [hash] of the Note [noteId], or stops it when
  /// it is the one playing.
  Future<void> toggle(String hash, {String? noteId}) async {
    if (value == hash) return stop();
    final recording = bytes(hash);
    if (recording == null) return;
    // The player may not know the speed yet, as after a start.
    await voice.setSpeed(speed.value);
    final plays = await voice.play(recording);
    _noteId = plays ? noteId : null;
    value = plays ? hash : null;
  }

  /// Plays recordings at [rate] from now on, the one playing included,
  /// and keeps it.
  Future<void> setSpeed(double rate) async {
    if (rate == speed.value) return;
    speed.value = rate;
    onSpeed?.call(rate);
    await voice.setSpeed(rate);
  }

  /// Stops playing, if anything plays.
  Future<void> stop() async {
    if (value == null) return;
    value = null;
    await voice.stopPlaying();
  }

  @override
  void dispose() {
    unawaited(_ended.cancel());
    speed.dispose();
    pages.dispose();
    super.dispose();
  }
}

/// The Note pages open, the editor's and the big screen's Note dialog,
/// by their route (obf-1155).
class NotePages extends ChangeNotifier {
  final _open = <Route<dynamic>, String>{};
  var _disposed = false;

  /// The Note the page [route] shows, or null when it shows none.
  String? noteOf(Route<dynamic> route) => _open[route];

  void _shown(Route<dynamic> route, String noteId) {
    if (_disposed) return;
    _open[route] = noteId;
    notifyListeners();
  }

  void _gone(Route<dynamic> route) {
    if (_disposed || _open.remove(route) == null) return;
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

/// Tells the playing bar that [child] is the page of the Note [noteId],
/// which shows its own stop button (obf-1155).
class NotePageMark extends StatefulWidget {
  /// Null while the page holds no Note yet, as Capture before it saves.
  final String? noteId;
  final Widget child;

  const NotePageMark({super.key, required this.noteId, required this.child});

  @override
  State<NotePageMark> createState() => _NotePageMarkState();
}

class _NotePageMarkState extends State<NotePageMark> {
  NotePages? _pages;
  Route<dynamic>? _route;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _pages = VoiceScope.playbackOf(context)?.pages;
    // Called again as the page is covered or uncovered.
    _route = ModalRoute.of(context);
    _tell();
  }

  @override
  void didUpdateWidget(NotePageMark old) {
    super.didUpdateWidget(old);
    if (old.noteId != widget.noteId) _tell();
  }

  @override
  void dispose() {
    final (pages, route) = (_pages, _route);
    if (pages != null && route != null) {
      // Not while the tree is built: the bar is built anew.
      WidgetsBinding.instance.addPostFrameCallback((_) => pages._gone(route));
    }
    super.dispose();
  }

  /// Tells the pages after this frame, as the bar may not be built anew
  /// while the tree is.
  void _tell() => WidgetsBinding.instance.addPostFrameCallback((_) {
    final (pages, route, noteId) = (_pages, _route, widget.noteId);
    if (!mounted || pages == null || route == null) return;
    if (noteId == null) {
      pages._gone(route);
    } else {
      pages._shown(route, noteId);
    }
  });

  @override
  Widget build(BuildContext context) => widget.child;
}

/// Hands the app's [VoicePlatform] and its [Playback] down to every
/// screen.
class VoiceScope extends InheritedWidget {
  final VoicePlatform voice;

  /// Null where nothing can be played back.
  final Playback? playback;

  const VoiceScope({
    super.key,
    required this.voice,
    this.playback,
    required super.child,
  });

  static VoiceScope? _of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<VoiceScope>();

  /// The app's [VoicePlatform]; read once, so it does not listen.
  static VoicePlatform of(BuildContext context) =>
      _of(context)?.voice ?? const NoVoice();

  /// The app's [Playback], or null where there is none.
  static Playback? playbackOf(BuildContext context) => _of(context)?.playback;

  @override
  bool updateShouldNotify(VoiceScope oldWidget) =>
      voice != oldWidget.voice || playback != oldWidget.playback;
}

/// Plays or stops the voice recording [hash]: a compact icon button on a
/// card, a labelled one in the editor when [compact] is false.
class RecordingButton extends StatelessWidget {
  final String hash;

  /// The Note the recording is shown in, for the playing bar (obf-1155);
  /// null for one of no Note yet.
  final String? noteId;
  final bool compact;

  const RecordingButton({
    super.key,
    required this.hash,
    required this.noteId,
    this.compact = true,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final playback = VoiceScope.playbackOf(context);
    if (playback == null) {
      return Icon(Icons.mic_none, semanticLabel: t.voiceRecording);
    }
    return ValueListenableBuilder<String?>(
      valueListenable: playback,
      builder: (context, playing, _) {
        final on = playing == hash;
        final icon = Icon(on ? Icons.stop : Icons.play_arrow);
        void onPressed() => unawaited(playback.toggle(hash, noteId: noteId));
        // The speed goes under the button where both do not fit.
        return Wrap(
          spacing: 4,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            if (compact)
              IconButton.filledTonal(
                tooltip: on ? t.stopPlaying : t.playRecording,
                onPressed: onPressed,
                icon: icon,
              )
            else
              OutlinedButton.icon(
                onPressed: onPressed,
                icon: icon,
                label: Text(on ? t.stopPlaying : t.voiceRecording),
              ),
            PlaybackSpeedButton(playback: playback),
          ],
        );
      },
    );
  }
}

/// [speed] as the locale [locale] writes it: "1", "1.5", or "1,5" in
/// German.
String speedNumber(String locale, double speed) =>
    NumberFormat.decimalPattern(locale).format(speed);

/// The current playback speed beside a play button ("1×"); a tap opens
/// the speeds where the finger is, the current one marked.
class PlaybackSpeedButton extends StatefulWidget {
  final Playback playback;

  const PlaybackSpeedButton({super.key, required this.playback});

  @override
  State<PlaybackSpeedButton> createState() => _PlaybackSpeedButtonState();
}

class _PlaybackSpeedButtonState extends State<PlaybackSpeedButton> {
  /// Where the finger last went down on the button.
  Offset? _finger;

  Future<void> _open() async {
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    final box = context.findRenderObject() as RenderBox?;
    if (overlay == null || box == null) return;
    // Opened without a finger, as by a screen reader: at the button.
    final at = _finger ?? box.localToGlobal(box.size.center(Offset.zero));
    _finger = null;
    final current = widget.playback.speed.value;
    final picked = await showMenu<double>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        PopupMenuItem(enabled: false, child: Text(t.playbackSpeed)),
        for (final speed in playbackSpeeds)
          PopupMenuItem(
            key: ValueKey('playbackSpeed-$speed'),
            value: speed,
            child: Semantics(
              checked: speed == current,
              inMutuallyExclusiveGroup: true,
              child: Row(
                children: [
                  Icon(
                    speed == current
                        ? Icons.radio_button_checked
                        : Icons.radio_button_unchecked,
                  ),
                  const SizedBox(width: 12),
                  Text(t.playbackSpeedOption(speedNumber(t.localeName, speed))),
                ],
              ),
            ),
          ),
      ],
    );
    if (picked == null || !mounted) return;
    await widget.playback.setSpeed(picked);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Listener(
      onPointerDown: (event) => _finger = event.position,
      child: ValueListenableBuilder<double>(
        valueListenable: widget.playback.speed,
        builder: (context, speed, _) => TextButton(
          key: const ValueKey('playbackSpeed'),
          onPressed: () => unawaited(_open()),
          child: Text(
            t.playbackSpeedLabel(speedNumber(t.localeName, speed)),
            semanticsLabel: t.playbackSpeedSemantics(
              speedNumber(t.localeName, speed),
            ),
          ),
        ),
      ),
    );
  }
}

/// Explains why voice Notes need the microphone and that the recording
/// stays on the user's Devices, then asks for it when the user agrees.
/// Returns whether it is allowed now.
Future<bool> explainAndAskMicrophone(
  BuildContext context,
  VoicePlatform voice,
) async {
  final agreed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        icon: const Icon(Icons.mic_none),
        title: Text(t.microphoneExplainTitle),
        content: Text(t.microphoneExplainBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.notNow),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.allowAccess),
          ),
        ],
      );
    },
  );
  if (agreed != true) return false;
  return voice.request();
}

/// [elapsed] as a recording's running time: minutes and seconds, hours
/// in front once there are any.
String elapsedText(Duration elapsed) {
  String two(int n) => n.toString().padLeft(2, '0');
  final seconds = two(elapsed.inSeconds % 60);
  if (elapsed.inHours > 0) {
    return '${elapsed.inHours}:${two(elapsed.inMinutes % 60)}:$seconds';
  }
  return '${elapsed.inMinutes}:$seconds';
}

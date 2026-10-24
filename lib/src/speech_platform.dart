import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// The speech model Transcripts are made with (ADR 0007): whisper.cpp's
/// multilingual `base` model, for English and German. Fetched once on
/// request from Old Brain Fix's own GitHub release, never from anywhere
/// else, or picked from a file; kept only when its SHA-256 is this one.
abstract final class WhisperModel {
  /// The release asset; GitHub may redirect it to its download host.
  static final url = Uri.parse(
    'https://github.com/paxel/oldbrainfix/releases/download/whisper-model-v1/ggml-base.bin',
  );

  /// The SHA-256 of whisper.cpp's `ggml-base.bin` (SHA-1
  /// 465707469ff3a37a2b9b8d8f89f2f99de7299dac in whisper.cpp's model
  /// list).
  static const sha256 =
      '60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe';

  /// Its size in megabytes (147,951,465 bytes), as told before the
  /// download.
  static const megabytes = 148;
}

/// How far a download has come: bytes so far, and of how many (null
/// while unknown).
typedef ModelProgress = ({int received, int? total});

/// The platform seam for Transcripts: Whisper and its model file
/// (android/…/Speech.kt). It moves and hashes the model; [SpeechModel]
/// decides what is kept. Widget tests hand in a fake.
abstract interface class SpeechPlatform {
  /// Whether this platform has a speech layer at all; the speech model
  /// page only shows where it has.
  bool get available;

  /// Whether Whisper runs on this Device.
  Future<bool> supported();

  /// The SHA-256 of the model in use, or null when there is none.
  Future<String?> modelHash();

  /// Downloads [url] to the side; its SHA-256, or null when it failed or
  /// was cancelled. Reports to [progress] on the way.
  Future<String?> download(Uri url);

  /// How far the running download has come.
  Stream<ModelProgress> get progress;

  /// Lets the user pick a model file and copies it to the side; its
  /// SHA-256, or null when none was picked or it could not be read.
  Future<String?> pick();

  /// Stops a download or copy running.
  Future<void> cancel();

  /// Keeps the model on the side as the one in use; whether it did.
  Future<bool> install();

  /// Drops the model on the side.
  Future<void> discard();

  /// Drops the model in use.
  Future<void> remove();

  /// The Transcript of the voice recording [bytes]; null when it could
  /// not be transcribed.
  Future<String?> transcribe(Uint8List bytes);

  /// Nothing more to transcribe for now: the model's memory can go.
  Future<void> idle();
}

/// No speech layer: the default where none is handed in.
class NoSpeech implements SpeechPlatform {
  const NoSpeech();

  @override
  bool get available => false;

  @override
  Future<bool> supported() async => false;

  @override
  Future<String?> modelHash() async => null;

  @override
  Future<String?> download(Uri url) async => null;

  @override
  Stream<ModelProgress> get progress => const Stream.empty();

  @override
  Future<String?> pick() async => null;

  @override
  Future<void> cancel() async {}

  @override
  Future<bool> install() async => false;

  @override
  Future<void> discard() async {}

  @override
  Future<void> remove() async {}

  @override
  Future<String?> transcribe(Uint8List bytes) async => null;

  @override
  Future<void> idle() async {}
}

/// whisper.cpp built into the app, behind the app's method channel; the
/// model downloaded with Android's own HTTPS client. No Google library.
class AndroidSpeech implements SpeechPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/speech');
  final _progress = StreamController<ModelProgress>.broadcast();

  AndroidSpeech() {
    _channel.setMethodCallHandler((call) async {
      if (call.method != 'progress') return;
      final args = call.arguments as Map<Object?, Object?>;
      final total = (args['total'] as num?)?.toInt() ?? -1;
      _progress.add((
        received: (args['received'] as num?)?.toInt() ?? 0,
        total: total > 0 ? total : null,
      ));
    });
  }

  @override
  bool get available => true;

  @override
  Future<bool> supported() async => await _ask<bool>('supported') ?? false;

  @override
  Future<String?> modelHash() => _ask<String>('modelHash');

  @override
  Future<String?> download(Uri url) => _ask<String>('download', '$url');

  @override
  Stream<ModelProgress> get progress => _progress.stream;

  @override
  Future<String?> pick() => _ask<String>('pick');

  @override
  Future<void> cancel() => _ask<void>('cancel');

  @override
  Future<bool> install() async => await _ask<bool>('install') ?? false;

  @override
  Future<void> discard() => _ask<void>('discard');

  @override
  Future<void> remove() => _ask<void>('remove');

  @override
  Future<String?> transcribe(Uint8List bytes) =>
      _ask<String>('transcribe', bytes);

  @override
  Future<void> idle() => _ask<void>('idle');

  static Future<T?> _ask<T>(String method, [Object? arguments]) async {
    try {
      return await _channel.invokeMethod<T>(method, arguments);
    } on PlatformException {
      return null;
    }
  }
}

/// Where the speech model stands on this Device.
enum ModelStatus {
  /// Looking whether there is one, and checking it.
  checking,

  /// Whisper does not run on this Device.
  unsupported,

  /// None yet: it can be downloaded or picked.
  missing,

  /// Being downloaded or copied in, then checked.
  fetching,

  /// Checked against [WhisperModel.sha256] and in use.
  ready,
}

/// How fetching the speech model ended.
enum ModelOutcome {
  /// Checked and in use.
  installed,

  /// Stopped by the user, or no file picked.
  cancelled,

  /// The download or copy broke off.
  failed,

  /// Not the pinned model: not kept.
  mismatch,
}

/// The speech model on this Device: checked against the pinned checksum
/// once per start before anything is transcribed, fetched on request,
/// and kept only when it is the pinned one.
class SpeechModel extends ChangeNotifier {
  final SpeechPlatform platform;

  late final StreamSubscription<ModelProgress> _progressing;
  var _status = ModelStatus.checking;
  ModelProgress _progress = (received: 0, total: null);
  var _downloading = false;
  var _cancelling = false;
  var _disposed = false;

  SpeechModel(this.platform) {
    _progressing = platform.progress.listen((progress) {
      _progress = progress;
      _changed();
    });
  }

  ModelStatus get status => _status;

  /// How far the download running has come.
  ModelProgress get progress => _progress;

  /// Whether the fetch running is a download, not a picked file.
  bool get downloading => _downloading;

  /// Looks whether a model is here and whether it is the pinned one; a
  /// model that is not is dropped, and so is a fetch cut off when the
  /// app was last closed.
  Future<void> check() async {
    _set(ModelStatus.checking);
    if (!await platform.supported()) return _set(ModelStatus.unsupported);
    await platform.discard();
    final hash = await platform.modelHash();
    if (hash == WhisperModel.sha256) return _set(ModelStatus.ready);
    if (hash != null) await platform.remove();
    _set(ModelStatus.missing);
  }

  /// Downloads the model from [WhisperModel.url].
  Future<ModelOutcome> download() => _fetch(
    () => platform.download(WhisperModel.url),
    downloading: true,
    nothing: ModelOutcome.failed,
  );

  /// Takes the model from a file the user picks.
  Future<ModelOutcome> pick() => _fetch(
    platform.pick,
    downloading: false,
    nothing: ModelOutcome.cancelled,
  );

  /// Stops the download or copy running.
  Future<void> cancel() async {
    if (_status != ModelStatus.fetching) return;
    _cancelling = true;
    await platform.cancel();
  }

  /// Fetches a model to the side with [fetch] and keeps it when its
  /// checksum is the pinned one; [nothing] is the outcome when [fetch]
  /// brings none and no one cancelled.
  Future<ModelOutcome> _fetch(
    Future<String?> Function() fetch, {
    required bool downloading,
    required ModelOutcome nothing,
  }) async {
    if (_status != ModelStatus.missing) return ModelOutcome.cancelled;
    _downloading = downloading;
    _cancelling = false;
    _progress = (received: 0, total: null);
    _set(ModelStatus.fetching);
    final hash = await fetch();
    final outcome = hash == null
        ? (_cancelling ? ModelOutcome.cancelled : nothing)
        : hash == WhisperModel.sha256
        ? (await platform.install()
              ? ModelOutcome.installed
              : ModelOutcome.failed)
        : ModelOutcome.mismatch;
    if (outcome != ModelOutcome.installed) await platform.discard();
    _set(
      outcome == ModelOutcome.installed
          ? ModelStatus.ready
          : ModelStatus.missing,
    );
    return outcome;
  }

  void _set(ModelStatus status) {
    _status = status;
    _changed();
  }

  void _changed() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    unawaited(_progressing.cancel());
    super.dispose();
  }
}

/// Transcribes voice recordings in the background, one at a time, while
/// the speech model is ready: each new one after its Note is saved, and
/// those from before the model came. A recording that cannot be
/// transcribed is left alone until the next start; it is never lost,
/// as the recording itself is always kept.
class Transcriber extends ChangeNotifier {
  final Notes notes;
  final SpeechModel model;
  final SpeechPlatform platform;

  /// Recordings that could not be transcribed since the start.
  final _skipped = <String>{};
  String? _current;
  var _running = false;
  var _queued = false;

  /// The log changed while transcribing: look once more at the end.
  var _again = false;
  var _disposed = false;

  Transcriber({
    required this.notes,
    required this.model,
    required this.platform,
  }) {
    notes.store.addListener(_kick);
    model.addListener(_kick);
    _kick();
  }

  /// The recording being transcribed, by its hash; null while none is.
  String? get current => _current;

  /// The Transcript of the recording [hash] of the Note [noteId] as it
  /// is now, or null when it has none.
  String? transcriptOf(String noteId, String hash) =>
      notes.byId(noteId)?.transcripts[hash];

  /// Starts transcribing once whatever changed the log is done, unless
  /// it runs already.
  void _kick() {
    if (_running) _again = true;
    if (_queued || _running || model.status != ModelStatus.ready) return;
    _queued = true;
    scheduleMicrotask(() {
      _queued = false;
      if (!_disposed) unawaited(_run());
    });
  }

  Future<void> _run() async {
    if (_running) return;
    _running = true;
    _again = false;
    try {
      while (!_disposed && model.status == ModelStatus.ready) {
        final next = notes
            .untranscribed()
            .where((r) => !_skipped.contains(r.hash))
            .firstOrNull;
        if (next == null) break;
        final bytes = notes.recordingBytes(next.hash);
        String? text;
        if (bytes != null) {
          _current = next.hash;
          notifyListeners();
          text = await platform.transcribe(bytes);
          if (_disposed) return;
          _current = null;
        }
        // Still there: the Note may have been trashed meanwhile.
        final note = notes.byId(next.noteId);
        if (text == null) {
          _skipped.add(next.hash);
        } else if (note != null && note.recordings.contains(next.hash)) {
          notes.setTranscript(next.noteId, next.hash, text);
        }
        notifyListeners();
      }
      await platform.idle();
    } finally {
      _running = false;
    }
    if (_again) _kick();
  }

  @override
  void dispose() {
    _disposed = true;
    notes.store.removeListener(_kick);
    model.removeListener(_kick);
    super.dispose();
  }
}

/// Hands the app's speech model and [Transcriber] down to every screen.
class SpeechScope extends InheritedWidget {
  final SpeechModel model;
  final Transcriber transcriber;

  const SpeechScope({
    super.key,
    required this.model,
    required this.transcriber,
    required super.child,
  });

  /// The app's [SpeechScope], or null where there is none; read once, so
  /// it does not listen.
  static SpeechScope? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<SpeechScope>();

  @override
  bool updateShouldNotify(SpeechScope oldWidget) =>
      model != oldWidget.model || transcriber != oldWidget.transcriber;
}

/// The Transcript of the voice recording [hash] of the Note [noteId],
/// shown under its play button: [transcript] as read with the Note, and
/// whatever the [Transcriber] writes while it shows. While the recording
/// is being transcribed it says so; with no Transcript it shows nothing.
/// A card shows the first few lines when [compact] is set.
class TranscriptText extends StatefulWidget {
  final String noteId;
  final String hash;
  final String? transcript;
  final bool compact;

  const TranscriptText({
    super.key,
    required this.noteId,
    required this.hash,
    required this.transcript,
    this.compact = true,
  });

  @override
  State<TranscriptText> createState() => _TranscriptTextState();
}

class _TranscriptTextState extends State<TranscriptText> {
  Transcriber? _transcriber;
  late String? _text = widget.transcript;
  var _transcribing = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final transcriber = SpeechScope.of(context)?.transcriber;
    if (transcriber == _transcriber) return;
    _transcriber?.removeListener(_changed);
    _transcriber = transcriber?..addListener(_changed);
    _transcribing = transcriber?.current == widget.hash;
  }

  @override
  void didUpdateWidget(TranscriptText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.transcript != oldWidget.transcript) _text = widget.transcript;
  }

  @override
  void dispose() {
    _transcriber?.removeListener(_changed);
    super.dispose();
  }

  void _changed() {
    final transcriber = _transcriber;
    if (transcriber == null) return;
    final transcribing = transcriber.current == widget.hash;
    final text = _text ?? transcriber.transcriptOf(widget.noteId, widget.hash);
    if (transcribing == _transcribing && text == _text) return;
    setState(() {
      _transcribing = transcribing;
      _text = text;
    });
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final quiet = theme.textTheme.bodyMedium?.copyWith(
      fontStyle: FontStyle.italic,
      color: theme.colorScheme.onSurfaceVariant,
    );
    final text = _text;
    if (text == null && !_transcribing) return const SizedBox.shrink();
    return Padding(
      // Apart from the play button; nothing at all while there is no text.
      padding: EdgeInsets.only(top: widget.compact ? 4 : 8),
      child: switch (text) {
        null => Text(t.transcribing, style: quiet),
        '' => Text(t.noWordsHeard, style: quiet),
        _ => Text(
          text,
          maxLines: widget.compact ? 3 : null,
          overflow: widget.compact ? TextOverflow.ellipsis : null,
          style: widget.compact
              ? theme.textTheme.bodyMedium
              : theme.textTheme.bodyLarge,
        ),
      },
    );
  }
}

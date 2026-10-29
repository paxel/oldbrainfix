import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// The platform seam for Photo Texts: ocrs with its models bundled in
/// the app (android/ocr, android/…/Ocr.kt), so it works offline and
/// needs nothing fetched. Widget tests hand in a fake.
abstract interface class OcrPlatform {
  /// Whether ocrs runs on this Device.
  Future<bool> supported();

  /// The text in the photo [jpeg], one line per line of text, empty when
  /// there is none; null when the photo could not be read.
  Future<String?> read(Uint8List jpeg);

  /// Nothing more to read for now: the models' memory can go.
  Future<void> idle();
}

/// No text recognition: the default where none is handed in.
class NoOcr implements OcrPlatform {
  const NoOcr();

  @override
  Future<bool> supported() async => false;

  @override
  Future<String?> read(Uint8List jpeg) async => null;

  @override
  Future<void> idle() async {}
}

/// ocrs built into the app, behind the app's method channel. No Google
/// library, no network.
class AndroidOcr implements OcrPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/ocr');

  const AndroidOcr();

  @override
  Future<bool> supported() async => await _ask<bool>('supported') ?? false;

  @override
  Future<String?> read(Uint8List jpeg) => _ask<String>('read', jpeg);

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

/// Reads the text in photos in the background, one at a time: each new
/// one after its Note is saved, and those from before. Capture never
/// waits for it. A photo that cannot be read is left alone until the
/// next start; the photo itself is always kept. Asked to, it reads one
/// photo again, ahead of the others and whether or not it has a Photo
/// Text ([readAgain]).
class PhotoReader extends ChangeNotifier {
  final Notes notes;
  final OcrPlatform platform;

  /// Photos that could not be read since the start.
  final _skipped = <String>{};

  /// Photos the user asked to read again, oldest ask first.
  final _asked = <({String noteId, String hash, Completer<bool> done})>[];
  bool? _supported;
  String? _current;
  var _running = false;
  var _queued = false;

  /// The log changed while reading: look once more at the end.
  var _again = false;
  var _disposed = false;

  PhotoReader({required this.notes, required this.platform}) {
    notes.store.addListener(_kick);
    _kick();
  }

  /// The photo being read, by its hash; null while none is.
  String? get current => _current;

  /// The Photo Text of the photo [hash] of the Note [noteId] as it is
  /// now, or null when it has none.
  String? photoTextOf(String noteId, String hash) =>
      notes.byId(noteId)?.photoTexts[hash];

  /// Whether ocrs runs here: null until known.
  bool? get supported => _supported;

  /// Reads the text in the photo [hash] of the Note [noteId] again, after
  /// the one running if any, and keeps it as its Photo Text in place of
  /// the one before or of a removed one; whether it did. A photo that
  /// could not be read before is tried again.
  Future<bool> readAgain(String noteId, String hash) {
    if (_disposed || _supported == false) return Future.value(false);
    final done = Completer<bool>();
    _asked.add((noteId: noteId, hash: hash, done: done));
    _kick();
    return done.future;
  }

  /// Starts reading once whatever changed the log is done, unless it
  /// runs already or ocrs does not run here.
  void _kick() {
    if (_running) _again = true;
    if (_queued || _running || _supported == false) return;
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
    Completer<bool>? waiting;
    try {
      final supported = _supported ??= await platform.supported();
      if (!supported || _disposed) return;
      var read = false;
      while (!_disposed) {
        final asked = _asked.isEmpty ? null : _asked.removeAt(0);
        waiting = asked?.done;
        final next = asked == null
            ? notes
                  .unreadPhotos()
                  .where((p) => !_skipped.contains(p.hash))
                  .firstOrNull
            : (noteId: asked.noteId, hash: asked.hash);
        if (next == null) break;
        final bytes = notes.photoBytes(next.hash);
        String? text;
        if (bytes != null) {
          read = true;
          _current = next.hash;
          notifyListeners();
          text = await platform.read(bytes);
          if (_disposed) return;
          _current = null;
        }
        // Still there: the Note may have been trashed meanwhile.
        final note = notes.byId(next.noteId);
        final kept =
            text != null && note != null && note.photos.contains(next.hash);
        if (text == null) {
          _skipped.add(next.hash);
        } else if (kept) {
          _skipped.remove(next.hash);
          notes.setPhotoText(next.noteId, next.hash, text);
        }
        waiting?.complete(kept);
        waiting = null;
        notifyListeners();
      }
      if (read) await platform.idle();
    } finally {
      _running = false;
      waiting?.complete(false);
      // Where ocrs does not run, or gone: nothing asked for is read.
      if (_disposed || _supported == false) _dropAsked();
    }
    if (_again) _kick();
  }

  /// Tells everyone still waiting that their photo was not read.
  void _dropAsked() {
    for (final asked in _asked) {
      asked.done.complete(false);
    }
    _asked.clear();
  }

  @override
  void dispose() {
    _disposed = true;
    _dropAsked();
    notes.store.removeListener(_kick);
    super.dispose();
  }
}

/// Hands the app's [PhotoReader] down to every screen.
class PhotoReaderScope extends InheritedWidget {
  final PhotoReader reader;

  const PhotoReaderScope({
    super.key,
    required this.reader,
    required super.child,
  });

  /// The app's [PhotoReader], or null where there is none; read once, so
  /// it does not listen.
  static PhotoReader? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PhotoReaderScope>()?.reader;

  @override
  bool updateShouldNotify(PhotoReaderScope oldWidget) =>
      reader != oldWidget.reader;
}

/// A Transcript or Photo Text under its [label], both dimmed, so it does
/// not read as typed text.
class GeneratedTextLabel extends StatelessWidget {
  final String label;
  final Widget child;

  const GeneratedTextLabel({
    super.key,
    required this.label,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        child,
      ],
    );
  }
}

/// The Photo Text of the photo [hash] of the Note [noteId], shown under
/// the photo: [photoText] as read with the Note, and whatever the
/// [PhotoReader] writes while it shows. A photo without text, or not
/// read yet, shows nothing. [compact], on a card, shows the first three
/// lines; [labelled], on a full card, shows it whole under the word
/// "Photo Text", so it does not read as typed text.
class PhotoText extends StatefulWidget {
  final String noteId;
  final String hash;
  final String? photoText;
  final bool compact;
  final bool labelled;

  const PhotoText({
    super.key,
    required this.noteId,
    required this.hash,
    required this.photoText,
    this.compact = false,
    this.labelled = false,
  });

  @override
  State<PhotoText> createState() => _PhotoTextState();
}

class _PhotoTextState extends State<PhotoText> {
  PhotoReader? _reader;
  late String? _text = widget.photoText;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final reader = PhotoReaderScope.of(context);
    if (reader == _reader) return;
    _reader?.removeListener(_changed);
    _reader = reader?..addListener(_changed);
    // Read since the Note was: a lazy list builds this late.
    _text ??= reader?.photoTextOf(widget.noteId, widget.hash);
  }

  @override
  void didUpdateWidget(PhotoText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.photoText != oldWidget.photoText) _text = widget.photoText;
  }

  @override
  void dispose() {
    _reader?.removeListener(_changed);
    super.dispose();
  }

  void _changed() {
    final text = _text ?? _reader?.photoTextOf(widget.noteId, widget.hash);
    if (text != _text) setState(() => _text = text);
  }

  @override
  Widget build(BuildContext context) {
    final text = _text;
    if (text == null || text.isEmpty) return const SizedBox.shrink();
    final theme = Theme.of(context);
    final shown = Text(
      text,
      maxLines: widget.compact ? 3 : null,
      overflow: widget.compact ? TextOverflow.ellipsis : null,
      style: theme.textTheme.bodyMedium?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
      ),
    );
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: widget.labelled
          ? GeneratedTextLabel(label: context.t.photoTextLabel, child: shown)
          : shown,
    );
  }
}

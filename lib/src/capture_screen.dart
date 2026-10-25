import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'context_platform.dart';
import 'l10n.dart';
import 'location.dart';
import 'photo_platform.dart';
import 'photo_views.dart';
import 'voice_platform.dart';

/// A Note just captured, and for its location and for the rest of its
/// Capture Context whether it got attached, each once that is known.
typedef Captured = ({String noteId, List<Future<bool>> attached});

/// Opens text Capture, or voice Capture recording at once when [voice] is
/// set, or the camera at once when [photo] is set, filing the new Note into the Area or Topic [place] when one is
/// given, or with the [shared] item from another app filled in. [onChanged] runs once the user left Capture, for the caller to
/// show the new Note, and again when its location or more of its Capture
/// Context arrived later.
/// [onCaptured] gets the new Note's id first, when there is one.
///
/// After this Device's first Capture, while the location is not allowed
/// yet, it explains why Notes record where they were captured and asks;
/// Capture itself never asks anything (UI laws, "Capture").
Future<void> captureNote(
  BuildContext context,
  Notes notes, {
  String? place,
  SharedItem? shared,
  bool voice = false,
  bool photo = false,
  required VoidCallback onChanged,
  ValueChanged<String>? onCaptured,
}) async {
  final location = LocationScope.of(context);
  final photos = PhotoScope.of(context);
  final microphone = VoiceScope.of(context);
  final contextSource = ContextScope.of(context);
  final captured = await Navigator.of(context).push<Captured>(
    MaterialPageRoute(
      // Capture never needs unlocking.
      settings: const RouteSettings(name: LockFreeRoutes.capture),
      builder: (_) => CaptureScreen(
        notes: notes,
        location: location,
        photos: photos,
        voice: microphone,
        contextSource: contextSource,
        place: place,
        shared: shared,
        startVoice: voice,
        startPhoto: photo,
      ),
    ),
  );
  if (captured != null) onCaptured?.call(captured.noteId);
  onChanged();
  if (captured == null) return;
  // Each shows as soon as it arrives, without waiting for the others.
  for (final attached in captured.attached) {
    unawaited(attached.then((attached) => attached ? onChanged() : null));
  }
  final store = notes.store;
  if (!location.available || store.locationExplained) return;
  if (await location.granted()) return;
  if (!context.mounted) return;
  store.locationExplained = true;
  if (await explainAndAskLocation(context, location)) {
    // The position may take a while; the list does not wait for it.
    unawaited(
      attachLocation(
        notes,
        captured.noteId,
        location.current(),
      ).then((located) => located ? onChanged() : null),
    );
  }
}

/// Text capture: opens with the keyboard up, Save writes the Note. The
/// position, and the calendar events and Bluetooth devices where they
/// are switched on, are looked up from the start, without asking, and
/// attached when they arrive: they never hold the Note back. Leaving by
/// Back saves too, and so does the app going to the background. While
/// Unsorted the Note is nudged in 10 min, 30 min or 1 h, picked with the
/// buttons at the bottom, each of which saves; Save means 1 h (UI laws,
/// "Capture").
///
/// The camera button takes a photo into the Note; the text is then
/// optional. Something shared in from another app opens here with its
/// text and photos filled in, and the app it came from recorded. Photos
/// are written at once, so none is lost.
///
/// The mic button switches to voice: recording starts at once, the
/// elapsed time runs, and one big stop button ends it and saves, as do
/// the Nudge buttons and Back; there is no hold-to-talk and no stop on
/// silence (UI laws, "Capture"). The first time, while the microphone is
/// not allowed yet, it explains why and asks. The app going to the
/// background ends the recording and keeps it. Text typed before goes
/// into the same Note.
class CaptureScreen extends StatefulWidget {
  final Notes notes;
  final LocationSource location;
  final PhotoPlatform photos;
  final VoicePlatform voice;
  final ContextSource contextSource;

  /// The Area or Topic the new Note is filed into; Unsorted when null.
  final String? place;

  /// What another app shared, filled in; null for a plain Capture.
  final SharedItem? shared;

  /// Opens in voice, recording at once.
  final bool startVoice;

  /// Opens the camera at once; back from it, the screen is text Capture
  /// with the photo.
  final bool startPhoto;

  const CaptureScreen({
    super.key,
    required this.notes,
    this.location = const NoLocation(),
    this.photos = const NoPhotos(),
    this.voice = const NoVoice(),
    this.contextSource = const NoContext(),
    this.place,
    this.shared,
    this.startVoice = false,
    this.startPhoto = false,
  });

  @override
  State<CaptureScreen> createState() => _CaptureScreenState();
}

class _CaptureScreenState extends State<CaptureScreen> {
  final _text = TextEditingController();
  late final Future<Coordinates?> _here;
  late final Future<ContextExtras> _extras;
  late final AppLifecycleListener _lifecycle;

  /// The Note once written: by leaving, by a photo, or when the app went
  /// to the background meanwhile, after which typing changes the same
  /// Note.
  Captured? _captured;

  /// The hashes of the Note's photos, as written.
  List<String> _photos = const [];

  /// The hashes of the Note's voice recordings, as written.
  List<String> _recordings = const [];

  /// Whether the screen shows voice instead of the text field.
  var _voiceMode = false;

  /// Whether voice found the microphone not allowed or not working.
  var _micMissing = false;

  /// The recording being started and then running, whether it did start;
  /// null while none runs.
  Future<bool>? _recording;

  /// Ending the recording and keeping it, once asked for.
  Future<void>? _stopping;

  /// Counts the recording's seconds while it runs.
  Timer? _ticker;
  var _elapsed = Duration.zero;

  /// Set once leaving began, so a second tap does not leave twice.
  var _leaving = false;

  @override
  void initState() {
    super.initState();
    // The fix may take a while; it starts while the user types.
    _here = currentIfGranted(widget.location);
    _extras = currentExtras(widget.notes.store, widget.contextSource);
    // A share is meant to be kept: it is written at once.
    final shared = widget.shared;
    if (shared != null && !shared.isEmpty) {
      _text.text = shared.text;
      _addPhotos(shared.photos);
      _write(null);
    }
    // The app may be killed in the background: the text is kept.
    _lifecycle = AppLifecycleListener(
      onStateChange: (state) {
        if (state == AppLifecycleState.paused ||
            state == AppLifecycleState.hidden) {
          _write(null);
          // Recording stops in the background: what is there is kept.
          unawaited(_stopRecording());
        }
      },
    );
    if (widget.startVoice) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _openVoice());
    } else if (widget.startPhoto && widget.photos.available) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _takePhoto());
    }
  }

  @override
  void dispose() {
    _ticker?.cancel();
    _lifecycle.dispose();
    _text.dispose();
    super.dispose();
  }

  /// Writes the text as a Note, or into the one already written; nothing
  /// while it is empty and there is no Note yet. A new Note is nudged
  /// [nudgeIn] from now, 1 h when null; an existing one only when
  /// [nudgeIn] is given. A photo Note keeps its text when it is emptied.
  void _write(Duration? nudgeIn) {
    final text = _text.text.trim();
    final captured = _captured;
    if (captured == null) {
      if (text.isNotEmpty) _create(text, nudgeIn: nudgeIn);
      return;
    }
    final note = widget.notes.byId(captured.noteId);
    if (text.isNotEmpty &&
        note != null &&
        note.textEditable &&
        note.text != text) {
      widget.notes.edit(captured.noteId, text);
    }
    if (nudgeIn != null) widget.notes.setNudge(captured.noteId, nudgeIn);
  }

  /// Captures the new Note with [text], [photos] and [recordings].
  void _create(
    String text, {
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
  }) {
    final note = widget.notes.capture(
      text,
      place: widget.place,
      // Kept for a filed Note too: sent back to Unsorted, it is due.
      nudgeIn: nudgeIn ?? Notes.defaultNudge,
      photos: photos,
      recordings: recordings,
      sourceApp: widget.shared?.sourceApp,
    );
    _captured = (
      noteId: note.id,
      attached: [
        attachLocation(widget.notes, note.id, _here),
        attachExtras(widget.notes, note.id, _extras),
      ],
    );
    _photos = note.photos;
    _recordings = note.recordings;
  }

  /// Writes [photos] into the Note, capturing it with the text so far
  /// when there is none yet.
  void _addPhotos(List<Uint8List> photos) {
    if (photos.isEmpty) return;
    final captured = _captured;
    if (captured == null) {
      _create(_text.text.trim(), photos: photos);
      return;
    }
    for (final bytes in photos) {
      widget.notes.addPhoto(captured.noteId, bytes);
    }
    _photos = widget.notes.byId(captured.noteId)?.photos ?? _photos;
  }

  /// Takes a photo with the camera into the Note.
  Future<void> _takePhoto() async {
    final bytes = await widget.photos.takePhoto();
    if (!mounted || bytes == null) return;
    setState(() => _addPhotos([bytes]));
  }

  /// Writes the recording [bytes] into the Note, capturing it with the
  /// text so far when there is none yet.
  void _addRecording(Uint8List bytes) {
    final captured = _captured;
    if (captured == null) {
      _create(_text.text.trim(), recordings: [bytes]);
      return;
    }
    widget.notes.addRecording(captured.noteId, bytes);
    _recordings = widget.notes.byId(captured.noteId)?.recordings ?? _recordings;
  }

  /// Switches to voice and starts recording, asking for the microphone
  /// first, with an explanation, while it is not allowed.
  Future<void> _openVoice() async {
    if (_recording != null || _leaving) return;
    FocusScope.of(context).unfocus();
    setState(() {
      _voiceMode = true;
      _micMissing = false;
      _elapsed = Duration.zero;
    });
    final voice = widget.voice;
    var allowed = await voice.granted();
    if (!mounted || _leaving) return;
    if (!allowed) allowed = await explainAndAskMicrophone(context, voice);
    if (!mounted || _leaving) return;
    if (!allowed) {
      setState(() => _micMissing = true);
      return;
    }
    // Recording ends any playing, and the play buttons show it.
    await VoiceScope.playbackOf(context)?.stop();
    if (!mounted || _leaving) return;
    final recording = _recording = voice.start();
    final started = await recording;
    // Stopped meanwhile, by Back or the background: nothing to count.
    if (!mounted || _recording != recording) return;
    if (!started) {
      _recording = null;
      setState(() => _micMissing = true);
      return;
    }
    _ticker = Timer.periodic(const Duration(seconds: 1), (timer) {
      setState(() => _elapsed = Duration(seconds: timer.tick));
    });
  }

  /// Ends the recording and writes it into the Note; the same ending
  /// when asked again, nothing when none runs.
  Future<void> _stopRecording() {
    final recording = _recording;
    if (recording != null) {
      _recording = null;
      _ticker?.cancel();
      _stopping = _keep(recording);
    }
    return _stopping ?? Future.value();
  }

  /// Waits for [recording] to have started, ends it and keeps what was
  /// recorded; back to the text field afterwards.
  Future<void> _keep(Future<bool> recording) async {
    if (!await recording) return;
    final bytes = await widget.voice.stop();
    if (bytes != null) _addRecording(bytes);
    _stopping = null;
    // Leaving, the text field never comes back to pop the keyboard up.
    if (mounted && !_leaving) setState(() => _voiceMode = false);
  }

  /// Saves and leaves: by Save, Back or the stop button (1 h), or a
  /// picked Nudge time. A running recording is ended and kept first.
  Future<void> _save([Duration? nudgeIn]) async {
    if (_leaving) return;
    _leaving = true;
    await _stopRecording();
    if (!mounted) return;
    _write(nudgeIn);
    Navigator.of(context).pop(_captured);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final recording = _voiceMode && !_micMissing;
    // Leaving by Back saves as well (UI laws, "Capture").
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) unawaited(_save());
      },
      child: Scaffold(
        appBar: AppBar(
          // While recording, the big stop button is the one way out.
          actions: recording
              ? null
              : [
                  if (widget.voice.available && !_voiceMode)
                    IconButton(
                      tooltip: t.recordVoice,
                      onPressed: _openVoice,
                      icon: const Icon(Icons.mic_none),
                    ),
                  if (widget.photos.available)
                    IconButton(
                      tooltip: t.takePhoto,
                      onPressed: _takePhoto,
                      icon: const Icon(Icons.photo_camera_outlined),
                    ),
                  TextButton(onPressed: _save, child: Text(t.save)),
                ],
        ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                if (_photos.isNotEmpty) ...[
                  PhotoStrip(
                    hashes: _photos,
                    size: 96,
                    onTap: (hash) => viewPhoto(context, hash),
                  ),
                  const SizedBox(height: 8),
                ],
                if (_recordings.isNotEmpty && !_voiceMode) ...[
                  Align(
                    alignment: Alignment.centerLeft,
                    child: Wrap(
                      spacing: 8,
                      children: [
                        for (final hash in _recordings)
                          RecordingButton(key: ValueKey(hash), hash: hash),
                      ],
                    ),
                  ),
                  const SizedBox(height: 8),
                ],
                Expanded(
                  child: _voiceMode
                      ? _voice(context)
                      : TextField(
                          controller: _text,
                          autofocus: !widget.startVoice && !widget.startPhoto,
                          maxLines: null,
                          decoration: InputDecoration(
                            hintText: _photos.isEmpty && _recordings.isEmpty
                                ? t.captureHint
                                : t.capturePhotoHint,
                          ),
                        ),
                ),
                NudgeButtons(onPicked: _save),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Voice: the text typed so far, then the running time and the big
  /// stop button; or, without the microphone, why there is none.
  Widget _voice(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final typed = _text.text.trim();
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
              child: _micMissing
                  ? Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(t.microphoneMissing, textAlign: TextAlign.center),
                        const SizedBox(height: 16),
                        FilledButton(
                          onPressed: _openVoice,
                          child: Text(t.allowAccess),
                        ),
                        TextButton(
                          onPressed: () => setState(() => _voiceMode = false),
                          child: Text(t.typeInstead),
                        ),
                      ],
                    )
                  : Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(t.recording, style: theme.textTheme.titleMedium),
                        const SizedBox(height: 8),
                        Text(
                          elapsedText(_elapsed),
                          key: const ValueKey('elapsed'),
                          style: theme.textTheme.displayMedium,
                        ),
                        const SizedBox(height: 24),
                        SizedBox.square(
                          dimension: 160,
                          child: FilledButton(
                            key: const ValueKey('stopRecording'),
                            onPressed: _save,
                            style: FilledButton.styleFrom(
                              shape: const CircleBorder(),
                              padding: EdgeInsets.zero,
                            ),
                            child: Icon(
                              Icons.stop,
                              size: 96,
                              semanticLabel: t.stopAndSave,
                            ),
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(t.stopAndSave),
                      ],
                    ),
            ),
          ),
        ),
      ],
    );
  }
}

/// The Nudge choices at the bottom of a capture screen: remind me in
/// 10 min, 30 min or 1 h. Part of the page, not a bar (UI laws,
/// "Capture").
class NudgeButtons extends StatelessWidget {
  /// The choices, each with the key its button carries.
  static const choices = [
    (Duration(minutes: 10), ValueKey('nudge10')),
    (Duration(minutes: 30), ValueKey('nudge30')),
    (Duration(hours: 1), ValueKey('nudge60')),
  ];

  final ValueChanged<Duration> onPicked;

  const NudgeButtons({super.key, required this.onPicked});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final labels = [t.snooze10, t.snooze30, t.snooze60];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(t.nudgeHeading, style: Theme.of(context).textTheme.labelLarge),
        const SizedBox(height: 8),
        Row(
          children: [
            for (final (i, (delay, key)) in choices.indexed) ...[
              if (i > 0) const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton(
                  key: key,
                  onPressed: () => onPicked(delay),
                  child: Text(labels[i]),
                ),
              ),
            ],
          ],
        ),
      ],
    );
  }
}

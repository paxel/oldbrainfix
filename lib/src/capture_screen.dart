import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'context_platform.dart';
import 'l10n.dart';
import 'location.dart';
import 'photo_platform.dart';
import 'photo_views.dart';

/// A Note just captured, and for its location and for the rest of its
/// Capture Context whether it got attached, each once that is known.
typedef Captured = ({String noteId, List<Future<bool>> attached});

/// Opens text Capture, filing the new Note into the Area or Topic [place]
/// when one is given, or with the [shared] item from another app filled
/// in. [onChanged] runs once the user left Capture, for the caller to
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
  required VoidCallback onChanged,
  ValueChanged<String>? onCaptured,
}) async {
  final location = LocationScope.of(context);
  final photos = PhotoScope.of(context);
  final contextSource = ContextScope.of(context);
  final captured = await Navigator.of(context).push<Captured>(
    MaterialPageRoute(
      builder: (_) => CaptureScreen(
        notes: notes,
        location: location,
        photos: photos,
        contextSource: contextSource,
        place: place,
        shared: shared,
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
class CaptureScreen extends StatefulWidget {
  final Notes notes;
  final LocationSource location;
  final PhotoPlatform photos;
  final ContextSource contextSource;

  /// The Area or Topic the new Note is filed into; Unsorted when null.
  final String? place;

  /// What another app shared, filled in; null for a plain Capture.
  final SharedItem? shared;

  const CaptureScreen({
    super.key,
    required this.notes,
    this.location = const NoLocation(),
    this.photos = const NoPhotos(),
    this.contextSource = const NoContext(),
    this.place,
    this.shared,
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
        }
      },
    );
  }

  @override
  void dispose() {
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
    if (text.isNotEmpty && widget.notes.byId(captured.noteId)?.text != text) {
      widget.notes.edit(captured.noteId, text);
    }
    if (nudgeIn != null) widget.notes.setNudge(captured.noteId, nudgeIn);
  }

  /// Captures the new Note with [text] and [photos].
  void _create(
    String text, {
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
  }) {
    final note = widget.notes.capture(
      text,
      place: widget.place,
      // Kept for a filed Note too: sent back to Unsorted, it is due.
      nudgeIn: nudgeIn ?? Notes.defaultNudge,
      photos: photos,
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

  /// Saves and leaves: by Save or Back (1 h), or a picked Nudge time.
  void _save([Duration? nudgeIn]) {
    _write(nudgeIn);
    Navigator.of(context).pop(_captured);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    // Leaving by Back saves as well (UI laws, "Capture").
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _save();
      },
      child: Scaffold(
        appBar: AppBar(
          actions: [
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
                Expanded(
                  child: TextField(
                    controller: _text,
                    autofocus: true,
                    maxLines: null,
                    decoration: InputDecoration(
                      hintText: _photos.isEmpty
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

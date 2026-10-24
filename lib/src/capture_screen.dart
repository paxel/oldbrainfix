import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'location.dart';

/// A Note just captured, and whether its location got attached, once
/// that is known.
typedef Captured = ({String noteId, Future<bool> located});

/// Opens text Capture, filing the new Note into the Area or Topic [place]
/// when one is given. [onChanged] runs once the user left Capture, for
/// the caller to show the new Note, and again when its location arrived
/// later. [onCaptured] gets the new Note's id first, when there is one.
///
/// After this Device's first Capture, while the location is not allowed
/// yet, it explains why Notes record where they were captured and asks;
/// Capture itself never asks anything (UI laws, "Capture").
Future<void> captureNote(
  BuildContext context,
  Notes notes, {
  String? place,
  required VoidCallback onChanged,
  ValueChanged<String>? onCaptured,
}) async {
  final location = LocationScope.of(context);
  final captured = await Navigator.of(context).push<Captured>(
    MaterialPageRoute(
      builder: (_) =>
          CaptureScreen(notes: notes, location: location, place: place),
    ),
  );
  if (captured != null) onCaptured?.call(captured.noteId);
  onChanged();
  if (captured == null) return;
  unawaited(captured.located.then((located) => located ? onChanged() : null));
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
/// position is looked up from the start, without asking, and attached
/// when it arrives: it never holds the Note back. Leaving by Back saves
/// too, and so does the app going to the background. While Unsorted the
/// Note is nudged in 10 min, 30 min or 1 h, picked with the buttons at
/// the bottom, each of which saves; Save means 1 h (UI laws, "Capture").
class CaptureScreen extends StatefulWidget {
  final Notes notes;
  final LocationSource location;

  /// The Area or Topic the new Note is filed into; Unsorted when null.
  final String? place;

  const CaptureScreen({
    super.key,
    required this.notes,
    this.location = const NoLocation(),
    this.place,
  });

  @override
  State<CaptureScreen> createState() => _CaptureScreenState();
}

class _CaptureScreenState extends State<CaptureScreen> {
  final _text = TextEditingController();
  late final Future<Coordinates?> _here;
  late final AppLifecycleListener _lifecycle;

  /// The Note once written: by leaving, or when the app went to the
  /// background meanwhile, after which typing changes the same Note.
  Captured? _captured;

  @override
  void initState() {
    super.initState();
    // The fix may take a while; it starts while the user types.
    _here = currentIfGranted(widget.location);
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
  /// while it is empty. A new Note is nudged [nudgeIn] from now, 1 h when
  /// null; an existing one only when [nudgeIn] is given.
  void _write(Duration? nudgeIn) {
    final text = _text.text.trim();
    if (text.isEmpty) return;
    final captured = _captured;
    if (captured == null) {
      final note = widget.notes.capture(
        text,
        place: widget.place,
        // Kept for a filed Note too: sent back to Unsorted, it is due.
        nudgeIn: nudgeIn ?? Notes.defaultNudge,
      );
      _captured = (
        noteId: note.id,
        located: attachLocation(widget.notes, note.id, _here),
      );
      return;
    }
    if (widget.notes.byId(captured.noteId)?.text != text) {
      widget.notes.edit(captured.noteId, text);
    }
    if (nudgeIn != null) widget.notes.setNudge(captured.noteId, nudgeIn);
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
          actions: [TextButton(onPressed: _save, child: Text(t.save))],
        ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                Expanded(
                  child: TextField(
                    controller: _text,
                    autofocus: true,
                    maxLines: null,
                    decoration: InputDecoration(hintText: t.captureHint),
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

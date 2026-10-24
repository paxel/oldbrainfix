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
/// later.
///
/// After this Device's first Capture, while the location is not allowed
/// yet, it explains why Notes record where they were captured and asks;
/// Capture itself never asks anything (UI laws, "Capture").
Future<void> captureNote(
  BuildContext context,
  Notes notes, {
  String? place,
  required VoidCallback onChanged,
}) async {
  final location = LocationScope.of(context);
  final captured = await Navigator.of(context).push<Captured>(
    MaterialPageRoute(
      builder: (_) =>
          CaptureScreen(notes: notes, location: location, place: place),
    ),
  );
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
/// when it arrives: it never holds the Note back.
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

  @override
  void initState() {
    super.initState();
    // The fix may take a while; it starts while the user types.
    _here = currentIfGranted(widget.location);
  }

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  void _save() {
    final text = _text.text.trim();
    Captured? captured;
    if (text.isNotEmpty) {
      final note = widget.notes.capture(text, place: widget.place);
      captured = (
        noteId: note.id,
        located: attachLocation(widget.notes, note.id, _here),
      );
    }
    Navigator.of(context).pop(captured);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(
        actions: [TextButton(onPressed: _save, child: Text(t.save))],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: TextField(
          controller: _text,
          autofocus: true,
          maxLines: null,
          decoration: InputDecoration(hintText: t.captureHint),
        ),
      ),
    );
  }
}

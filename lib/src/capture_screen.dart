import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// Text capture: opens with the keyboard up, Save writes the Note.
class CaptureScreen extends StatefulWidget {
  final Notes notes;

  /// The Area or Topic the new Note is filed into; Unsorted when null.
  final String? place;

  const CaptureScreen({super.key, required this.notes, this.place});

  @override
  State<CaptureScreen> createState() => _CaptureScreenState();
}

class _CaptureScreenState extends State<CaptureScreen> {
  final _text = TextEditingController();

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  void _save() {
    final text = _text.text.trim();
    if (text.isNotEmpty) {
      widget.notes.capture(text, place: widget.place);
    }
    Navigator.of(context).pop();
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

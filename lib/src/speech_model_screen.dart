import 'package:flutter/material.dart';

import 'announcements.dart';
import 'l10n.dart';
import 'look.dart';
import 'speech_platform.dart';

/// The speech model for Transcripts: what it is for, how big it is and
/// where it comes from, then a download on request or a file of the
/// user's own. A model that is not the pinned one is never kept. Lives in
/// the notes browser until Settings exist.
class SpeechModelScreen extends StatelessWidget {
  final SpeechModel model;

  const SpeechModelScreen({super.key, required this.model});

  Future<void> _fetch(
    BuildContext context,
    Future<ModelOutcome> Function() fetch,
  ) async {
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    final outcome = await fetch();
    final text = switch (outcome) {
      ModelOutcome.failed => t.speechModelFailed,
      ModelOutcome.mismatch => t.speechModelMismatch,
      ModelOutcome.installed || ModelOutcome.cancelled => null,
    };
    if (text != null) announcements?.announce(text);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.speechModelTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(t.speechModelIntro),
          const SizedBox(height: 8),
          Text(t.speechModelSource(WhisperModel.megabytes)),
          const SizedBox(height: 16),
          Embossed(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: ListenableBuilder(
                listenable: model,
                builder: (context, _) => _state(context),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _state(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    switch (model.status) {
      case ModelStatus.checking:
        return Row(
          children: [
            const SizedBox.square(
              dimension: 24,
              child: CircularProgressIndicator(strokeWidth: 3),
            ),
            const SizedBox(width: 16),
            Expanded(child: Text(t.speechModelChecking)),
          ],
        );
      case ModelStatus.unsupported:
        return Text(t.speechModelUnsupported);
      case ModelStatus.ready:
        return Row(
          children: [
            Icon(
              Icons.check_circle_outline,
              color: Theme.of(context).colorScheme.primary,
            ),
            const SizedBox(width: 16),
            Expanded(child: Text(t.speechModelReady)),
          ],
        );
      case ModelStatus.missing:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(t.speechModelMissing, style: text.titleMedium),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: () => _fetch(context, model.download),
              icon: const Icon(Icons.download),
              label: Text(t.speechModelDownload(WhisperModel.megabytes)),
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: () => _fetch(context, model.pick),
              icon: const Icon(Icons.folder_open),
              label: Text(t.speechModelPick),
            ),
            const SizedBox(height: 8),
            Text(t.speechModelPickDetail, style: text.bodySmall),
          ],
        );
      case ModelStatus.fetching:
        final (:received, :total) = model.progress;
        final downloading = model.downloading;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              downloading
                  ? t.speechModelDownloading(
                      _megabytes(received),
                      total == null ? '?' : _megabytes(total),
                    )
                  : t.speechModelChecking,
            ),
            const SizedBox(height: 12),
            LinearProgressIndicator(
              value: downloading && total != null ? received / total : null,
            ),
            const SizedBox(height: 12),
            TextButton(onPressed: model.cancel, child: Text(t.cancel)),
          ],
        );
    }
  }

  /// [bytes] in megabytes, rounded.
  static String _megabytes(int bytes) => '${(bytes / 1e6).round()}';
}

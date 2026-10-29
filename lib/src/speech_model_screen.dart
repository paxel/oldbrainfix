import 'package:flutter/material.dart';

import 'announcements.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'look.dart';
import 'speech_platform.dart';
import 'system_bars.dart';

/// Fetches the speech model with [fetch] and announces a failure at the
/// top when it broke off or was not the pinned one, why behind the tap.
Future<void> fetchSpeechModel(
  BuildContext context,
  Future<ModelOutcome> Function() fetch,
) async {
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final t = context.t;
  final outcome = await fetch();
  final why = switch (outcome) {
    ModelOutcome.failed => (t.speechModelFailedCause, t.speechModelFailedFix),
    ModelOutcome.mismatch => (
      t.speechModelMismatchCause,
      t.speechModelMismatchFix,
    ),
    ModelOutcome.installed || ModelOutcome.cancelled => null,
  };
  if (why == null) return;
  final (cause, fix) = why;
  announceFailureWith(
    announcements,
    navigator,
    t.speechModelNotKept,
    title: t.speechModelNotKeptTitle,
    cause: cause,
    fix: fix,
    details: 'speech model: ${outcome.name}',
  );
}

/// The speech model for Transcripts: what it is for and where it comes
/// from, where it stands, then the choice of model, each with its size:
/// choosing one downloads it, and only the chosen one is kept. A file of
/// the user's own works too. A model that is not a pinned one is never
/// kept. Opens from Settings.
class SpeechModelScreen extends StatelessWidget {
  final SpeechModel model;

  const SpeechModelScreen({super.key, required this.model});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.speechModelTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          Text(t.speechModelIntro),
          const SizedBox(height: 8),
          Text(t.speechModelSourceModels),
          const SizedBox(height: 8),
          Text(t.speechModelCreditModels),
          const SizedBox(height: 16),
          ListenableBuilder(
            listenable: model,
            builder: (context, _) => Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Embossed(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: _state(context),
                  ),
                ),
                if (_choosable) ...[
                  const SizedBox(height: 16),
                  Embossed(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: _choice(context),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// Whether a model can be chosen, or is being fetched.
  bool get _choosable => switch (model.status) {
    ModelStatus.missing || ModelStatus.ready || ModelStatus.fetching => true,
    ModelStatus.checking || ModelStatus.unsupported => false,
  };

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
        return Text(t.speechModelMissing, style: text.titleMedium);
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

  /// The models with their sizes and trade-offs; choosing one downloads
  /// it. Nothing to choose while a model is being fetched.
  Widget _choice(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final idle = model.status != ModelStatus.fetching;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(t.speechModelChoice, style: text.titleMedium),
        RadioColumn<WhisperModel?>(
          value: model.chosen,
          options: [
            RadioOption(
              key: const ValueKey('speech-model-base'),
              value: WhisperModel.base,
              title: Text(t.speechModelBase(WhisperModel.base.megabytes)),
              subtitle: Text(t.speechModelBaseDetail),
            ),
            RadioOption(
              key: const ValueKey('speech-model-small'),
              value: WhisperModel.small,
              title: Text(t.speechModelSmall(WhisperModel.small.megabytes)),
              subtitle: Text(t.speechModelSmallDetail),
            ),
          ],
          onChanged: idle
              ? (picked) {
                  if (picked == null) return;
                  fetchSpeechModel(context, () => model.download(picked));
                }
              : null,
        ),
        const SizedBox(height: 8),
        Text(t.speechModelKeepsOne, style: text.bodySmall),
        const SizedBox(height: 16),
        OutlinedButton.icon(
          onPressed: idle ? () => fetchSpeechModel(context, model.pick) : null,
          icon: const Icon(Icons.folder_open),
          label: Text(t.speechModelPick),
        ),
        const SizedBox(height: 8),
        Text(t.speechModelPickEither, style: text.bodySmall),
      ],
    );
  }

  /// [bytes] in megabytes, rounded.
  static String _megabytes(int bytes) => '${(bytes / 1e6).round()}';
}

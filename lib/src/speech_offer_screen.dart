import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'page_bar.dart';
import 'speech_platform.dart';
import 'system_bars.dart';

/// First start, after the Starter Set: offers the speech model for
/// Transcripts. [onDownload] starts the download and setup goes on at
/// once; [onSkip] goes on without it. The speech model page in Settings
/// offers it again, and a file of the user's own. After a restore it
/// offers the model the Backup names, [restored] (obf-1098): that one to
/// download, or, taken from a file before, [onPick] to pick the file
/// again.
class SpeechOfferScreen extends StatefulWidget {
  final SpeechModel model;
  final SpeechModelChoice? restored;
  final VoidCallback onDownload;
  final VoidCallback? onPick;
  final VoidCallback onSkip;

  const SpeechOfferScreen({
    super.key,
    required this.model,
    this.restored,
    required this.onDownload,
    this.onPick,
    required this.onSkip,
  });

  @override
  State<SpeechOfferScreen> createState() => _SpeechOfferScreenState();
}

class _SpeechOfferScreenState extends State<SpeechOfferScreen> {
  /// Whether the Device is on mobile data, asked once.
  var _mobileData = false;

  /// The model offered: the restored Backup's, else the base one.
  late final _offered =
      WhisperModel.values.asNameMap()[widget.restored?.name] ??
      WhisperModel.base;

  /// Whether the restored Backup's model came from a file of the user's.
  late final _fromFile =
      widget.restored?.source == SpeechModelSource.file &&
      widget.onPick != null;

  @override
  void initState() {
    super.initState();
    // Fire and forget: the platform answers false where it fails, and
    // anything else reaches the crash guard.
    unawaited(_askMobileData());
  }

  Future<void> _askMobileData() async {
    final mobileData = await widget.model.platform.onMobileData();
    if (!mounted || !mobileData) return;
    setState(() => _mobileData = true);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final megabytes = _offered.megabytes;
    final restored = widget.restored != null;
    final name = switch (_offered) {
      WhisperModel.base => t.speechModelBase(megabytes),
      WhisperModel.small => t.speechModelSmall(megabytes),
    };
    return Scaffold(
      appBar: pageBar(context, title: t.speechModelTitle),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(24)),
        children: [
          Text(t.speechOfferQuestion, style: text.headlineSmall),
          const SizedBox(height: 8),
          Text(
            !restored
                ? t.speechOfferDetail(megabytes)
                : _fromFile
                ? t.speechOfferRestoredFile(name)
                : t.speechOfferRestored(name),
          ),
          const SizedBox(height: 24),
          if (_mobileData && !_fromFile) ...[
            Row(
              key: const ValueKey('speech-offer-mobile-data'),
              children: [
                const Icon(Icons.signal_cellular_alt),
                const SizedBox(width: 16),
                Expanded(child: Text(t.speechOfferMobileData(megabytes))),
              ],
            ),
            const SizedBox(height: 8),
          ],
          // Still checking for a model in place: the download waits. A
          // restored Backup's model may take the place of another.
          ListenableBuilder(
            listenable: widget.model,
            builder: (context, _) {
              final status = widget.model.status;
              final idle =
                  status == ModelStatus.missing ||
                  (restored && status == ModelStatus.ready);
              return _fromFile
                  ? FilledButton.icon(
                      key: const ValueKey('speech-offer-pick'),
                      onPressed: idle ? widget.onPick : null,
                      icon: const Icon(Icons.folder_open),
                      label: Text(t.speechOfferPickAgain),
                    )
                  : FilledButton.icon(
                      onPressed: idle ? widget.onDownload : null,
                      icon: const Icon(Icons.download),
                      label: Text(t.speechModelDownload(megabytes)),
                    );
            },
          ),
          const SizedBox(height: 8),
          OutlinedButton(onPressed: widget.onSkip, child: Text(t.notNow)),
        ],
      ),
    );
  }
}
